use criterion::{Criterion, criterion_group, criterion_main};
use wr_world_gen::{
    EcologicalPlacementConfig, EcologicalPlacementSet, RedwoodForestGraphConfig,
    RedwoodForestGraphSet, TerrainFieldConfig, TerrainScalarFieldSet,
};
use wr_world_seed::RootSeed;

fn bench_generate_redwood_tree_graphs(c: &mut Criterion) {
    let mut group = c.benchmark_group("redwood_tree_graphs");
    let seed = RootSeed::parse_hex("0xDEADBEEF").expect("canonical seed should parse");
    let fields = TerrainScalarFieldSet::generate(
        seed,
        TerrainFieldConfig { cache_resolution: 65, ..TerrainFieldConfig::default() },
    )
    .expect("field set should generate");
    let placements =
        EcologicalPlacementSet::generate(seed, &fields, EcologicalPlacementConfig::default())
            .expect("placements should generate");
    let graph_config = RedwoodForestGraphConfig::default();

    group.bench_function("hero_forest", |bench| {
        bench.iter(|| {
            RedwoodForestGraphSet::generate(seed, &fields, &placements, graph_config)
                .expect("redwood graphs should generate");
        });
    });
    group.finish();
}

criterion_group!(benches, bench_generate_redwood_tree_graphs);
criterion_main!(benches);
