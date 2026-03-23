use criterion::{Criterion, criterion_group, criterion_main};
use wr_world_gen::{
    EcologicalPlacementConfig, EcologicalPlacementSet, TerrainFieldConfig, TerrainScalarFieldSet,
};
use wr_world_seed::RootSeed;

fn bench_generate_ecological_placements(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecological_placements");
    let seed = RootSeed::parse_hex("0xDEADBEEF").expect("seed should parse");
    let field_config = TerrainFieldConfig::default();
    let fields =
        TerrainScalarFieldSet::generate(seed, field_config).expect("terrain field generation");
    let placement_config = EcologicalPlacementConfig::default();

    group.bench_function("solve_hero_biome_layout", |b| {
        b.iter(|| {
            EcologicalPlacementSet::generate(seed, &fields, placement_config)
                .expect("placement solve should succeed");
        });
    });

    group.finish();
}

criterion_group!(benches, bench_generate_ecological_placements);
criterion_main!(benches);
