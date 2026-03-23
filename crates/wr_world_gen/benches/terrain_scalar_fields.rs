use criterion::{Criterion, criterion_group, criterion_main};
use wr_world_gen::{TerrainFieldConfig, TerrainScalarFieldSet};
use wr_world_seed::RootSeed;

fn bench_generate_scalar_fields(c: &mut Criterion) {
    let mut group = c.benchmark_group("terrain_scalar_fields");
    let seed = RootSeed::parse_hex("0xDEADBEEF").expect("seed should parse");
    let config = TerrainFieldConfig::default();

    group.bench_function("generate_hero_biome_cache", |b| {
        b.iter(|| {
            TerrainScalarFieldSet::generate(seed, config)
                .expect("terrain field generation should succeed");
        });
    });

    group.finish();
}

criterion_group!(benches, bench_generate_scalar_fields);
criterion_main!(benches);
