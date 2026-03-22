#![forbid(unsafe_code)]

use wr_core::{CrateBoundary, CrateEntryPoint};

pub const fn init_entrypoint() -> CrateEntryPoint {
    CrateEntryPoint::new("wr_world_seed", CrateBoundary::Subsystem, false)
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedError {
    reason: String,
}

impl SeedError {
    fn invalid_hex(value: &str) -> Self {
        Self { reason: format!("seed `{value}` must be a 0x-prefixed hexadecimal value") }
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

#[cfg(test)]
mod tests {
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
}
