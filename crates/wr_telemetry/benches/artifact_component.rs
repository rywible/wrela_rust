use criterion::{Criterion, criterion_group, criterion_main};
use wr_telemetry::artifact_component;

fn bench_artifact_component(c: &mut Criterion) {
    let mut group = c.benchmark_group("artifact_component");
    let long_label = "cargo xtask verify -> nextest::workspace / junit.xml";

    group.bench_function("normalize_verify_label", |b| {
        b.iter(|| artifact_component(long_label));
    });

    group.finish();
}

criterion_group!(benches, bench_artifact_component);
criterion_main!(benches);
