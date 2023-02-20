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
        let s = s.as_ref().trim();
        if s == "*" {
            return Ok(VersionBound::new((0, 0, 0), 0));
        }

        let s = s.strip_prefix('v').unwrap_or(s);

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

    pub fn less_sim(&self, other: &Version) -> bool {
        if self.n == 0 {
            true
        } else if self.n == 1 {
            self.major <= other.major
        } else if self.n == 2 {
            (self.major, self.minor) <= (other.major, other.minor)
        } else if self.n == 3 {
            (self.major, self.minor, self.patch) <= (other.major, other.minor, other.patch)
        } else {
            unreachable!("n should be in [0, 3]")
        }
    }

    pub fn greater_sim(&self, other: &Version) -> bool {
        if self.n == 0 {
            true
        } else if self.n == 1 {
            self.major >= other.major
        } else if self.n == 2 {
            (self.major, self.minor) >= (other.major, other.minor)
        } else if self.n == 3 {
            (self.major, self.minor, self.patch) >= (other.major, other.minor, other.patch)
        } else {
            unreachable!("n should be in [0, 3]")
        }
    }
}

impl Display for VersionBound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.n {
            0 => write!(f, "*"),
            1 => write!(f, "{}", self.major),
            2 => write!(f, "{}.{}", self.major, self.minor),
            3 => write!(f, "{}.{}.{}", self.major, self.minor, self.patch),
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        assert_eq!(VersionBound::parse("1.2.3").unwrap(), VersionBound::new((1, 2, 3), 3));
        assert_eq!(VersionBound::parse("1.2").unwrap(), VersionBound::new((1, 2, 0), 2));
        assert_eq!(VersionBound::parse("1").unwrap(), VersionBound::new((1, 0, 0), 1));
        assert_eq!(VersionBound::parse("*").unwrap(), VersionBound::new((0, 0, 0), 0));
        assert_eq!(VersionBound::parse("v1.2.3").unwrap(), VersionBound::new((1, 2, 3), 3));
        assert_eq!(VersionBound::parse("v1.2").unwrap(), VersionBound::new((1, 2, 0), 2));
        assert_eq!(VersionBound::parse("v1").unwrap(), VersionBound::new((1, 0, 0), 1));
    }

    #[test]
    fn test_cmp() {
        let v1 = Version::parse("1.2.3").unwrap();
        let v2 = Version::parse("1.2.4").unwrap();
        let v3 = Version::parse("1.3.0").unwrap();
        let v4 = Version::parse("2.0.0").unwrap();

        let b1 = VersionBound::parse("1.2.3").unwrap();
        let b2 = VersionBound::parse("1.2").unwrap();
        let b3 = VersionBound::parse("1").unwrap();
        let b4 = VersionBound::parse("*").unwrap();

        assert!(b1.less_sim(&v2));
        assert!(b2.less_sim(&v2));
        assert!(b3.less_sim(&v2));
        assert!(b4.less_sim(&v2));

        assert!(b1.less_sim(&v3));
        assert!(b2.less_sim(&v3));
        assert!(b3.less_sim(&v3));
        assert!(b4.less_sim(&v3));

        assert!(b1.less_sim(&v4));
        assert!(b2.less_sim(&v4));
        assert!(b3.less_sim(&v4));
        assert!(b4.less_sim(&v4));

        assert!(!b1.greater_sim(&v2));
        assert!(b2.greater_sim(&v2));
        assert!(b3.greater_sim(&v2));
        assert!(b4.greater_sim(&v2));
        
        assert!(b1.greater_sim(&v1));
        assert!(b2.greater_sim(&v1));
        assert!(b3.greater_sim(&v1));
        assert!(b4.greater_sim(&v1));
    }
}
