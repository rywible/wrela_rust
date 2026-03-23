use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const TWEAK_PACK_SCHEMA_VERSION: &str = "wr_tweak_pack/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TweakNamespace {
    World,
    Atmosphere,
    Lighting,
    Foliage,
    Player,
    Combat,
    Wraith,
    Vfx,
}

impl TweakNamespace {
    pub const ALL: [Self; 8] = [
        Self::World,
        Self::Atmosphere,
        Self::Lighting,
        Self::Foliage,
        Self::Player,
        Self::Combat,
        Self::Wraith,
        Self::Vfx,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::World => "world",
            Self::Atmosphere => "atmosphere",
            Self::Lighting => "lighting",
            Self::Foliage => "foliage",
            Self::Player => "player",
            Self::Combat => "combat",
            Self::Wraith => "wraith",
            Self::Vfx => "vfx",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::World => "World",
            Self::Atmosphere => "Atmosphere",
            Self::Lighting => "Lighting",
            Self::Foliage => "Foliage",
            Self::Player => "Player",
            Self::Combat => "Combat",
            Self::Wraith => "Wraith",
            Self::Vfx => "VFX",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TweakValueKind {
    Scalar,
    Toggle,
}

impl TweakValueKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Scalar => "scalar",
            Self::Toggle => "toggle",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum TweakValue {
    Scalar(f32),
    Toggle(bool),
}

impl TweakValue {
    pub const fn kind(self) -> TweakValueKind {
        match self {
            Self::Scalar(_) => TweakValueKind::Scalar,
            Self::Toggle(_) => TweakValueKind::Toggle,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TweakDefinition {
    pub key: &'static str,
    pub namespace: TweakNamespace,
    pub label: &'static str,
    pub description: &'static str,
    pub default_value: TweakValue,
}

impl TweakDefinition {
    pub const fn new(
        key: &'static str,
        namespace: TweakNamespace,
        label: &'static str,
        description: &'static str,
        default_value: TweakValue,
    ) -> Self {
        Self { key, namespace, label, description, default_value }
    }
}

const DEFAULT_TWEAK_DEFINITIONS: [TweakDefinition; 8] = [
    TweakDefinition::new(
        "world.wind_strength",
        TweakNamespace::World,
        "Wind Strength",
        "Controls broad environmental motion cues used by the biome runtime.",
        TweakValue::Scalar(0.35),
    ),
    TweakDefinition::new(
        "atmosphere.mie_strength",
        TweakNamespace::Atmosphere,
        "Mie Strength",
        "Scales the late-afternoon atmospheric forward scattering push.",
        TweakValue::Scalar(1.0),
    ),
    TweakDefinition::new(
        "lighting.sun_warmth",
        TweakNamespace::Lighting,
        "Sun Warmth",
        "Biases the key light grade toward the warm hero look.",
        TweakValue::Scalar(0.65),
    ),
    TweakDefinition::new(
        "foliage.sway_strength",
        TweakNamespace::Foliage,
        "Sway Strength",
        "Controls secondary sway intensity for needles, fronds, and understory cards.",
        TweakValue::Scalar(0.45),
    ),
    TweakDefinition::new(
        "player.camera_bob_enabled",
        TweakNamespace::Player,
        "Camera Bob",
        "Enables or disables camera bob for tests and readability checks.",
        TweakValue::Toggle(true),
    ),
    TweakDefinition::new(
        "combat.hitstop_scale",
        TweakNamespace::Combat,
        "Hitstop Scale",
        "Scales the readable pause on successful clashes and hit-confirms.",
        TweakValue::Scalar(1.0),
    ),
    TweakDefinition::new(
        "wraith.ribbon_emissive",
        TweakNamespace::Wraith,
        "Ribbon Emissive",
        "Controls how strongly the wraith silhouette ribbons glow against fog.",
        TweakValue::Scalar(0.55),
    ),
    TweakDefinition::new(
        "vfx.trail_opacity",
        TweakNamespace::Vfx,
        "Trail Opacity",
        "Controls the visible weight of telekinetic weapon trails and wisps.",
        TweakValue::Scalar(0.75),
    ),
];

pub fn default_tweak_definitions() -> &'static [TweakDefinition] {
    &DEFAULT_TWEAK_DEFINITIONS
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TweakPack {
    pub schema_version: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub entries: BTreeMap<String, TweakValue>,
}

impl TweakPack {
    pub fn new(entries: BTreeMap<String, TweakValue>) -> Self {
        Self { schema_version: TWEAK_PACK_SCHEMA_VERSION.to_owned(), entries }
    }

    pub fn validate(&self) -> Result<(), TweakError> {
        if self.schema_version != TWEAK_PACK_SCHEMA_VERSION {
            return Err(TweakError::InvalidSchemaVersion { found: self.schema_version.clone() });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TweakRegistry {
    values: BTreeMap<String, TweakValue>,
    dirty_namespaces: BTreeSet<TweakNamespace>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TweakRegistryEntry {
    pub key: &'static str,
    pub namespace: TweakNamespace,
    pub label: &'static str,
    pub description: &'static str,
    pub default_value: TweakValue,
    pub value: TweakValue,
    pub namespace_dirty: bool,
}

impl Default for TweakRegistry {
    fn default() -> Self {
        let values = default_tweak_definitions()
            .iter()
            .map(|definition| (definition.key.to_owned(), definition.default_value))
            .collect();
        Self { values, dirty_namespaces: BTreeSet::new() }
    }
}

impl TweakRegistry {
    pub fn definitions(&self) -> &'static [TweakDefinition] {
        default_tweak_definitions()
    }

    pub fn definition(&self, key: &str) -> Option<&'static TweakDefinition> {
        definition_for_key(key)
    }

    pub fn entries(&self) -> Vec<TweakRegistryEntry> {
        self.definitions().iter().map(|definition| self.entry_from_definition(definition)).collect()
    }

    pub fn entries_in_namespace(&self, namespace: TweakNamespace) -> Vec<TweakRegistryEntry> {
        self.definitions()
            .iter()
            .filter(|definition| definition.namespace == namespace)
            .map(|definition| self.entry_from_definition(definition))
            .collect()
    }

    pub fn entry(&self, key: &str) -> Option<TweakRegistryEntry> {
        self.definition(key).map(|definition| self.entry_from_definition(definition))
    }

    pub fn value(&self, key: &str) -> Option<TweakValue> {
        self.values.get(key).copied()
    }

    pub fn set_value(&mut self, key: &str, value: TweakValue) -> Result<bool, TweakError> {
        let definition = definition_for_key(key)
            .ok_or_else(|| TweakError::UnknownKey { key: key.to_owned() })?;
        if definition.default_value.kind() != value.kind() {
            return Err(TweakError::TypeMismatch {
                key: key.to_owned(),
                expected: definition.default_value.kind(),
                found: value.kind(),
            });
        }

        match self.values.get(key) {
            Some(current) if *current == value => Ok(false),
            _ => {
                self.values.insert(key.to_owned(), value);
                self.dirty_namespaces.insert(definition.namespace);
                Ok(true)
            }
        }
    }

    pub fn apply_pack(&mut self, pack: &TweakPack) -> Result<(), TweakError> {
        pack.validate()?;
        for (key, value) in &pack.entries {
            self.set_value(key, *value)?;
        }
        Ok(())
    }

    pub fn snapshot_pack(&self) -> TweakPack {
        let entries = self
            .definitions()
            .iter()
            .filter_map(|definition| {
                let value = self.value(definition.key).unwrap_or(definition.default_value);
                (value != definition.default_value).then(|| (definition.key.to_owned(), value))
            })
            .collect();

        TweakPack::new(entries)
    }

    pub fn load_pack_from_path(&mut self, path: impl AsRef<Path>) -> Result<PathBuf, TweakError> {
        let path = path.as_ref();
        let pack = load_tweak_pack_ron(path)?;
        self.apply_pack(&pack)?;
        Ok(path.to_path_buf())
    }

    pub fn save_pack_to_path(&self, path: impl AsRef<Path>) -> Result<PathBuf, TweakError> {
        write_tweak_pack_ron(path, &self.snapshot_pack())
    }

    pub fn dirty_namespaces(&self) -> &BTreeSet<TweakNamespace> {
        &self.dirty_namespaces
    }

    pub fn clear_dirty_namespaces(&mut self) {
        self.dirty_namespaces.clear();
    }

    pub fn is_namespace_dirty(&self, namespace: TweakNamespace) -> bool {
        self.dirty_namespaces.contains(&namespace)
    }

    pub fn dirty_namespace_count(&self) -> usize {
        self.dirty_namespaces.len()
    }

    fn entry_from_definition(&self, definition: &'static TweakDefinition) -> TweakRegistryEntry {
        TweakRegistryEntry {
            key: definition.key,
            namespace: definition.namespace,
            label: definition.label,
            description: definition.description,
            default_value: definition.default_value,
            value: self.value(definition.key).unwrap_or(definition.default_value),
            namespace_dirty: self.is_namespace_dirty(definition.namespace),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TweakError {
    Io(String),
    Serialize(String),
    Deserialize(String),
    InvalidSchemaVersion { found: String },
    UnknownKey { key: String },
    TypeMismatch { key: String, expected: TweakValueKind, found: TweakValueKind },
}

impl std::fmt::Display for TweakError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "i/o error: {error}"),
            Self::Serialize(error) => write!(f, "tweak pack serialization failed: {error}"),
            Self::Deserialize(error) => write!(f, "tweak pack deserialization failed: {error}"),
            Self::InvalidSchemaVersion { found } => {
                write!(
                    f,
                    "expected tweak pack schema version `{TWEAK_PACK_SCHEMA_VERSION}`, found `{found}`"
                )
            }
            Self::UnknownKey { key } => write!(f, "unknown tweak key `{key}`"),
            Self::TypeMismatch { key, expected, found } => {
                write!(f, "tweak `{key}` expected {} but found {}", expected.label(), found.label())
            }
        }
    }
}

impl std::error::Error for TweakError {}

pub fn parse_tweak_pack_ron(source: &str) -> Result<TweakPack, TweakError> {
    let pack: TweakPack =
        ron::de::from_str(source).map_err(|error| TweakError::Deserialize(error.to_string()))?;
    pack.validate()?;
    Ok(pack)
}

pub fn serialize_tweak_pack_ron(pack: &TweakPack) -> Result<String, TweakError> {
    pack.validate()?;
    ron::ser::to_string_pretty(pack, ron::ser::PrettyConfig::default())
        .map_err(|error| TweakError::Serialize(error.to_string()))
}

pub fn load_tweak_pack_ron(path: impl AsRef<Path>) -> Result<TweakPack, TweakError> {
    let source = std::fs::read_to_string(path.as_ref())
        .map_err(|error| TweakError::Io(error.to_string()))?;
    parse_tweak_pack_ron(&source)
}

pub fn write_tweak_pack_ron(
    path: impl AsRef<Path>,
    pack: &TweakPack,
) -> Result<PathBuf, TweakError> {
    let path = path.as_ref();
    let parent = path.parent().map(ToOwned::to_owned).unwrap_or_else(|| PathBuf::from("."));
    std::fs::create_dir_all(&parent).map_err(|error| TweakError::Io(error.to_string()))?;
    let mut serialized = serialize_tweak_pack_ron(pack)?;
    serialized.push('\n');
    std::fs::write(path, serialized).map_err(|error| TweakError::Io(error.to_string()))?;
    Ok(path.to_path_buf())
}

fn definition_for_key(key: &str) -> Option<&'static TweakDefinition> {
    default_tweak_definitions().iter().find(|definition| definition.key == key)
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use insta::assert_snapshot;

    use super::*;

    #[test]
    fn tweak_pack_roundtrips_through_ron() {
        let mut registry = TweakRegistry::default();
        registry
            .set_value("atmosphere.mie_strength", TweakValue::Scalar(1.2))
            .expect("known scalar tweak should update");
        registry
            .set_value("player.camera_bob_enabled", TweakValue::Toggle(false))
            .expect("known toggle tweak should update");

        let pack = registry.snapshot_pack();
        let serialized = serialize_tweak_pack_ron(&pack).expect("pack should serialize");
        let reparsed = parse_tweak_pack_ron(&serialized).expect("pack should parse");

        assert_eq!(reparsed, pack);
    }

    #[test]
    fn snapshot_pack_omits_default_values() {
        let registry = TweakRegistry::default();

        assert_eq!(registry.snapshot_pack(), TweakPack::new(BTreeMap::new()));
    }

    #[test]
    fn registry_entries_expose_live_values_and_dirty_flags() {
        let mut registry = TweakRegistry::default();
        registry
            .set_value("combat.hitstop_scale", TweakValue::Scalar(1.25))
            .expect("known scalar tweak should update");

        let combat_entry =
            registry.entry("combat.hitstop_scale").expect("combat tweak should be discoverable");

        assert_eq!(combat_entry.label, "Hitstop Scale");
        assert_eq!(combat_entry.value, TweakValue::Scalar(1.25));
        assert_eq!(combat_entry.default_value, TweakValue::Scalar(1.0));
        assert!(combat_entry.namespace_dirty);
    }

    #[test]
    fn registry_covers_all_namespaces_and_documents_every_entry() {
        let registry = TweakRegistry::default();
        let entries = registry.entries();
        let namespaces = entries.iter().map(|entry| entry.namespace).collect::<BTreeSet<_>>();

        assert_eq!(namespaces, TweakNamespace::ALL.into_iter().collect());
        assert!(entries
            .iter()
            .all(|entry| !entry.label.trim().is_empty() && !entry.description.trim().is_empty()));
    }

    #[test]
    fn applying_pack_marks_only_the_touched_namespaces_dirty() {
        let mut registry = TweakRegistry::default();
        let pack = TweakPack::new(BTreeMap::from([
            ("world.wind_strength".to_owned(), TweakValue::Scalar(0.5)),
            ("vfx.trail_opacity".to_owned(), TweakValue::Scalar(0.9)),
        ]));

        registry.apply_pack(&pack).expect("pack should apply");

        assert_eq!(
            registry.dirty_namespaces(),
            &BTreeSet::from([TweakNamespace::World, TweakNamespace::Vfx])
        );
    }

    #[test]
    fn tweak_pack_snapshot_format_is_diff_friendly() {
        let pack = TweakPack::new(BTreeMap::from([
            ("combat.hitstop_scale".to_owned(), TweakValue::Scalar(1.1)),
            ("player.camera_bob_enabled".to_owned(), TweakValue::Toggle(false)),
        ]));

        let serialized = serialize_tweak_pack_ron(&pack).expect("pack should serialize");

        assert_snapshot!(
            serialized,
            @r#"
(
    schema_version: "wr_tweak_pack/v1",
    entries: {
        "combat.hitstop_scale": (
            type: scalar,
            value: 1.1,
        ),
        "player.camera_bob_enabled": (
            type: toggle,
            value: false,
        ),
    },
)
"#
        );
    }
}
