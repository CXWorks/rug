use std::str::FromStr;
use std::ops::Deref;
use std::fmt;
use std::time::{Duration as StdDuration, SystemTime};

use crate::duration::{self, parse_duration, format_duration};
use crate::date::{self, parse_rfc3339_weak, format_rfc3339};

/// A wrapper for duration that has `FromStr` implementation
///
/// This is useful if you want to use it somewhere where `FromStr` is
/// expected.
///
/// See `parse_duration` for the description of the format.
///
/// # Example
///
/// ```
/// use std::time::Duration;
/// let x: Duration;
/// x = "12h 5min 2ns".parse::<humantime::Duration>().unwrap().into();
/// assert_eq!(x, Duration::new(12*3600 + 5*60, 2))
/// ```
///
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct Duration(StdDuration);

/// A wrapper for SystemTime that has `FromStr` implementation
///
/// This is useful if you want to use it somewhere where `FromStr` is
/// expected.
///
/// See `parse_rfc3339_weak` for the description of the format. The "weak"
/// format is used as it's more pemissive for human input as this is the
/// expected use of the type (e.g. command-line parsing).
///
/// # Example
///
/// ```
/// use std::time::SystemTime;
/// let x: SystemTime;
/// x = "2018-02-16T00:31:37Z".parse::<humantime::Timestamp>().unwrap().into();
/// assert_eq!(humantime::format_rfc3339(x).to_string(), "2018-02-16T00:31:37Z");
/// ```
///
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Timestamp(SystemTime);

impl AsRef<StdDuration> for Duration {
    fn as_ref(&self) -> &StdDuration { &self.0 }
}

impl Deref for Duration {
    type Target = StdDuration;
    fn deref(&self) -> &StdDuration { &self.0 }
}

impl Into<StdDuration> for Duration {
    fn into(self) -> StdDuration { self.0 }
}

impl From<StdDuration> for Duration {
    fn from(dur: StdDuration) -> Duration { Duration(dur) }
}

impl FromStr for Duration {
    type Err = duration::Error;
    fn from_str(s: &str) -> Result<Duration, Self::Err> {
        parse_duration(s).map(Duration)
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        format_duration(self.0).fmt(f)
    }
}

impl AsRef<SystemTime> for Timestamp {
    fn as_ref(&self) -> &SystemTime { &self.0 }
}

impl Deref for Timestamp {
    type Target = SystemTime;
    fn deref(&self) -> &SystemTime { &self.0 }
}

impl Into<SystemTime> for Timestamp {
    fn into(self) -> SystemTime { self.0 }
}

impl From<SystemTime> for Timestamp {
    fn from(dur: SystemTime) -> Timestamp { Timestamp(dur) }
}

impl FromStr for Timestamp {
    type Err = date::Error;
    fn from_str(s: &str) -> Result<Timestamp, Self::Err> {
        parse_rfc3339_weak(s).map(Timestamp)
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        format_rfc3339(self.0).fmt(f)
    }
}
#[cfg(test)]
mod tests_llm_16_3 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_as_ref() {
        let duration = wrapper::Duration::from_str("1h").unwrap();
        let as_ref_duration = duration.as_ref();
        assert_eq!(*as_ref_duration, std::time::Duration::from_secs(3600));
    }
}#[cfg(test)]
mod tests_llm_16_4 {
    use crate::wrapper::Duration;
    use std::time::Duration as StdDuration;
    use std::str::FromStr;
    
    #[test]
    fn test_from() {
        let dur: StdDuration = StdDuration::new(10, 0);
        let res: Duration = Duration::from(dur);
        assert_eq!(res, Duration(StdDuration::new(10, 0)));
    }
}#[cfg(test)]
mod tests_llm_16_5 {
    use std::time::Duration;
    use crate::{Duration as HDuration, parse_duration};

    #[test]
    fn test_into_duration() {
        let h_duration: HDuration = "12h 5min 2ns".parse().unwrap();
        let std_duration: Duration = h_duration.into();
        assert_eq!(std_duration, Duration::new(12 * 3600 + 5 * 60, 2));
    }
}#[cfg(test)]
mod tests_llm_16_6 {
    use super::*;

use crate::*;
    use std::time::Duration as StdDuration;
    
    #[test]
    fn test_deref() {
        let duration = Duration(StdDuration::new(10, 0));
        let dereferenced = duration.deref();
        assert_eq!(dereferenced, &StdDuration::new(10, 0));
    }
}#[cfg(test)]
mod tests_llm_16_9 {
    use super::*;

use crate::*;
    use std::str::FromStr;

    #[test]
    fn test_as_ref() {
        let timestamp = Timestamp(SystemTime::UNIX_EPOCH);
        let result = timestamp.as_ref();
        assert_eq!(result, &SystemTime::UNIX_EPOCH);
    }
}mod tests_llm_16_10 {
    use super::*;

use crate::*;
    use std::str::FromStr;

    #[test]
    fn test_from() {
        let dur = SystemTime::UNIX_EPOCH;
        let timestamp = Timestamp::from(dur);
        assert_eq!(dur, *timestamp);
    }

    #[test]
    fn test_from_str() {
        let timestamp = Timestamp::from_str("2018-02-16T00:31:37Z").unwrap();
        assert_eq!(timestamp.to_string(), "2018-02-16T00:31:37Z");
    }
}#[cfg(test)]
mod tests_llm_16_11 {
    use super::*;

use crate::*;
    
    #[test]
    fn test_into() {
        let timestamp = Timestamp(SystemTime::UNIX_EPOCH);
        let system_time: SystemTime = timestamp.into();
        assert_eq!(system_time, SystemTime::UNIX_EPOCH);
    }
}#[cfg(test)]
mod tests_llm_16_12 {
    use super::*;

use crate::*;
    use std::time::SystemTime;
    
    #[test]
    fn test_deref() {
        let time = SystemTime::UNIX_EPOCH;
        let timestamp = Timestamp::from(time);
        
        assert_eq!(timestamp.deref(), &time);
    }
}