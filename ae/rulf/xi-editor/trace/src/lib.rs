// Copyright 2018 The xi-editor Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
#![cfg_attr(feature = "benchmarks", feature(test))]
#![allow(clippy::identity_op, clippy::new_without_default, clippy::trivially_copy_pass_by_ref)]

#[macro_use]
extern crate lazy_static;
extern crate time;

#[macro_use]
extern crate serde_derive;

extern crate serde;

#[macro_use]
extern crate log;

extern crate libc;

#[cfg(feature = "benchmarks")]
extern crate test;

#[cfg(any(test, feature = "json_payload", feature = "chroma_trace_dump"))]
#[cfg_attr(any(test), macro_use)]
extern crate serde_json;

mod fixed_lifo_deque;
mod sys_pid;
mod sys_tid;

#[cfg(feature = "chrome_trace_event")]
pub mod chrome_trace_dump;

use crate::fixed_lifo_deque::FixedLifoDeque;
use std::borrow::Cow;
use std::cmp;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::hash::{Hash, Hasher};
use std::mem::size_of;
use std::path::Path;
use std::string::ToString;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::Mutex;

pub type StrCow = Cow<'static, str>;

#[derive(Clone, Debug)]
pub enum CategoriesT {
    StaticArray(&'static [&'static str]),
    DynamicArray(Vec<String>),
}

trait StringArrayEq<Rhs: ?Sized = Self> {
    fn arr_eq(&self, other: &Rhs) -> bool;
}

impl StringArrayEq<[&'static str]> for Vec<String> {
    fn arr_eq(&self, other: &[&'static str]) -> bool {
        if self.len() != other.len() {
            return false;
        }

        for i in 0..self.len() {
            if self[i] != other[i] {
                return false;
            }
        }
        true
    }
}

impl StringArrayEq<Vec<String>> for &'static [&'static str] {
    fn arr_eq(&self, other: &Vec<String>) -> bool {
        if self.len() != other.len() {
            return false;
        }
        for i in 0..self.len() {
            if self[i] != other[i] {
                return false;
            }
        }
        true
    }
}

impl PartialEq for CategoriesT {
    fn eq(&self, other: &CategoriesT) -> bool {
        match *self {
            CategoriesT::StaticArray(ref self_arr) => match *other {
                CategoriesT::StaticArray(ref other_arr) => self_arr.eq(other_arr),
                CategoriesT::DynamicArray(ref other_arr) => self_arr.arr_eq(other_arr),
            },
            CategoriesT::DynamicArray(ref self_arr) => match *other {
                CategoriesT::StaticArray(ref other_arr) => self_arr.arr_eq(other_arr),
                CategoriesT::DynamicArray(ref other_arr) => self_arr.eq(other_arr),
            },
        }
    }
}

impl Eq for CategoriesT {}

impl serde::Serialize for CategoriesT {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.join(",").serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for CategoriesT {
    fn deserialize<D>(deserializer: D) -> Result<CategoriesT, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Visitor;
        struct CategoriesTVisitor;

        impl<'de> Visitor<'de> for CategoriesTVisitor {
            type Value = CategoriesT;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("comma-separated strings")
            }

            fn visit_str<E>(self, v: &str) -> Result<CategoriesT, E>
            where
                E: serde::de::Error,
            {
                let categories = v.split(',').map(ToString::to_string).collect();
                Ok(CategoriesT::DynamicArray(categories))
            }
        }

        deserializer.deserialize_str(CategoriesTVisitor)
    }
}

impl CategoriesT {
    pub fn join(&self, sep: &str) -> String {
        match *self {
            CategoriesT::StaticArray(ref arr) => arr.join(sep),
            CategoriesT::DynamicArray(ref vec) => vec.join(sep),
        }
    }
}

macro_rules! categories_from_constant_array {
    ($num_args: expr) => {
        impl From<&'static [&'static str; $num_args]> for CategoriesT {
            fn from(c: &'static [&'static str; $num_args]) -> CategoriesT {
                CategoriesT::StaticArray(c)
            }
        }
    };
}

categories_from_constant_array!(0);
categories_from_constant_array!(1);
categories_from_constant_array!(2);
categories_from_constant_array!(3);
categories_from_constant_array!(4);
categories_from_constant_array!(5);
categories_from_constant_array!(6);
categories_from_constant_array!(7);
categories_from_constant_array!(8);
categories_from_constant_array!(9);
categories_from_constant_array!(10);

impl From<Vec<String>> for CategoriesT {
    fn from(c: Vec<String>) -> CategoriesT {
        CategoriesT::DynamicArray(c)
    }
}

#[cfg(not(feature = "json_payload"))]
pub type TracePayloadT = StrCow;

#[cfg(feature = "json_payload")]
pub type TracePayloadT = serde_json::Value;

/// How tracing should be configured.
#[derive(Copy, Clone)]
pub struct Config {
    sample_limit_count: usize,
}

impl Config {
    /// The maximum number of bytes the tracing data should take up.  This limit
    /// won't be exceeded by the underlying storage itself (i.e. rounds down).
    pub fn with_limit_bytes(size: usize) -> Self {
        Self::with_limit_count(size / size_of::<Sample>())
    }

    /// The maximum number of entries the tracing data should allow.  Total
    /// storage allocated will be limit * size_of<Sample>
    pub fn with_limit_count(limit: usize) -> Self {
        Self { sample_limit_count: limit }
    }

    /// The default amount of storage to allocate for tracing.  Currently 1 MB.
    pub fn default() -> Self {
        // 1 MB
        Self::with_limit_bytes(1 * 1024 * 1024)
    }

    /// The maximum amount of space the tracing data will take up.  This does
    /// not account for any overhead of storing the data itself (i.e. pointer to
    /// the heap, counters, etc); just the data itself.
    pub fn max_size_in_bytes(self) -> usize {
        self.sample_limit_count * size_of::<Sample>()
    }

    /// The maximum number of samples that should be stored.
    pub fn max_samples(self) -> usize {
        self.sample_limit_count
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleEventType {
    DurationBegin,
    DurationEnd,
    CompleteDuration,
    Instant,
    AsyncStart,
    AsyncInstant,
    AsyncEnd,
    FlowStart,
    FlowInstant,
    FlowEnd,
    ObjectCreated,
    ObjectSnapshot,
    ObjectDestroyed,
    Metadata,
}

impl SampleEventType {
    // TODO(vlovich): Replace all of this with serde flatten + rename once
    // https://github.com/serde-rs/serde/issues/1189 is fixed.
    #[inline]
    fn into_chrome_id(self) -> char {
        match self {
            SampleEventType::DurationBegin => 'B',
            SampleEventType::DurationEnd => 'E',
            SampleEventType::CompleteDuration => 'X',
            SampleEventType::Instant => 'i',
            SampleEventType::AsyncStart => 'b',
            SampleEventType::AsyncInstant => 'n',
            SampleEventType::AsyncEnd => 'e',
            SampleEventType::FlowStart => 's',
            SampleEventType::FlowInstant => 't',
            SampleEventType::FlowEnd => 'f',
            SampleEventType::ObjectCreated => 'N',
            SampleEventType::ObjectSnapshot => 'O',
            SampleEventType::ObjectDestroyed => 'D',
            SampleEventType::Metadata => 'M',
        }
    }

    #[inline]
    fn from_chrome_id(symbol: char) -> Self {
        match symbol {
            'B' => SampleEventType::DurationBegin,
            'E' => SampleEventType::DurationEnd,
            'X' => SampleEventType::CompleteDuration,
            'i' => SampleEventType::Instant,
            'b' => SampleEventType::AsyncStart,
            'n' => SampleEventType::AsyncInstant,
            'e' => SampleEventType::AsyncEnd,
            's' => SampleEventType::FlowStart,
            't' => SampleEventType::FlowInstant,
            'f' => SampleEventType::FlowEnd,
            'N' => SampleEventType::ObjectCreated,
            'O' => SampleEventType::ObjectSnapshot,
            'D' => SampleEventType::ObjectDestroyed,
            'M' => SampleEventType::Metadata,
            _ => panic!("Unexpected chrome sample type '{}'", symbol),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum MetadataType {
    ProcessName {
        name: String,
    },
    #[allow(dead_code)]
    ProcessLabels {
        labels: String,
    },
    #[allow(dead_code)]
    ProcessSortIndex {
        sort_index: i32,
    },
    ThreadName {
        name: String,
    },
    #[allow(dead_code)]
    ThreadSortIndex {
        sort_index: i32,
    },
}

impl MetadataType {
    fn sample_name(&self) -> &'static str {
        match *self {
            MetadataType::ProcessName { .. } => "process_name",
            MetadataType::ProcessLabels { .. } => "process_labels",
            MetadataType::ProcessSortIndex { .. } => "process_sort_index",
            MetadataType::ThreadName { .. } => "thread_name",
            MetadataType::ThreadSortIndex { .. } => "thread_sort_index",
        }
    }

    fn consume(self) -> (Option<String>, Option<i32>) {
        match self {
            MetadataType::ProcessName { name } => (Some(name), None),
            MetadataType::ThreadName { name } => (Some(name), None),
            MetadataType::ProcessSortIndex { sort_index } => (None, Some(sort_index)),
            MetadataType::ThreadSortIndex { sort_index } => (None, Some(sort_index)),
            MetadataType::ProcessLabels { .. } => (None, None),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SampleArgs {
    /// An arbitrary payload to associate with the sample.  The type is
    /// controlled by features (default string).
    #[serde(rename = "xi_payload")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<TracePayloadT>,

    /// The name to associate with the pid/tid.  Whether it's associated with
    /// the pid or the tid depends on the name of the event
    /// via process_name/thread_name respectively.
    #[serde(rename = "name")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_name: Option<StrCow>,

    /// Sorting priority between processes/threads in the view.
    #[serde(rename = "sort_index")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_sort_index: Option<i32>,
}

#[inline]
fn ns_to_us(ns: u64) -> u64 {
    ns / 1000
}

//NOTE: serde requires this to take a reference
fn serialize_event_type<S>(ph: &SampleEventType, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_char(ph.into_chrome_id())
}

fn deserialize_event_type<'de, D>(d: D) -> Result<SampleEventType, D::Error>
where
    D: serde::Deserializer<'de>,
{
    serde::Deserialize::deserialize(d).map(SampleEventType::from_chrome_id)
}

/// Stores the relevant data about a sample for later serialization.
/// The payload associated with any sample is by default a string but may be
/// configured via the `json_payload` feature (there is an
/// associated performance hit across the board for turning it on).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Sample {
    /// The name of the event to be shown.
    pub name: StrCow,
    /// List of categories the event applies to.
    #[serde(rename = "cat")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub categories: Option<CategoriesT>,
    /// When was the sample started.
    #[serde(rename = "ts")]
    pub timestamp_us: u64,
    /// What kind of sample this is.
    #[serde(rename = "ph")]
    #[serde(serialize_with = "serialize_event_type")]
    #[serde(deserialize_with = "deserialize_event_type")]
    pub event_type: SampleEventType,
    #[serde(rename = "dur")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_us: Option<u64>,
    /// The process the sample was captured in.
    pub pid: u64,
    /// The thread the sample was captured on.  Omitted for Metadata events that
    /// want to set the process name (if provided then sets the thread name).
    pub tid: u64,
    #[serde(skip_serializing)]
    pub thread_name: Option<StrCow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<SampleArgs>,
}

fn to_cow_str<S>(s: S) -> StrCow
where
    S: Into<StrCow>,
{
    s.into()
}

impl Sample {
    fn thread_name() -> Option<StrCow> {
        let thread = std::thread::current();
        thread.name().map(|ref s| to_cow_str((*s).to_string()))
    }

    /// Constructs a Begin or End sample.  Should not be used directly.  Instead
    /// should be constructed via SampleGuard.
    pub fn new_duration_marker<S, C>(
        name: S,
        categories: C,
        payload: Option<TracePayloadT>,
        event_type: SampleEventType,
    ) -> Self
    where
        S: Into<StrCow>,
        C: Into<CategoriesT>,
    {
        Self {
            name: name.into(),
            categories: Some(categories.into()),
            timestamp_us: ns_to_us(time::precise_time_ns()),
            event_type,
            duration_us: None,
            tid: sys_tid::current_tid().unwrap(),
            thread_name: Sample::thread_name(),
            pid: sys_pid::current_pid(),
            args: Some(SampleArgs { payload, metadata_name: None, metadata_sort_index: None }),
        }
    }

    /// Constructs a Duration sample.  For use via xi_trace::closure.
    pub fn new_duration<S, C>(
        name: S,
        categories: C,
        payload: Option<TracePayloadT>,
        start_ns: u64,
        duration_ns: u64,
    ) -> Self
    where
        S: Into<StrCow>,
        C: Into<CategoriesT>,
    {
        Self {
            name: name.into(),
            categories: Some(categories.into()),
            timestamp_us: ns_to_us(start_ns),
            event_type: SampleEventType::CompleteDuration,
            duration_us: Some(ns_to_us(duration_ns)),
            tid: sys_tid::current_tid().unwrap(),
            thread_name: Sample::thread_name(),
            pid: sys_pid::current_pid(),
            args: Some(SampleArgs { payload, metadata_name: None, metadata_sort_index: None }),
        }
    }

    /// Constructs an instantaneous sample.
    pub fn new_instant<S, C>(name: S, categories: C, payload: Option<TracePayloadT>) -> Self
    where
        S: Into<StrCow>,
        C: Into<CategoriesT>,
    {
        Self {
            name: name.into(),
            categories: Some(categories.into()),
            timestamp_us: ns_to_us(time::precise_time_ns()),
            event_type: SampleEventType::Instant,
            duration_us: None,
            tid: sys_tid::current_tid().unwrap(),
            thread_name: Sample::thread_name(),
            pid: sys_pid::current_pid(),
            args: Some(SampleArgs { payload, metadata_name: None, metadata_sort_index: None }),
        }
    }

    fn new_metadata(timestamp_ns: u64, meta: MetadataType, tid: u64) -> Self {
        let sample_name = to_cow_str(meta.sample_name());
        let (metadata_name, sort_index) = meta.consume();

        Self {
            name: sample_name,
            categories: None,
            timestamp_us: ns_to_us(timestamp_ns),
            event_type: SampleEventType::Metadata,
            duration_us: None,
            tid,
            thread_name: None,
            pid: sys_pid::current_pid(),
            args: Some(SampleArgs {
                payload: None,
                metadata_name: metadata_name.map(Cow::Owned),
                metadata_sort_index: sort_index,
            }),
        }
    }
}

impl PartialEq for Sample {
    fn eq(&self, other: &Sample) -> bool {
        self.timestamp_us == other.timestamp_us
            && self.name == other.name
            && self.categories == other.categories
            && self.pid == other.pid
            && self.tid == other.tid
            && self.event_type == other.event_type
            && self.args == other.args
    }
}

impl Eq for Sample {}

impl PartialOrd for Sample {
    fn partial_cmp(&self, other: &Sample) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Sample {
    fn cmp(&self, other: &Sample) -> cmp::Ordering {
        self.timestamp_us.cmp(&other.timestamp_us)
    }
}

impl Hash for Sample {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.pid, self.timestamp_us).hash(state);
    }
}

#[must_use]
pub struct SampleGuard<'a> {
    sample: Option<Sample>,
    trace: Option<&'a Trace>,
}

impl<'a> SampleGuard<'a> {
    #[inline]
    pub fn new_disabled() -> Self {
        Self { sample: None, trace: None }
    }

    #[inline]
    fn new<S, C>(trace: &'a Trace, name: S, categories: C, payload: Option<TracePayloadT>) -> Self
    where
        S: Into<StrCow>,
        C: Into<CategoriesT>,
    {
        // TODO(vlovich): optimize this path to use the Complete event type
        // rather than emitting an explicit start/stop to reduce the size of
        // the generated JSON.
        let guard = Self {
            sample: Some(Sample::new_duration_marker(
                name,
                categories,
                payload,
                SampleEventType::DurationBegin,
            )),
            trace: Some(&trace),
        };
        trace.record(guard.sample.as_ref().unwrap().clone());
        guard
    }
}

impl<'a> Drop for SampleGuard<'a> {
    fn drop(&mut self) {
        if let Some(ref mut trace) = self.trace {
            let mut sample = self.sample.take().unwrap();
            sample.timestamp_us = ns_to_us(time::precise_time_ns());
            sample.event_type = SampleEventType::DurationEnd;
            trace.record(sample);
        }
    }
}

/// Returns the file name of the EXE if possible, otherwise the full path, or
/// None if an irrecoverable error occured.
fn exe_name() -> Option<String> {
    match std::env::current_exe() {
        Ok(exe_name) => match exe_name.file_name() {
            Some(filename) => filename.to_str().map(ToString::to_string),
            None => {
                let full_path = exe_name.into_os_string();
                let full_path_str = full_path.into_string();
                match full_path_str {
                    Ok(s) => Some(s),
                    Err(e) => {
                        warn!("Failed to get string representation: {:?}", e);
                        None
                    }
                }
            }
        },
        Err(ref e) => {
            warn!("Failed to get path to current exe: {:?}", e);
            None
        }
    }
}

/// Stores the tracing data.
pub struct Trace {
    enabled: AtomicBool,
    samples: Mutex<FixedLifoDeque<Sample>>,
}

impl Trace {
    pub fn disabled() -> Self {
        Self { enabled: AtomicBool::new(false), samples: Mutex::new(FixedLifoDeque::new()) }
    }

    pub fn enabled(config: Config) -> Self {
        Self {
            enabled: AtomicBool::new(true),
            samples: Mutex::new(FixedLifoDeque::with_limit(config.max_samples())),
        }
    }

    pub fn disable(&self) {
        let mut all_samples = self.samples.lock().unwrap();
        all_samples.reset_limit(0);
        self.enabled.store(false, AtomicOrdering::Relaxed);
    }

    #[inline]
    pub fn enable(&self) {
        self.enable_config(Config::default());
    }

    pub fn enable_config(&self, config: Config) {
        let mut all_samples = self.samples.lock().unwrap();
        all_samples.reset_limit(config.max_samples());
        self.enabled.store(true, AtomicOrdering::Relaxed);
    }

    /// Generally racy since the underlying storage might be mutated in a separate thread.
    /// Exposed for unit tests.
    pub fn get_samples_count(&self) -> usize {
        self.samples.lock().unwrap().len()
    }

    /// Exposed for unit tests only.
    pub fn get_samples_limit(&self) -> usize {
        self.samples.lock().unwrap().limit()
    }

    #[inline]
    pub(crate) fn record(&self, sample: Sample) {
        let mut all_samples = self.samples.lock().unwrap();
        all_samples.push_back(sample);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(AtomicOrdering::Relaxed)
    }

    pub fn instant<S, C>(&self, name: S, categories: C)
    where
        S: Into<StrCow>,
        C: Into<CategoriesT>,
    {
        if self.is_enabled() {
            self.record(Sample::new_instant(name, categories, None));
        }
    }

    pub fn instant_payload<S, C, P>(&self, name: S, categories: C, payload: P)
    where
        S: Into<StrCow>,
        C: Into<CategoriesT>,
        P: Into<TracePayloadT>,
    {
        if self.is_enabled() {
            self.record(Sample::new_instant(name, categories, Some(payload.into())));
        }
    }

    pub fn block<S, C>(&self, name: S, categories: C) -> SampleGuard
    where
        S: Into<StrCow>,
        C: Into<CategoriesT>,
    {
        if !self.is_enabled() {
            SampleGuard::new_disabled()
        } else {
            SampleGuard::new(&self, name, categories, None)
        }
    }

    pub fn block_payload<S, C, P>(&self, name: S, categories: C, payload: P) -> SampleGuard
    where
        S: Into<StrCow>,
        C: Into<CategoriesT>,
        P: Into<TracePayloadT>,
    {
        if !self.is_enabled() {
            SampleGuard::new_disabled()
        } else {
            SampleGuard::new(&self, name, categories, Some(payload.into()))
        }
    }

    pub fn closure<S, C, F, R>(&self, name: S, categories: C, closure: F) -> R
    where
        S: Into<StrCow>,
        C: Into<CategoriesT>,
        F: FnOnce() -> R,
    {
        // TODO: simplify this through the use of scopeguard crate
        let start = time::precise_time_ns();
        let result = closure();
        let end = time::precise_time_ns();
        if self.is_enabled() {
            self.record(Sample::new_duration(name, categories, None, start, end - start));
        }
        result
    }

    pub fn closure_payload<S, C, P, F, R>(
        &self,
        name: S,
        categories: C,
        closure: F,
        payload: P,
    ) -> R
    where
        S: Into<StrCow>,
        C: Into<CategoriesT>,
        P: Into<TracePayloadT>,
        F: FnOnce() -> R,
    {
        // TODO: simplify this through the use of scopeguard crate
        let start = time::precise_time_ns();
        let result = closure();
        let end = time::precise_time_ns();
        if self.is_enabled() {
            self.record(Sample::new_duration(
                name,
                categories,
                Some(payload.into()),
                start,
                end - start,
            ));
        }
        result
    }

    pub fn samples_cloned_unsorted(&self) -> Vec<Sample> {
        let all_samples = self.samples.lock().unwrap();
        if all_samples.is_empty() {
            return Vec::with_capacity(0);
        }

        let mut as_vec = Vec::with_capacity(all_samples.len() + 10);
        let first_sample_timestamp = all_samples.front().map_or(0, |ref s| s.timestamp_us);
        let tid =
            all_samples.front().map_or_else(|| sys_tid::current_tid().unwrap(), |ref s| s.tid);

        if let Some(exe_name) = exe_name() {
            as_vec.push(Sample::new_metadata(
                first_sample_timestamp,
                MetadataType::ProcessName { name: exe_name },
                tid,
            ));
        }

        let mut thread_names: HashMap<u64, StrCow> = HashMap::new();

        for sample in all_samples.iter() {
            if let Some(ref thread_name) = sample.thread_name {
                let previous_name = thread_names.insert(sample.tid, thread_name.clone());
                if previous_name.is_none() || previous_name.unwrap() != *thread_name {
                    as_vec.push(Sample::new_metadata(
                        first_sample_timestamp,
                        MetadataType::ThreadName { name: thread_name.to_string() },
                        sample.tid,
                    ));
                }
            }
        }

        as_vec.extend(all_samples.iter().cloned());
        as_vec
    }

    #[inline]
    pub fn samples_cloned_sorted(&self) -> Vec<Sample> {
        let mut samples = self.samples_cloned_unsorted();
        samples.sort_unstable();
        samples
    }

    pub fn save<P: AsRef<Path>>(
        &self,
        path: P,
        sort: bool,
    ) -> Result<(), chrome_trace_dump::Error> {
        let traces = if sort { samples_cloned_sorted() } else { samples_cloned_unsorted() };
        let path: &Path = path.as_ref();

        if path.exists() {
            return Err(chrome_trace_dump::Error::already_exists());
        }

        let mut trace_file = fs::File::create(&path)?;

        chrome_trace_dump::serialize(&traces, &mut trace_file)
    }
}

lazy_static! {
    static ref TRACE: Trace = Trace::disabled();
}

/// Enable tracing with the default configuration.  See Config::default.
/// Tracing is disabled initially on program launch.
#[inline]
pub fn enable_tracing() {
    TRACE.enable();
}

/// Enable tracing with a specific configuration. Tracing is disabled initially
/// on program launch.
#[inline]
pub fn enable_tracing_with_config(config: Config) {
    TRACE.enable_config(config);
}

/// Disable tracing.  This clears all trace data (& frees the memory).
#[inline]
pub fn disable_tracing() {
    TRACE.disable();
}

/// Is tracing enabled.  Technically doesn't guarantee any samples will be
/// stored as tracing could still be enabled but set with a limit of 0.
#[inline]
pub fn is_enabled() -> bool {
    TRACE.is_enabled()
}

/// Create an instantaneous sample without any payload.  This is the lowest
/// overhead tracing routine available.
///
/// # Performance
/// The `json_payload` feature makes this ~1.3-~1.5x slower.
/// See `trace_payload` for a more complete discussion.
///
/// # Arguments
///
/// * `name` - A string that provides some meaningful name to this sample.
/// Usage of static strings is encouraged for best performance to avoid copies.
/// However, anything that can be converted into a Cow string can be passed as
/// an argument.
///
/// * `categories` - A static array of static strings that tags the samples in
/// some way.
///
/// # Examples
///
/// ```
/// xi_trace::trace("something happened", &["rpc", "response"]);
/// ```
#[inline]
pub fn trace<S, C>(name: S, categories: C)
where
    S: Into<StrCow>,
    C: Into<CategoriesT>,
{
    TRACE.instant(name, categories);
}

/// Create an instantaneous sample with a payload.  The type the payload
/// conforms to is currently determined by the feature this library is compiled
/// with.  By default, the type is string-like just like name.  If compiled with
/// the `json_payload` then a `serde_json::Value` is expected and  the library
/// acquires a dependency on the `serde_json` crate.
///
/// # Performance
/// A static string has the lowest overhead as no copies are necessary, roughly
/// equivalent performance to a regular trace.  A string that needs to be copied
/// first can make it ~1.7x slower than a regular trace.
///
/// When compiling with `json_payload`, this is ~2.1x slower than a string that
/// needs to be copied (or ~4.5x slower than a static string)
///
/// # Arguments
///
/// * `name` - A string that provides some meaningful name to this sample.
/// Usage of static strings is encouraged for best performance to avoid copies.
/// However, anything that can be converted into a Cow string can be passed as
/// an argument.
///
/// * `categories` - A static array of static strings that tags the samples in
/// some way.
///
/// # Examples
///
/// ```
/// xi_trace::trace_payload("something happened", &["rpc", "response"], "a note about this");
/// ```
///
/// With `json_payload` feature:
///
/// ```rust,ignore
/// xi_trace::trace_payload("my event", &["rpc", "response"], json!({"key": "value"}));
/// ```
#[inline]
pub fn trace_payload<S, C, P>(name: S, categories: C, payload: P)
where
    S: Into<StrCow>,
    C: Into<CategoriesT>,
    P: Into<TracePayloadT>,
{
    TRACE.instant_payload(name, categories, payload);
}

/// Creates a duration sample.  The sample is finalized (end_ns set) when the
/// returned value is dropped.  `trace_closure` may be prettier to read.
///
/// # Performance
/// See `trace_payload` for a more complete discussion.
///
/// # Arguments
///
/// * `name` - A string that provides some meaningful name to this sample.
/// Usage of static strings is encouraged for best performance to avoid copies.
/// However, anything that can be converted into a Cow string can be passed as
/// an argument.
///
/// * `categories` - A static array of static strings that tags the samples in
/// some way.
///
/// # Returns
/// A guard that when dropped will update the Sample with the timestamp & then
/// record it.
///
/// # Examples
///
/// ```
/// fn something_expensive() {
/// }
///
/// fn something_else_expensive() {
/// }
///
/// let trace_guard = xi_trace::trace_block("something_expensive", &["rpc", "request"]);
/// something_expensive();
/// std::mem::drop(trace_guard); // finalize explicitly if
///
/// {
///     let _guard = xi_trace::trace_block("something_else_expensive", &["rpc", "response"]);
///     something_else_expensive();
/// }
/// ```
#[inline]
pub fn trace_block<'a, S, C>(name: S, categories: C) -> SampleGuard<'a>
where
    S: Into<StrCow>,
    C: Into<CategoriesT>,
{
    TRACE.block(name, categories)
}

/// See `trace_block` for how the block works and `trace_payload` for a
/// discussion on payload.
#[inline]
pub fn trace_block_payload<'a, S, C, P>(name: S, categories: C, payload: P) -> SampleGuard<'a>
where
    S: Into<StrCow>,
    C: Into<CategoriesT>,
    P: Into<TracePayloadT>,
{
    TRACE.block_payload(name, categories, payload)
}

/// Creates a duration sample that measures how long the closure took to execute.
///
/// # Performance
/// See `trace_payload` for a more complete discussion.
///
/// # Arguments
///
/// * `name` - A string that provides some meaningful name to this sample.
/// Usage of static strings is encouraged for best performance to avoid copies.
/// However, anything that can be converted into a Cow string can be passed as
/// an argument.
///
/// * `categories` - A static array of static strings that tags the samples in
/// some way.
///
/// # Returns
/// The result of the closure.
///
/// # Examples
///
/// ```
/// fn something_expensive() -> u32 {
///     0
/// }
///
/// fn something_else_expensive(value: u32) {
/// }
///
/// let result = xi_trace::trace_closure("something_expensive", &["rpc", "request"], || {
///     something_expensive()
/// });
/// xi_trace::trace_closure("something_else_expensive", &["rpc", "response"], || {
///     something_else_expensive(result);
/// });
/// ```
#[inline]
pub fn trace_closure<S, C, F, R>(name: S, categories: C, closure: F) -> R
where
    S: Into<StrCow>,
    C: Into<CategoriesT>,
    F: FnOnce() -> R,
{
    TRACE.closure(name, categories, closure)
}

/// See `trace_closure` for how the closure works and `trace_payload` for a
/// discussion on payload.
#[inline]
pub fn trace_closure_payload<S, C, P, F, R>(name: S, categories: C, closure: F, payload: P) -> R
where
    S: Into<StrCow>,
    C: Into<CategoriesT>,
    P: Into<TracePayloadT>,
    F: FnOnce() -> R,
{
    TRACE.closure_payload(name, categories, closure, payload)
}

#[inline]
pub fn samples_len() -> usize {
    TRACE.get_samples_count()
}

/// Returns all the samples collected so far.  There is no guarantee that the
/// samples are ordered chronologically for several reasons:
///
/// 1. Samples that span sections of code may be inserted on end instead of
/// beginning.
/// 2. Performance optimizations might have per-thread buffers.  Keeping all
/// that sorted would be prohibitively expensive.
/// 3. You may not care about them always being sorted if you're merging samples
/// from multiple distributed sources (i.e. you want to sort the merged result
/// rather than just this processe's samples).
#[inline]
pub fn samples_cloned_unsorted() -> Vec<Sample> {
    TRACE.samples_cloned_unsorted()
}

/// Returns all the samples collected so far ordered chronologically by
/// creation.  Roughly corresponds to start_ns but instead there's a
/// monotonically increasing single global integer (when tracing) per creation
/// of Sample that determines order.
#[inline]
pub fn samples_cloned_sorted() -> Vec<Sample> {
    TRACE.samples_cloned_sorted()
}

/// Save tracing data to to supplied path, using the Trace Viewer format. Trace file can be opened
/// using the Chrome browser by visiting the URL `about:tracing`. If `sorted_chronologically` is
/// true then sort output traces chronologically by each trace's time of creation.
#[inline]
pub fn save<P: AsRef<Path>>(path: P, sort: bool) -> Result<(), chrome_trace_dump::Error> {
    TRACE.save(path, sort)
}

#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use super::*;
    #[cfg(feature = "benchmarks")]
    use test::Bencher;
    #[cfg(feature = "benchmarks")]
    use test::black_box;

    #[cfg(not(feature = "json_payload"))]
    fn to_payload(value: &'static str) -> &'static str {
        value
    }

    #[cfg(feature = "json_payload")]
    fn to_payload(value: &'static str) -> TracePayloadT {
        json!({"test": value})
    }

    #[test]
    fn test_samples_pulse() {
        let trace = Trace::enabled(Config::with_limit_count(10));
        for _i in 0..50 {
            trace.instant("test_samples_pulse", &["test"]);
        }
    }

    #[test]
    fn test_samples_block() {
        let trace = Trace::enabled(Config::with_limit_count(10));
        for _i in 0..50 {
            let _ = trace.block("test_samples_block", &["test"]);
        }
    }

    #[test]
    fn test_samples_closure() {
        let trace = Trace::enabled(Config::with_limit_count(10));
        for _i in 0..50 {
            trace.closure("test_samples_closure", &["test"], || {});
        }
    }

    #[test]
    fn test_disable_drops_all_samples() {
        let trace = Trace::enabled(Config::with_limit_count(10));
        assert_eq!(trace.is_enabled(), true);
        trace.instant("1", &["test"]);
        trace.instant("2", &["test"]);
        trace.instant("3", &["test"]);
        trace.instant("4", &["test"]);
        trace.instant("5", &["test"]);
        assert_eq!(trace.get_samples_count(), 5);
        // 1 for exe name & 1 for the thread name
        assert_eq!(trace.samples_cloned_unsorted().len(), 7);
        trace.disable();
        assert_eq!(trace.get_samples_count(), 0);
    }

    #[test]
    fn test_get_samples() {
        let trace = Trace::enabled(Config::with_limit_count(20));
        assert_eq!(trace.samples_cloned_unsorted().len(), 0);

        assert_eq!(trace.is_enabled(), true);
        assert_eq!(trace.get_samples_limit(), 20);
        assert_eq!(trace.samples_cloned_unsorted().len(), 0);

        trace.closure_payload("x", &["test"], || (),
                              to_payload("test_get_samples"));
        assert_eq!(trace.get_samples_count(), 1);
        // +2 for exe & thread name.
        assert_eq!(trace.samples_cloned_unsorted().len(), 3);

        trace.closure_payload("y", &["test"], || {},
                              to_payload("test_get_samples"));
        assert_eq!(trace.samples_cloned_unsorted().len(), 4);

        trace.closure_payload("z", &["test"], || {},
                              to_payload("test_get_samples"));

        let snapshot = trace.samples_cloned_unsorted();
        assert_eq!(snapshot.len(), 5);

        assert_eq!(snapshot[0].name, "process_name");
        assert_eq!(snapshot[0].args.as_ref().unwrap().metadata_name.as_ref().is_some(), true);
        assert_eq!(snapshot[1].name, "thread_name");
        assert_eq!(snapshot[1].args.as_ref().unwrap().metadata_name.as_ref().is_some(), true);
        assert_eq!(snapshot[2].name, "x");
        assert_eq!(snapshot[3].name, "y");
        assert_eq!(snapshot[4].name, "z");
    }

    #[test]
    fn test_trace_disabled() {
        let trace = Trace::disabled();
        assert_eq!(trace.get_samples_limit(), 0);
        assert_eq!(trace.get_samples_count(), 0);

        {
            trace.instant("something", &[]);
            let _x = trace.block("something", &[]);
            trace.closure("something", &[], || ());
        }

        assert_eq!(trace.get_samples_count(), 0);
    }

    #[test]
    fn test_get_samples_nested_trace() {
        let trace = Trace::enabled(Config::with_limit_count(11));
        assert_eq!(trace.is_enabled(), true);
        assert_eq!(trace.get_samples_limit(), 11);

        // current recording mechanism should see:
        // a, b, y, z, c, x
        // even though the actual sampling order (from timestamp of
        // creation) is:
        // x, a, y, b, z, c
        // This might be an over-specified test as it will
        // probably change as the recording internals change.
        trace.closure_payload("x", &["test"], || {
            trace.instant_payload("a", &["test"], to_payload("test_get_samples_nested_trace"));
            trace.closure_payload("y", &["test"], || {
                trace.instant_payload("b", &["test"], to_payload("test_get_samples_nested_trace"));
            }, to_payload("test_get_samples_nested_trace"));
            let _ = trace.block_payload("z", &["test"], to_payload("test_get_samples_nested_trace"));
            trace.instant_payload("c", &["test"], to_payload("test_get_samples_nested_trace"));
        }, to_payload("test_get_samples_nested_trace"));

        let snapshot = trace.samples_cloned_unsorted();
        // +2 for exe & thread name
        assert_eq!(snapshot.len(), 9);

        assert_eq!(snapshot[0].name, "process_name");
        assert_eq!(snapshot[0].args.as_ref().unwrap().metadata_name.as_ref().is_some(), true);
        assert_eq!(snapshot[1].name, "thread_name");
        assert_eq!(snapshot[1].args.as_ref().unwrap().metadata_name.as_ref().is_some(), true);
        assert_eq!(snapshot[2].name, "a");
        assert_eq!(snapshot[3].name, "b");
        assert_eq!(snapshot[4].name, "y");
        assert_eq!(snapshot[5].name, "z");
        assert_eq!(snapshot[6].name, "z");
        assert_eq!(snapshot[7].name, "c");
        assert_eq!(snapshot[8].name, "x");
    }

    #[test]
    fn test_get_sorted_samples() {
        let trace = Trace::enabled(Config::with_limit_count(10));

        // current recording mechanism should see:
        // a, b, y, z, c, x
        // even though the actual sampling order (from timestamp of
        // creation) is:
        // x, a, y, b, z, c
        // This might be an over-specified test as it will
        // probably change as the recording internals change.

        // NOTE: 1 us sleeps are inserted as the first line of a closure to
        // ensure that when the samples are sorted by time they come out in a
        // stable order since the resolution of timestamps is 1us.
        // NOTE 2: from_micros is currently in unstable so using new
        trace.closure_payload("x", &["test"], || {
            std::thread::sleep(std::time::Duration::new(0, 1000));
            trace.instant_payload("a", &["test"], to_payload("test_get_sorted_samples"));
            trace.closure_payload("y", &["test"], || {
                std::thread::sleep(std::time::Duration::new(0, 1000));
                trace.instant_payload("b", &["test"], to_payload("test_get_sorted_samples"));
            }, to_payload("test_get_sorted_samples"));
            let _ = trace.block_payload("z", &["test"], to_payload("test_get_sorted_samples"));
            trace.instant("c", &["test"]);
        }, to_payload("test_get_sorted_samples"));

        let snapshot = trace.samples_cloned_sorted();
        // +2 for exe & thread name.
        assert_eq!(snapshot.len(), 9);

        assert_eq!(snapshot[0].name, "process_name");
        assert_eq!(snapshot[0].args.as_ref().unwrap().metadata_name.as_ref().is_some(), true);
        assert_eq!(snapshot[1].name, "thread_name");
        assert_eq!(snapshot[1].args.as_ref().unwrap().metadata_name.as_ref().is_some(), true);
        assert_eq!(snapshot[2].name, "x");
        assert_eq!(snapshot[3].name, "a");
        assert_eq!(snapshot[4].name, "y");
        assert_eq!(snapshot[5].name, "b");
        assert_eq!(snapshot[6].name, "z");
        assert_eq!(snapshot[7].name, "z");
        assert_eq!(snapshot[8].name, "c");
    }

    #[test]
    fn test_cross_process_samples() {
        let mut samples = vec![
            Sample::new_instant("local pid", &[], None),
            Sample::new_instant("remote pid", &[], None)];
        samples[0].pid = 1;
        samples[0].timestamp_us = 10;

        samples[1].pid = 2;
        samples[1].timestamp_us = 5;

        samples.sort();

        assert_eq!(samples[0].name, "remote pid");
        assert_eq!(samples[1].name, "local pid");
    }

    #[cfg(feature = "benchmarks")]
    #[bench]
    fn bench_trace_instant_disabled(b: &mut Bencher) {
        let trace = Trace::disabled();

        b.iter(|| black_box(trace.instant("nothing", &["benchmark"])));
    }

    #[cfg(feature = "benchmarks")]
    #[bench]
    fn bench_trace_instant(b: &mut Bencher) {
        let trace = Trace::enabled(Config::default());
        b.iter(|| black_box(trace.instant("something", &["benchmark"])));
    }

    #[cfg(feature = "benchmarks")]
    #[bench]
    fn bench_trace_instant_with_payload(b: &mut Bencher) {
        let trace = Trace::enabled(Config::default());
        b.iter(|| black_box(trace.instant_payload(
            "something", &["benchmark"],
            to_payload("some description of the trace"))));
    }

    #[cfg(feature = "benchmarks")]
    #[bench]
    fn bench_trace_block_disabled(b: &mut Bencher) {
        let trace = Trace::disabled();
        b.iter(|| black_box(trace.block("something", &["benchmark"])));
    }

    #[cfg(feature = "benchmarks")]
    #[bench]
    fn bench_trace_block(b: &mut Bencher) {
        let trace = Trace::enabled(Config::default());
        b.iter(|| black_box(trace.block("something", &["benchmark"])));
    }


    #[cfg(feature = "benchmarks")]
    #[bench]
    fn bench_trace_block_payload(b: &mut Bencher) {
        let trace = Trace::enabled(Config::default());
        b.iter(|| {
            black_box(|| {
                let _ = trace.block_payload(
                    "something", &["benchmark"],
                    to_payload("some payload for the block"));
            });
        });
    }

    #[cfg(feature = "benchmarks")]
    #[bench]
    fn bench_trace_closure_disabled(b: &mut Bencher) {
        let trace = Trace::disabled();

        b.iter(|| black_box(trace.closure("something", &["benchmark"], || {})));
    }

    #[cfg(feature = "benchmarks")]
    #[bench]
    fn bench_trace_closure(b: &mut Bencher) {
        let trace = Trace::enabled(Config::default());
        b.iter(|| black_box(trace.closure("something", &["benchmark"], || {})));
    }

    #[cfg(feature = "benchmarks")]
    #[bench]
    fn bench_trace_closure_payload(b: &mut Bencher) {
        let trace = Trace::enabled(Config::default());
        b.iter(|| black_box(trace.closure_payload(
                    "something", &["benchmark"], || {},
                    to_payload("some description of the closure"))));
    }

    // this is the cost contributed by the timestamp to trace()
    #[cfg(feature = "benchmarks")]
    #[bench]
    fn bench_single_timestamp(b: &mut Bencher) {
        b.iter(|| black_box(time::precise_time_ns()));
    }

    // this is the cost contributed by the timestamp to
    // trace_block()/trace_closure
    #[cfg(feature = "benchmarks")]
    #[bench]
    fn bench_two_timestamps(b: &mut Bencher) {
        b.iter(|| {
            black_box(time::precise_time_ns());
            black_box(time::precise_time_ns());
        });
    }

    #[cfg(feature = "benchmarks")]
    #[bench]
    fn bench_get_tid(b: &mut Bencher) {
        b.iter(|| black_box(sys_tid::current_tid()));
    }

    #[cfg(feature = "benchmarks")]
    #[bench]
    fn bench_get_pid(b: &mut Bencher) {
        b.iter(|| sys_pid::current_pid());
    }
}
#[cfg(test)]
mod tests_llm_16_4 {
    use super::*;

use crate::*;

    #[test]
    fn test_arr_eq() {
        let vec: Vec<String> = vec!["hello".to_string(), "world".to_string()];
        let arr: [&'static str; 2] = ["hello", "world"];

        assert!(vec.arr_eq(&arr));
    }
}#[cfg(test)]
mod tests_llm_16_10_llm_16_9 {
    use serde::de::DeserializeOwned;
    use serde_json;
    use serde::Deserialize;
    use std::fmt;
    use std::error::Error;

    #[derive(Debug, PartialEq)]
    enum CategoriesT {
        DynamicArray(Vec<String>),
    }

    impl<'de> Deserialize<'de> for CategoriesT {
        fn deserialize<D>(deserializer: D) -> Result<CategoriesT, D::Error>
        where
            D: serde::de::Deserializer<'de>,
        {
            use serde::de::Visitor;

            struct CategoriesTVisitor;

            impl<'de> Visitor<'de> for CategoriesTVisitor {
                type Value = CategoriesT;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("comma-separated strings")
                }

                fn visit_str<E>(self, v: &str) -> Result<CategoriesT, E>
                where
                    E: serde::de::Error,
                {
                    let categories = v.split(',').map(ToString::to_string).collect();
                    Ok(CategoriesT::DynamicArray(categories))
                }

                fn visit_string<E>(self, v: String) -> Result<CategoriesT, E>
                where
                    E: serde::de::Error,
                {
                    let categories = v.split(',').map(ToString::to_string).collect();
                    Ok(CategoriesT::DynamicArray(categories))
                }
            }

            deserializer.deserialize_str(CategoriesTVisitor)
        }
    }

    fn assert_deserialize<'de, T>(input: &'de str, expected: T)
    where
        T: PartialEq + DeserializeOwned + fmt::Debug,
    {
        let deserialized: T = serde_json::from_str(input).unwrap();
        assert_eq!(deserialized, expected);
    }

    #[test]
    fn test_deserialize_dynamic_array() {
        let input = r#" "category1,category2,category3" "#;
        let expected = CategoriesT::DynamicArray(vec![
            "category1".to_string(),
            "category2".to_string(),
            "category3".to_string(),
        ]);
        assert_deserialize(input, expected);
    }
}#[cfg(test)]
mod tests_llm_16_15 {
    use super::*;

use crate::*;

    #[test]
    fn test_from_static_array_to_categories_t() {
        let arr: &[&'static str; 0] = &[];
        let result: CategoriesT = <CategoriesT as std::convert::From<_>>::from(arr);

        match result {
            CategoriesT::StaticArray(c) => assert_eq!(c.len(), 0),
            _ => panic!("Expected StaticArray variant"),
        }
    }
}#[cfg(test)]
mod tests_llm_16_24 {
    use super::*;

use crate::*;
    use serde_json;

    #[test]
    fn test_from_static_array() {
        let arr: &'static [&'static str; 4] = &["foo", "bar", "baz", "qux"];
        let categories: CategoriesT = CategoriesT::from(arr);
        assert_eq!(categories, CategoriesT::StaticArray(arr));
    }

    #[test]
    fn test_from_dynamic_array() {
        let vec: Vec<String> = vec!["foo".to_string(), "bar".to_string(), "baz".to_string(), "qux".to_string()];
        let categories: CategoriesT = CategoriesT::from(vec.clone());
        assert_eq!(categories, CategoriesT::DynamicArray(vec));
    }

    #[test]
    fn test_join_static_array() {
        let arr: &'static [&'static str; 4] = &["foo", "bar", "baz", "qux"];
        let categories: CategoriesT = CategoriesT::StaticArray(arr);
        assert_eq!(categories.join(","), "foo,bar,baz,qux");
    }

    #[test]
    fn test_join_dynamic_array() {
        let vec: Vec<String> = vec!["foo".to_string(), "bar".to_string(), "baz".to_string(), "qux".to_string()];
        let categories: CategoriesT = CategoriesT::DynamicArray(vec);
        assert_eq!(categories.join(","), "foo,bar,baz,qux");
    }

    #[test]
    fn test_serialize_static_array() {
        let arr: &'static [&'static str; 4] = &["foo", "bar", "baz", "qux"];
        let categories: CategoriesT = CategoriesT::StaticArray(arr);
        let json = serde_json::to_string(&categories).unwrap();
        assert_eq!(json, "\"foo,bar,baz,qux\"");
    }

    #[test]
    fn test_serialize_dynamic_array() {
        let vec: Vec<String> = vec!["foo".to_string(), "bar".to_string(), "baz".to_string(), "qux".to_string()];
        let categories: CategoriesT = CategoriesT::DynamicArray(vec);
        let json = serde_json::to_string(&categories).unwrap();
        assert_eq!(json, "\"foo,bar,baz,qux\"");
    }

    #[test]
    fn test_deserialize_static_array() {
        let json = "\"foo,bar,baz,qux\"";
        let categories: CategoriesT = serde_json::from_str(json).unwrap();
        let arr: &'static [&'static str; 4] = &["foo", "bar", "baz", "qux"];
        assert_eq!(categories, CategoriesT::StaticArray(arr));
    }

    #[test]
    fn test_deserialize_dynamic_array() {
        let json = "\"foo,bar,baz,qux\"";
        let categories: CategoriesT = serde_json::from_str(json).unwrap();
        let vec: Vec<String> = vec!["foo".to_string(), "bar".to_string(), "baz".to_string(), "qux".to_string()];
        assert_eq!(categories, CategoriesT::DynamicArray(vec));
    }
}#[cfg(test)]
mod tests_llm_16_29 {
    use crate::CategoriesT;

    #[test]
    fn test_from() {
        let c: &'static [&'static str; 7] = &["foo", "bar", "baz", "qux", "quux", "corge", "grault"];
        let result: CategoriesT = <CategoriesT as std::convert::From<&'static [&'static str; 7]>>::from(c);

        assert_eq!(result, CategoriesT::StaticArray(c));
    }
}#[cfg(test)]
mod tests_llm_16_31_llm_16_30 {
    use super::*;

use crate::*;
    use serde_json;

    #[test]
    fn test_from_static_array() {
        let arr = &["one", "two", "three"];
        let categories: CategoriesT = <CategoriesT as std::convert::From<&'static [&'static str; 3]>>::from(arr);
        assert_eq!(categories, CategoriesT::StaticArray(arr));
    }

    #[test]
    fn test_from_dynamic_array() {
        let vec = vec!["one".to_string(), "two".to_string(), "three".to_string()];
        let categories: CategoriesT = <CategoriesT as std::convert::From<Vec<String>>>::from(vec.clone());
        assert_eq!(categories, CategoriesT::DynamicArray(vec));
    }

    #[test]
    fn test_join_static_array() {
        let arr = &["one", "two", "three"];
        let categories = CategoriesT::StaticArray(arr);
        assert_eq!(categories.join("-"), "one-two-three");
    }

    #[test]
    fn test_join_dynamic_array() {
        let vec = vec!["one".to_string(), "two".to_string(), "three".to_string()];
        let categories = CategoriesT::DynamicArray(vec);
        assert_eq!(categories.join("-"), "one-two-three");
    }

    #[test]
    fn test_eq_static_array() {
        let arr1 = &["one", "two", "three"];
        let categories1 = CategoriesT::StaticArray(arr1);
        let arr2 = &["one", "two", "three"];
        let categories2 = CategoriesT::StaticArray(arr2);

        assert_eq!(categories1, categories2);
    }

    #[test]
    fn test_eq_dynamic_array() {
        let vec1 = vec!["one".to_string(), "two".to_string(), "three".to_string()];
        let categories1 = CategoriesT::DynamicArray(vec1);
        let vec2 = vec!["one".to_string(), "two".to_string(), "three".to_string()];
        let categories2 = CategoriesT::DynamicArray(vec2);

        assert_eq!(categories1, categories2);
    }

    #[test]
    fn test_serialize_static_array() {
        let arr = &["one", "two", "three"];
        let categories = CategoriesT::StaticArray(arr);
        let serialized = serde_json::to_string(&categories).unwrap();
        assert_eq!(serialized, "\"one,two,three\"");
    }

    #[test]
    fn test_serialize_dynamic_array() {
        let vec = vec!["one".to_string(), "two".to_string(), "three".to_string()];
        let categories = CategoriesT::DynamicArray(vec);
        let serialized = serde_json::to_string(&categories).unwrap();
        assert_eq!(serialized, "\"one,two,three\"");
    }

    #[test]
    fn test_deserialize() {
        let deserialized: CategoriesT = serde_json::from_str("\"one,two,three\"").unwrap();
        let vec = vec!["one".to_string(), "two".to_string(), "three".to_string()];
        let categories = CategoriesT::DynamicArray(vec);
        assert_eq!(deserialized, categories);
    }
}#[cfg(test)]
mod tests_llm_16_40 {
    use super::*;

use crate::*;

    #[test]
    fn test_partial_cmp() {
        let sample1 = Sample {
            name: "sample1".into(),
            categories: None,
            timestamp_us: 0,
            event_type: SampleEventType::Instant,
            duration_us: None,
            pid: 1,
            tid: 1,
            thread_name: None,
            args: None,
        };

        let sample2 = Sample {
            name: "sample2".into(),
            categories: None,
            timestamp_us: 1,
            event_type: SampleEventType::Instant,
            duration_us: None,
            pid: 2,
            tid: 2,
            thread_name: None,
            args: None,
        };

        let sample3 = Sample {
            name: "sample3".into(),
            categories: None,
            timestamp_us: 2,
            event_type: SampleEventType::Instant,
            duration_us: None,
            pid: 3,
            tid: 3,
            thread_name: None,
            args: None,
        };

        assert_eq!(sample1.partial_cmp(&sample1), Some(cmp::Ordering::Equal));
        assert_eq!(sample1.partial_cmp(&sample2), Some(cmp::Ordering::Less));
        assert_eq!(sample2.partial_cmp(&sample1), Some(cmp::Ordering::Greater));
        assert_eq!(sample2.partial_cmp(&sample3), Some(cmp::Ordering::Less));
        assert_eq!(sample3.partial_cmp(&sample2), Some(cmp::Ordering::Greater));
    }
}#[cfg(test)]
mod tests_llm_16_61 {
    use super::*;

use crate::*;

    struct MockStringArray {}

    impl StringArrayEq<[&'static str]> for MockStringArray {
        fn arr_eq(&self, other: &[&'static str]) -> bool {
            unimplemented!()
        }
    }

    #[test]
    fn test_arr_eq_should_return_true_for_equal_arrays() {
        let arr1: Vec<String> = vec!["hello".to_string(), "world".to_string()];
        let arr2: Vec<&'static str> = vec!["hello", "world"];

        let mock = MockStringArray {};
        assert_eq!(arr1.arr_eq(&arr2), mock.arr_eq(&arr2));
    }

    #[test]
    fn test_arr_eq_should_return_false_for_arrays_with_different_lengths() {
        let arr1: Vec<String> = vec!["hello".to_string(), "world".to_string()];
        let arr2: Vec<&'static str> = vec!["hello"];

        let mock = MockStringArray {};
        assert_eq!(arr1.arr_eq(&arr2), mock.arr_eq(&arr2));
    }

    #[test]
    fn test_arr_eq_should_return_false_for_arrays_with_different_elements() {
        let arr1: Vec<String> = vec!["hello".to_string(), "world".to_string()];
        let arr2: Vec<&'static str> = vec!["hello", "universe"];

        let mock = MockStringArray {};
        assert_eq!(arr1.arr_eq(&arr2), mock.arr_eq(&arr2));
    }
}#[cfg(test)]
mod tests_llm_16_62 {
    use super::*;

use crate::*;
    
    use serde_json;
    
    #[test]
    fn test_join_static_array() {
        let categories = CategoriesT::StaticArray(&["apple", "banana", "orange"]);
        assert_eq!(categories.join(","), "apple,banana,orange");
    }

    #[test]
    fn test_join_dynamic_array() {
        let categories = CategoriesT::DynamicArray(vec!["apple".to_string(), "banana".to_string(), "orange".to_string()]);
        assert_eq!(categories.join(","), "apple,banana,orange");
    }

    #[test]
    fn test_join_with_empty_string() {
        let categories = CategoriesT::StaticArray(&["apple", "banana", "orange"]);
        assert_eq!(categories.join(""), "applebananaorange");
    }

    #[test]
    fn test_join_with_whitespace() {
        let categories = CategoriesT::StaticArray(&["apple", "banana", "orange"]);
        assert_eq!(categories.join(" "), "apple banana orange");
    }

    #[test]
    fn test_serialize_static_array() {
        let categories = CategoriesT::StaticArray(&["apple", "banana", "orange"]);
        let json = serde_json::to_string(&categories).unwrap();
        assert_eq!(json, "\"apple,banana,orange\"");
    }

    #[test]
    fn test_serialize_dynamic_array() {
        let categories = CategoriesT::DynamicArray(vec!["apple".to_string(), "banana".to_string(), "orange".to_string()]);
        let json = serde_json::to_string(&categories).unwrap();
        assert_eq!(json, "\"apple,banana,orange\"");
    }

    #[test]
    fn test_deserialize() {
        let json = "\"apple,banana,orange\"";
        let categories: CategoriesT = serde_json::from_str(json).unwrap();
        assert_eq!(categories, CategoriesT::DynamicArray(vec!["apple".to_string(), "banana".to_string(), "orange".to_string()]));
    }
}#[cfg(test)]
mod tests_llm_16_63 {
    use super::*;

use crate::*;

    #[test]
    fn test_default() {
        let config = Config::default();
        assert_eq!(config.sample_limit_count, 1 * 1024 * 1024 / std::mem::size_of::<Sample>());
    }
}#[cfg(test)]
mod tests_llm_16_64 {
    use super::*;

use crate::*;

    #[test]
    fn test_max_samples() {
        let config = Config::with_limit_count(100);
        assert_eq!(config.max_samples(), 100);
    }
}#[cfg(test)]
mod tests_llm_16_65 {
    use super::*;

use crate::*;
    use std::mem::size_of;

    #[test]
    fn test_max_size_in_bytes() {
        let config = Config::with_limit_bytes(1 * 1024 * 1024);
        let expected = config.sample_limit_count * size_of::<Sample>();
        assert_eq!(config.max_size_in_bytes(), expected);
    }
}#[cfg(test)]
mod tests_llm_16_66 {
    use super::*;

use crate::*;

    #[test]
    fn test_with_limit_bytes() {
        let size = 1024;
        let config = Config::with_limit_bytes(size);
        assert_eq!(config.sample_limit_count, size / size_of::<Sample>());
    }
}#[cfg(test)]
mod tests_llm_16_67 {
    use super::*;

use crate::*;

    #[test]
    fn test_with_limit_count() {
        let limit = 100;
        let config = Config::with_limit_count(limit);
        assert_eq!(config.sample_limit_count, limit);
    }
}#[cfg(test)]
mod tests_llm_16_68 {
    use super::*;

use crate::*;

    #[test]
    fn test_consume_process_name() {
        let metadata_type = MetadataType::ProcessName {
            name: "test_name".to_string(),
        };
        let (name, sort_index) = metadata_type.consume();
        assert_eq!(name, Some("test_name".to_string()));
        assert_eq!(sort_index, None);
    }

    #[test]
    fn test_consume_thread_name() {
        let metadata_type = MetadataType::ThreadName {
            name: "test_name".to_string(),
        };
        let (name, sort_index) = metadata_type.consume();
        assert_eq!(name, Some("test_name".to_string()));
        assert_eq!(sort_index, None);
    }

    #[test]
    fn test_consume_process_sort_index() {
        let metadata_type = MetadataType::ProcessSortIndex {
            sort_index: 10,
        };
        let (name, sort_index) = metadata_type.consume();
        assert_eq!(name, None);
        assert_eq!(sort_index, Some(10));
    }

    #[test]
    fn test_consume_thread_sort_index() {
        let metadata_type = MetadataType::ThreadSortIndex {
            sort_index: 10,
        };
        let (name, sort_index) = metadata_type.consume();
        assert_eq!(name, None);
        assert_eq!(sort_index, Some(10));
    }

    #[test]
    fn test_consume_process_labels() {
        let metadata_type = MetadataType::ProcessLabels {
            labels: "test_labels".to_string(),
        };
        let (name, sort_index) = metadata_type.consume();
        assert_eq!(name, None);
        assert_eq!(sort_index, None);
    }
}#[cfg(test)]
mod tests_llm_16_69 {
    use super::*;

use crate::*;

    #[test]
    fn test_sample_name_process_name() {
        let metadata_type = MetadataType::ProcessName {
            name: String::from("test_process"),
        };
        assert_eq!(metadata_type.sample_name(), "process_name");
    }

    #[test]
    fn test_sample_name_process_labels() {
        let metadata_type = MetadataType::ProcessLabels {
            labels: String::from("test_labels"),
        };
        assert_eq!(metadata_type.sample_name(), "process_labels");
    }

    #[test]
    fn test_sample_name_process_sort_index() {
        let metadata_type = MetadataType::ProcessSortIndex {
            sort_index: 123,
        };
        assert_eq!(metadata_type.sample_name(), "process_sort_index");
    }

    #[test]
    fn test_sample_name_thread_name() {
        let metadata_type = MetadataType::ThreadName {
            name: String::from("test_thread"),
        };
        assert_eq!(metadata_type.sample_name(), "thread_name");
    }

    #[test]
    fn test_sample_name_thread_sort_index() {
        let metadata_type = MetadataType::ThreadSortIndex {
            sort_index: 456,
        };
        assert_eq!(metadata_type.sample_name(), "thread_sort_index");
    }
}#[cfg(test)]
mod tests_llm_16_77_llm_16_76 {
    use super::*;

use crate::*;
    use sys_pid;
    use sys_tid;

    #[test]
    fn test_new_metadata() {
        let timestamp_ns = 0;
        let meta = MetadataType::ProcessName {
            name: "test".to_string(),
        };
        let tid = 0;
        let sample = Sample::new_metadata(timestamp_ns, meta, tid);

        assert_eq!(sample.name, "process_name");
        assert_eq!(sample.categories, None);
        assert_eq!(sample.timestamp_us, 0);
        assert_eq!(sample.event_type, SampleEventType::Metadata);
        assert_eq!(sample.duration_us, None);
        assert_eq!(sample.tid, 0);
        assert_eq!(sample.thread_name, None);
        assert_eq!(sample.pid, sys_pid::current_pid());
        assert_eq!(sample.args.as_ref().unwrap().payload, None);
        assert_eq!(
            sample.args.as_ref().unwrap().metadata_name.as_ref().unwrap().to_string(),
            "test"
        );
        assert_eq!(sample.args.as_ref().unwrap().metadata_sort_index, None);
    }
}#[cfg(test)]
mod tests_llm_16_80 {
    use super::*;

use crate::*;

    #[test]
    fn test_from_chrome_id() {
        assert_eq!(SampleEventType::from_chrome_id('B'), SampleEventType::DurationBegin);
        assert_eq!(SampleEventType::from_chrome_id('E'), SampleEventType::DurationEnd);
        assert_eq!(SampleEventType::from_chrome_id('X'), SampleEventType::CompleteDuration);
        assert_eq!(SampleEventType::from_chrome_id('i'), SampleEventType::Instant);
        assert_eq!(SampleEventType::from_chrome_id('b'), SampleEventType::AsyncStart);
        assert_eq!(SampleEventType::from_chrome_id('n'), SampleEventType::AsyncInstant);
        assert_eq!(SampleEventType::from_chrome_id('e'), SampleEventType::AsyncEnd);
        assert_eq!(SampleEventType::from_chrome_id('s'), SampleEventType::FlowStart);
        assert_eq!(SampleEventType::from_chrome_id('t'), SampleEventType::FlowInstant);
        assert_eq!(SampleEventType::from_chrome_id('f'), SampleEventType::FlowEnd);
        assert_eq!(SampleEventType::from_chrome_id('N'), SampleEventType::ObjectCreated);
        assert_eq!(SampleEventType::from_chrome_id('O'), SampleEventType::ObjectSnapshot);
        assert_eq!(SampleEventType::from_chrome_id('D'), SampleEventType::ObjectDestroyed);
        assert_eq!(SampleEventType::from_chrome_id('M'), SampleEventType::Metadata);
    }
}#[cfg(test)]
mod tests_llm_16_81 {
    use super::*;

use crate::*;

    #[test]
    fn test_into_chrome_id() {
        assert_eq!(SampleEventType::DurationBegin.into_chrome_id(), 'B');
        assert_eq!(SampleEventType::DurationEnd.into_chrome_id(), 'E');
        assert_eq!(SampleEventType::CompleteDuration.into_chrome_id(), 'X');
        assert_eq!(SampleEventType::Instant.into_chrome_id(), 'i');
        assert_eq!(SampleEventType::AsyncStart.into_chrome_id(), 'b');
        assert_eq!(SampleEventType::AsyncInstant.into_chrome_id(), 'n');
        assert_eq!(SampleEventType::AsyncEnd.into_chrome_id(), 'e');
        assert_eq!(SampleEventType::FlowStart.into_chrome_id(), 's');
        assert_eq!(SampleEventType::FlowInstant.into_chrome_id(), 't');
        assert_eq!(SampleEventType::FlowEnd.into_chrome_id(), 'f');
        assert_eq!(SampleEventType::ObjectCreated.into_chrome_id(), 'N');
        assert_eq!(SampleEventType::ObjectSnapshot.into_chrome_id(), 'O');
        assert_eq!(SampleEventType::ObjectDestroyed.into_chrome_id(), 'D');
        assert_eq!(SampleEventType::Metadata.into_chrome_id(), 'M');
    }
}#[cfg(test)]
mod tests_llm_16_84 {
    use super::*;

use crate::*;
    use std::mem;

    #[test]
    fn test_new_disabled() {
        let guard: SampleGuard = SampleGuard::new_disabled();
        assert_eq!(mem::size_of::<SampleGuard>(), 2 * mem::size_of::<usize>());
    }
}#[cfg(test)]
mod tests_llm_16_85 {
    use super::*;

use crate::*;
    use std::sync::Arc;

    #[test]
    fn test_block_disabled() {
        let trace = Trace::disabled();
        let result = trace.block("test", &["category1", "category2"]);
        assert!(result.sample.is_none());
        assert!(result.trace.is_none());
    }

    #[test]
    fn test_block_enabled() {
        let trace = Trace::disabled();
        trace.enable();
        let result = trace.block("test", &["category1", "category2"]);
        assert!(result.sample.is_some());
        assert!(result.trace.is_some());
        assert!(trace.is_enabled());
    }
}#[cfg(test)]
mod tests_llm_16_92 {
    use super::*;

use crate::*;

    #[test]
    fn test_disable() {
        let trace = Trace::disabled();
        trace.disable();
        assert_eq!(trace.is_enabled(), false);
    }
}#[cfg(test)]
mod tests_llm_16_93 {
    use super::*;

use crate::*;

    #[test]
    fn test_disabled() {
        let trace = Trace::disabled();
        assert_eq!(trace.is_enabled(), false);
        assert_eq!(trace.get_samples_count(), 0);
        assert_eq!(trace.get_samples_limit(), 0);
    }
}#[cfg(test)]
mod tests_llm_16_94 {
    use std::sync::{Arc, Mutex};
    use crate::Trace;

    #[test]
    fn test_enable() {
        let trace = Arc::new(Trace::disabled());

        trace.enable();

        assert_eq!(trace.is_enabled(), true);
    }
}#[cfg(test)]
mod tests_llm_16_95 {
    use super::*;

use crate::*;

    use std::mem::size_of;

    #[test]
    fn test_enable_config() {
        let trace = Trace::disabled();
        let config = Config::with_limit_count(100);

        trace.enable_config(config);

        let samples_count = trace.get_samples_count();
        let samples_limit = trace.get_samples_limit();

        let expected_count = config.max_samples();
        let expected_limit = config.max_samples() * size_of::<Sample>();

        assert_eq!(samples_count, expected_count);
        assert_eq!(samples_limit, expected_limit);
        assert_eq!(trace.is_enabled(), true);
    }
}#[cfg(test)]
mod tests_llm_16_102 {
    use super::*;

use crate::*;
    use std::path::Path;

    #[test]
    fn test_instant() {
        let trace = Trace::disabled();
        trace.enable();

        trace.instant("test", &[]);

        assert_eq!(trace.get_samples_count(), 1);
        assert_eq!(trace.get_samples_limit(), 0);
    }
}#[cfg(test)]
mod tests_llm_16_104_llm_16_103 {
    use super::*;

use crate::*;

    #[test]
    fn test_instant_payload() {
        let trace = Trace::disabled();
        let name = "test".to_string();
        let categories = vec!["category".to_string()];
        let payload = "payload".to_string();
        trace.instant_payload(name, categories, payload);
        assert_eq!(trace.get_samples_count(), 1);
        assert_eq!(trace.get_samples_limit(), 0);
    }
}#[cfg(test)]
mod tests_llm_16_105 {
    use super::*;

use crate::*;

    #[test]
    fn test_is_enabled() {
        let trace = Trace::disabled();
        assert_eq!(trace.is_enabled(), false);
    }
}#[cfg(test)]
mod tests_llm_16_106 {
    use super::*;

use crate::*;

    #[test]
    fn test_record() {
        let trace = Trace::disabled();
        let sample = Sample {
            name: StrCow::Borrowed("test"),
            categories: None,
            timestamp_us: 0,
            event_type: SampleEventType::Instant,
            duration_us: None,
            pid: 0,
            tid: 0,
            thread_name: None,
            args: None,
        };
        trace.record(sample);
        assert_eq!(trace.get_samples_count(), 1);
    }
}#[cfg(test)]
mod tests_llm_16_124 {
    use crate::enable_tracing;

    #[test]
    fn test_enable_tracing() {
        enable_tracing();
        // Add assertions here if needed
    }
}#[cfg(test)]
mod tests_llm_16_125 {
    use super::*;

use crate::*;
    use std::mem::size_of;

    #[test]
    fn test_enable_tracing_with_config() {
        // Create a Config with limit count of 100
        let config = Config::with_limit_count(100);
        
        // Call enable_tracing_with_config with the config
        enable_tracing_with_config(config);
    }
}#[cfg(test)]
mod tests_llm_16_126 {
    use super::*;

use crate::*;

    #[test]
    fn test_exe_name_returns_exe_name_if_possible() {
        assert_eq!(exe_name().unwrap(), "<insert expected value here>");
    }

    #[test]
    fn test_exe_name_returns_full_path_if_name_unavailable() {
        assert_eq!(exe_name().unwrap(), "<insert expected value here>");
    }

    #[test]
    fn test_exe_name_returns_none_if_error_occurs() {
        assert_eq!(exe_name(), None);
    }
}#[cfg(test)]
mod tests_llm_16_163 {
    use super::*;

use crate::*;

    #[test]
    fn test_is_enabled() {
        assert!(is_enabled());
    }
}#[cfg(test)]
mod tests_llm_16_164 {
    use crate::ns_to_us;

    #[test]
    fn test_ns_to_us() {
        assert_eq!(ns_to_us(0), 0);
        assert_eq!(ns_to_us(1000), 1);
        assert_eq!(ns_to_us(1500), 1);
        assert_eq!(ns_to_us(999), 0);
        assert_eq!(ns_to_us(1001), 1);
        assert_eq!(ns_to_us(1999), 1);
    }
}#[cfg(test)]
mod tests_llm_16_167 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_samples_cloned_unsorted() {
        assert_eq!(samples_cloned_unsorted(), TRACE.samples_cloned_unsorted());
    }
}#[cfg(test)]
mod tests_llm_16_168 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_samples_len() {
        assert_eq!(samples_len(), 0);
    }
}#[cfg(test)]
mod tests_llm_16_169 {
    use super::*;

use crate::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_save() {
        // Arrange
        let path: PathBuf = "trace.json".into();
        let sort = true;
        
        // Act
        let result = save(path, sort);
        
        // Assert
        assert!(result.is_ok());
    }
}#[cfg(test)]
mod tests_llm_16_171_llm_16_170 {
    use super::*;

use crate::*;
    use serde::Serialize;
    use serde_json::ser::to_string as serde_to_string;

    #[derive(Serialize)]
    pub enum SampleEventType {
        DurationBegin,
        DurationEnd,
        CompleteDuration,
        Instant,
        AsyncStart,
        AsyncInstant,
        AsyncEnd,
        FlowStart,
        FlowInstant,
        FlowEnd,
        ObjectCreated,
        ObjectSnapshot,
        ObjectDestroyed,
        Metadata,
    }

    #[test]
    fn test_serialize_event_type() {
        let ph = SampleEventType::DurationBegin;
        let expected = "\"B\"";

        let result = serde_to_string(&ph).unwrap();

        assert_eq!(result, expected);
    }
}#[cfg(test)]
mod tests_llm_16_174 {
    use super::*;

use crate::*;
    use std::borrow::Cow;

    #[test]
    fn test_to_cow_str() {
        assert_eq!(to_cow_str("hello"), Cow::Borrowed("hello"));
        assert_eq!(to_cow_str(String::from("world")), Cow::Borrowed("world"));
        assert_eq!(to_cow_str(Cow::Borrowed("foo")), Cow::Borrowed("foo"));
        assert_eq!(to_cow_str(Cow::Owned(String::from("bar"))), Cow::Borrowed("bar"));
    }
}#[cfg(test)]
mod tests_llm_16_176 {
    use crate::trace;

    #[test]
    fn test_trace() {
        let name = "something happened";
        let categories = &["rpc", "response"];
        trace(name, categories);
    }
}#[cfg(test)]
mod tests_llm_16_185 {
    use super::*;

use crate::*;

    #[test]
    #[should_panic]
    fn test_trace_payload() {
        trace_payload("test", &["category1", "category2"], "payload");
    }
}        
#[cfg(test)]
mod tests_rug_207 {
    use super::*;
    use serde::de::value::Error;
    use serde::de::value::U64Deserializer;

    #[test]
    fn test_rug() {
        let mut v98: U64Deserializer<Error> = U64Deserializer::new();
        let p0 = &mut v98;
        
        crate::deserialize_event_type(p0);

    }
}
    #[cfg(test)]
mod tests_rug_208 {
    use super::*;
    use crate::trace::TRACE;

    #[test]
    fn test_disable_tracing() {
        disable_tracing();
        assert_eq!(TRACE.is_enabled(), false);
    }
}
#[cfg(test)]
mod tests_rug_209 {
    use super::*;
    use crate::{SampleGuard, StrCow, CategoriesT};
    
    #[test]
    fn test_trace_block() {
        let mut p0: StrCow = "something_expensive".into();
        let mut p1: CategoriesT = &["rpc", "request"];
                
        crate::trace_block(p0, p1);
    }
}
#[cfg(test)]
mod tests_rug_210 {
    use super::*;
    use crate::StrCow;
    use crate::CategoriesT;
    use crate::TracePayloadT;

    #[test]
    fn test_rug() {
        let mut p0: std::sync::mpmc::select::Selected = unimplemented!();
        let mut p1: std::sync::mpmc::select::Selected = unimplemented!();
        let mut p2: std::sync::mpmc::select::Selected = unimplemented!();

        crate::trace_block_payload(p0, p1, p2);
    }
}#[cfg(test)]
mod tests_rug_211 {
    use super::*;
    use std::sync::mpmc::select::Selected;
    use std::process::ExitStatusError;
    use core::str::CharEscapeUnicode;
    
    #[test]
    fn test_rug() {
        let mut p0: Selected<Node> = unimplemented!(); // fill in the sample variable here
        let mut p1: ExitStatusError = unimplemented!(); // fill in the sample variable here
        let mut p2: CharEscapeUnicode = unimplemented!(); // fill in the sample variable here
    
        crate::trace_closure(p0, p1, p2);
        
    }
}#[cfg(test)]
mod tests_rug_212 {

    use super::*;
    use std::process::ExitStatusError;
    use std::sys::unix::process::process_inner::ExitStatusError;
    use crate::trace::core::const_closure::{ConstFnMutClosure, Function};

    struct A;
    struct B;
    struct C;

    #[test]
    fn test_rug() {
        let mut p0: std::process::ExitStatusError = ExitStatusError;
        let mut p1: std::sys::unix::process::process_inner::ExitStatusError = ExitStatusError { code: 1, signal: None };
        let a = A;
        let b = B;
        let c = C;
        let mut p2: core::const_closure::ConstFnMutClosure<(&mut A, &mut B, &mut C), Function> =
            ConstFnMutClosure::<(&mut A, &mut B, &mut C), Function>::new(|_a: &mut A, _b: &mut B, _c: &mut C| {
                // Your closure implementation here
            });
        let mut p3: std::sys::unix::process::process_inner::ExitStatusError =
            ExitStatusError { code: 1, signal: None };

        crate::trace_closure_payload(p0, p1, p2, p3);
    }
}                    
#[cfg(test)]
mod tests_rug_213 {
    use super::*;
    use crate::trace::Sample;
    
    #[test]
    fn test_rug() {
        let samples: Vec<Sample> = vec![]; // construct the variable based on hints or sample data
        
        let result = samples_cloned_sorted();
        
        // perform assertions on the result
    }
}
#[cfg(test)]
mod tests_rug_214 {
    use super::*;
    use crate::trace::CategoriesT;
    use crate::trace::CategoriesT::*;
    
    #[test]
    fn test_rug() {
        let mut p0: CategoriesT = CategoriesT::from(vec![
            "category1".to_string(),
            "category2".to_string(),
            "category3".to_string(),
        ]);
        let mut p1: CategoriesT = CategoriesT::from(vec![
            "category1".to_string(),
            "category2".to_string(),
            "category3".to_string(),
        ]);
        
        <CategoriesT as std::cmp::PartialEq>::eq(&p0, &p1);
    }
}
#[cfg(test)]
mod tests_rug_215 {
    use super::*;
    use crate::trace::CategoriesT;
    use serde::private::ser::content::ContentSerializer;
    use crate::serde::Serialize;

    #[test]
    fn test_rug() {
        #[cfg(test)]
        mod tests_rug_215_prepare {
            use crate::trace::CategoriesT;

            #[test]
            fn sample() {
                let mut v148: CategoriesT = CategoriesT::from(vec![
                    "category1".to_string(),
                    "category2".to_string(),
                    "category3".to_string(),
                ]);
            }
        }

        #[cfg(test)]
        mod tests_rug_215_prepare2 {
            use serde::private::ser::content::ContentSerializer;
            use crate::{E, serde};

            #[test]
            fn sample() {
                let mut v149 = ContentSerializer::<E>::new();
            }
        }

        let mut p0: CategoriesT = CategoriesT::from(vec![
            "category1".to_string(),
            "category2".to_string(),
            "category3".to_string(),
        ]);
        let mut p1 = ContentSerializer::<E>::new();

        <CategoriesT as serde::Serialize>::serialize(&p0, &mut p1);
    }
}#[cfg(test)]
mod tests_rug_216 {
    use super::*;
    use crate::trace;
    use crate::serde::de::Visitor;
    use std::fmt::Formatter;

    #[test]
    fn test_expecting() {
        let mut p0: <CategoriesT as serde::Deserialize<'static>>::deserialize::CategoriesTVisitor = <CategoriesT as serde::Deserialize<'static>>::deserialize::CategoriesTVisitor;
        let mut p1: Formatter<'_> = trace::Formatter::new();

        p0.expecting(&mut p1);
        
        // Additional assertions if necessary
    }
}#[cfg(test)]
mod tests_rug_217 {
    use super::*;
    use crate::serde::de::Visitor;

    #[test]
    fn test_visit_str() {
        let mut p0: <<CategoriesT as serde::Deserialize<'static>>::deserialize::CategoriesTVisitor as Visitor>::Value = CategoriesT::DynamicArray(vec![]);
        let p1: &str = "test_data";
        
        p0.visit_str(&p1);
    }
}#[cfg(test)]
mod tests_rug_218 {
    use super::*;
    use crate::std::convert::From;

    #[test]
    fn test_rug() {
        let p0: [&'static str; 1] = [&"sample_str"];

        <CategoriesT as std::convert::From<&'static [&'static str; 1]>>::from(p0);
    }
}#[cfg(test)]
mod tests_rug_219 {
    use super::*;
    use crate::std::convert::From;

    #[test]
    fn test_from() {
        let p0: [&'static str; 2] = ["arg1", "arg2"];

        <CategoriesT as std::convert::From<&'static [&'static str; 2]>>::from(p0);
    }
}
#[cfg(test)]
mod tests_rug_220 {
    use super::*;
    use crate::CategoriesT;
    
    #[test]
    fn test_from() {
        let p0: &'static [&'static str; 3] = &["category1", "category2", "category3"];
        assert_eq!(<CategoriesT as std::convert::From<&'static [&'static str; 3]>>::from(p0), CategoriesT::StaticArray(p0));
    }
}
#[cfg(test)]
mod tests_rug_221 {
    use super::*;
    use crate::std::convert::From;

    #[test]
    fn test_rug() {
        let mut p0: [&'static str; 5] = ["str1", "str2", "str3", "str4", "str5"];

        <CategoriesT as std::convert::From<&'static [&'static str; 5]>>::from(p0);
    }
}#[cfg(test)]
        mod tests_rug_223 {
            use super::*;
            use crate::std::convert::From;
            #[test]
            fn test_from() {
                let mut p0: [&'static str; 9] = ["apple", "banana", "cherry", "date", "elderberry", "fig", "grape", "honeydew", "kiwi"];

                
                <CategoriesT as std::convert::From<&'static [&'static str; 9]>>::from(&p0);
            }
        }#[cfg(test)]
mod tests_rug_224 {
    use super::*;
    use crate::std::convert::From;

    #[test]
    fn test_rug() {
        let mut p0: [&'static str; 10] = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"];
        <CategoriesT as std::convert::From<&'static [&'static str; 10]>>::from(p0);
    }
}#[cfg(test)]
mod tests_rug_225 {
    use super::*;
    use std::convert::From;

    #[test]
    fn test_rug() {
        let p0: Vec<String> = vec!["category1".to_string(), "category2".to_string()];
        
        <CategoriesT as From<Vec<String>>>::from(p0);

    }
}#[cfg(test)]
mod tests_rug_226 {
    use super::*;
    use crate::{Sample, to_cow_str};

    #[test]
    fn test_thread_name() {
        let thread_name = thread_name();
        // Add assertions here
    }
}#[cfg(test)]
mod tests_rug_227 {
    use super::*;
    use std::sync::mpmc::select::Selected;
    use std::option::Option;
    use std::borrow::Cow;
    use crate::trace::{Sample, StrCow, CategoriesT, TracePayloadT, SampleEventType};

    #[test]
    fn test_rug() {
        let mut p0: Selected = {
            let (sx, rx) = sync_channel::<Node>(0);
            let sel = Arc::new(Mutex::new(vec![rx]));
            Selected(sel)
        };
        let mut p1: Selected = {
            let (sx, rx) = sync_channel::<Node>(0);
            let sel = Arc::new(Mutex::new(vec![rx]));
            Selected(sel)
        };
        let mut p2: Option<Cow<'static, str>> = Some(Cow::Borrowed("sample data"));
        let mut p3: SampleEventType = SampleEventType::from_chrome_id('X');

        
        <Sample>::new_duration_marker(p0, p1, p2, p3);

    }
}
#[cfg(test)]
mod tests_rug_228 {
    use super::*;
    use crate::{
        StrCow, 
        CategoriesT, 
        TracePayloadT,
        Sample,
        SampleEventType,
        ns_to_us,
        SampleArgs
    };

    #[test]
    fn test_new_duration() {

        //  Sample 1
        #[derive(Debug)]
        struct CustomPayload;
        
        let p0 = xi_trace::categories![
            xi_trace::category!("Category 1"),
            xi_trace::category!("Category 2"),
            xi_trace::category!("Category 3")
        ];
        
        let p1 = xi_trace::categories![
            xi_trace::category!("Category 4"),
            xi_trace::category!("Category 5")
        ];
        
        let p2: Option<TracePayloadT> = Some(Box::new(CustomPayload));
        
        let p3 = ns_to_us(1632132312312);
        
        let p4 = ns_to_us(15087021417000);

                
        Sample::new_duration(p0, p1, p2, p3, p4);

    }
}#[cfg(test)]
mod tests_rug_229 {
    use super::*;
    use std::sync::mpsc::sync_channel;
    use std::sys::unix::process::process_inner::ExitStatusError;
    use std::option::Option;
    use std::borrow::Cow;

    #[test]
    fn test_rug() {
        let mut p0 = {
            let (sx, rx) = sync_channel::<Node>(0);
            let sel = Arc::new(Mutex::new(vec![rx]));
            xi_rope::rope::tree::Selected(sel)
        };

        let mut p1 = {
            let mut v3 = ExitStatusError {
                code: 1,
                signal: None,
            };
            
            // Add any necessary initialization for v3
            
            v3
        };

        let mut p2: Option<Cow<'static, str>> = Some(Cow::Borrowed("sample data"));

        <Sample>::new_instant(p0, p1, p2);
    }
}#[cfg(test)]
mod tests_rug_230 {
    use super::*;
    use crate::std::cmp::PartialEq;

    #[test]
    fn test_rug() {
        use crate::trace::trace_events::{CategoriesT, Sample, SampleEventType};
        
        let p0 = Sample::new_duration_marker(
            "sample_name",
            CategoriesT::new(),
            None,
            SampleEventType::Begin,
        );
        
        let p1 = Sample::new_duration_marker(
            "sample_name",
            CategoriesT::new(),
            None,
            SampleEventType::Begin,
        );

        <Sample as std::cmp::PartialEq>::eq(&p0, &p1);
    }
}
#[cfg(test)]
mod tests_rug_231 {
    use super::*;
    use crate::std::cmp::Ord;
    use trace::trace;
    use trace::trace_events::{CategoriesT, Sample, SampleEventType};

        #[test]
        fn test_rug() {
            let mut p0 = Sample::new_duration_marker(
                "sample_name",
                CategoriesT::new(),
                None,
                SampleEventType::Begin,
            );
            let mut p1 = Sample::new_duration_marker(
                "sample_name",
                CategoriesT::new(),
                None,
                SampleEventType::Begin,
            );
                
            <Sample>::cmp(&p0, &p1);

        }
    
}#[cfg(test)]
mod tests_rug_232 {
    use super::*;
    use crate::trace::Hasher;
    use std::hash::{Hash, Hasher as StdHasher};
    
    #[test]
    fn test_rug() {
        use trace::trace_events::{CategoriesT, Sample, SampleEventType};
        
        let v159 = Sample::new_duration_marker(
            "sample_name",
            CategoriesT::new(),
            None,
            SampleEventType::Begin,
        );
        
        let mut v160: Hasher<S> = StdHasher::hasher();
        // Additional code if necessary to initialize the variable
        
        <Sample as std::hash::Hash>::hash(&v159, &mut v160);
    }
}#[cfg(test)]
mod tests_rug_233 {
    use super::*;
    use crate::{Trace, StrCow, CategoriesT, TracePayloadT, Sample, SampleEventType};

    #[test]
    fn test_rug() {
        // Construct the variables
        let config = xi_trace::Config::default();
        let trace = Trace::enabled(config);
        let name: StrCow = "sample name".into();
        let categories: CategoriesT = xi_trace::categories!("category1", "category2");
        let payload: Option<TracePayloadT> = Some(xi_trace::payload!("key1" => "value1", "key2" => "value2"));

        SampleGuard::new(&trace, name, categories, payload);
    }
}#[cfg(test)]
mod tests_rug_234 {
    use super::*;
    use crate::std::ops::Drop;
    use crate::SampleGuard;
    
    #[test]
    fn test_drop() {
        let mut p0: SampleGuard<'static> = SampleGuard {
            trace: Some(/* trace value */),
            sample: Some(/* sample value */),
        };

        <SampleGuard<'static> as std::ops::Drop>::drop(&mut p0);
    }
}#[cfg(test)]
mod tests_rug_235 {
    use super::*;
    use crate::trace::Config;
    use crate::trace::Trace;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    };

    #[test]
    fn test_enabled() {
        let p0 = Config::default();

        Trace::<Config>::enabled(p0);
    }
}
#[cfg(test)]
mod tests_rug_236 {
    use super::*;
    use crate::{Trace, Config, AtomicBool, AtomicOrdering, Mutex, FixedLifoDeque};
    use std::path::Path;

    #[test]
    fn test_rug() {
        let config = Config::default();
        let v162 = Trace::enabled(config);

        let mut p0 = v162;
        
        <Trace>::get_samples_count(&mut p0);
    }
}
#[cfg(test)]
mod tests_rug_237 {
    use super::*;
    use crate::{Trace, Config, AtomicBool, AtomicOrdering, Mutex, FixedLifoDeque};
    use std::path::Path;

    #[test]
    fn test_rug() {
        let config = Config::default(); // fill in with the desired config
        let v162 = Trace::enabled(config); // create the local variable v162 with type Trace using enabled() constructor function
        let mut p0 = v162;

        <Trace>::get_samples_limit(&p0);

    }
}
#[cfg(test)]
mod tests_rug_238 {
    use super::*;
    use crate::{Trace, Config, AtomicBool, AtomicOrdering, Mutex, FixedLifoDeque};
    use std::path::Path;
    use std::process::ExitStatusError;
    use std::sys::unix::process::process_inner::ExitStatusError;

    #[test]
    fn test_rug() {
        let config = Config::default(); 
        let v162 = Trace::enabled(config); 
        
        let mut p0 = v162;
        
        let mut v4: ExitStatusError = ExitStatusError;
        let mut p1 = v4;

        let mut v3 = ExitStatusError {
            code: 1,
            signal: None,
        };
        
        // Fill in any necessary initialization for v3
        
        let mut p2 = v3;
        
         // Fill in the necessary code to construct p3
        
        <Trace>::block_payload(p0, p1, p2, p3);

    }
}#[cfg(test)]
mod tests_rug_239 {
    use super::*;
    use crate::{Trace, Config, AtomicBool, AtomicOrdering, Mutex, FixedLifoDeque};
    use std::sys::unix::process::process_inner::ExitStatusError;
    use std::process::ExitStatusError;
    use std::panic::AssertUnwindSafe;
    use xi_rope::batch::Builder;

    #[test]
    fn test_rug() {
        let config = Config::default();
        let v162 = Trace::enabled(config);
        let mut v3 = ExitStatusError {
            code: 1,
            signal: None,
        };
        let mut v4: ExitStatusError = ExitStatusError;
        let mut v64: AssertUnwindSafe<Builder> = AssertUnwindSafe::new(Builder::new());

        Trace::<_, _, _, _>::closure(v162, v3, v4, || {
            // Your closure code here
            // TODO: update closure code with actual logic
        });
    }
}
#[cfg(test)]
mod tests_rug_240 {
    use super::*;
    use crate::{Trace, Config, AtomicBool, AtomicOrdering, Mutex, FixedLifoDeque};
    use std::path::Path;
    use std::process::ExitStatusError;
    use xi_rope::rope::tree::Node;
    use std::sync::mpsc::sync_channel;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use crate::trace::CharEscapeDefault;

    
   #[test]
   fn test_closure_payload() {
      let config = Config::default(); // fill in with the desired config
      let v162 = Trace::enabled(config); // create the local variable v162 with type Trace using enabled() constructor function

      let mut v4: ExitStatusError = ExitStatusError; // initialize v4 with type std::process:ExitStatusError

      let (sx, rx) = sync_channel::<Node>(0);
      let sel = Arc::new(Mutex::new(vec![rx]));
      let v67 = xi_rope:rope:tree:Selected(sel); 

      let mut v166: CharEscapeDefault = CharEscapeDefault; 

     <Trace>::closure_payload(v162, v4, v67, v166);
        
   }
}
#[cfg(test)]
mod tests_rug_241 {
    use super::*;
    use crate::{Config, Atom, Sample, MetadataType, StrCow};
    use std::sync::{Arc, Mutex};
    use std::collections::HashMap;
    use sys_tid;

    #[test]
    fn test_samples_cloned_unsorted() {
        let config = Config::default();
        let v162 = Trace::enabled(config);

        let p0 = Arc::new(Mutex::new(v162));

        Trace::samples_cloned_unsorted(&p0.lock().unwrap());
    }
}
#[cfg(test)]
mod tests_rug_242 {
    use super::*;
    use crate::{Trace, Config, AtomicBool, AtomicOrdering, Mutex, FixedLifoDeque};
    use std::path::Path;

    #[test]
    fn test_rug() {
        let config = Config::default();
        let mut v162 = Trace::enabled(config);

        let mut p0: Vec<Sample> = v162.samples_cloned_sorted();
    }
}


#[cfg(test)]
mod tests_rug_243 {
    use super::*;
    use crate::{Trace, Config, AtomicBool, AtomicOrdering, Mutex, FixedLifoDeque};
    use std::path::Path;
    use std::vec::IntoIter;
    
    #[test]
    fn test_rug() {
        // construct the Trace object
        let config = Config::default();
        let v162 = Trace::enabled(config);

        // construct the Iterator object
        let v168: IntoIter<u32> = vec![1, 6, 8].into_iter();

        // construct the bool value
        let v170: bool = true;

        // call the save function with the constructed variables
        <Trace>::save(&v162, &v168, v170);
    }
}