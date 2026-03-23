#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use wr_core::{CrateBoundary, CrateEntryPoint};
use wr_core::{SeedConfigPack, SeedConfigPackError};

pub const fn init_entrypoint() -> CrateEntryPoint {
    CrateEntryPoint::new("wr_world_seed", CrateBoundary::Subsystem, false)
}

pub const STANDARD_SEED_GRAPH_VERSION: &str = "wr_seed_graph/v1";

const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x00000100000001b3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RootSeed(u64);

impl RootSeed {
    pub fn parse_hex(value: &str) -> Result<Self, SeedError> {
        let trimmed = value.trim();
        let trimmed = trimmed
            .strip_prefix("0x")
            .or_else(|| trimmed.strip_prefix("0X"))
            .ok_or_else(|| SeedError::invalid_hex(value))?;

        if trimmed.is_empty() || trimmed.len() > 16 {
            return Err(SeedError::invalid_hex(value));
        }

        let parsed = u64::from_str_radix(trimmed, 16).map_err(|_| SeedError::invalid_hex(value))?;
        Ok(Self(parsed))
    }

    pub fn value(self) -> u64 {
        self.0
    }

    pub fn to_hex(self) -> String {
        format!("0x{:016X}", self.0)
    }

    pub fn derive_stream_hex(self, label: &str) -> String {
        stable_hash_hex_bytes(self.record_bytes(label))
    }

    pub fn derive_stream_u64(self, label: &str) -> u64 {
        stable_hash_u64_bytes(self.record_bytes(label))
    }

    fn record_bytes(self, label: &str) -> Vec<u8> {
        let mut bytes = self.0.to_be_bytes().to_vec();
        bytes.extend_from_slice(label.as_bytes());
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeedRng {
    state: u64,
}

impl SeedRng {
    pub fn new(seed: RootSeed) -> Self {
        Self { state: seed.value() }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeedDerivationMode {
    Derived,
    Override,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeedDerivation {
    pub path: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_path: Option<String>,
    pub value_hex: String,
    pub mode: SeedDerivationMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeedConfigOverride {
    pub path: String,
    pub value_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeedGraph {
    pub topology_version: String,
    pub root_hex: String,
    pub config_pack_name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overrides: Vec<SeedConfigOverride>,
    pub derivations: Vec<SeedDerivation>,
}

impl SeedGraph {
    pub fn standard(root: RootSeed, pack: Option<&SeedConfigPack>) -> Result<Self, SeedError> {
        let config_pack = match pack {
            Some(pack) => {
                pack.validate().map_err(SeedError::config_pack)?;
                pack.clone()
            }
            None => SeedConfigPack::named("default").map_err(SeedError::config_pack)?,
        };

        let override_seeds = parse_override_seeds(&config_pack)?;
        validate_override_paths(override_seeds.keys().map(String::as_str))?;

        let mut seeds = BTreeMap::new();
        let mut derivations = Vec::with_capacity(STANDARD_SEED_PATHS.len());

        for spec in STANDARD_SEED_PATHS {
            let parent_seed =
                spec.parent_path.and_then(|parent| seeds.get(parent).copied()).unwrap_or(root);
            let (seed, mode) = match override_seeds.get(spec.path) {
                Some(seed) => (*seed, SeedDerivationMode::Override),
                None => (
                    RootSeed(parent_seed.derive_stream_u64(spec.label)),
                    SeedDerivationMode::Derived,
                ),
            };

            seeds.insert(spec.path, seed);
            derivations.push(SeedDerivation {
                path: spec.path.to_owned(),
                label: spec.label.to_owned(),
                parent_path: spec.parent_path.map(str::to_owned),
                value_hex: seed.to_hex(),
                mode,
            });
        }

        let overrides = config_pack
            .seed_overrides
            .iter()
            .map(|(path, value_hex)| {
                Ok(SeedConfigOverride {
                    path: path.clone(),
                    value_hex: normalize_seed_hex(value_hex)?,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            topology_version: STANDARD_SEED_GRAPH_VERSION.to_owned(),
            root_hex: root.to_hex(),
            config_pack_name: config_pack.pack_name,
            overrides,
            derivations,
        })
    }

    pub fn seed_for_path(&self, path: &str) -> Option<RootSeed> {
        self.derivations
            .iter()
            .find(|derivation| derivation.path == path)
            .and_then(|derivation| RootSeed::parse_hex(&derivation.value_hex).ok())
    }
}

struct SeedPathSpec {
    path: &'static str,
    label: &'static str,
    parent_path: Option<&'static str>,
}

const STANDARD_SEED_PATHS: [SeedPathSpec; 7] = [
    SeedPathSpec { path: "terrain", label: "terrain", parent_path: None },
    SeedPathSpec { path: "ecology", label: "ecology", parent_path: None },
    SeedPathSpec { path: "trees", label: "trees", parent_path: None },
    SeedPathSpec { path: "wraiths", label: "wraiths", parent_path: None },
    SeedPathSpec { path: "combat", label: "combat", parent_path: None },
    SeedPathSpec { path: "combat.scenarios", label: "scenarios", parent_path: Some("combat") },
    SeedPathSpec { path: "vfx", label: "vfx", parent_path: None },
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedError {
    reason: String,
}

impl SeedError {
    fn invalid_hex(value: &str) -> Self {
        Self { reason: format!("seed `{value}` must be a 0x-prefixed hexadecimal value") }
    }

    fn unknown_path(path: &str) -> Self {
        Self {
            reason: format!(
                "seed config override path `{path}` is not part of the standard seed graph"
            ),
        }
    }

    fn config_pack(error: SeedConfigPackError) -> Self {
        Self { reason: error.to_string() }
    }
}

impl std::fmt::Display for SeedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.reason)
    }
}

impl std::error::Error for SeedError {}

pub fn stable_hash_hex(parts: impl IntoIterator<Item = impl AsRef<[u8]>>) -> String {
    let mut hash = FNV_OFFSET_BASIS;
    for part in parts {
        hash = update_hash(hash, part.as_ref());
    }

    format!("0x{hash:016X}")
}

pub fn stable_hash_hex_bytes(bytes: impl AsRef<[u8]>) -> String {
    stable_hash_hex([bytes.as_ref()])
}

pub fn stable_hash_u64_bytes(bytes: impl AsRef<[u8]>) -> u64 {
    update_hash(FNV_OFFSET_BASIS, bytes.as_ref())
}

fn update_hash(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn parse_override_seeds(pack: &SeedConfigPack) -> Result<BTreeMap<String, RootSeed>, SeedError> {
    pack.seed_overrides
        .iter()
        .map(|(path, value_hex)| Ok((path.clone(), RootSeed::parse_hex(value_hex)?)))
        .collect()
}

fn validate_override_paths<'a>(paths: impl IntoIterator<Item = &'a str>) -> Result<(), SeedError> {
    let known_paths = STANDARD_SEED_PATHS.iter().map(|spec| spec.path).collect::<BTreeSet<_>>();
    for path in paths {
        if !known_paths.contains(path) {
            return Err(SeedError::unknown_path(path));
        }
    }
    Ok(())
}

fn normalize_seed_hex(value: &str) -> Result<String, SeedError> {
    RootSeed::parse_hex(value).map(RootSeed::to_hex)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use insta::assert_json_snapshot;
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn parses_hex_root_seed_values() {
        let seed = RootSeed::parse_hex("0xDEADBEEF").expect("seed should parse");

        assert_eq!(seed.value(), 0xDEADBEEF);
    }

    #[test]
    fn derived_streams_are_stable() {
        let seed = RootSeed::parse_hex("0xDEADBEEF").expect("seed should parse");

        assert_eq!(seed.derive_stream_hex("player"), seed.derive_stream_hex("player"));
        assert_ne!(seed.derive_stream_hex("player"), seed.derive_stream_hex("enemy"));
    }

    #[test]
    fn stable_hash_hex_uses_uppercase_zero_prefixed_output() {
        assert_eq!(stable_hash_hex(["abc"]), "0xE71FA2190541574B");
    }

    #[test]
    fn seed_rng_is_repeatable_for_the_same_seed() {
        let root = RootSeed::parse_hex("0xDEADBEEF").expect("seed should parse");
        let mut first = SeedRng::new(root);
        let mut second = SeedRng::new(root);

        let first_values = (0..4).map(|_| first.next_u64()).collect::<Vec<_>>();
        let second_values = (0..4).map(|_| second.next_u64()).collect::<Vec<_>>();

        assert_eq!(first_values, second_values);
    }

    #[test]
    fn standard_seed_graph_snapshot_is_stable() {
        let root = RootSeed::parse_hex("0xDEADBEEF").expect("seed should parse");
        let pack = SeedConfigPack::new(
            "duel_focus",
            BTreeMap::from([("combat.scenarios".to_owned(), "0xC0FFEE01".to_owned())]),
        )
        .expect("pack should validate");

        let graph = SeedGraph::standard(root, Some(&pack)).expect("graph should build");

        assert_json_snapshot!(graph, @r#"
        {
          "topology_version": "wr_seed_graph/v1",
          "root_hex": "0x00000000DEADBEEF",
          "config_pack_name": "duel_focus",
          "overrides": [
            {
              "path": "combat.scenarios",
              "value_hex": "0x00000000C0FFEE01"
            }
          ],
          "derivations": [
            {
              "path": "terrain",
              "label": "terrain",
              "value_hex": "0xCA379F03C759982C",
              "mode": "derived"
            },
            {
              "path": "ecology",
              "label": "ecology",
              "value_hex": "0x517C4643CB4ED25D",
              "mode": "derived"
            },
            {
              "path": "trees",
              "label": "trees",
              "value_hex": "0x630977A9CD5352E4",
              "mode": "derived"
            },
            {
              "path": "wraiths",
              "label": "wraiths",
              "value_hex": "0xBAB04A6CF613F1E5",
              "mode": "derived"
            },
            {
              "path": "combat",
              "label": "combat",
              "value_hex": "0xA1F72785A5203293",
              "mode": "derived"
            },
            {
              "path": "combat.scenarios",
              "label": "scenarios",
              "parent_path": "combat",
              "value_hex": "0x00000000C0FFEE01",
              "mode": "override"
            },
            {
              "path": "vfx",
              "label": "vfx",
              "value_hex": "0x3D55A2CCF0A62AA9",
              "mode": "derived"
            }
          ]
        }
        "#);
    }

    #[test]
    fn overriding_one_branch_changes_only_that_branch_and_its_children() {
        let root = RootSeed::parse_hex("0xDEADBEEF").expect("seed should parse");
        let baseline = SeedGraph::standard(root, None).expect("graph should build");
        let pack = SeedConfigPack::new(
            "terrain_override",
            BTreeMap::from([("terrain".to_owned(), "0xF00DFACE".to_owned())]),
        )
        .expect("pack should validate");
        let overridden = SeedGraph::standard(root, Some(&pack)).expect("graph should build");

        for path in ["ecology", "trees", "wraiths", "combat", "combat.scenarios", "vfx"] {
            assert_eq!(
                baseline.seed_for_path(path),
                overridden.seed_for_path(path),
                "override path `{path}` should stay unchanged when terrain is overridden"
            );
        }

        assert_ne!(baseline.seed_for_path("terrain"), overridden.seed_for_path("terrain"));
    }

    #[test]
    fn overriding_parent_branch_changes_child_branch_and_preserves_siblings() {
        let root = RootSeed::parse_hex("0xDEADBEEF").expect("seed should parse");
        let baseline = SeedGraph::standard(root, None).expect("graph should build");
        let pack = SeedConfigPack::new(
            "combat_override",
            BTreeMap::from([("combat".to_owned(), "0xF00DFACE".to_owned())]),
        )
        .expect("pack should validate");
        let overridden = SeedGraph::standard(root, Some(&pack)).expect("graph should build");

        for path in ["terrain", "ecology", "trees", "wraiths", "vfx"] {
            assert_eq!(
                baseline.seed_for_path(path),
                overridden.seed_for_path(path),
                "sibling branch `{path}` should remain stable when combat is overridden"
            );
        }

        assert_ne!(baseline.seed_for_path("combat"), overridden.seed_for_path("combat"));
        assert_ne!(
            baseline.seed_for_path("combat.scenarios"),
            overridden.seed_for_path("combat.scenarios")
        );
    }

    #[test]
    fn same_seed_and_config_pack_produce_identical_generated_stats() {
        let root = RootSeed::parse_hex("0xDEADBEEF").expect("seed should parse");
        let pack = SeedConfigPack::new(
            "duel_focus",
            BTreeMap::from([("combat.scenarios".to_owned(), "0xC0FFEE01".to_owned())]),
        )
        .expect("pack should validate");
        let first = sample_generation_stats(
            &SeedGraph::standard(root, Some(&pack)).expect("graph should build"),
        );
        let second = sample_generation_stats(
            &SeedGraph::standard(root, Some(&pack)).expect("graph should build"),
        );

        assert_eq!(first, second);
    }

    fn sample_generation_stats(graph: &SeedGraph) -> BTreeMap<String, u64> {
        graph
            .derivations
            .iter()
            .map(|derivation| {
                let mut rng = SeedRng::new(
                    graph.seed_for_path(&derivation.path).expect("seed path should exist"),
                );
                (derivation.path.clone(), rng.next_u64())
            })
            .collect()
    }

    proptest! {
        #[test]
        fn standard_seed_graph_has_unique_and_stable_paths(root in any::<u64>()) {
            let graph = SeedGraph::standard(RootSeed(root), None).expect("graph should build");
            let paths = graph.derivations.iter().map(|derivation| derivation.path.clone()).collect::<Vec<_>>();
            let unique_paths = paths.iter().cloned().collect::<BTreeSet<_>>();

            prop_assert_eq!(paths.len(), unique_paths.len());
            prop_assert_eq!(
                graph,
                SeedGraph::standard(RootSeed(root), None).expect("graph should rebuild deterministically")
            );
        }
    }
}
