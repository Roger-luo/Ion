use std::fmt::Display;

use super::Version;
use super::VersionBound;
use anyhow::anyhow;
use anyhow::Result;
use regex::Regex;
use serde::{
    de::{Deserialize, Visitor},
    ser::{Serialize, Serializer},
};

#[derive(Debug, Clone, Copy)]
pub struct VersionRange {
    lower: VersionBound,
    upper: VersionBound,
}

impl VersionRange {
    pub fn new(lower: VersionBound, upper: VersionBound) -> Self {
        Self { lower, upper }
    }

    pub fn parse(s: impl AsRef<str>) -> Result<Self> {
        let s = s.as_ref().trim();
        let version = "v?([0-9]+?)(?:\\.([0-9]+?))?(?:\\.([0-9]+?))?";
        let semver = Regex::new(format!("^([~^]?)?{version}$").as_str()).unwrap();
        let inequality = Regex::new(
            format!("^((?:≥\\s*)|(?:>=\\s*)|(?:=\\s*)|(?:<\\s*)|(?:=\\s*))v?{version}$").as_str(),
        )
        .unwrap();
        let hyphen =
            Regex::new(format!("^[\\s]*{version}[\\s]*?\\s-\\s[\\s]*?{version}[\\s]*$").as_str())
                .unwrap();

        if semver.is_match(s) {
            let cap = semver
                .captures(s)
                .ok_or_else(|| anyhow!("Invalid version range: {}", s))?;
            VersionRange::parse_semver(s, cap)
        } else if inequality.is_match(s) {
            let cap = inequality
                .captures(s)
                .ok_or_else(|| anyhow!("Invalid version range: {}", s))?;
            VersionRange::parse_inequality(s, cap)
        } else if hyphen.is_match(s) {
            let cap = hyphen
                .captures(s)
                .ok_or_else(|| anyhow!("Invalid version range: {}", s))?;
            VersionRange::parse_hyphen(s, cap)
        } else {
            anyhow::bail!("Invalid version range: {}", s);
        }
    }

    fn parse_semver(s: &str, cap: regex::Captures) -> Result<Self> {
        let (typ, v) = VersionRange::unpack_spec_four(s, cap)?;

        if typ.is_empty() || typ == "^" {
            if v.major != 0 {
                Ok(VersionRange::new(v, VersionBound::new((v.major, 0, 0), 1)))
            } else if v.minor != 0 {
                Ok(VersionRange::new(
                    v,
                    VersionBound::new((v.major, v.minor, 0), 2),
                ))
            } else if v.n < 3 {
                Ok(VersionRange::new(v, VersionBound::new((0, 0, 0), v.n)))
            } else {
                Ok(VersionRange::new(
                    v,
                    VersionBound::new((0, 0, v.patch), v.n),
                ))
            }
        } else if typ == "~" {
            if v.n == 3 || v.n == 2 {
                Ok(VersionRange::new(
                    v,
                    VersionBound::new((v.major, v.minor, 0), 2),
                ))
            } else {
                Ok(VersionRange::new(v, VersionBound::new((v.major, 0, 0), 1)))
            }
        } else {
            anyhow::bail!("Invalid version range: {}", s);
        }
    }

    fn parse_inequality(s: &str, cap: regex::Captures) -> Result<Self> {
        let (typ, v) = VersionRange::unpack_spec_four(s, cap)?;
        let typ = typ.as_str();
        if Regex::new(r#"^<\s*$"#)?.is_match(typ) {
            let nil = VersionBound::nil();
            let upper = if v.patch == 0 {
                if v.minor == 0 {
                    if v.major > 0 {
                        VersionBound::new((v.major - 1, 0, 0), 1)
                    } else {
                        anyhow::bail!("Invalid version range: {}", s);
                    }
                } else {
                    VersionBound::new((v.major, v.minor - 1, 0), 2)
                }
            } else {
                VersionBound::new((v.major, v.minor, v.patch - 1), 3)
            };
            Ok(VersionRange::new(nil, upper))
        } else if Regex::new(r#"^=\s*$"#)?.is_match(typ) {
            Ok(VersionRange::new(v, v))
        } else if Regex::new(r#"^>=\s*$"#)?.is_match(typ) || Regex::new(r#"^≥\s*$"#)?.is_match(typ)
        {
            Ok(VersionRange::new(v, VersionBound::inf()))
        } else {
            anyhow::bail!("invalid prefix {typ}")
        }
    }

    fn parse_hyphen(s: &str, cap: regex::Captures) -> Result<Self> {
        let err = || anyhow!("Invliad version range: {}", s);

        if cap.len() != 7 {
            return Err(err());
        }

        let lower_major = cap.get(1).ok_or_else(err)?.as_str().parse::<u64>()?;
        let lower_minor = cap.get(2);
        let lower_patch = cap.get(3);

        let upper_major = cap.get(4).ok_or_else(err)?.as_str().parse::<u64>()?;
        let upper_minor = cap.get(5);
        let upper_patch = cap.get(6);

        let lower = match (lower_minor, lower_patch) {
            (Some(minor), Some(patch)) => {
                let minor = minor.as_str().parse::<u64>()?;
                let patch = patch.as_str().parse::<u64>()?;
                VersionBound::new((lower_major, minor, patch), 3)
            }
            (Some(minor), None) => {
                let minor = minor.as_str().parse::<u64>()?;
                VersionBound::new((lower_major, minor, 0), 2)
            }
            (None, None) => VersionBound::new((lower_major, 0, 0), 1),
            _ => unreachable!(),
        };

        let upper = match (upper_minor, upper_patch) {
            (Some(minor), Some(patch)) => {
                let minor = minor.as_str().parse::<u64>()?;
                let patch = patch.as_str().parse::<u64>()?;
                VersionBound::new((upper_major, minor, patch), 3)
            }
            (Some(minor), None) => {
                let minor = minor.as_str().parse::<u64>()?;
                VersionBound::new((upper_major, minor, 0), 2)
            }
            (None, None) => VersionBound::new((upper_major, 0, 0), 1),
            _ => unreachable!(),
        };

        Ok(VersionRange::new(lower, upper))
    }

    fn unpack_spec_four(s: &str, cap: regex::Captures) -> Result<(String, VersionBound)> {
        if cap.len() != 5 {
            anyhow::bail!("Invalid version range: {}", s);
        }

        let n_significant = cap.iter().filter(|c| c.is_some()).count() - 2;
        let typ = cap
            .get(1)
            .ok_or_else(|| anyhow!("Invalid version range: {}", s))?
            .as_str();
        let major = cap
            .get(2)
            .ok_or_else(|| anyhow!("Invalid version range: {}", s))?
            .as_str()
            .parse::<u64>()?;

        let minor = if n_significant < 2 {
            0
        } else {
            cap.get(3)
                .ok_or_else(|| anyhow!("Invalid version range: {}", s))?
                .as_str()
                .parse::<u64>()?
        };

        let patch = if n_significant < 3 {
            0
        } else {
            cap.get(4)
                .ok_or_else(|| anyhow!("Invalid version range: {}", s))?
                .as_str()
                .parse::<u64>()?
        };

        if n_significant == 3 && major == 0 && minor == 0 && patch == 0 {
            anyhow::bail!("Invalid version range: 0.0.0");
        }

        Ok((
            typ.into(),
            VersionBound::new((major, minor, patch), n_significant),
        ))
    }

    pub fn contains(&self, version: &Version) -> bool {
        self.lower.less_sim(version) && self.upper.greater_sim(version)
    }
}

impl Display for VersionRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.lower.n, self.upper.n) {
            (0, 0) => write!(f, "*"),
            (0, _) => write!(f, "0 - {}", self.upper),
            (_, 0) => write!(f, "{} - *", self.lower),
            _ => write!(f, "{} - {}", self.lower, self.upper),
        }
    }
}

impl<'de> Deserialize<'de> for VersionRange {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct VersionRangeVisitor;

        impl<'de> Visitor<'de> for VersionRangeVisitor {
            type Value = VersionRange;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a version range")
            }

            fn visit_str<E>(self, s: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                VersionRange::parse(s).map_err(E::custom)
            }
        }

        deserializer.deserialize_str(VersionRangeVisitor)
    }
}

impl Serialize for VersionRange {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::version;

    use super::*;
    use anyhow::Result;

    #[test]
    fn test_semver() -> Result<()> {
        let bound = VersionRange::parse("0.1.0")?;
        assert_eq!(bound.to_string(), "0.1.0 - 0.1");
        assert!(bound.contains(&version!("0.1.0")?));

        let bound = VersionRange::parse("0.1")?;
        assert_eq!(bound.to_string(), "0.1 - 0.1");
        assert!(bound.contains(&version!("0.1.0")?));

        let bound = VersionRange::parse("0")?;
        assert_eq!(bound.to_string(), "0 - 0");
        assert!(bound.contains(&version!("0.1.0")?));

        let bound = VersionRange::parse("1.1.0")?;
        assert_eq!(bound.to_string(), "1.1.0 - 1");
        assert!(bound.contains(&version!("1.1.0")?));

        let bound = VersionRange::parse("1.1")?;
        assert_eq!(bound.to_string(), "1.1 - 1");
        assert!(bound.contains(&version!("1.1.0")?));

        let bound = VersionRange::parse("1")?;
        assert_eq!(bound.to_string(), "1 - 1");
        assert!(bound.contains(&version!("1.1.0")?));

        let bound = VersionRange::parse("^0.1.0")?;
        assert_eq!(bound.to_string(), "0.1.0 - 0.1");
        assert!(bound.contains(&version!("0.1.0")?));

        let bound = VersionRange::parse("^0.1")?;
        assert_eq!(bound.to_string(), "0.1 - 0.1");
        assert!(bound.contains(&version!("0.1.0")?));

        let bound = VersionRange::parse("^0")?;
        assert_eq!(bound.to_string(), "0 - 0");
        assert!(bound.contains(&version!("0.1.0")?));

        Ok(())
    }

    #[test]
    fn test_inequality() -> Result<()> {
        let bound = VersionRange::parse(">=0.1.0")?;
        assert_eq!(bound.to_string(), "0.1.0 - *");
        assert!(bound.contains(&version!("0.1.0")?));

        let bound = VersionRange::parse(">=0.1")?;
        assert_eq!(bound.to_string(), "0.1 - *");
        assert!(bound.contains(&version!("0.1.0")?));

        let bound = VersionRange::parse(">=0")?;
        assert_eq!(bound.to_string(), "0 - *");
        assert!(bound.contains(&version!("0.1.0")?));
        assert!(bound.contains(&version!("1.1.0")?));

        let bound = VersionRange::parse(">=1.1.0")?;
        assert_eq!(bound.to_string(), "1.1.0 - *");
        assert!(bound.contains(&version!("1.1.0")?));

        let bound = VersionRange::parse(">=1.1")?;
        assert_eq!(bound.to_string(), "1.1 - *");
        assert!(bound.contains(&version!("1.1.0")?));

        let bound = VersionRange::parse(">=1")?;
        assert_eq!(bound.to_string(), "1 - *");
        assert!(bound.contains(&version!("1.1.0")?));

        let bound = VersionRange::parse("<0.1.0")?;
        assert_eq!(bound.to_string(), "0.0.0 - 0.0");
        assert!(bound.contains(&version!("0.0.0")?));

        let bound = VersionRange::parse("<0.1")?;
        assert_eq!(bound.to_string(), "0.0.0 - 0.0");
        assert!(bound.contains(&version!("0.0.0")?));

        let err = VersionRange::parse("<0").unwrap_err();
        assert_eq!(err.to_string(), "Invalid version range: <0");

        let bound = VersionRange::parse("<1.1.0")?;
        assert_eq!(bound.to_string(), "0.0.0 - 1.0");
        assert!(bound.contains(&version!("0.0.0")?));

        let bound = VersionRange::parse("<1.1")?;
        assert_eq!(bound.to_string(), "0.0.0 - 1.0");
        assert!(bound.contains(&version!("0.0.0")?));

        Ok(())
    }

    #[test]
    fn test_hyphen() -> Result<()> {
        let bound = VersionRange::parse("0.1.0 - 0.2.0")?;
        assert_eq!(bound.to_string(), "0.1.0 - 0.2.0");

        let bound = VersionRange::parse("0.1.0 - 0.2")?;
        assert_eq!(bound.to_string(), "0.1.0 - 0.2");

        let bound = VersionRange::parse("0.1.0 - 0")?;
        assert_eq!(bound.to_string(), "0.1.0 - 0");

        let bound = VersionRange::parse("0.1.0 - 1")?;
        assert_eq!(bound.to_string(), "0.1.0 - 1");

        let bound = VersionRange::parse("0.1.0 - 1.2.3")?;
        assert_eq!(bound.to_string(), "0.1.0 - 1.2.3");

        let bound = VersionRange::parse("0.1.0 - 1.2")?;
        assert_eq!(bound.to_string(), "0.1.0 - 1.2");
        Ok(())
    }
}
