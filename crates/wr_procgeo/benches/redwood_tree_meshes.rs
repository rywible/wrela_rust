use criterion::{Criterion, criterion_group, criterion_main};
use wr_procgeo::{RedwoodForestMeshSet, RedwoodMeshBuildConfig};
use wr_world_gen::{
    EcologicalPlacementConfig, EcologicalPlacementSet, RedwoodForestGraphConfig,
    RedwoodForestGraphSet, TerrainFieldConfig, TerrainScalarFieldSet,
};
use wr_world_seed::RootSeed;

fn bench_redwood_tree_meshes(criterion: &mut Criterion) {
    let seed = RootSeed::parse_hex("0xDEADBEEF").expect("seed should parse");
    let fields =
        TerrainScalarFieldSet::generate(seed, TerrainFieldConfig::default()).expect("fields");
    let placements =
        EcologicalPlacementSet::generate(seed, &fields, EcologicalPlacementConfig::default())
            .expect("placements");
    let graphs = RedwoodForestGraphSet::generate(
        seed,
        &fields,
        &placements,
        RedwoodForestGraphConfig::default(),
    )
    .expect("graphs");

    criterion.bench_function("redwood_tree_meshes", |bencher| {
        bencher.iter(|| {
            RedwoodForestMeshSet::build(&graphs, RedwoodMeshBuildConfig::default())
                .expect("mesh set should build")
        });
    });
}

criterion_group!(benches, bench_redwood_tree_meshes);
criterion_main!(benches);
