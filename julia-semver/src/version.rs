use anyhow::{format_err, Result};
use serde::{
    de::{Deserialize, Visitor},
    ser::{Serialize, Serializer},
};
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Pre {
    Numeric(u64),
    AlphaNumeric(String),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Build {
    Numeric(u64),
    AlphaNumeric(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub(crate) major: u64,
    pub(crate) minor: u64,
    pub(crate) patch: u64,
    pre: Vec<Pre>,
    build: Vec<Build>,
}

impl Version {
    pub fn parse(s: impl AsRef<str>) -> Result<Self> {
        let s = s.as_ref();
        let mut parts = s.splitn(2, '-');
        let version = parts.next().unwrap();
        let pre = parts.next();
        let mut parts = version.splitn(2, '+');
        let version = parts.next().unwrap();
        let build = parts.next();
        let mut parts = version.splitn(3, '.');
        let major = parts
            .next()
            .ok_or_else(|| format_err!("expect major version"))?
            .parse::<u64>()?;
        let minor = parts
            .next()
            .ok_or_else(|| format_err!("expect minor version"))?
            .parse::<u64>()?;
        let patch = parts
            .next()
            .ok_or_else(|| format_err!("expect patch version"))?
            .parse::<u64>()?;

        let pre = match pre {
            Some(pre) => pre
                .split('.')
                .map(|s| {
                    if let Ok(n) = s.parse::<u64>() {
                        Ok(Pre::Numeric(n))
                    } else {
                        Ok(Pre::AlphaNumeric(s.to_string()))
                    }
                })
                .collect::<Result<Vec<_>>>()?,
            None => Vec::new(),
        };

        let build = match build {
            Some(build) => build
                .split('.')
                .map(|s| {
                    if let Ok(n) = s.parse::<u64>() {
                        Ok(Build::Numeric(n))
                    } else {
                        Ok(Build::AlphaNumeric(s.to_string()))
                    }
                })
                .collect::<Result<Vec<_>>>()?,
            None => Vec::new(),
        };

        Ok(Self {
            major,
            minor,
            patch,
            pre,
            build,
        })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.major != other.major {
            return self.major.partial_cmp(&other.major);
        }
        if self.minor != other.minor {
            return self.minor.partial_cmp(&other.minor);
        }
        if self.patch != other.patch {
            return self.patch.partial_cmp(&other.patch);
        }
        if self.pre.len() != other.pre.len() {
            return self.pre.len().partial_cmp(&other.pre.len());
        }
        for (a, b) in self.pre.iter().zip(other.pre.iter()) {
            match (a, b) {
                (Pre::Numeric(a), Pre::Numeric(b)) => {
                    if a != b {
                        return a.partial_cmp(b);
                    }
                }
                (Pre::AlphaNumeric(a), Pre::AlphaNumeric(b)) => {
                    if a != b {
                        return a.partial_cmp(b);
                    }
                }
                (Pre::Numeric(_), Pre::AlphaNumeric(_)) => {
                    return Some(std::cmp::Ordering::Less);
                }
                (Pre::AlphaNumeric(_), Pre::Numeric(_)) => {
                    return Some(std::cmp::Ordering::Greater);
                }
            }
        }
        if self.build.len() != other.build.len() {
            return self.build.len().partial_cmp(&other.build.len());
        }
        for (a, b) in self.build.iter().zip(other.build.iter()) {
            match (a, b) {
                (Build::Numeric(a), Build::Numeric(b)) => {
                    if a != b {
                        return a.partial_cmp(b);
                    }
                }
                (Build::AlphaNumeric(a), Build::AlphaNumeric(b)) => {
                    if a != b {
                        return a.partial_cmp(b);
                    }
                }
                (Build::Numeric(_), Build::AlphaNumeric(_)) => {
                    return Some(std::cmp::Ordering::Less);
                }
                (Build::AlphaNumeric(_), Build::Numeric(_)) => {
                    return Some(std::cmp::Ordering::Greater);
                }
            }
        }
        Some(std::cmp::Ordering::Equal)
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if !self.pre.is_empty() {
            write!(f, "-")?;
            for (i, pre) in self.pre.iter().enumerate() {
                if i > 0 {
                    write!(f, ".")?;
                }
                match pre {
                    Pre::Numeric(n) => write!(f, "{}", n)?,
                    Pre::AlphaNumeric(s) => write!(f, "{}", s)?,
                }
            }
        }
        if !self.build.is_empty() {
            write!(f, "+")?;
            for (i, build) in self.build.iter().enumerate() {
                if i > 0 {
                    write!(f, ".")?;
                }
                match build {
                    Build::Numeric(n) => write!(f, "{}", n)?,
                    Build::AlphaNumeric(s) => write!(f, "{}", s)?,
                }
            }
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct VersionVisitor;

        impl<'de> Visitor<'de> for VersionVisitor {
            type Value = Version;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a version string")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Version::parse(value).map_err(|e| E::custom(e.to_string()))
            }
        }

        deserializer.deserialize_str(VersionVisitor)
    }
}

impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let version = Version::parse("1.2.3").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
        assert!(version.pre.is_empty());
        assert!(version.build.is_empty());
        assert_eq!(version.to_string(), "1.2.3");

        let version = Version::parse("1.2.3-alpha").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
        assert_eq!(version.pre.len(), 1);
        assert_eq!(version.pre[0], Pre::AlphaNumeric("alpha".to_string()));
        assert!(version.build.is_empty());
        assert_eq!(version.to_string(), "1.2.3-alpha");

        let version = Version::parse("1.2.3-alpha.1").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
        assert_eq!(version.pre.len(), 2);
        assert_eq!(version.pre[0], Pre::AlphaNumeric("alpha".to_string()));
        assert_eq!(version.pre[1], Pre::Numeric(1));
        assert!(version.build.is_empty());
        assert_eq!(version.to_string(), "1.2.3-alpha.1");

        let version = Version::parse("1.2.3+build.1").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
        assert!(version.pre.is_empty());
        assert_eq!(version.build.len(), 2);
        assert_eq!(version.build[0], Build::AlphaNumeric("build".to_string()));
        assert_eq!(version.build[1], Build::Numeric(1));
        assert_eq!(version.to_string(), "1.2.3+build.1");
    }

    #[test]
    fn test_ord() {
        let version1 = Version::parse("1.2.3").unwrap();
        let version2 = Version::parse("1.2.3").unwrap();
        assert_eq!(version1.cmp(&version2), std::cmp::Ordering::Equal);

        let version1 = Version::parse("1.2.3").unwrap();
        let version2 = Version::parse("1.2.4").unwrap();
        assert_eq!(version1.cmp(&version2), std::cmp::Ordering::Less);

        let version1 = Version::parse("1.2.3").unwrap();
        let version2 = Version::parse("1.2.2").unwrap();
        assert_eq!(version1.cmp(&version2), std::cmp::Ordering::Greater);

        let version1 = Version::parse("1.2.3").unwrap();
        let version2 = Version::parse("1.3.3").unwrap();
        assert_eq!(version1.cmp(&version2), std::cmp::Ordering::Less);

        let version1 = Version::parse("1.2.3").unwrap();
        let version2 = Version::parse("1.1.3").unwrap();
        assert_eq!(version1.cmp(&version2), std::cmp::Ordering::Greater);

        let version1 = Version::parse("1.2.3").unwrap();
        let version2 = Version::parse("2.2.3").unwrap();
        assert_eq!(version1.cmp(&version2), std::cmp::Ordering::Less);

        let version1 = Version::parse("1.2.3").unwrap();
        let version2 = Version::parse("0.2.3").unwrap();
        assert_eq!(version1.cmp(&version2), std::cmp::Ordering::Greater);

        let version1 = Version::parse("1.2.3-alpha").unwrap();
        let version2 = Version::parse("1.2.3-alpha").unwrap();
        assert_eq!(version1.cmp(&version2), std::cmp::Ordering::Equal);

        let version1 = Version::parse("1.2.3-alpha").unwrap();
        let version2 = Version::parse("1.2.3-alpha.1").unwrap();
        assert_eq!(version1.cmp(&version2), std::cmp::Ordering::Less);
    }
}
