pub mod bound;
pub mod range;
pub mod version;

pub use bound::VersionBound;
pub use range::VersionRange;
pub use version::Version;

use anyhow::Result;

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
            .map(|s| VersionRange::parse(s))
            .collect::<Result<Vec<_>>>()?;
        Ok(VersionSpec::new(ranges))
    }

    pub fn contains(&self, version: &Version) -> bool {
        self.ranges.iter().any(|r| r.contains(version))
    }
}
