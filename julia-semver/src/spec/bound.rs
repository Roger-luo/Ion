use std::fmt::Display;

use super::Version;
use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct VersionBound {
    pub(crate) major: u64,
    pub(crate) minor: u64,
    pub(crate) patch: u64,
    pub(crate) n: usize,
}

impl VersionBound {
    pub fn inf() -> Self {
        Self {
            major: 0,
            minor: 0,
            patch: 0,
            n: 0,
        }
    }

    pub fn nil() -> Self {
        Self {
            major: 0,
            minor: 0,
            patch: 0,
            n: 3,
        }
    }

    pub fn new(t: (u64, u64, u64), n: usize) -> Self {
        Self {
            major: t.0,
            minor: t.1,
            patch: t.2,
            n,
        }
    }

    pub fn parse(s: impl AsRef<str>) -> Result<Self> {
        let mut s = s.as_ref().trim();

        if s == "*" {
            return Ok(VersionBound::new((0, 0, 0), 0));
        }

        s = if s.starts_with("v") { &s[1..] } else { s };

        let mut parts = s.splitn(3, '.');
        match parts.next() {
            Some(major) => {
                let major = major.parse::<u64>()?;
                match parts.next() {
                    Some(minor) => {
                        let minor = minor.parse::<u64>()?;
                        match parts.next() {
                            Some(patch) => {
                                let patch = patch.parse::<u64>()?;
                                Ok(Self::new((major, minor, patch), 3))
                            }
                            None => Ok(Self::new((major, minor, 0), 2)),
                        }
                    }
                    None => Ok(Self::new((major, 0, 0), 1)),
                }
            }
            None => unreachable!("splitn always returns at least one element"),
        }
    }
}

impl PartialEq<Version> for VersionBound {
    fn eq(&self, other: &Version) -> bool {
        if self.n == 0 {
            true
        } else if self.n == 1 {
            other.major == self.major
        } else if self.n == 2 {
            other.major == self.major && other.minor == self.minor
        } else if self.n == 3 {
            other.major == self.major && other.minor == self.minor && other.patch == self.patch
        } else {
            false
        }
    }
}

impl PartialOrd<Version> for VersionBound {
    fn le(&self, other: &Version) -> bool {
        if self.n == 0 {
            true
        } else if self.n == 1 {
            self.major <= other.major
        } else if self.n == 2 {
            self.major <= other.major && self.minor <= other.minor
        } else if self.n == 3 {
            self.major <= other.major  && self.minor <= other.minor && self.patch <= other.patch
        } else {
            false
        }
    }

    fn ge(&self, other: &Version) -> bool {
        if self.n == 0 {
            true
        } else if self.n == 1 {
            self.major >= other.major
        } else if self.n == 2 {
            self.major >= other.major && self.minor >= other.minor
        } else if self.n == 3 {
            self.major >= other.major && self.minor >= other.minor && self.patch >= other.patch
        } else {
            false
        }
    }

    fn partial_cmp(&self, other: &Version) -> Option<std::cmp::Ordering> {
        if self == other {
            Some(std::cmp::Ordering::Equal)
        } else if self < other {
            Some(std::cmp::Ordering::Less)
        } else if self > other {
            Some(std::cmp::Ordering::Greater)
        } else {
            None
        }
    }
}

impl Display for VersionBound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.n {
            0 => return write!(f, "*"),
            1 => return write!(f, "{}.x.x", self.major),
            2 => return write!(f, "{}.{}.x", self.major, self.minor),
            3 => return write!(f, "{}.{}.{}", self.major, self.minor, self.patch),
            _ => unreachable!(),
        }
    }
}
