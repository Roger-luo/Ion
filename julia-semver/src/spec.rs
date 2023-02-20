use super::*;
use anyhow::Result;
use serde::{
    de::{Deserialize, Visitor},
    ser::{Serialize, Serializer},
};
use std::fmt::Display;

#[macro_export]
macro_rules! version {
    ($str:expr) => {
        Version::parse($str)
    };
}

#[derive(Debug, Clone)]
pub struct VersionSpec {
    ranges: Vec<VersionRange>,
}

impl VersionSpec {
    pub fn new(ranges: Vec<VersionRange>) -> Self {
        Self { ranges }
    }

    pub fn parse(s: impl AsRef<str>) -> Result<Self> {
        let s = s.as_ref().trim();
        let ranges = s
            .split(',')
            .map(VersionRange::parse)
            .collect::<Result<Vec<_>>>()?;
        Ok(VersionSpec::new(ranges))
    }

    pub fn contains(&self, version: &Version) -> bool {
        self.ranges.iter().any(|r| r.contains(version))
    }
}

impl Display for VersionSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for r in &self.ranges {
            if first {
                first = false;
            } else {
                write!(f, ", ")?;
            }
            write!(f, "{r}")?;
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for VersionSpec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        struct VersionSpecVisitor;

        impl<'de> Visitor<'de> for VersionSpecVisitor {
            type Value = VersionSpec;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a version spec")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                VersionSpec::parse(v).map_err(|e| E::custom(e.to_string()))
            }
        }

        deserializer.deserialize_str(VersionSpecVisitor)
    }
}

impl Serialize for VersionSpec {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
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
    fn test_spec() {
        let spec = VersionSpec::parse("1.0.0, 1.1.0").unwrap();
        assert!(spec.contains(&Version::parse("1.0.0").unwrap()));
        assert!(spec.contains(&Version::parse("1.1.0").unwrap()));
        assert!(spec.contains(&Version::parse("1.2.0").unwrap()));

        assert_eq!(spec.to_string(), "1.0.0 - 1, 1.1.0 - 1");
    }
}
