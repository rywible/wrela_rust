use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const SEED_CONFIG_PACK_SCHEMA_VERSION: &str = "wr_seed_config_pack/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeedConfigPack {
    pub schema_version: String,
    pub pack_name: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub seed_overrides: BTreeMap<String, String>,
}

impl SeedConfigPack {
    pub fn new(
        pack_name: impl Into<String>,
        seed_overrides: BTreeMap<String, String>,
    ) -> Result<Self, SeedConfigPackError> {
        let pack = Self {
            schema_version: SEED_CONFIG_PACK_SCHEMA_VERSION.to_owned(),
            pack_name: pack_name.into(),
            seed_overrides,
        };
        pack.validate()?;
        Ok(pack)
    }

    pub fn named(pack_name: impl Into<String>) -> Result<Self, SeedConfigPackError> {
        Self::new(pack_name, BTreeMap::new())
    }

    pub fn validate(&self) -> Result<(), SeedConfigPackError> {
        if self.schema_version != SEED_CONFIG_PACK_SCHEMA_VERSION {
            return Err(SeedConfigPackError::InvalidSchemaVersion {
                found: self.schema_version.clone(),
            });
        }

        if self.pack_name.trim().is_empty() {
            return Err(SeedConfigPackError::InvalidPackName);
        }

        if let Some((path, _)) = self
            .seed_overrides
            .iter()
            .find(|(path, value)| path.trim().is_empty() || value.trim().is_empty())
        {
            return Err(SeedConfigPackError::InvalidOverridePath { path: path.clone() });
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeedConfigPackError {
    Io(String),
    Serialize(String),
    Deserialize(String),
    InvalidSchemaVersion { found: String },
    InvalidPackName,
    InvalidOverridePath { path: String },
}

impl std::fmt::Display for SeedConfigPackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "i/o error: {error}"),
            Self::Serialize(error) => write!(f, "seed config pack serialization failed: {error}"),
            Self::Deserialize(error) => {
                write!(f, "seed config pack deserialization failed: {error}")
            }
            Self::InvalidSchemaVersion { found } => {
                write!(
                    f,
                    "expected seed config pack schema version `{SEED_CONFIG_PACK_SCHEMA_VERSION}`, found `{found}`"
                )
            }
            Self::InvalidPackName => write!(f, "seed config pack name must not be blank"),
            Self::InvalidOverridePath { path } => {
                write!(f, "seed config pack override path `{path}` must not be blank")
            }
        }
    }
}

impl std::error::Error for SeedConfigPackError {}

pub fn parse_seed_config_pack_ron(source: &str) -> Result<SeedConfigPack, SeedConfigPackError> {
    let pack: SeedConfigPack = ron::de::from_str(source)
        .map_err(|error| SeedConfigPackError::Deserialize(error.to_string()))?;
    pack.validate()?;
    Ok(pack)
}

pub fn serialize_seed_config_pack_ron(
    pack: &SeedConfigPack,
) -> Result<String, SeedConfigPackError> {
    pack.validate()?;
    ron::ser::to_string_pretty(pack, ron::ser::PrettyConfig::default())
        .map_err(|error| SeedConfigPackError::Serialize(error.to_string()))
}

pub fn load_seed_config_pack_ron(
    path: impl AsRef<Path>,
) -> Result<SeedConfigPack, SeedConfigPackError> {
    let source = std::fs::read_to_string(path.as_ref())
        .map_err(|error| SeedConfigPackError::Io(error.to_string()))?;
    parse_seed_config_pack_ron(&source)
}

pub fn write_seed_config_pack_ron(
    path: impl AsRef<Path>,
    pack: &SeedConfigPack,
) -> Result<PathBuf, SeedConfigPackError> {
    let path = path.as_ref();
    let parent = path.parent().map(ToOwned::to_owned).unwrap_or_else(|| PathBuf::from("."));
    std::fs::create_dir_all(&parent).map_err(|error| SeedConfigPackError::Io(error.to_string()))?;
    let mut serialized = serialize_seed_config_pack_ron(pack)?;
    serialized.push('\n');
    std::fs::write(path, serialized).map_err(|error| SeedConfigPackError::Io(error.to_string()))?;
    Ok(path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use insta::assert_snapshot;

    use super::*;

    #[test]
    fn seed_config_pack_roundtrips_through_ron() {
        let pack = SeedConfigPack::new(
            "hero_forest",
            BTreeMap::from([
                ("combat.scenarios".to_owned(), "0xF00DFACE".to_owned()),
                ("vfx".to_owned(), "0xBADC0FFE".to_owned()),
            ]),
        )
        .expect("pack should validate");

        let serialized = serialize_seed_config_pack_ron(&pack).expect("pack should serialize");
        let reparsed = parse_seed_config_pack_ron(&serialized).expect("pack should parse");

        assert_eq!(reparsed, pack);
    }

    #[test]
    fn seed_config_pack_snapshot_is_diff_friendly() {
        let pack = SeedConfigPack::new(
            "combat_variant",
            BTreeMap::from([
                ("combat".to_owned(), "0xC0FFEE01".to_owned()),
                ("combat.scenarios".to_owned(), "0xF00DFACE".to_owned()),
            ]),
        )
        .expect("pack should validate");

        let serialized = serialize_seed_config_pack_ron(&pack).expect("pack should serialize");

        assert_snapshot!(
            serialized,
            @r#"
(
    schema_version: "wr_seed_config_pack/v1",
    pack_name: "combat_variant",
    seed_overrides: {
        "combat": "0xC0FFEE01",
        "combat.scenarios": "0xF00DFACE",
    },
)
"#
        );
    }
}
