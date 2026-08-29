#[cfg(feature = "serialize")]
use super::wire::{Serialize, Write};

use core::fmt::{self, Display};
use core::time::Duration;

#[cfg(feature = "std")]
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct TimeTagError(Duration);

impl Display for TimeTagError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "time tag underflow")
    }
}

impl core::error::Error for TimeTagError {}

impl TimeTagError {
    pub fn duration(&self) -> Duration {
        self.0
    }
}

/// NTP epoch: 1900-01-01T00:00:00Z
/// Unix epoch: 1970-01-01T00:00:00Z
///
/// Leap years (13 in total):
/// 1904, 1908, 1912, 1916, 1920, 1924, 1928, 1932, 1936, 1940, 1944, 1948,
/// 1952, 1956, 1960, 1964, 1968
///
/// Days: 13 * 366 + (70 - 13) * 365 = 25_567
/// Seconds: 25_567 * 24 * 3_600 = 2_208_988_800
const NTP_TO_UNIX_OFFSET: u32 = 2_208_988_800;

const fn frac_to_nanos(frac: u32) -> u32 {
    let temp = frac as u64 * 1_000_000_000u64;
    (temp >> 32) as _
}

const fn nanos_to_frac(nanos: u32) -> u32 {
    let temp = (nanos as u64) << 32;
    (temp / 1_000_000_000u64) as _
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimeTag(u64);

impl Default for TimeTag {
    fn default() -> Self {
        Self::IMMEDIATE
    }
}

impl TimeTag {
    pub const IMMEDIATE: Self = Self(1);
    pub const NTP_EPOCH: Self = Self(0);
    pub const UNIX_EPOCH: Self = Self::new(NTP_TO_UNIX_OFFSET, 0);

    #[inline]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    #[inline]
    pub const fn as_raw(&self) -> u64 {
        self.0
    }

    #[inline]
    pub const fn new(secs: u32, frac: u32) -> Self {
        Self((secs as u64) << 32 | frac as u64)
    }

    #[inline]
    pub const fn new_nanos(secs: u32, nanos: u32) -> Self {
        Self::new(secs, nanos_to_frac(nanos))
    }

    #[inline]
    pub const fn immediate() -> Self {
        Self::IMMEDIATE
    }

    #[inline]
    pub const fn secs(&self) -> u32 {
        (self.0 >> 32) as _
    }

    #[inline]
    pub const fn frac(&self) -> u32 {
        self.0 as _
    }

    #[inline]
    pub const fn nanos(&self) -> u32 {
        frac_to_nanos(self.frac())
    }

    #[inline]
    pub fn is_immediate(&self) -> bool {
        #[cfg(not(feature = "compat_immediate_zero"))]
        {
            self.0 == 1u64
        }
        #[cfg(feature = "compat_immediate_zero")]
        {
            self.0 == 0u64 || self.0 == 1u64
        }
    }

    #[cfg(feature = "std")]
    pub fn to_system_time(&self) -> Result<SystemTime, TimeTagError> {
        Ok(SystemTime::UNIX_EPOCH + self.duration_since(Self::UNIX_EPOCH)?)
    }

    pub const fn duration_since(&self, earlier: TimeTag) -> Result<Duration, TimeTagError> {
        if self.0 >= earlier.0 {
            Ok(TimeTag(self.0 - earlier.0).to_duration())
        } else {
            Err(TimeTagError(TimeTag(earlier.0 - self.0).to_duration()))
        }
    }

    pub const fn to_duration(&self) -> Duration {
        Duration::new(self.secs() as _, self.nanos())
    }
}

#[cfg(feature = "std")]
impl From<SystemTime> for TimeTag {
    fn from(time: SystemTime) -> Self {
        let duration_unix = time.duration_since(SystemTime::UNIX_EPOCH).unwrap();
        let secs = NTP_TO_UNIX_OFFSET + duration_unix.as_secs() as u32;
        Self::new_nanos(secs, duration_unix.subsec_nanos() as _)
    }
}

#[cfg(feature = "serialize")]
impl Serialize for TimeTag {
    fn serialize<W: Write>(&self, w: &mut W) -> Result<(), W::Error> {
        w.write_be_u64(self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn immediate() {
        assert!(TimeTag::immediate().is_immediate());
        #[cfg(not(feature = "compat_immediate_zero"))]
        assert!(!TimeTag::from_raw(0).is_immediate());
        #[cfg(feature = "compat_immediate_zero")]
        assert!(TimeTag::from_raw(0).is_immediate());
    }
}
