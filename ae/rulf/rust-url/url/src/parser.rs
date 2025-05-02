use std::error::Error;
use std::fmt::{self, Formatter, Write};
use std::str;
use crate::host::{Host, HostInternal};
use crate::Url;
use form_urlencoded::EncodingOverride;
use percent_encoding::{percent_encode, utf8_percent_encode, AsciiSet, CONTROLS};
/// https://url.spec.whatwg.org/#fragment-percent-encode-set
const FRAGMENT: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');
/// https://url.spec.whatwg.org/#path-percent-encode-set
const PATH: &AsciiSet = &FRAGMENT.add(b'#').add(b'?').add(b'{').add(b'}');
/// https://url.spec.whatwg.org/#userinfo-percent-encode-set
pub(crate) const USERINFO: &AsciiSet = &PATH
    .add(b'/')
    .add(b':')
    .add(b';')
    .add(b'=')
    .add(b'@')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'|');
pub(crate) const PATH_SEGMENT: &AsciiSet = &PATH.add(b'/').add(b'%');
pub(crate) const SPECIAL_PATH_SEGMENT: &AsciiSet = &PATH_SEGMENT.add(b'\\');
const QUERY: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'#').add(b'<').add(b'>');
const SPECIAL_QUERY: &AsciiSet = &QUERY.add(b'\'');
pub type ParseResult<T> = Result<T, ParseError>;
macro_rules! simple_enum_error {
    ($($name:ident => $description:expr,)+) => {
        #[doc = " Errors that can occur during parsing."] #[doc = ""] #[doc =
        " This may be extended in the future so exhaustive matching is"] #[doc =
        " discouraged with an unused variant."] #[allow(clippy::manual_non_exhaustive)]
        #[derive(PartialEq, Eq, Clone, Copy, Debug)] pub enum ParseError { $($name,)+
        #[doc = " Unused variant enable non-exhaustive matching"] #[doc(hidden)]
        __FutureProof, } impl fmt::Display for ParseError { fn fmt(& self, fmt : & mut
        Formatter <'_ >) -> fmt::Result { match * self { $(ParseError::$name => fmt
        .write_str($description),)+ ParseError::__FutureProof => {
        unreachable!("Don't abuse the FutureProof!"); } } } }
    };
}
impl Error for ParseError {}
simple_enum_error! {
    EmptyHost => "empty host", IdnaError => "invalid international domain name",
    InvalidPort => "invalid port number", InvalidIpv4Address => "invalid IPv4 address",
    InvalidIpv6Address => "invalid IPv6 address", InvalidDomainCharacter =>
    "invalid domain character", RelativeUrlWithoutBase => "relative URL without a base",
    RelativeUrlWithCannotBeABaseBase => "relative URL with a cannot-be-a-base base",
    SetHostOnCannotBeABaseUrl => "a cannot-be-a-base URL doesn’t have a host to set",
    Overflow => "URLs more than 4 GB are not supported",
}
impl From<::idna::Errors> for ParseError {
    fn from(_: ::idna::Errors) -> ParseError {
        ParseError::IdnaError
    }
}
macro_rules! syntax_violation_enum {
    ($($name:ident => $description:expr,)+) => {
        #[doc = " Non-fatal syntax violations that can occur during parsing."] #[doc =
        ""] #[doc = " This may be extended in the future so exhaustive matching is"]
        #[doc = " discouraged with an unused variant."]
        #[allow(clippy::manual_non_exhaustive)] #[derive(PartialEq, Eq, Clone, Copy,
        Debug)] pub enum SyntaxViolation { $($name,)+ #[doc =
        " Unused variant enable non-exhaustive matching"] #[doc(hidden)] __FutureProof, }
        impl SyntaxViolation { pub fn description(& self) -> &'static str { match * self
        { $(SyntaxViolation::$name => $description,)+ SyntaxViolation::__FutureProof => {
        unreachable!("Don't abuse the FutureProof!"); } } } }
    };
}
syntax_violation_enum! {
    Backslash => "backslash", C0SpaceIgnored =>
    "leading or trailing control or space character are ignored in URLs",
    EmbeddedCredentials =>
    "embedding authentication information (username or password) \
         in an URL is not recommended",
    ExpectedDoubleSlash => "expected //", ExpectedFileDoubleSlash =>
    "expected // after file:", FileWithHostAndWindowsDrive =>
    "file: with host and Windows drive letter", NonUrlCodePoint => "non-URL code point",
    NullInFragment => "NULL characters are ignored in URL fragment identifiers",
    PercentDecode => "expected 2 hex digits after %", TabOrNewlineIgnored =>
    "tabs or newlines are ignored in URLs", UnencodedAtSign =>
    "unencoded @ sign in username or password",
}
impl fmt::Display for SyntaxViolation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.description(), f)
    }
}
#[derive(Copy, Clone, PartialEq)]
pub enum SchemeType {
    File,
    SpecialNotFile,
    NotSpecial,
}
impl SchemeType {
    pub fn is_special(&self) -> bool {
        !matches!(* self, SchemeType::NotSpecial)
    }
    pub fn is_file(&self) -> bool {
        matches!(* self, SchemeType::File)
    }
    pub fn from(s: &str) -> Self {
        match s {
            "http" | "https" | "ws" | "wss" | "ftp" => SchemeType::SpecialNotFile,
            "file" => SchemeType::File,
            _ => SchemeType::NotSpecial,
        }
    }
}
pub fn default_port(scheme: &str) -> Option<u16> {
    match scheme {
        "http" | "ws" => Some(80),
        "https" | "wss" => Some(443),
        "ftp" => Some(21),
        _ => None,
    }
}
#[derive(Clone)]
pub struct Input<'i> {
    chars: str::Chars<'i>,
}
impl<'i> Input<'i> {
    pub fn new(input: &'i str) -> Self {
        Input::with_log(input, None)
    }
    pub fn no_trim(input: &'i str) -> Self {
        Input { chars: input.chars() }
    }
    pub fn trim_tab_and_newlines(
        original_input: &'i str,
        vfn: Option<&dyn Fn(SyntaxViolation)>,
    ) -> Self {
        let input = original_input.trim_matches(ascii_tab_or_new_line);
        if let Some(vfn) = vfn {
            if input.len() < original_input.len() {
                vfn(SyntaxViolation::C0SpaceIgnored)
            }
            if input.chars().any(|c| matches!(c, '\t' | '\n' | '\r')) {
                vfn(SyntaxViolation::TabOrNewlineIgnored)
            }
        }
        Input { chars: input.chars() }
    }
    pub fn with_log(
        original_input: &'i str,
        vfn: Option<&dyn Fn(SyntaxViolation)>,
    ) -> Self {
        let input = original_input.trim_matches(c0_control_or_space);
        if let Some(vfn) = vfn {
            if input.len() < original_input.len() {
                vfn(SyntaxViolation::C0SpaceIgnored)
            }
            if input.chars().any(|c| matches!(c, '\t' | '\n' | '\r')) {
                vfn(SyntaxViolation::TabOrNewlineIgnored)
            }
        }
        Input { chars: input.chars() }
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.clone().next().is_none()
    }
    #[inline]
    fn starts_with<P: Pattern>(&self, p: P) -> bool {
        p.split_prefix(&mut self.clone())
    }
    #[inline]
    pub fn split_prefix<P: Pattern>(&self, p: P) -> Option<Self> {
        let mut remaining = self.clone();
        if p.split_prefix(&mut remaining) { Some(remaining) } else { None }
    }
    #[inline]
    fn split_first(&self) -> (Option<char>, Self) {
        let mut remaining = self.clone();
        (remaining.next(), remaining)
    }
    #[inline]
    fn count_matching<F: Fn(char) -> bool>(&self, f: F) -> (u32, Self) {
        let mut count = 0;
        let mut remaining = self.clone();
        loop {
            let mut input = remaining.clone();
            if matches!(input.next(), Some(c) if f(c)) {
                remaining = input;
                count += 1;
            } else {
                return (count, remaining);
            }
        }
    }
    #[inline]
    fn next_utf8(&mut self) -> Option<(char, &'i str)> {
        loop {
            let utf8 = self.chars.as_str();
            match self.chars.next() {
                Some(c) => {
                    if !matches!(c, '\t' | '\n' | '\r') {
                        return Some((c, &utf8[..c.len_utf8()]));
                    }
                }
                None => return None,
            }
        }
    }
}
pub trait Pattern {
    fn split_prefix<'i>(self, input: &mut Input<'i>) -> bool;
}
impl Pattern for char {
    fn split_prefix<'i>(self, input: &mut Input<'i>) -> bool {
        input.next() == Some(self)
    }
}
impl<'a> Pattern for &'a str {
    fn split_prefix<'i>(self, input: &mut Input<'i>) -> bool {
        for c in self.chars() {
            if input.next() != Some(c) {
                return false;
            }
        }
        true
    }
}
impl<F: FnMut(char) -> bool> Pattern for F {
    fn split_prefix<'i>(self, input: &mut Input<'i>) -> bool {
        input.next().map_or(false, self)
    }
}
impl<'i> Iterator for Input<'i> {
    type Item = char;
    fn next(&mut self) -> Option<char> {
        self.chars.by_ref().find(|&c| !matches!(c, '\t' | '\n' | '\r'))
    }
}
pub struct Parser<'a> {
    pub serialization: String,
    pub base_url: Option<&'a Url>,
    pub query_encoding_override: EncodingOverride<'a>,
    pub violation_fn: Option<&'a dyn Fn(SyntaxViolation)>,
    pub context: Context,
}
#[derive(PartialEq, Eq, Copy, Clone)]
pub enum Context {
    UrlParser,
    Setter,
    PathSegmentSetter,
}
impl<'a> Parser<'a> {
    fn log_violation(&self, v: SyntaxViolation) {
        if let Some(f) = self.violation_fn {
            f(v)
        }
    }
    fn log_violation_if(&self, v: SyntaxViolation, test: impl FnOnce() -> bool) {
        if let Some(f) = self.violation_fn {
            if test() {
                f(v)
            }
        }
    }
    pub fn for_setter(serialization: String) -> Parser<'a> {
        Parser {
            serialization,
            base_url: None,
            query_encoding_override: None,
            violation_fn: None,
            context: Context::Setter,
        }
    }
    /// https://url.spec.whatwg.org/#concept-basic-url-parser
    pub fn parse_url(mut self, input: &str) -> ParseResult<Url> {
        let input = Input::with_log(input, self.violation_fn);
        if let Ok(remaining) = self.parse_scheme(input.clone()) {
            return self.parse_with_scheme(remaining);
        }
        if let Some(base_url) = self.base_url {
            if input.starts_with('#') {
                self.fragment_only(base_url, input)
            } else if base_url.cannot_be_a_base() {
                Err(ParseError::RelativeUrlWithCannotBeABaseBase)
            } else {
                let scheme_type = SchemeType::from(base_url.scheme());
                if scheme_type.is_file() {
                    self.parse_file(input, scheme_type, Some(base_url))
                } else {
                    self.parse_relative(input, scheme_type, base_url)
                }
            }
        } else {
            Err(ParseError::RelativeUrlWithoutBase)
        }
    }
    pub fn parse_scheme<'i>(&mut self, mut input: Input<'i>) -> Result<Input<'i>, ()> {
        if input.is_empty() || !input.starts_with(ascii_alpha) {
            return Err(());
        }
        debug_assert!(self.serialization.is_empty());
        while let Some(c) = input.next() {
            match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '+' | '-' | '.' => {
                    self.serialization.push(c.to_ascii_lowercase())
                }
                ':' => return Ok(input),
                _ => {
                    self.serialization.clear();
                    return Err(());
                }
            }
        }
        if self.context == Context::Setter {
            Ok(input)
        } else {
            self.serialization.clear();
            Err(())
        }
    }
    fn parse_with_scheme(mut self, input: Input<'_>) -> ParseResult<Url> {
        use crate::SyntaxViolation::{ExpectedDoubleSlash, ExpectedFileDoubleSlash};
        let scheme_end = to_u32(self.serialization.len())?;
        let scheme_type = SchemeType::from(&self.serialization);
        self.serialization.push(':');
        match scheme_type {
            SchemeType::File => {
                self.log_violation_if(
                    ExpectedFileDoubleSlash,
                    || !input.starts_with("//"),
                );
                let base_file_url = self
                    .base_url
                    .and_then(|base| {
                        if base.scheme() == "file" { Some(base) } else { None }
                    });
                self.serialization.clear();
                self.parse_file(input, scheme_type, base_file_url)
            }
            SchemeType::SpecialNotFile => {
                let (slashes_count, remaining) = input
                    .count_matching(|c| matches!(c, '/' | '\\'));
                if let Some(base_url) = self.base_url {
                    if slashes_count < 2
                        && base_url.scheme()
                            == &self.serialization[..scheme_end as usize]
                    {
                        debug_assert!(! base_url.cannot_be_a_base());
                        self.serialization.clear();
                        return self.parse_relative(input, scheme_type, base_url);
                    }
                }
                self.log_violation_if(
                    ExpectedDoubleSlash,
                    || {
                        input
                            .clone()
                            .take_while(|&c| matches!(c, '/' | '\\'))
                            .collect::<String>() != "//"
                    },
                );
                self.after_double_slash(remaining, scheme_type, scheme_end)
            }
            SchemeType::NotSpecial => {
                self.parse_non_special(input, scheme_type, scheme_end)
            }
        }
    }
    /// Scheme other than file, http, https, ws, ws, ftp.
    fn parse_non_special(
        mut self,
        input: Input<'_>,
        scheme_type: SchemeType,
        scheme_end: u32,
    ) -> ParseResult<Url> {
        if let Some(input) = input.split_prefix("//") {
            return self.after_double_slash(input, scheme_type, scheme_end);
        }
        let path_start = to_u32(self.serialization.len())?;
        let username_end = path_start;
        let host_start = path_start;
        let host_end = path_start;
        let host = HostInternal::None;
        let port = None;
        let remaining = if let Some(input) = input.split_prefix('/') {
            let path_start = self.serialization.len();
            self.serialization.push('/');
            self.parse_path(scheme_type, &mut false, path_start, input)
        } else {
            self.parse_cannot_be_a_base_path(input)
        };
        self.with_query_and_fragment(
            scheme_type,
            scheme_end,
            username_end,
            host_start,
            host_end,
            host,
            port,
            path_start,
            remaining,
        )
    }
    fn parse_file(
        mut self,
        input: Input<'_>,
        scheme_type: SchemeType,
        base_file_url: Option<&Url>,
    ) -> ParseResult<Url> {
        use crate::SyntaxViolation::Backslash;
        debug_assert!(self.serialization.is_empty());
        let (first_char, input_after_first_char) = input.split_first();
        if matches!(first_char, Some('/') | Some('\\')) {
            self.log_violation_if(
                SyntaxViolation::Backslash,
                || first_char == Some('\\'),
            );
            let (next_char, input_after_next_char) = input_after_first_char
                .split_first();
            if matches!(next_char, Some('/') | Some('\\')) {
                self.log_violation_if(Backslash, || next_char == Some('\\'));
                self.serialization.push_str("file://");
                let scheme_end = "file".len() as u32;
                let host_start = "file://".len() as u32;
                let (path_start, mut host, remaining) = self
                    .parse_file_host(input_after_next_char)?;
                let mut host_end = to_u32(self.serialization.len())?;
                let mut has_host = !matches!(host, HostInternal::None);
                let remaining = if path_start {
                    self.parse_path_start(SchemeType::File, &mut has_host, remaining)
                } else {
                    let path_start = self.serialization.len();
                    self.serialization.push('/');
                    self.parse_path(
                        SchemeType::File,
                        &mut has_host,
                        path_start,
                        remaining,
                    )
                };
                if !has_host {
                    self.serialization.drain(host_start as usize..host_end as usize);
                    host_end = host_start;
                    host = HostInternal::None;
                }
                let (query_start, fragment_start) = self
                    .parse_query_and_fragment(scheme_type, scheme_end, remaining)?;
                return Ok(Url {
                    serialization: self.serialization,
                    scheme_end,
                    username_end: host_start,
                    host_start,
                    host_end,
                    host,
                    port: None,
                    path_start: host_end,
                    query_start,
                    fragment_start,
                });
            } else {
                self.serialization.push_str("file://");
                let scheme_end = "file".len() as u32;
                let host_start = "file://".len();
                let mut host_end = host_start;
                let mut host = HostInternal::None;
                if !starts_with_windows_drive_letter_segment(&input_after_first_char) {
                    if let Some(base_url) = base_file_url {
                        let first_segment = base_url
                            .path_segments()
                            .unwrap()
                            .next()
                            .unwrap();
                        if is_normalized_windows_drive_letter(first_segment) {
                            self.serialization.push('/');
                            self.serialization.push_str(first_segment);
                        } else if let Some(host_str) = base_url.host_str() {
                            self.serialization.push_str(host_str);
                            host_end = self.serialization.len();
                            host = base_url.host;
                        }
                    }
                }
                let parse_path_input = if let Some(c) = first_char {
                    if c == '/' || c == '\\' || c == '?' || c == '#' {
                        input
                    } else {
                        input_after_first_char
                    }
                } else {
                    input_after_first_char
                };
                let remaining = self
                    .parse_path(
                        SchemeType::File,
                        &mut false,
                        host_end,
                        parse_path_input,
                    );
                let host_start = host_start as u32;
                let (query_start, fragment_start) = self
                    .parse_query_and_fragment(scheme_type, scheme_end, remaining)?;
                let host_end = host_end as u32;
                return Ok(Url {
                    serialization: self.serialization,
                    scheme_end,
                    username_end: host_start,
                    host_start,
                    host_end,
                    host,
                    port: None,
                    path_start: host_end,
                    query_start,
                    fragment_start,
                });
            }
        }
        if let Some(base_url) = base_file_url {
            match first_char {
                None => {
                    let before_fragment = match base_url.fragment_start {
                        Some(i) => &base_url.serialization[..i as usize],
                        None => &*base_url.serialization,
                    };
                    self.serialization.push_str(before_fragment);
                    Ok(Url {
                        serialization: self.serialization,
                        fragment_start: None,
                        ..*base_url
                    })
                }
                Some('?') => {
                    let before_query = match (
                        base_url.query_start,
                        base_url.fragment_start,
                    ) {
                        (None, None) => &*base_url.serialization,
                        (Some(i), _) | (None, Some(i)) => base_url.slice(..i),
                    };
                    self.serialization.push_str(before_query);
                    let (query_start, fragment_start) = self
                        .parse_query_and_fragment(
                            scheme_type,
                            base_url.scheme_end,
                            input,
                        )?;
                    Ok(Url {
                        serialization: self.serialization,
                        query_start,
                        fragment_start,
                        ..*base_url
                    })
                }
                Some('#') => self.fragment_only(base_url, input),
                _ => {
                    if !starts_with_windows_drive_letter_segment(&input) {
                        let before_query = match (
                            base_url.query_start,
                            base_url.fragment_start,
                        ) {
                            (None, None) => &*base_url.serialization,
                            (Some(i), _) | (None, Some(i)) => base_url.slice(..i),
                        };
                        self.serialization.push_str(before_query);
                        self.shorten_path(
                            SchemeType::File,
                            base_url.path_start as usize,
                        );
                        let remaining = self
                            .parse_path(
                                SchemeType::File,
                                &mut true,
                                base_url.path_start as usize,
                                input,
                            );
                        self.with_query_and_fragment(
                            SchemeType::File,
                            base_url.scheme_end,
                            base_url.username_end,
                            base_url.host_start,
                            base_url.host_end,
                            base_url.host,
                            base_url.port,
                            base_url.path_start,
                            remaining,
                        )
                    } else {
                        self.serialization.push_str("file:///");
                        let scheme_end = "file".len() as u32;
                        let path_start = "file://".len();
                        let remaining = self
                            .parse_path(SchemeType::File, &mut false, path_start, input);
                        let (query_start, fragment_start) = self
                            .parse_query_and_fragment(
                                SchemeType::File,
                                scheme_end,
                                remaining,
                            )?;
                        let path_start = path_start as u32;
                        Ok(Url {
                            serialization: self.serialization,
                            scheme_end,
                            username_end: path_start,
                            host_start: path_start,
                            host_end: path_start,
                            host: HostInternal::None,
                            port: None,
                            path_start,
                            query_start,
                            fragment_start,
                        })
                    }
                }
            }
        } else {
            self.serialization.push_str("file:///");
            let scheme_end = "file".len() as u32;
            let path_start = "file://".len();
            let remaining = self
                .parse_path(SchemeType::File, &mut false, path_start, input);
            let (query_start, fragment_start) = self
                .parse_query_and_fragment(SchemeType::File, scheme_end, remaining)?;
            let path_start = path_start as u32;
            Ok(Url {
                serialization: self.serialization,
                scheme_end,
                username_end: path_start,
                host_start: path_start,
                host_end: path_start,
                host: HostInternal::None,
                port: None,
                path_start,
                query_start,
                fragment_start,
            })
        }
    }
    fn parse_relative(
        mut self,
        input: Input<'_>,
        scheme_type: SchemeType,
        base_url: &Url,
    ) -> ParseResult<Url> {
        debug_assert!(self.serialization.is_empty());
        let (first_char, input_after_first_char) = input.split_first();
        match first_char {
            None => {
                let before_fragment = match base_url.fragment_start {
                    Some(i) => &base_url.serialization[..i as usize],
                    None => &*base_url.serialization,
                };
                self.serialization.push_str(before_fragment);
                Ok(Url {
                    serialization: self.serialization,
                    fragment_start: None,
                    ..*base_url
                })
            }
            Some('?') => {
                let before_query = match (
                    base_url.query_start,
                    base_url.fragment_start,
                ) {
                    (None, None) => &*base_url.serialization,
                    (Some(i), _) | (None, Some(i)) => base_url.slice(..i),
                };
                self.serialization.push_str(before_query);
                let (query_start, fragment_start) = self
                    .parse_query_and_fragment(scheme_type, base_url.scheme_end, input)?;
                Ok(Url {
                    serialization: self.serialization,
                    query_start,
                    fragment_start,
                    ..*base_url
                })
            }
            Some('#') => self.fragment_only(base_url, input),
            Some('/') | Some('\\') => {
                let (slashes_count, remaining) = input
                    .count_matching(|c| matches!(c, '/' | '\\'));
                if slashes_count >= 2 {
                    self.log_violation_if(
                        SyntaxViolation::ExpectedDoubleSlash,
                        || {
                            input
                                .clone()
                                .take_while(|&c| matches!(c, '/' | '\\'))
                                .collect::<String>() != "//"
                        },
                    );
                    let scheme_end = base_url.scheme_end;
                    debug_assert!(base_url.byte_at(scheme_end) == b':');
                    self.serialization.push_str(base_url.slice(..scheme_end + 1));
                    if let Some(after_prefix) = input.split_prefix("//") {
                        return self
                            .after_double_slash(after_prefix, scheme_type, scheme_end);
                    }
                    return self.after_double_slash(remaining, scheme_type, scheme_end);
                }
                let path_start = base_url.path_start;
                self.serialization.push_str(base_url.slice(..path_start));
                self.serialization.push('/');
                let remaining = self
                    .parse_path(
                        scheme_type,
                        &mut true,
                        path_start as usize,
                        input_after_first_char,
                    );
                self.with_query_and_fragment(
                    scheme_type,
                    base_url.scheme_end,
                    base_url.username_end,
                    base_url.host_start,
                    base_url.host_end,
                    base_url.host,
                    base_url.port,
                    base_url.path_start,
                    remaining,
                )
            }
            _ => {
                let before_query = match (
                    base_url.query_start,
                    base_url.fragment_start,
                ) {
                    (None, None) => &*base_url.serialization,
                    (Some(i), _) | (None, Some(i)) => base_url.slice(..i),
                };
                self.serialization.push_str(before_query);
                self.pop_path(scheme_type, base_url.path_start as usize);
                if self.serialization.len() == base_url.path_start as usize
                    && (SchemeType::from(base_url.scheme()).is_special()
                        || !input.is_empty())
                {
                    self.serialization.push('/');
                }
                let remaining = match input.split_first() {
                    (Some('/'), remaining) => {
                        self
                            .parse_path(
                                scheme_type,
                                &mut true,
                                base_url.path_start as usize,
                                remaining,
                            )
                    }
                    _ => {
                        self
                            .parse_path(
                                scheme_type,
                                &mut true,
                                base_url.path_start as usize,
                                input,
                            )
                    }
                };
                self.with_query_and_fragment(
                    scheme_type,
                    base_url.scheme_end,
                    base_url.username_end,
                    base_url.host_start,
                    base_url.host_end,
                    base_url.host,
                    base_url.port,
                    base_url.path_start,
                    remaining,
                )
            }
        }
    }
    fn after_double_slash(
        mut self,
        input: Input<'_>,
        scheme_type: SchemeType,
        scheme_end: u32,
    ) -> ParseResult<Url> {
        self.serialization.push('/');
        self.serialization.push('/');
        let before_authority = self.serialization.len();
        let (username_end, remaining) = self.parse_userinfo(input, scheme_type)?;
        let has_authority = before_authority != self.serialization.len();
        let host_start = to_u32(self.serialization.len())?;
        let (host_end, host, port, remaining) = self
            .parse_host_and_port(remaining, scheme_end, scheme_type)?;
        if host == HostInternal::None && has_authority {
            return Err(ParseError::EmptyHost);
        }
        let path_start = to_u32(self.serialization.len())?;
        let remaining = self.parse_path_start(scheme_type, &mut true, remaining);
        self.with_query_and_fragment(
            scheme_type,
            scheme_end,
            username_end,
            host_start,
            host_end,
            host,
            port,
            path_start,
            remaining,
        )
    }
    /// Return (username_end, remaining)
    fn parse_userinfo<'i>(
        &mut self,
        mut input: Input<'i>,
        scheme_type: SchemeType,
    ) -> ParseResult<(u32, Input<'i>)> {
        let mut last_at = None;
        let mut remaining = input.clone();
        let mut char_count = 0;
        while let Some(c) = remaining.next() {
            match c {
                '@' => {
                    if last_at.is_some() {
                        self.log_violation(SyntaxViolation::UnencodedAtSign)
                    } else {
                        self.log_violation(SyntaxViolation::EmbeddedCredentials)
                    }
                    last_at = Some((char_count, remaining.clone()));
                }
                '/' | '?' | '#' => break,
                '\\' if scheme_type.is_special() => break,
                _ => {}
            }
            char_count += 1;
        }
        let (mut userinfo_char_count, remaining) = match last_at {
            None => return Ok((to_u32(self.serialization.len())?, input)),
            Some((0, remaining)) => {
                if let (Some(c), _) = remaining.split_first() {
                    if c == '/' || c == '?' || c == '#'
                        || (scheme_type.is_special() && c == '\\')
                    {
                        return Err(ParseError::EmptyHost);
                    }
                }
                return Ok((to_u32(self.serialization.len())?, remaining));
            }
            Some(x) => x,
        };
        let mut username_end = None;
        let mut has_password = false;
        let mut has_username = false;
        while userinfo_char_count > 0 {
            let (c, utf8_c) = input.next_utf8().unwrap();
            userinfo_char_count -= 1;
            if c == ':' && username_end.is_none() {
                username_end = Some(to_u32(self.serialization.len())?);
                if userinfo_char_count > 0 {
                    self.serialization.push(':');
                    has_password = true;
                }
            } else {
                if !has_password {
                    has_username = true;
                }
                self.check_url_code_point(c, &input);
                self.serialization.extend(utf8_percent_encode(utf8_c, USERINFO));
            }
        }
        let username_end = match username_end {
            Some(i) => i,
            None => to_u32(self.serialization.len())?,
        };
        if has_username || has_password {
            self.serialization.push('@');
        }
        Ok((username_end, remaining))
    }
    fn parse_host_and_port<'i>(
        &mut self,
        input: Input<'i>,
        scheme_end: u32,
        scheme_type: SchemeType,
    ) -> ParseResult<(u32, HostInternal, Option<u16>, Input<'i>)> {
        let (host, remaining) = Parser::parse_host(input, scheme_type)?;
        write!(& mut self.serialization, "{}", host).unwrap();
        let host_end = to_u32(self.serialization.len())?;
        if let Host::Domain(h) = &host {
            if h.is_empty() {
                if remaining.starts_with(":") {
                    return Err(ParseError::EmptyHost);
                }
                if scheme_type.is_special() {
                    return Err(ParseError::EmptyHost);
                }
            }
        }
        let (port, remaining) = if let Some(remaining) = remaining.split_prefix(':') {
            let scheme = || default_port(&self.serialization[..scheme_end as usize]);
            Parser::parse_port(remaining, scheme, self.context)?
        } else {
            (None, remaining)
        };
        if let Some(port) = port {
            write!(& mut self.serialization, ":{}", port).unwrap()
        }
        Ok((host_end, host.into(), port, remaining))
    }
    pub fn parse_host(
        mut input: Input<'_>,
        scheme_type: SchemeType,
    ) -> ParseResult<(Host<String>, Input<'_>)> {
        if scheme_type.is_file() {
            return Parser::get_file_host(input);
        }
        let input_str = input.chars.as_str();
        let mut inside_square_brackets = false;
        let mut has_ignored_chars = false;
        let mut non_ignored_chars = 0;
        let mut bytes = 0;
        for c in input_str.chars() {
            match c {
                ':' if !inside_square_brackets => break,
                '\\' if scheme_type.is_special() => break,
                '/' | '?' | '#' => break,
                '\t' | '\n' | '\r' => {
                    has_ignored_chars = true;
                }
                '[' => {
                    inside_square_brackets = true;
                    non_ignored_chars += 1;
                }
                ']' => {
                    inside_square_brackets = false;
                    non_ignored_chars += 1;
                }
                _ => non_ignored_chars += 1,
            }
            bytes += c.len_utf8();
        }
        let replaced: String;
        let host_str;
        {
            let host_input = input.by_ref().take(non_ignored_chars);
            if has_ignored_chars {
                replaced = host_input.collect();
                host_str = &*replaced;
            } else {
                for _ in host_input {}
                host_str = &input_str[..bytes];
            }
        }
        if scheme_type == SchemeType::SpecialNotFile && host_str.is_empty() {
            return Err(ParseError::EmptyHost);
        }
        if !scheme_type.is_special() {
            let host = Host::parse_opaque(host_str)?;
            return Ok((host, input));
        }
        let host = Host::parse(host_str)?;
        Ok((host, input))
    }
    fn get_file_host(input: Input<'_>) -> ParseResult<(Host<String>, Input<'_>)> {
        let (_, host_str, remaining) = Parser::file_host(input)?;
        let host = match Host::parse(&host_str)? {
            Host::Domain(ref d) if d == "localhost" => Host::Domain("".to_string()),
            host => host,
        };
        Ok((host, remaining))
    }
    fn parse_file_host<'i>(
        &mut self,
        input: Input<'i>,
    ) -> ParseResult<(bool, HostInternal, Input<'i>)> {
        let has_host;
        let (_, host_str, remaining) = Parser::file_host(input)?;
        let host = if host_str.is_empty() {
            has_host = false;
            HostInternal::None
        } else {
            match Host::parse(&host_str)? {
                Host::Domain(ref d) if d == "localhost" => {
                    has_host = false;
                    HostInternal::None
                }
                host => {
                    write!(& mut self.serialization, "{}", host).unwrap();
                    has_host = true;
                    host.into()
                }
            }
        };
        Ok((has_host, host, remaining))
    }
    pub fn file_host<'i>(input: Input<'i>) -> ParseResult<(bool, String, Input<'i>)> {
        let input_str = input.chars.as_str();
        let mut has_ignored_chars = false;
        let mut non_ignored_chars = 0;
        let mut bytes = 0;
        for c in input_str.chars() {
            match c {
                '/' | '\\' | '?' | '#' => break,
                '\t' | '\n' | '\r' => has_ignored_chars = true,
                _ => non_ignored_chars += 1,
            }
            bytes += c.len_utf8();
        }
        let replaced: String;
        let host_str;
        let mut remaining = input.clone();
        {
            let host_input = remaining.by_ref().take(non_ignored_chars);
            if has_ignored_chars {
                replaced = host_input.collect();
                host_str = &*replaced;
            } else {
                for _ in host_input {}
                host_str = &input_str[..bytes];
            }
        }
        if is_windows_drive_letter(host_str) {
            return Ok((false, "".to_string(), input));
        }
        Ok((true, host_str.to_string(), remaining))
    }
    pub fn parse_port<P>(
        mut input: Input<'_>,
        default_port: P,
        context: Context,
    ) -> ParseResult<(Option<u16>, Input<'_>)>
    where
        P: Fn() -> Option<u16>,
    {
        let mut port: u32 = 0;
        let mut has_any_digit = false;
        while let (Some(c), remaining) = input.split_first() {
            if let Some(digit) = c.to_digit(10) {
                port = port * 10 + digit;
                if port > ::std::u16::MAX as u32 {
                    return Err(ParseError::InvalidPort);
                }
                has_any_digit = true;
            } else if context == Context::UrlParser
                && !matches!(c, '/' | '\\' | '?' | '#')
            {
                return Err(ParseError::InvalidPort);
            } else {
                break;
            }
            input = remaining;
        }
        let mut opt_port = Some(port as u16);
        if !has_any_digit || opt_port == default_port() {
            opt_port = None;
        }
        Ok((opt_port, input))
    }
    pub fn parse_path_start<'i>(
        &mut self,
        scheme_type: SchemeType,
        has_host: &mut bool,
        input: Input<'i>,
    ) -> Input<'i> {
        let path_start = self.serialization.len();
        let (maybe_c, remaining) = input.split_first();
        if scheme_type.is_special() {
            if maybe_c == Some('\\') {
                self.log_violation(SyntaxViolation::Backslash);
            }
            if !self.serialization.ends_with('/') {
                self.serialization.push('/');
                if maybe_c == Some('/') || maybe_c == Some('\\') {
                    return self.parse_path(scheme_type, has_host, path_start, remaining);
                }
            }
            return self.parse_path(scheme_type, has_host, path_start, input);
        } else if maybe_c == Some('?') || maybe_c == Some('#') {
            return input;
        }
        if maybe_c != None && maybe_c != Some('/') {
            self.serialization.push('/');
        }
        self.parse_path(scheme_type, has_host, path_start, input)
    }
    pub fn parse_path<'i>(
        &mut self,
        scheme_type: SchemeType,
        has_host: &mut bool,
        path_start: usize,
        mut input: Input<'i>,
    ) -> Input<'i> {
        loop {
            let segment_start = self.serialization.len();
            let mut ends_with_slash = false;
            loop {
                let input_before_c = input.clone();
                let (c, utf8_c) = if let Some(x) = input.next_utf8() {
                    x
                } else {
                    break;
                };
                match c {
                    '/' if self.context != Context::PathSegmentSetter => {
                        self.serialization.push(c);
                        ends_with_slash = true;
                        break;
                    }
                    '\\' if self.context != Context::PathSegmentSetter
                        && scheme_type.is_special() => {
                        self.log_violation(SyntaxViolation::Backslash);
                        self.serialization.push('/');
                        ends_with_slash = true;
                        break;
                    }
                    '?' | '#' if self.context == Context::UrlParser => {
                        input = input_before_c;
                        break;
                    }
                    _ => {
                        self.check_url_code_point(c, &input);
                        if self.context == Context::PathSegmentSetter {
                            if scheme_type.is_special() {
                                self.serialization
                                    .extend(utf8_percent_encode(utf8_c, SPECIAL_PATH_SEGMENT));
                            } else {
                                self.serialization
                                    .extend(utf8_percent_encode(utf8_c, PATH_SEGMENT));
                            }
                        } else {
                            self.serialization.extend(utf8_percent_encode(utf8_c, PATH));
                        }
                    }
                }
            }
            let before_slash_string = if ends_with_slash {
                self.serialization[segment_start..self.serialization.len() - 1]
                    .to_owned()
            } else {
                self.serialization[segment_start..self.serialization.len()].to_owned()
            };
            let segment_before_slash: &str = &before_slash_string;
            match segment_before_slash {
                ".." | "%2e%2e" | "%2e%2E" | "%2E%2e" | "%2E%2E" | "%2e." | "%2E."
                | ".%2e" | ".%2E" => {
                    debug_assert!(
                        self.serialization.as_bytes() [segment_start - 1] == b'/'
                    );
                    self.serialization.truncate(segment_start);
                    if self.serialization.ends_with('/')
                        && Parser::last_slash_can_be_removed(
                            &self.serialization,
                            path_start,
                        )
                    {
                        self.serialization.pop();
                    }
                    self.shorten_path(scheme_type, path_start);
                    if ends_with_slash && !self.serialization.ends_with('/') {
                        self.serialization.push('/');
                    }
                }
                "." | "%2e" | "%2E" => {
                    self.serialization.truncate(segment_start);
                    if !self.serialization.ends_with('/') {
                        self.serialization.push('/');
                    }
                }
                _ => {
                    if scheme_type.is_file()
                        && is_windows_drive_letter(segment_before_slash)
                    {
                        if let Some(c) = segment_before_slash.chars().next() {
                            self.serialization.truncate(segment_start);
                            self.serialization.push(c);
                            self.serialization.push(':');
                            if ends_with_slash {
                                self.serialization.push('/');
                            }
                        }
                        if *has_host {
                            self.log_violation(
                                SyntaxViolation::FileWithHostAndWindowsDrive,
                            );
                            *has_host = false;
                        }
                    }
                }
            }
            if !ends_with_slash {
                break;
            }
        }
        if scheme_type.is_file() {
            let path = self.serialization.split_off(path_start);
            self.serialization.push('/');
            self.serialization.push_str(&path.trim_start_matches('/'));
        }
        input
    }
    fn last_slash_can_be_removed(serialization: &str, path_start: usize) -> bool {
        let url_before_segment = &serialization[..serialization.len() - 1];
        if let Some(segment_before_start) = url_before_segment.rfind('/') {
            segment_before_start >= path_start
                && !path_starts_with_windows_drive_letter(
                    &serialization[segment_before_start..],
                )
        } else {
            false
        }
    }
    /// https://url.spec.whatwg.org/#shorten-a-urls-path
    fn shorten_path(&mut self, scheme_type: SchemeType, path_start: usize) {
        if self.serialization.len() == path_start {
            return;
        }
        if scheme_type.is_file()
            && is_normalized_windows_drive_letter(&self.serialization[path_start..])
        {
            return;
        }
        self.pop_path(scheme_type, path_start);
    }
    /// https://url.spec.whatwg.org/#pop-a-urls-path
    fn pop_path(&mut self, scheme_type: SchemeType, path_start: usize) {
        if self.serialization.len() > path_start {
            let slash_position = self.serialization[path_start..].rfind('/').unwrap();
            let segment_start = path_start + slash_position + 1;
            if !(scheme_type.is_file()
                && is_normalized_windows_drive_letter(
                    &self.serialization[segment_start..],
                ))
            {
                self.serialization.truncate(segment_start);
            }
        }
    }
    pub fn parse_cannot_be_a_base_path<'i>(
        &mut self,
        mut input: Input<'i>,
    ) -> Input<'i> {
        loop {
            let input_before_c = input.clone();
            match input.next_utf8() {
                Some(('?', _))
                | Some(('#', _)) if self.context == Context::UrlParser => {
                    return input_before_c;
                }
                Some((c, utf8_c)) => {
                    self.check_url_code_point(c, &input);
                    self.serialization.extend(utf8_percent_encode(utf8_c, CONTROLS));
                }
                None => return input,
            }
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn with_query_and_fragment(
        mut self,
        scheme_type: SchemeType,
        scheme_end: u32,
        username_end: u32,
        host_start: u32,
        host_end: u32,
        host: HostInternal,
        port: Option<u16>,
        path_start: u32,
        remaining: Input<'_>,
    ) -> ParseResult<Url> {
        let (query_start, fragment_start) = self
            .parse_query_and_fragment(scheme_type, scheme_end, remaining)?;
        Ok(Url {
            serialization: self.serialization,
            scheme_end,
            username_end,
            host_start,
            host_end,
            host,
            port,
            path_start,
            query_start,
            fragment_start,
        })
    }
    /// Return (query_start, fragment_start)
    fn parse_query_and_fragment(
        &mut self,
        scheme_type: SchemeType,
        scheme_end: u32,
        mut input: Input<'_>,
    ) -> ParseResult<(Option<u32>, Option<u32>)> {
        let mut query_start = None;
        match input.next() {
            Some('#') => {}
            Some('?') => {
                query_start = Some(to_u32(self.serialization.len())?);
                self.serialization.push('?');
                let remaining = self.parse_query(scheme_type, scheme_end, input);
                if let Some(remaining) = remaining {
                    input = remaining
                } else {
                    return Ok((query_start, None));
                }
            }
            None => return Ok((None, None)),
            _ => {
                panic!(
                    "Programming error. parse_query_and_fragment() called without ? or #"
                )
            }
        }
        let fragment_start = to_u32(self.serialization.len())?;
        self.serialization.push('#');
        self.parse_fragment(input);
        Ok((query_start, Some(fragment_start)))
    }
    pub fn parse_query<'i>(
        &mut self,
        scheme_type: SchemeType,
        scheme_end: u32,
        mut input: Input<'i>,
    ) -> Option<Input<'i>> {
        let mut query = String::new();
        let mut remaining = None;
        while let Some(c) = input.next() {
            if c == '#' && self.context == Context::UrlParser {
                remaining = Some(input);
                break;
            } else {
                self.check_url_code_point(c, &input);
                query.push(c);
            }
        }
        let encoding = match &self.serialization[..scheme_end as usize] {
            "http" | "https" | "file" | "ftp" => self.query_encoding_override,
            _ => None,
        };
        let query_bytes = if let Some(o) = encoding {
            o(&query)
        } else {
            query.as_bytes().into()
        };
        let set = if scheme_type.is_special() { SPECIAL_QUERY } else { QUERY };
        self.serialization.extend(percent_encode(&query_bytes, set));
        remaining
    }
    fn fragment_only(
        mut self,
        base_url: &Url,
        mut input: Input<'_>,
    ) -> ParseResult<Url> {
        let before_fragment = match base_url.fragment_start {
            Some(i) => base_url.slice(..i),
            None => &*base_url.serialization,
        };
        debug_assert!(self.serialization.is_empty());
        self.serialization.reserve(before_fragment.len() + input.chars.as_str().len());
        self.serialization.push_str(before_fragment);
        self.serialization.push('#');
        let next = input.next();
        debug_assert!(next == Some('#'));
        self.parse_fragment(input);
        Ok(Url {
            serialization: self.serialization,
            fragment_start: Some(to_u32(before_fragment.len())?),
            ..*base_url
        })
    }
    pub fn parse_fragment(&mut self, mut input: Input<'_>) {
        while let Some((c, utf8_c)) = input.next_utf8() {
            if c == '\0' {
                self.log_violation(SyntaxViolation::NullInFragment)
            } else {
                self.check_url_code_point(c, &input);
            }
            self.serialization.extend(utf8_percent_encode(utf8_c, FRAGMENT));
        }
    }
    fn check_url_code_point(&self, c: char, input: &Input<'_>) {
        if let Some(vfn) = self.violation_fn {
            if c == '%' {
                let mut input = input.clone();
                if !matches!(
                    (input.next(), input.next()), (Some(a), Some(b)) if
                    is_ascii_hex_digit(a) && is_ascii_hex_digit(b)
                ) {
                    vfn(SyntaxViolation::PercentDecode)
                }
            } else if !is_url_code_point(c) {
                vfn(SyntaxViolation::NonUrlCodePoint)
            }
        }
    }
}
#[inline]
fn is_ascii_hex_digit(c: char) -> bool {
    matches!(c, 'a'..='f' | 'A'..='F' | '0'..='9')
}
#[inline]
fn is_url_code_point(c: char) -> bool {
    matches!(
        c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '!' | '$' | '&' | '\'' | '(' | ')' | '*' |
        '+' | ',' | '-' | '.' | '/' | ':' | ';' | '=' | '?' | '@' | '_' | '~' | '\u{A0}'
        ..='\u{D7FF}' | '\u{E000}'..='\u{FDCF}' | '\u{FDF0}'..='\u{FFFD}' | '\u{10000}'
        ..='\u{1FFFD}' | '\u{20000}'..='\u{2FFFD}' | '\u{30000}'..='\u{3FFFD}' |
        '\u{40000}'..='\u{4FFFD}' | '\u{50000}'..='\u{5FFFD}' | '\u{60000}'..='\u{6FFFD}'
        | '\u{70000}'..='\u{7FFFD}' | '\u{80000}'..='\u{8FFFD}' | '\u{90000}'
        ..='\u{9FFFD}' | '\u{A0000}'..='\u{AFFFD}' | '\u{B0000}'..='\u{BFFFD}' |
        '\u{C0000}'..='\u{CFFFD}' | '\u{D0000}'..='\u{DFFFD}' | '\u{E1000}'..='\u{EFFFD}'
        | '\u{F0000}'..='\u{FFFFD}' | '\u{100000}'..='\u{10FFFD}'
    )
}
/// https://url.spec.whatwg.org/#c0-controls-and-space
#[inline]
fn c0_control_or_space(ch: char) -> bool {
    ch <= ' '
}
/// https://infra.spec.whatwg.org/#ascii-tab-or-newline
#[inline]
fn ascii_tab_or_new_line(ch: char) -> bool {
    matches!(ch, '\t' | '\r' | '\n')
}
/// https://url.spec.whatwg.org/#ascii-alpha
#[inline]
pub fn ascii_alpha(ch: char) -> bool {
    matches!(ch, 'a'..='z' | 'A'..='Z')
}
#[inline]
pub fn to_u32(i: usize) -> ParseResult<u32> {
    if i <= ::std::u32::MAX as usize { Ok(i as u32) } else { Err(ParseError::Overflow) }
}
fn is_normalized_windows_drive_letter(segment: &str) -> bool {
    is_windows_drive_letter(segment) && segment.as_bytes()[1] == b':'
}
/// Wether the scheme is file:, the path has a single segment, and that segment
/// is a Windows drive letter
#[inline]
pub fn is_windows_drive_letter(segment: &str) -> bool {
    segment.len() == 2 && starts_with_windows_drive_letter(segment)
}
/// Wether path starts with a root slash
/// and a windows drive letter eg: "/c:" or "/a:/"
fn path_starts_with_windows_drive_letter(s: &str) -> bool {
    if let Some(c) = s.as_bytes().get(0) {
        matches!(c, b'/' | b'\\' | b'?' | b'#')
            && starts_with_windows_drive_letter(&s[1..])
    } else {
        false
    }
}
fn starts_with_windows_drive_letter(s: &str) -> bool {
    s.len() >= 2 && ascii_alpha(s.as_bytes()[0] as char)
        && matches!(s.as_bytes() [1], b':' | b'|')
        && (s.len() == 2 || matches!(s.as_bytes() [2], b'/' | b'\\' | b'?' | b'#'))
}
/// https://url.spec.whatwg.org/#start-with-a-windows-drive-letter
fn starts_with_windows_drive_letter_segment(input: &Input<'_>) -> bool {
    let mut input = input.clone();
    match (input.next(), input.next(), input.next()) {
        (
            Some(a),
            Some(b),
            Some(c),
        ) if ascii_alpha(a) && matches!(b, ':' | '|')
            && matches!(c, '/' | '\\' | '?' | '#') => true,
        (Some(a), Some(b), None) if ascii_alpha(a) && matches!(b, ':' | '|') => true,
        _ => false,
    }
}
#[cfg(test)]
mod tests_llm_16_49 {
    use crate::parser::Input;
    #[test]
    fn test_count_matching() {
        let _rug_st_tests_llm_16_49_rrrruuuugggg_test_count_matching = 0;
        let rug_fuzz_0 = "Hello, World!";
        let rug_fuzz_1 = "12345";
        let rug_fuzz_2 = "^^^abc";
        let input = Input::new(rug_fuzz_0);
        let f = |c: char| c.is_ascii_alphabetic();
        let (count, remaining) = input.count_matching(f);
        debug_assert_eq!(count, 5);
        debug_assert_eq!(remaining.collect:: < String > (), ", World!");
        let input = Input::new(rug_fuzz_1);
        let f = |c: char| c.is_numeric();
        let (count, remaining) = input.count_matching(f);
        debug_assert_eq!(count, 5);
        debug_assert_eq!(remaining.collect:: < String > (), "");
        let input = Input::new(rug_fuzz_2);
        let f = |c: char| c.is_ascii_punctuation();
        let (count, remaining) = input.count_matching(f);
        debug_assert_eq!(count, 3);
        debug_assert_eq!(remaining.collect:: < String > (), "abc");
        let _rug_ed_tests_llm_16_49_rrrruuuugggg_test_count_matching = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_50 {
    use super::*;
    use crate::*;
    #[test]
    fn test_is_empty_with_empty_input() {
        let _rug_st_tests_llm_16_50_rrrruuuugggg_test_is_empty_with_empty_input = 0;
        let rug_fuzz_0 = "";
        let input = Input::new(rug_fuzz_0);
        debug_assert_eq!(input.is_empty(), true);
        let _rug_ed_tests_llm_16_50_rrrruuuugggg_test_is_empty_with_empty_input = 0;
    }
    #[test]
    fn test_is_empty_with_non_empty_input() {
        let _rug_st_tests_llm_16_50_rrrruuuugggg_test_is_empty_with_non_empty_input = 0;
        let rug_fuzz_0 = "https://example.com";
        let input = Input::new(rug_fuzz_0);
        debug_assert_eq!(input.is_empty(), false);
        let _rug_ed_tests_llm_16_50_rrrruuuugggg_test_is_empty_with_non_empty_input = 0;
    }
    #[test]
    fn test_is_empty_with_input_only_containing_spaces() {
        let _rug_st_tests_llm_16_50_rrrruuuugggg_test_is_empty_with_input_only_containing_spaces = 0;
        let rug_fuzz_0 = "    ";
        let input = Input::new(rug_fuzz_0);
        debug_assert_eq!(input.is_empty(), true);
        let _rug_ed_tests_llm_16_50_rrrruuuugggg_test_is_empty_with_input_only_containing_spaces = 0;
    }
    #[test]
    fn test_is_empty_with_input_containing_tabs_and_newlines() {
        let _rug_st_tests_llm_16_50_rrrruuuugggg_test_is_empty_with_input_containing_tabs_and_newlines = 0;
        let rug_fuzz_0 = "\t\n";
        let input = Input::new(rug_fuzz_0);
        debug_assert_eq!(input.is_empty(), true);
        let _rug_ed_tests_llm_16_50_rrrruuuugggg_test_is_empty_with_input_containing_tabs_and_newlines = 0;
    }
    #[test]
    fn test_is_empty_with_input_containing_tabs_and_newlines_and_text() {
        let _rug_st_tests_llm_16_50_rrrruuuugggg_test_is_empty_with_input_containing_tabs_and_newlines_and_text = 0;
        let rug_fuzz_0 = "\t\nHello, World!\n\n";
        let input = Input::new(rug_fuzz_0);
        debug_assert_eq!(input.is_empty(), false);
        let _rug_ed_tests_llm_16_50_rrrruuuugggg_test_is_empty_with_input_containing_tabs_and_newlines_and_text = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_51 {
    use super::*;
    use crate::*;
    #[test]
    fn test_new() {
        let _rug_st_tests_llm_16_51_rrrruuuugggg_test_new = 0;
        let rug_fuzz_0 = "http://example.com";
        let input = rug_fuzz_0;
        let parser_input = Input::new(input);
        debug_assert_eq!(parser_input.chars.as_str(), input);
        let _rug_ed_tests_llm_16_51_rrrruuuugggg_test_new = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_52 {
    use super::*;
    use crate::*;
    #[test]
    fn test_next_utf8() {
        let _rug_st_tests_llm_16_52_rrrruuuugggg_test_next_utf8 = 0;
        let rug_fuzz_0 = "Hello, 世界";
        let input_str = rug_fuzz_0;
        let mut input = Input::new(input_str);
        debug_assert_eq!(input.next_utf8(), Some(('H', "H")));
        debug_assert_eq!(input.next_utf8(), Some(('e', "e")));
        debug_assert_eq!(input.next_utf8(), Some(('l', "l")));
        debug_assert_eq!(input.next_utf8(), Some(('l', "l")));
        debug_assert_eq!(input.next_utf8(), Some(('o', "o")));
        debug_assert_eq!(input.next_utf8(), Some((',', ",")));
        debug_assert_eq!(input.next_utf8(), Some((' ', " ")));
        debug_assert_eq!(input.next_utf8(), Some(('世', "世")));
        debug_assert_eq!(input.next_utf8(), Some(('界', "界")));
        debug_assert_eq!(input.next_utf8(), None);
        let _rug_ed_tests_llm_16_52_rrrruuuugggg_test_next_utf8 = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_53 {
    use super::*;
    use crate::*;
    use parser::Input;
    #[test]
    fn test_no_trim() {
        let _rug_st_tests_llm_16_53_rrrruuuugggg_test_no_trim = 0;
        let rug_fuzz_0 = "   example   ";
        let input = rug_fuzz_0;
        let result = Input::no_trim(input);
        let expected_chars: Vec<char> = input.trim().chars().collect();
        let result_chars: Vec<char> = result.collect();
        debug_assert_eq!(result_chars, expected_chars);
        let _rug_ed_tests_llm_16_53_rrrruuuugggg_test_no_trim = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_58 {
    use super::*;
    use crate::*;
    #[test]
    fn test_starts_with() {
        let _rug_st_tests_llm_16_58_rrrruuuugggg_test_starts_with = 0;
        let rug_fuzz_0 = "Hello World";
        let input = Input::new(rug_fuzz_0);
        let pattern = |c: char| c.is_alphabetic();
        debug_assert_eq!(input.starts_with(pattern), true);
        let _rug_ed_tests_llm_16_58_rrrruuuugggg_test_starts_with = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_63 {
    use super::*;
    use crate::*;
    #[test]
    fn test_after_double_slash() {
        let _rug_st_tests_llm_16_63_rrrruuuugggg_test_after_double_slash = 0;
        let rug_fuzz_0 = "example.com/path?query#fragment";
        let rug_fuzz_1 = 7;
        let mut parser = Parser {
            serialization: String::new(),
            base_url: None,
            query_encoding_override: None,
            violation_fn: None,
            context: parser::Context::UrlParser,
        };
        let input = parser::Input::new(rug_fuzz_0);
        let scheme_type = parser::SchemeType::SpecialNotFile;
        let scheme_end = rug_fuzz_1;
        let result = parser.after_double_slash(input, scheme_type, scheme_end);
        debug_assert_eq!(result.is_ok(), true);
        let _rug_ed_tests_llm_16_63_rrrruuuugggg_test_after_double_slash = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_73 {
    use super::*;
    use crate::*;
    use crate::parser::SyntaxViolation;
    #[test]
    fn test_log_violation() {
        let _rug_st_tests_llm_16_73_rrrruuuugggg_test_log_violation = 0;
        let violation: SyntaxViolation = SyntaxViolation::NonUrlCodePoint;
        let mut parser = Parser::<'static> {
            serialization: String::new(),
            base_url: None,
            query_encoding_override: None,
            violation_fn: None,
            context: Context::UrlParser,
        };
        parser.log_violation(violation);
        let _rug_ed_tests_llm_16_73_rrrruuuugggg_test_log_violation = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_74 {
    use super::*;
    use crate::*;
    use std::rc::Rc;
    #[test]
    fn test_log_violation_if() {
        let _rug_st_tests_llm_16_74_rrrruuuugggg_test_log_violation_if = 0;
        let rug_fuzz_0 = true;
        let violation_fn: Option<&dyn Fn(SyntaxViolation)> = Some(
            &|v| {
                println!("Violation: {}", v);
            },
        );
        let parser = Parser {
            serialization: String::new(),
            base_url: None,
            query_encoding_override: None,
            violation_fn,
            context: Context::UrlParser,
        };
        parser.log_violation_if(SyntaxViolation::NonUrlCodePoint, || rug_fuzz_0);
        let _rug_ed_tests_llm_16_74_rrrruuuugggg_test_log_violation_if = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_76_llm_16_75 {
    use super::*;
    use crate::*;
    use crate::parser::Context;
    use crate::parser::Input;
    use std::fmt;
    impl<'i> PartialEq for Input<'i> {
        fn eq(&self, other: &Self) -> bool {
            self.chars.as_str() == other.chars.as_str()
        }
    }
    impl<'i> fmt::Debug for Input<'i> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Input").field("chars", &self.chars.as_str()).finish()
        }
    }
    #[test]
    fn test_parse_cannot_be_a_base_path() {
        let _rug_st_tests_llm_16_76_llm_16_75_rrrruuuugggg_test_parse_cannot_be_a_base_path = 0;
        let rug_fuzz_0 = "?#";
        let rug_fuzz_1 = "?";
        let mut parser = Parser {
            serialization: String::new(),
            base_url: None,
            query_encoding_override: None,
            violation_fn: None,
            context: Context::UrlParser,
        };
        let input = Input::new(rug_fuzz_0);
        let expected = Input::new(rug_fuzz_1);
        let result = parser.parse_cannot_be_a_base_path(input);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_76_llm_16_75_rrrruuuugggg_test_parse_cannot_be_a_base_path = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_77 {
    use super::*;
    use crate::*;
    #[test]
    fn test_parse_file() {
        let _rug_st_tests_llm_16_77_rrrruuuugggg_test_parse_file = 0;
        let rug_fuzz_0 = "";
        let mut parser = Parser::<'static> {
            serialization: String::new(),
            base_url: None,
            query_encoding_override: None,
            violation_fn: None,
            context: Context::UrlParser,
        };
        let input: Input<'static> = Input::new(rug_fuzz_0);
        let scheme_type = SchemeType::File;
        let base_file_url: Option<&Url> = None;
        let result = parser.parse_file(input, scheme_type, base_file_url);
        debug_assert!(result.is_err());
        let _rug_ed_tests_llm_16_77_rrrruuuugggg_test_parse_file = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_80 {
    use super::*;
    use crate::*;
    use crate::parser::Context;
    #[test]
    fn test_parse_fragment() {
        let _rug_st_tests_llm_16_80_rrrruuuugggg_test_parse_fragment = 0;
        let rug_fuzz_0 = "test#fragment";
        let mut parser = Parser {
            serialization: String::new(),
            base_url: None,
            query_encoding_override: None,
            violation_fn: None,
            context: Context::UrlParser,
        };
        let input = Input::new(rug_fuzz_0);
        parser.parse_fragment(input);
        debug_assert_eq!(parser.serialization, "fragment");
        let _rug_ed_tests_llm_16_80_rrrruuuugggg_test_parse_fragment = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_81 {
    use super::*;
    use crate::*;
    use crate::parser::{Host, ParseResult, ParseError};
    #[test]
    fn test_parse_host_file_scheme_type() {
        let _rug_st_tests_llm_16_81_rrrruuuugggg_test_parse_host_file_scheme_type = 0;
        let rug_fuzz_0 = "example.com";
        let rug_fuzz_1 = "example.com";
        let rug_fuzz_2 = "";
        let input = Input::new(rug_fuzz_0);
        let scheme_type = SchemeType::File;
        let expected = Ok((
            Host::Domain(rug_fuzz_1.to_string()),
            Input::new(rug_fuzz_2),
        ));
        let result = Parser::parse_host(input, scheme_type);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_81_rrrruuuugggg_test_parse_host_file_scheme_type = 0;
    }
    #[test]
    fn test_parse_host_not_file_scheme_type() {
        let _rug_st_tests_llm_16_81_rrrruuuugggg_test_parse_host_not_file_scheme_type = 0;
        let rug_fuzz_0 = "example.com";
        let rug_fuzz_1 = "example.com";
        let rug_fuzz_2 = "";
        let input = Input::new(rug_fuzz_0);
        let scheme_type = SchemeType::NotSpecial;
        let expected = Ok((
            Host::parse_opaque(rug_fuzz_1).unwrap(),
            Input::new(rug_fuzz_2),
        ));
        let result = Parser::parse_host(input, scheme_type);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_81_rrrruuuugggg_test_parse_host_not_file_scheme_type = 0;
    }
    #[test]
    fn test_parse_host_special_not_file_scheme_type() {
        let _rug_st_tests_llm_16_81_rrrruuuugggg_test_parse_host_special_not_file_scheme_type = 0;
        let rug_fuzz_0 = "example.com";
        let rug_fuzz_1 = "example.com";
        let rug_fuzz_2 = "";
        let input = Input::new(rug_fuzz_0);
        let scheme_type = SchemeType::SpecialNotFile;
        let expected = Ok((Host::parse(rug_fuzz_1).unwrap(), Input::new(rug_fuzz_2)));
        let result = Parser::parse_host(input, scheme_type);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_81_rrrruuuugggg_test_parse_host_special_not_file_scheme_type = 0;
    }
    #[test]
    fn test_parse_host_empty_host() {
        let _rug_st_tests_llm_16_81_rrrruuuugggg_test_parse_host_empty_host = 0;
        let rug_fuzz_0 = "";
        let input = Input::new(rug_fuzz_0);
        let scheme_type = SchemeType::NotSpecial;
        let expected = Err(ParseError::EmptyHost);
        let result = Parser::parse_host(input, scheme_type);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_81_rrrruuuugggg_test_parse_host_empty_host = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_84 {
    use super::*;
    use crate::*;
    #[test]
    fn test_parse_non_special() {
        let _rug_st_tests_llm_16_84_rrrruuuugggg_test_parse_non_special = 0;
        let rug_fuzz_0 = "http://example.com/path";
        let rug_fuzz_1 = 4;
        let input = Input::new(rug_fuzz_0);
        let scheme_type = SchemeType::SpecialNotFile;
        let scheme_end = rug_fuzz_1;
        let parser = Parser {
            serialization: String::new(),
            base_url: None,
            query_encoding_override: None,
            violation_fn: None,
            context: Context::UrlParser,
        };
        let result = parser.parse_non_special(input, scheme_type, scheme_end);
        debug_assert!(result.is_ok());
        let _rug_ed_tests_llm_16_84_rrrruuuugggg_test_parse_non_special = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_85 {
    use crate::parser::{Context, Input, Parser, SchemeType};
    use crate::{Host, HostInternal, Url};
    #[test]
    fn test_parse_path() {
        let _rug_st_tests_llm_16_85_rrrruuuugggg_test_parse_path = 0;
        let rug_fuzz_0 = false;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = "/path/to/file";
        let rug_fuzz_3 = "";
        let mut parser = Parser {
            serialization: String::new(),
            base_url: None,
            query_encoding_override: None,
            violation_fn: None,
            context: Context::UrlParser,
        };
        let mut has_host = rug_fuzz_0;
        let path_start = rug_fuzz_1;
        let input = Input::new(rug_fuzz_2);
        let result = parser
            .parse_path(SchemeType::SpecialNotFile, &mut has_host, path_start, input);
        let expected = Input::new(rug_fuzz_3);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_85_rrrruuuugggg_test_parse_path = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_87 {
    use super::*;
    use crate::*;
    #[test]
    fn test_parse_path_start() {
        let _rug_st_tests_llm_16_87_rrrruuuugggg_test_parse_path_start = 0;
        let rug_fuzz_0 = "http://example.com/";
        let rug_fuzz_1 = true;
        let rug_fuzz_2 = "example.com/";
        let mut parser = parser::Parser::for_setter(String::from(rug_fuzz_0));
        let mut has_host = rug_fuzz_1;
        let input = parser::Input::new(rug_fuzz_2);
        let result = parser
            .parse_path_start(
                parser::SchemeType::SpecialNotFile,
                &mut has_host,
                input.clone(),
            );
        debug_assert_eq!(result, input);
        debug_assert_eq!(parser.serialization, "http://example.com/");
        let _rug_ed_tests_llm_16_87_rrrruuuugggg_test_parse_path_start = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_88 {
    use super::*;
    use crate::*;
    use crate::parser::{Context, Input};
    #[test]
    fn test_parse_port_valid_input() {
        let _rug_st_tests_llm_16_88_rrrruuuugggg_test_parse_port_valid_input = 0;
        let rug_fuzz_0 = "8080";
        let rug_fuzz_1 = 80;
        let rug_fuzz_2 = 8080;
        let rug_fuzz_3 = "";
        let input = Input::new(rug_fuzz_0);
        let default_port = || Some(rug_fuzz_1);
        let context = Context::UrlParser;
        let expected = Ok((Some(rug_fuzz_2), Input::new(rug_fuzz_3)));
        let result = Parser::parse_port(input, default_port, context);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_88_rrrruuuugggg_test_parse_port_valid_input = 0;
    }
    #[test]
    fn test_parse_port_invalid_input() {
        let _rug_st_tests_llm_16_88_rrrruuuugggg_test_parse_port_invalid_input = 0;
        let rug_fuzz_0 = "abc";
        let rug_fuzz_1 = 80;
        let input = Input::new(rug_fuzz_0);
        let default_port = || Some(rug_fuzz_1);
        let context = Context::UrlParser;
        let expected = Err(ParseError::InvalidPort);
        let result = Parser::parse_port(input, default_port, context);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_88_rrrruuuugggg_test_parse_port_invalid_input = 0;
    }
    #[test]
    fn test_parse_port_empty_input() {
        let _rug_st_tests_llm_16_88_rrrruuuugggg_test_parse_port_empty_input = 0;
        let rug_fuzz_0 = "";
        let rug_fuzz_1 = 80;
        let rug_fuzz_2 = "";
        let input = Input::new(rug_fuzz_0);
        let default_port = || Some(rug_fuzz_1);
        let context = Context::UrlParser;
        let expected = Ok((None, Input::new(rug_fuzz_2)));
        let result = Parser::parse_port(input, default_port, context);
        debug_assert_eq!(result, expected);
        let _rug_ed_tests_llm_16_88_rrrruuuugggg_test_parse_port_empty_input = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_89 {
    use super::*;
    use crate::*;
    use crate::parser::*;
    #[test]
    fn parse_query_returns_some_remaining_input_when_input_starts_with_valid_query() {
        let _rug_st_tests_llm_16_89_rrrruuuugggg_parse_query_returns_some_remaining_input_when_input_starts_with_valid_query = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = "?key=value&param=true#fragment";
        let rug_fuzz_2 = "key=value&param=true#fragment";
        let mut parser = Parser {
            serialization: String::new(),
            base_url: None,
            query_encoding_override: None,
            violation_fn: None,
            context: Context::UrlParser,
        };
        let scheme_type = SchemeType::NotSpecial;
        let scheme_end = rug_fuzz_0;
        let input = Input::new(rug_fuzz_1);
        let expected_remaining = Some(Input::new(rug_fuzz_2));
        let actual_remaining = parser.parse_query(scheme_type, scheme_end, input);
        debug_assert_eq!(expected_remaining, actual_remaining);
        let _rug_ed_tests_llm_16_89_rrrruuuugggg_parse_query_returns_some_remaining_input_when_input_starts_with_valid_query = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_91 {
    use super::*;
    use crate::*;
    #[test]
    fn test_parse_query_and_fragment() {
        let _rug_st_tests_llm_16_91_rrrruuuugggg_test_parse_query_and_fragment = 0;
        let rug_fuzz_0 = "?query#fragment";
        let rug_fuzz_1 = 0;
        let mut parser = Parser {
            serialization: String::new(),
            base_url: None,
            query_encoding_override: None,
            violation_fn: None,
            context: Context::UrlParser,
        };
        let mut input = Input::new(rug_fuzz_0);
        let result = parser
            .parse_query_and_fragment(SchemeType::NotSpecial, rug_fuzz_1, input);
        debug_assert_eq!(result, Ok((Some(0), Some(7))));
        let _rug_ed_tests_llm_16_91_rrrruuuugggg_test_parse_query_and_fragment = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_92 {
    use super::*;
    use crate::*;
    #[test]
    fn test_parse_scheme() {
        let _rug_st_tests_llm_16_92_rrrruuuugggg_test_parse_scheme = 0;
        let rug_fuzz_0 = "http:";
        let rug_fuzz_1 = "htt_p:";
        let rug_fuzz_2 = "";
        let rug_fuzz_3 = "http";
        let mut parser = Parser {
            serialization: String::new(),
            base_url: None,
            query_encoding_override: None,
            violation_fn: None,
            context: Context::UrlParser,
        };
        let input = Input::new(rug_fuzz_0);
        debug_assert_eq!(parser.parse_scheme(input), Ok(Input { chars : "".chars() }));
        let input = Input::new(rug_fuzz_1);
        debug_assert_eq!(parser.parse_scheme(input), Err(()));
        let input = Input::new(rug_fuzz_2);
        debug_assert_eq!(parser.parse_scheme(input), Err(()));
        let input = Input::new(rug_fuzz_3);
        debug_assert_eq!(parser.parse_scheme(input), Err(()));
        let _rug_ed_tests_llm_16_92_rrrruuuugggg_test_parse_scheme = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_96_llm_16_95 {
    use super::*;
    use crate::*;
    use crate::parser::Input;
    #[test]
    fn test_parse_userinfo() {
        let _rug_st_tests_llm_16_96_llm_16_95_rrrruuuugggg_test_parse_userinfo = 0;
        let rug_fuzz_0 = "username:password@example.com";
        let mut parser = Parser {
            serialization: String::new(),
            base_url: None,
            query_encoding_override: None,
            violation_fn: None,
            context: Context::UrlParser,
        };
        let input = Input::new(rug_fuzz_0);
        let scheme_type = SchemeType::SpecialNotFile;
        let result = parser.parse_userinfo(input, scheme_type);
        debug_assert!(result.is_ok());
        let (_, remaining) = result.unwrap();
        debug_assert_eq!(remaining.chars.as_str(), "example.com");
        let _rug_ed_tests_llm_16_96_llm_16_95_rrrruuuugggg_test_parse_userinfo = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_99 {
    use super::*;
    use crate::*;
    #[test]
    fn test_pop_path() {
        let _rug_st_tests_llm_16_99_rrrruuuugggg_test_pop_path = 0;
        let rug_fuzz_0 = 0;
        let rug_fuzz_1 = "path/to/file";
        let mut parser = Parser::for_setter(String::new());
        let scheme_type = SchemeType::File;
        let path_start = rug_fuzz_0;
        let path = rug_fuzz_1;
        parser.serialization.push_str(path);
        let expected_result = path_start;
        parser.pop_path(scheme_type, path_start);
        let actual_result = parser.serialization.len();
        debug_assert_eq!(expected_result, actual_result);
        let _rug_ed_tests_llm_16_99_rrrruuuugggg_test_pop_path = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_106 {
    use super::*;
    use crate::*;
    use parser::SchemeType;
    #[test]
    fn test_is_file() {
        let _rug_st_tests_llm_16_106_rrrruuuugggg_test_is_file = 0;
        let scheme_file = SchemeType::File;
        let scheme_not_file = SchemeType::SpecialNotFile;
        let scheme_not_special = SchemeType::NotSpecial;
        debug_assert!(scheme_file.is_file());
        debug_assert!(! scheme_not_file.is_file());
        debug_assert!(! scheme_not_special.is_file());
        let _rug_ed_tests_llm_16_106_rrrruuuugggg_test_is_file = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_107 {
    use super::*;
    use crate::*;
    use parser::SchemeType;
    #[test]
    fn test_is_special() {
        let _rug_st_tests_llm_16_107_rrrruuuugggg_test_is_special = 0;
        let scheme_type: SchemeType = SchemeType::SpecialNotFile;
        debug_assert_eq!(scheme_type.is_special(), true);
        let scheme_type: SchemeType = SchemeType::File;
        debug_assert_eq!(scheme_type.is_special(), true);
        let scheme_type: SchemeType = SchemeType::NotSpecial;
        debug_assert_eq!(scheme_type.is_special(), false);
        let _rug_ed_tests_llm_16_107_rrrruuuugggg_test_is_special = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_110 {
    use crate::parser::ascii_alpha;
    #[test]
    fn test_ascii_alpha_true() {
        let _rug_st_tests_llm_16_110_rrrruuuugggg_test_ascii_alpha_true = 0;
        let rug_fuzz_0 = 'a';
        let rug_fuzz_1 = 'A';
        let rug_fuzz_2 = 'z';
        let rug_fuzz_3 = 'Z';
        debug_assert_eq!(ascii_alpha(rug_fuzz_0), true);
        debug_assert_eq!(ascii_alpha(rug_fuzz_1), true);
        debug_assert_eq!(ascii_alpha(rug_fuzz_2), true);
        debug_assert_eq!(ascii_alpha(rug_fuzz_3), true);
        let _rug_ed_tests_llm_16_110_rrrruuuugggg_test_ascii_alpha_true = 0;
    }
    #[test]
    fn test_ascii_alpha_false() {
        let _rug_st_tests_llm_16_110_rrrruuuugggg_test_ascii_alpha_false = 0;
        let rug_fuzz_0 = '0';
        let rug_fuzz_1 = '9';
        let rug_fuzz_2 = ' ';
        let rug_fuzz_3 = '_';
        let rug_fuzz_4 = '!';
        let rug_fuzz_5 = '.';
        debug_assert_eq!(ascii_alpha(rug_fuzz_0), false);
        debug_assert_eq!(ascii_alpha(rug_fuzz_1), false);
        debug_assert_eq!(ascii_alpha(rug_fuzz_2), false);
        debug_assert_eq!(ascii_alpha(rug_fuzz_3), false);
        debug_assert_eq!(ascii_alpha(rug_fuzz_4), false);
        debug_assert_eq!(ascii_alpha(rug_fuzz_5), false);
        let _rug_ed_tests_llm_16_110_rrrruuuugggg_test_ascii_alpha_false = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_111 {
    use crate::parser::ascii_tab_or_new_line;
    #[test]
    fn test_ascii_tab_or_new_line() {
        let _rug_st_tests_llm_16_111_rrrruuuugggg_test_ascii_tab_or_new_line = 0;
        let rug_fuzz_0 = '\t';
        let rug_fuzz_1 = '\r';
        let rug_fuzz_2 = '\n';
        let rug_fuzz_3 = ' ';
        let rug_fuzz_4 = 'a';
        let rug_fuzz_5 = '0';
        debug_assert_eq!(ascii_tab_or_new_line(rug_fuzz_0), true);
        debug_assert_eq!(ascii_tab_or_new_line(rug_fuzz_1), true);
        debug_assert_eq!(ascii_tab_or_new_line(rug_fuzz_2), true);
        debug_assert_eq!(ascii_tab_or_new_line(rug_fuzz_3), false);
        debug_assert_eq!(ascii_tab_or_new_line(rug_fuzz_4), false);
        debug_assert_eq!(ascii_tab_or_new_line(rug_fuzz_5), false);
        let _rug_ed_tests_llm_16_111_rrrruuuugggg_test_ascii_tab_or_new_line = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_112 {
    use crate::parser::c0_control_or_space;
    #[test]
    fn test_c0_control_or_space() {
        let _rug_st_tests_llm_16_112_rrrruuuugggg_test_c0_control_or_space = 0;
        let rug_fuzz_0 = '\u{0000}';
        let rug_fuzz_1 = '\u{0001}';
        let rug_fuzz_2 = '\u{001F}';
        let rug_fuzz_3 = ' ';
        let rug_fuzz_4 = 'A';
        let rug_fuzz_5 = 'z';
        let rug_fuzz_6 = '\u{0021}';
        let rug_fuzz_7 = '\u{00A0}';
        let rug_fuzz_8 = '\u{FFFF}';
        debug_assert_eq!(c0_control_or_space(rug_fuzz_0), true);
        debug_assert_eq!(c0_control_or_space(rug_fuzz_1), true);
        debug_assert_eq!(c0_control_or_space(rug_fuzz_2), true);
        debug_assert_eq!(c0_control_or_space(rug_fuzz_3), true);
        debug_assert_eq!(c0_control_or_space(rug_fuzz_4), false);
        debug_assert_eq!(c0_control_or_space(rug_fuzz_5), false);
        debug_assert_eq!(c0_control_or_space(rug_fuzz_6), false);
        debug_assert_eq!(c0_control_or_space(rug_fuzz_7), false);
        debug_assert_eq!(c0_control_or_space(rug_fuzz_8), false);
        let _rug_ed_tests_llm_16_112_rrrruuuugggg_test_c0_control_or_space = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_113 {
    use super::*;
    use crate::*;
    #[test]
    fn test_default_port() {
        let _rug_st_tests_llm_16_113_rrrruuuugggg_test_default_port = 0;
        let rug_fuzz_0 = "http";
        let rug_fuzz_1 = "ws";
        let rug_fuzz_2 = "https";
        let rug_fuzz_3 = "wss";
        let rug_fuzz_4 = "ftp";
        let rug_fuzz_5 = "invalid_scheme";
        debug_assert_eq!(default_port(rug_fuzz_0), Some(80));
        debug_assert_eq!(default_port(rug_fuzz_1), Some(80));
        debug_assert_eq!(default_port(rug_fuzz_2), Some(443));
        debug_assert_eq!(default_port(rug_fuzz_3), Some(443));
        debug_assert_eq!(default_port(rug_fuzz_4), Some(21));
        debug_assert_eq!(default_port(rug_fuzz_5), None);
        let _rug_ed_tests_llm_16_113_rrrruuuugggg_test_default_port = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_116 {
    use crate::parser::is_normalized_windows_drive_letter;
    #[test]
    fn test_is_normalized_windows_drive_letter() {
        let _rug_st_tests_llm_16_116_rrrruuuugggg_test_is_normalized_windows_drive_letter = 0;
        let rug_fuzz_0 = "C:";
        let rug_fuzz_1 = "d:";
        let rug_fuzz_2 = "X:";
        let rug_fuzz_3 = "C";
        let rug_fuzz_4 = "C::";
        let rug_fuzz_5 = "C:";
        let rug_fuzz_6 = "C:.txt";
        let rug_fuzz_7 = "C:/";
        let rug_fuzz_8 = "C:/test";
        let rug_fuzz_9 = "C:/test/";
        debug_assert_eq!(is_normalized_windows_drive_letter(rug_fuzz_0), true);
        debug_assert_eq!(is_normalized_windows_drive_letter(rug_fuzz_1), true);
        debug_assert_eq!(is_normalized_windows_drive_letter(rug_fuzz_2), true);
        debug_assert_eq!(is_normalized_windows_drive_letter(rug_fuzz_3), false);
        debug_assert_eq!(is_normalized_windows_drive_letter(rug_fuzz_4), false);
        debug_assert_eq!(is_normalized_windows_drive_letter(rug_fuzz_5), true);
        debug_assert_eq!(is_normalized_windows_drive_letter(rug_fuzz_6), false);
        debug_assert_eq!(is_normalized_windows_drive_letter(rug_fuzz_7), false);
        debug_assert_eq!(is_normalized_windows_drive_letter(rug_fuzz_8), false);
        debug_assert_eq!(is_normalized_windows_drive_letter(rug_fuzz_9), false);
        let _rug_ed_tests_llm_16_116_rrrruuuugggg_test_is_normalized_windows_drive_letter = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_118 {
    use crate::parser::is_url_code_point;
    #[test]
    fn test_is_url_code_point() {
        let _rug_st_tests_llm_16_118_rrrruuuugggg_test_is_url_code_point = 0;
        let rug_fuzz_0 = 'a';
        let rug_fuzz_1 = 'Z';
        let rug_fuzz_2 = '0';
        let rug_fuzz_3 = '!';
        let rug_fuzz_4 = '$';
        let rug_fuzz_5 = '&';
        let rug_fuzz_6 = '\'';
        let rug_fuzz_7 = '(';
        let rug_fuzz_8 = ')';
        let rug_fuzz_9 = '*';
        let rug_fuzz_10 = '+';
        let rug_fuzz_11 = ',';
        let rug_fuzz_12 = '-';
        let rug_fuzz_13 = '.';
        let rug_fuzz_14 = '/';
        let rug_fuzz_15 = ':';
        let rug_fuzz_16 = ';';
        let rug_fuzz_17 = '=';
        let rug_fuzz_18 = '?';
        let rug_fuzz_19 = '@';
        let rug_fuzz_20 = '_';
        let rug_fuzz_21 = '~';
        let rug_fuzz_22 = '\u{A0}';
        let rug_fuzz_23 = '\u{D7FF}';
        let rug_fuzz_24 = '\u{E000}';
        let rug_fuzz_25 = '\u{FDCF}';
        let rug_fuzz_26 = '\u{FDF0}';
        let rug_fuzz_27 = '\u{FFFD}';
        let rug_fuzz_28 = '\u{10000}';
        let rug_fuzz_29 = '\u{1FFFD}';
        let rug_fuzz_30 = '\u{20000}';
        let rug_fuzz_31 = '\u{2FFFD}';
        let rug_fuzz_32 = '\u{30000}';
        let rug_fuzz_33 = '\u{3FFFD}';
        let rug_fuzz_34 = '\u{40000}';
        let rug_fuzz_35 = '\u{4FFFD}';
        let rug_fuzz_36 = '\u{50000}';
        let rug_fuzz_37 = '\u{5FFFD}';
        let rug_fuzz_38 = '\u{60000}';
        let rug_fuzz_39 = '\u{6FFFD}';
        let rug_fuzz_40 = '\u{70000}';
        let rug_fuzz_41 = '\u{7FFFD}';
        let rug_fuzz_42 = '\u{80000}';
        let rug_fuzz_43 = '\u{8FFFD}';
        let rug_fuzz_44 = '\u{90000}';
        let rug_fuzz_45 = '\u{9FFFD}';
        let rug_fuzz_46 = '\u{A0000}';
        let rug_fuzz_47 = '\u{AFFFD}';
        let rug_fuzz_48 = '\u{B0000}';
        let rug_fuzz_49 = '\u{BFFFD}';
        let rug_fuzz_50 = '\u{C0000}';
        let rug_fuzz_51 = '\u{CFFFD}';
        let rug_fuzz_52 = '\u{D0000}';
        let rug_fuzz_53 = '\u{DFFFD}';
        let rug_fuzz_54 = '\u{E1000}';
        let rug_fuzz_55 = '\u{EFFFD}';
        let rug_fuzz_56 = '\u{F0000}';
        let rug_fuzz_57 = '\u{FFFFD}';
        let rug_fuzz_58 = '\u{100000}';
        let rug_fuzz_59 = '\u{10FFFD}';
        let rug_fuzz_60 = ' ';
        let rug_fuzz_61 = '\n';
        let rug_fuzz_62 = '\r';
        let rug_fuzz_63 = '\t';
        debug_assert!(is_url_code_point(rug_fuzz_0));
        debug_assert!(is_url_code_point(rug_fuzz_1));
        debug_assert!(is_url_code_point(rug_fuzz_2));
        debug_assert!(is_url_code_point(rug_fuzz_3));
        debug_assert!(is_url_code_point(rug_fuzz_4));
        debug_assert!(is_url_code_point(rug_fuzz_5));
        debug_assert!(is_url_code_point(rug_fuzz_6));
        debug_assert!(is_url_code_point(rug_fuzz_7));
        debug_assert!(is_url_code_point(rug_fuzz_8));
        debug_assert!(is_url_code_point(rug_fuzz_9));
        debug_assert!(is_url_code_point(rug_fuzz_10));
        debug_assert!(is_url_code_point(rug_fuzz_11));
        debug_assert!(is_url_code_point(rug_fuzz_12));
        debug_assert!(is_url_code_point(rug_fuzz_13));
        debug_assert!(is_url_code_point(rug_fuzz_14));
        debug_assert!(is_url_code_point(rug_fuzz_15));
        debug_assert!(is_url_code_point(rug_fuzz_16));
        debug_assert!(is_url_code_point(rug_fuzz_17));
        debug_assert!(is_url_code_point(rug_fuzz_18));
        debug_assert!(is_url_code_point(rug_fuzz_19));
        debug_assert!(is_url_code_point(rug_fuzz_20));
        debug_assert!(is_url_code_point(rug_fuzz_21));
        debug_assert!(is_url_code_point(rug_fuzz_22));
        debug_assert!(is_url_code_point(rug_fuzz_23));
        debug_assert!(is_url_code_point(rug_fuzz_24));
        debug_assert!(is_url_code_point(rug_fuzz_25));
        debug_assert!(is_url_code_point(rug_fuzz_26));
        debug_assert!(is_url_code_point(rug_fuzz_27));
        debug_assert!(is_url_code_point(rug_fuzz_28));
        debug_assert!(is_url_code_point(rug_fuzz_29));
        debug_assert!(is_url_code_point(rug_fuzz_30));
        debug_assert!(is_url_code_point(rug_fuzz_31));
        debug_assert!(is_url_code_point(rug_fuzz_32));
        debug_assert!(is_url_code_point(rug_fuzz_33));
        debug_assert!(is_url_code_point(rug_fuzz_34));
        debug_assert!(is_url_code_point(rug_fuzz_35));
        debug_assert!(is_url_code_point(rug_fuzz_36));
        debug_assert!(is_url_code_point(rug_fuzz_37));
        debug_assert!(is_url_code_point(rug_fuzz_38));
        debug_assert!(is_url_code_point(rug_fuzz_39));
        debug_assert!(is_url_code_point(rug_fuzz_40));
        debug_assert!(is_url_code_point(rug_fuzz_41));
        debug_assert!(is_url_code_point(rug_fuzz_42));
        debug_assert!(is_url_code_point(rug_fuzz_43));
        debug_assert!(is_url_code_point(rug_fuzz_44));
        debug_assert!(is_url_code_point(rug_fuzz_45));
        debug_assert!(is_url_code_point(rug_fuzz_46));
        debug_assert!(is_url_code_point(rug_fuzz_47));
        debug_assert!(is_url_code_point(rug_fuzz_48));
        debug_assert!(is_url_code_point(rug_fuzz_49));
        debug_assert!(is_url_code_point(rug_fuzz_50));
        debug_assert!(is_url_code_point(rug_fuzz_51));
        debug_assert!(is_url_code_point(rug_fuzz_52));
        debug_assert!(is_url_code_point(rug_fuzz_53));
        debug_assert!(is_url_code_point(rug_fuzz_54));
        debug_assert!(is_url_code_point(rug_fuzz_55));
        debug_assert!(is_url_code_point(rug_fuzz_56));
        debug_assert!(is_url_code_point(rug_fuzz_57));
        debug_assert!(is_url_code_point(rug_fuzz_58));
        debug_assert!(is_url_code_point(rug_fuzz_59));
        debug_assert!(! is_url_code_point(rug_fuzz_60));
        debug_assert!(! is_url_code_point(rug_fuzz_61));
        debug_assert!(! is_url_code_point(rug_fuzz_62));
        debug_assert!(! is_url_code_point(rug_fuzz_63));
        let _rug_ed_tests_llm_16_118_rrrruuuugggg_test_is_url_code_point = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_121 {
    use super::*;
    use crate::*;
    #[test]
    fn test_path_starts_with_windows_drive_letter() {
        let _rug_st_tests_llm_16_121_rrrruuuugggg_test_path_starts_with_windows_drive_letter = 0;
        let rug_fuzz_0 = "/c:";
        let rug_fuzz_1 = "/c:/";
        let rug_fuzz_2 = "/a:/";
        let rug_fuzz_3 = "/b:";
        let rug_fuzz_4 = "/b:/";
        let rug_fuzz_5 = "/";
        let rug_fuzz_6 = "";
        let rug_fuzz_7 = "c:";
        let rug_fuzz_8 = "c:/";
        let rug_fuzz_9 = "a:/";
        let rug_fuzz_10 = "b:";
        let rug_fuzz_11 = "b:/";
        let rug_fuzz_12 = "c:/path";
        debug_assert_eq!(path_starts_with_windows_drive_letter(rug_fuzz_0), true);
        debug_assert_eq!(path_starts_with_windows_drive_letter(rug_fuzz_1), true);
        debug_assert_eq!(path_starts_with_windows_drive_letter(rug_fuzz_2), true);
        debug_assert_eq!(path_starts_with_windows_drive_letter(rug_fuzz_3), true);
        debug_assert_eq!(path_starts_with_windows_drive_letter(rug_fuzz_4), true);
        debug_assert_eq!(path_starts_with_windows_drive_letter(rug_fuzz_5), false);
        debug_assert_eq!(path_starts_with_windows_drive_letter(rug_fuzz_6), false);
        debug_assert_eq!(path_starts_with_windows_drive_letter(rug_fuzz_7), false);
        debug_assert_eq!(path_starts_with_windows_drive_letter(rug_fuzz_8), false);
        debug_assert_eq!(path_starts_with_windows_drive_letter(rug_fuzz_9), false);
        debug_assert_eq!(path_starts_with_windows_drive_letter(rug_fuzz_10), false);
        debug_assert_eq!(path_starts_with_windows_drive_letter(rug_fuzz_11), false);
        debug_assert_eq!(path_starts_with_windows_drive_letter(rug_fuzz_12), false);
        let _rug_ed_tests_llm_16_121_rrrruuuugggg_test_path_starts_with_windows_drive_letter = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_122 {
    use super::*;
    use crate::*;
    use crate::parser::ascii_alpha;
    #[test]
    fn test_starts_with_windows_drive_letter() {
        let _rug_st_tests_llm_16_122_rrrruuuugggg_test_starts_with_windows_drive_letter = 0;
        let rug_fuzz_0 = "C:";
        let rug_fuzz_1 = "C|";
        let rug_fuzz_2 = "C:/";
        let rug_fuzz_3 = "C:\\";
        let rug_fuzz_4 = "C:?";
        let rug_fuzz_5 = "C:#";
        let rug_fuzz_6 = "::";
        let rug_fuzz_7 = "C";
        let rug_fuzz_8 = "D:";
        let rug_fuzz_9 = "1:";
        let rug_fuzz_10 = "C:/test";
        let rug_fuzz_11 = "C|/test";
        debug_assert_eq!(starts_with_windows_drive_letter(rug_fuzz_0), true);
        debug_assert_eq!(starts_with_windows_drive_letter(rug_fuzz_1), true);
        debug_assert_eq!(starts_with_windows_drive_letter(rug_fuzz_2), true);
        debug_assert_eq!(starts_with_windows_drive_letter(rug_fuzz_3), true);
        debug_assert_eq!(starts_with_windows_drive_letter(rug_fuzz_4), true);
        debug_assert_eq!(starts_with_windows_drive_letter(rug_fuzz_5), true);
        debug_assert_eq!(starts_with_windows_drive_letter(rug_fuzz_6), false);
        debug_assert_eq!(starts_with_windows_drive_letter(rug_fuzz_7), false);
        debug_assert_eq!(starts_with_windows_drive_letter(rug_fuzz_8), false);
        debug_assert_eq!(starts_with_windows_drive_letter(rug_fuzz_9), false);
        debug_assert_eq!(starts_with_windows_drive_letter(rug_fuzz_10), false);
        debug_assert_eq!(starts_with_windows_drive_letter(rug_fuzz_11), false);
        let _rug_ed_tests_llm_16_122_rrrruuuugggg_test_starts_with_windows_drive_letter = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_123 {
    use super::*;
    use crate::*;
    use crate::parser::Input;
    #[test]
    fn test_starts_with_windows_drive_letter_segment() {
        let _rug_st_tests_llm_16_123_rrrruuuugggg_test_starts_with_windows_drive_letter_segment = 0;
        let rug_fuzz_0 = "C:foo/bar";
        let rug_fuzz_1 = "D|baz";
        let rug_fuzz_2 = "E:qux";
        let rug_fuzz_3 = "C";
        let rug_fuzz_4 = "D";
        let rug_fuzz_5 = "E";
        let rug_fuzz_6 = "C:foo?";
        let rug_fuzz_7 = "D|bar/";
        let rug_fuzz_8 = "E:baz#";
        let rug_fuzz_9 = "C:foo\\";
        let rug_fuzz_10 = "D|baz\\";
        let rug_fuzz_11 = "E:qux\\";
        let rug_fuzz_12 = "D|";
        let rug_fuzz_13 = "E:";
        let rug_fuzz_14 = "A:foo/bar";
        let rug_fuzz_15 = "B|baz";
        let rug_fuzz_16 = "F:qux";
        let rug_fuzz_17 = "foobar";
        let rug_fuzz_18 = "";
        let input = Input::new(rug_fuzz_0);
        debug_assert_eq!(starts_with_windows_drive_letter_segment(& input), true);
        let input = Input::new(rug_fuzz_1);
        debug_assert_eq!(starts_with_windows_drive_letter_segment(& input), true);
        let input = Input::new(rug_fuzz_2);
        debug_assert_eq!(starts_with_windows_drive_letter_segment(& input), true);
        let input = Input::new(rug_fuzz_3);
        debug_assert_eq!(starts_with_windows_drive_letter_segment(& input), true);
        let input = Input::new(rug_fuzz_4);
        debug_assert_eq!(starts_with_windows_drive_letter_segment(& input), true);
        let input = Input::new(rug_fuzz_5);
        debug_assert_eq!(starts_with_windows_drive_letter_segment(& input), true);
        let input = Input::new(rug_fuzz_6);
        debug_assert_eq!(starts_with_windows_drive_letter_segment(& input), true);
        let input = Input::new(rug_fuzz_7);
        debug_assert_eq!(starts_with_windows_drive_letter_segment(& input), true);
        let input = Input::new(rug_fuzz_8);
        debug_assert_eq!(starts_with_windows_drive_letter_segment(& input), true);
        let input = Input::new(rug_fuzz_9);
        debug_assert_eq!(starts_with_windows_drive_letter_segment(& input), true);
        let input = Input::new(rug_fuzz_10);
        debug_assert_eq!(starts_with_windows_drive_letter_segment(& input), true);
        let input = Input::new(rug_fuzz_11);
        debug_assert_eq!(starts_with_windows_drive_letter_segment(& input), true);
        let input = Input::new(rug_fuzz_12);
        debug_assert_eq!(starts_with_windows_drive_letter_segment(& input), true);
        let input = Input::new(rug_fuzz_13);
        debug_assert_eq!(starts_with_windows_drive_letter_segment(& input), true);
        let input = Input::new(rug_fuzz_14);
        debug_assert_eq!(starts_with_windows_drive_letter_segment(& input), false);
        let input = Input::new(rug_fuzz_15);
        debug_assert_eq!(starts_with_windows_drive_letter_segment(& input), false);
        let input = Input::new(rug_fuzz_16);
        debug_assert_eq!(starts_with_windows_drive_letter_segment(& input), false);
        let input = Input::new(rug_fuzz_17);
        debug_assert_eq!(starts_with_windows_drive_letter_segment(& input), false);
        let input = Input::new(rug_fuzz_18);
        debug_assert_eq!(starts_with_windows_drive_letter_segment(& input), false);
        let _rug_ed_tests_llm_16_123_rrrruuuugggg_test_starts_with_windows_drive_letter_segment = 0;
    }
}
#[cfg(test)]
mod tests_llm_16_124 {
    use crate::parser::to_u32;
    use crate::parser::ParseResult;
    use crate::parser::ParseError;
    #[test]
    fn test_to_u32_with_valid_input() {
        let _rug_st_tests_llm_16_124_rrrruuuugggg_test_to_u32_with_valid_input = 0;
        let rug_fuzz_0 = 42;
        let rug_fuzz_1 = 42u32;
        let input = rug_fuzz_0;
        let expected = Ok(rug_fuzz_1);
        let actual = to_u32(input);
        debug_assert_eq!(expected, actual);
        let _rug_ed_tests_llm_16_124_rrrruuuugggg_test_to_u32_with_valid_input = 0;
    }
    #[test]
    fn test_to_u32_with_maximum_value() {
        let _rug_st_tests_llm_16_124_rrrruuuugggg_test_to_u32_with_maximum_value = 0;
        let input = std::u32::MAX as usize;
        let expected = Ok(std::u32::MAX);
        let actual = to_u32(input);
        debug_assert_eq!(expected, actual);
        let _rug_ed_tests_llm_16_124_rrrruuuugggg_test_to_u32_with_maximum_value = 0;
    }
    #[test]
    fn test_to_u32_with_overflow() {
        let _rug_st_tests_llm_16_124_rrrruuuugggg_test_to_u32_with_overflow = 0;
        let rug_fuzz_0 = 1;
        let input = std::u32::MAX as usize + rug_fuzz_0;
        let expected = Err(ParseError::Overflow);
        let actual = to_u32(input);
        debug_assert_eq!(expected, actual);
        let _rug_ed_tests_llm_16_124_rrrruuuugggg_test_to_u32_with_overflow = 0;
    }
}
#[cfg(test)]
mod tests_rug_12 {
    use super::*;
    #[test]
    fn test_is_ascii_hex_digit() {
        let _rug_st_tests_rug_12_rrrruuuugggg_test_is_ascii_hex_digit = 0;
        let rug_fuzz_0 = '0';
        let rug_fuzz_1 = true;
        let p0: char = rug_fuzz_0;
        debug_assert_eq!(rug_fuzz_1, crate ::parser::is_ascii_hex_digit(p0));
        let _rug_ed_tests_rug_12_rrrruuuugggg_test_is_ascii_hex_digit = 0;
    }
}
#[cfg(test)]
mod tests_rug_13 {
    use super::*;
    #[test]
    fn test_parser_is_windows_drive_letter() {
        let _rug_st_tests_rug_13_rrrruuuugggg_test_parser_is_windows_drive_letter = 0;
        let rug_fuzz_0 = "C:";
        let p0: &str = rug_fuzz_0;
        crate::parser::is_windows_drive_letter(&p0);
        let _rug_ed_tests_rug_13_rrrruuuugggg_test_parser_is_windows_drive_letter = 0;
    }
}
#[cfg(test)]
mod tests_rug_15 {
    use super::*;
    use crate::parser::SyntaxViolation;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_15_rrrruuuugggg_test_rug = 0;
        let mut p0 = SyntaxViolation::__FutureProof;
        SyntaxViolation::description(&p0);
        let _rug_ed_tests_rug_15_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_16 {
    use super::*;
    #[test]
    fn test_from() {
        let _rug_st_tests_rug_16_rrrruuuugggg_test_from = 0;
        let rug_fuzz_0 = "http";
        let p0: &str = rug_fuzz_0;
        crate::parser::SchemeType::from(&p0);
        let _rug_ed_tests_rug_16_rrrruuuugggg_test_from = 0;
    }
}
#[cfg(test)]
mod tests_rug_17 {
    use super::*;
    use crate::parser::{SyntaxViolation, Input};
    #[test]
    fn test_trim_tab_and_newlines() {
        let _rug_st_tests_rug_17_rrrruuuugggg_test_trim_tab_and_newlines = 0;
        let rug_fuzz_0 = "    sample input\twith tabs and\nnewlines    ";
        let p0 = rug_fuzz_0;
        let p1: Option<&dyn Fn(SyntaxViolation)> = Some(&|_| {});
        Input::<'static>::trim_tab_and_newlines(&p0, p1);
        let _rug_ed_tests_rug_17_rrrruuuugggg_test_trim_tab_and_newlines = 0;
    }
}
#[cfg(test)]
mod tests_rug_18 {
    use super::*;
    use crate::parser::{SyntaxViolation, Input};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_18_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://www.example.com";
        let p0: &str = rug_fuzz_0;
        let p1: Option<&dyn Fn(SyntaxViolation)> = Some(&|_| {});
        <Input<'_>>::with_log(&p0, p1);
        let _rug_ed_tests_rug_18_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_20 {
    use super::*;
    use crate::parser::Input;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_20_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "example.com";
        let mut p0: Input<'static> = Input::new(rug_fuzz_0);
        <Input<'static>>::split_first(&p0);
        let _rug_ed_tests_rug_20_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_22 {
    use super::*;
    use crate::parser::Pattern;
    use crate::parser::Input;
    #[test]
    fn test_split_prefix() {
        let _rug_st_tests_rug_22_rrrruuuugggg_test_split_prefix = 0;
        let rug_fuzz_0 = "example";
        let rug_fuzz_1 = "example";
        let str_input = rug_fuzz_0;
        let mut input = Input::new(str_input);
        let pattern = rug_fuzz_1;
        debug_assert_eq!(
            < & str as Pattern > ::split_prefix(pattern, & mut input), true
        );
        let _rug_ed_tests_rug_22_rrrruuuugggg_test_split_prefix = 0;
    }
}
#[cfg(test)]
mod tests_rug_25 {
    use super::*;
    use crate::parser::Parser;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_25_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "example";
        let mut p0 = String::from(rug_fuzz_0);
        Parser::<'static>::for_setter(p0);
        let _rug_ed_tests_rug_25_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_30 {
    use super::*;
    use crate::parser::{Parser, Input, Host, ParseResult};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_30_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "http://example.com";
        let mut p0: Input<'_> = Input::new(rug_fuzz_0);
        Parser::<'_>::get_file_host(p0);
        let _rug_ed_tests_rug_30_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_32 {
    use super::*;
    use crate::parser::{Input, ParseResult, Parser};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_32_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "...";
        let input_str = rug_fuzz_0;
        let input = Input::new(input_str);
        Parser::file_host(input);
        let _rug_ed_tests_rug_32_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_33 {
    use super::*;
    use crate::parser::Parser;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_33_rrrruuuugggg_test_rug = 0;
        let rug_fuzz_0 = "https://www.example.com/path/to/file/";
        let rug_fuzz_1 = 17;
        let mut p0 = rug_fuzz_0;
        let mut p1 = rug_fuzz_1;
        Parser::last_slash_can_be_removed(&p0, p1);
        let _rug_ed_tests_rug_33_rrrruuuugggg_test_rug = 0;
    }
}
