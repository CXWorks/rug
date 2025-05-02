use std::{cell::RefCell, collections::hash_map, env, fs, hash::Hasher, time::SystemTime};
use super::tz_info::TimeZone;
use super::{DateTime, FixedOffset, Local, NaiveDateTime};
use crate::{Datelike, LocalResult, Utc};
pub(super) fn now() -> DateTime<Local> {
    let now = Utc::now().naive_utc();
    naive_to_local(&now, false).unwrap()
}
pub(super) fn naive_to_local(
    d: &NaiveDateTime,
    local: bool,
) -> LocalResult<DateTime<Local>> {
    TZ_INFO
        .with(|maybe_cache| {
            maybe_cache.borrow_mut().get_or_insert_with(Cache::default).offset(*d, local)
        })
}
thread_local! {
    static TZ_INFO : RefCell < Option < Cache >> = Default::default();
}
enum Source {
    LocalTime { mtime: SystemTime },
    Environment { hash: u64 },
}
impl Source {
    fn new(env_tz: Option<&str>) -> Source {
        match env_tz {
            Some(tz) => {
                let mut hasher = hash_map::DefaultHasher::new();
                hasher.write(tz.as_bytes());
                let hash = hasher.finish();
                Source::Environment { hash }
            }
            None => {
                match fs::symlink_metadata("/etc/localtime") {
                    Ok(data) => {
                        Source::LocalTime {
                            mtime: data.modified().unwrap_or_else(|_| SystemTime::now()),
                        }
                    }
                    Err(_) => {
                        Source::LocalTime {
                            mtime: SystemTime::now(),
                        }
                    }
                }
            }
        }
    }
}
struct Cache {
    zone: TimeZone,
    source: Source,
    last_checked: SystemTime,
}
#[cfg(target_os = "android")]
const TZDB_LOCATION: &str = " /system/usr/share/zoneinfo";
#[cfg(target_os = "aix")]
const TZDB_LOCATION: &str = "/usr/share/lib/zoneinfo";
#[allow(dead_code)]
#[cfg(not(any(target_os = "android", target_os = "aix")))]
const TZDB_LOCATION: &str = "/usr/share/zoneinfo";
fn fallback_timezone() -> Option<TimeZone> {
    let tz_name = iana_time_zone::get_timezone().ok()?;
    let bytes = fs::read(format!("{}/{}", TZDB_LOCATION, tz_name)).ok()?;
    TimeZone::from_tz_data(&bytes).ok()
}
impl Default for Cache {
    fn default() -> Cache {
        let env_tz = env::var("TZ").ok();
        let env_ref = env_tz.as_deref();
        Cache {
            last_checked: SystemTime::now(),
            source: Source::new(env_ref),
            zone: current_zone(env_ref),
        }
    }
}
fn current_zone(var: Option<&str>) -> TimeZone {
    TimeZone::local(var).ok().or_else(fallback_timezone).unwrap_or_else(TimeZone::utc)
}
impl Cache {
    fn offset(&mut self, d: NaiveDateTime, local: bool) -> LocalResult<DateTime<Local>> {
        let now = SystemTime::now();
        match now.duration_since(self.last_checked) {
            Ok(d) if d.as_secs() < 1 => {}
            Ok(_) | Err(_) => {
                let env_tz = env::var("TZ").ok();
                let env_ref = env_tz.as_deref();
                let new_source = Source::new(env_ref);
                let out_of_date = match (&self.source, &new_source) {
                    (Source::Environment { .. }, Source::LocalTime { .. })
                    | (Source::LocalTime { .. }, Source::Environment { .. }) => true,
                    (
                        Source::LocalTime { mtime: old_mtime },
                        Source::LocalTime { mtime },
                    ) if old_mtime != mtime => true,
                    (
                        Source::Environment { hash: old_hash },
                        Source::Environment { hash },
                    ) if old_hash != hash => true,
                    _ => false,
                };
                if out_of_date {
                    self.zone = current_zone(env_ref);
                }
                self.last_checked = now;
                self.source = new_source;
            }
        }
        if !local {
            let offset = self
                .zone
                .find_local_time_type(d.timestamp())
                .expect("unable to select local time type")
                .offset();
            return match FixedOffset::east_opt(offset) {
                Some(offset) => LocalResult::Single(DateTime::from_utc(d, offset)),
                None => LocalResult::None,
            };
        }
        match self
            .zone
            .find_local_time_type_from_local(d.timestamp(), d.year())
            .expect("unable to select local time type")
        {
            LocalResult::None => LocalResult::None,
            LocalResult::Ambiguous(early, late) => {
                let early_offset = FixedOffset::east_opt(early.offset()).unwrap();
                let late_offset = FixedOffset::east_opt(late.offset()).unwrap();
                LocalResult::Ambiguous(
                    DateTime::from_utc(d - early_offset, early_offset),
                    DateTime::from_utc(d - late_offset, late_offset),
                )
            }
            LocalResult::Single(tt) => {
                let offset = FixedOffset::east_opt(tt.offset()).unwrap();
                LocalResult::Single(DateTime::from_utc(d - offset, offset))
            }
        }
    }
}
#[cfg(test)]
mod tests_rug_383 {
    use super::*;
    use crate::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_383_rrrruuuugggg_test_rug = 0;
        now();
        let _rug_ed_tests_rug_383_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_385 {
    use super::*;
    use crate::offset::TimeZone;
    use fs;
    use iana_time_zone;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_385_rrrruuuugggg_test_rug = 0;
        fallback_timezone();
        let _rug_ed_tests_rug_385_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_388 {
    use super::*;
    use crate::offset::local::inner::Cache;
    use std::default::Default;
    #[test]
    fn test_rug() {
        let _rug_st_tests_rug_388_rrrruuuugggg_test_rug = 0;
        let cache: Cache = <Cache as Default>::default();
        let _rug_ed_tests_rug_388_rrrruuuugggg_test_rug = 0;
    }
}
#[cfg(test)]
mod tests_rug_389 {
    use super::*;
    use crate::offset::local::inner::{Cache, Source};
    use crate::offset::{LocalResult, FixedOffset};
    use crate::DateTime;
    use crate::NaiveDateTime;
    use std::env;
    use std::time::SystemTime;
    #[test]
    fn test_offset() {
        let _rug_st_tests_rug_389_rrrruuuugggg_test_offset = 0;
        let rug_fuzz_0 = 1626759290;
        let rug_fuzz_1 = 0;
        let rug_fuzz_2 = true;
        let mut p0: Cache = Cache::default();
        let p1: NaiveDateTime = NaiveDateTime::from_timestamp(rug_fuzz_0, rug_fuzz_1);
        let p2: bool = rug_fuzz_2;
        p0.offset(p1, p2);
        let _rug_ed_tests_rug_389_rrrruuuugggg_test_offset = 0;
    }
}
