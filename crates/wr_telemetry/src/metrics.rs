use std::time::Duration;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use wr_core::TelemetryConfig;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
pub struct FrameMemorySnapshot {
    pub estimated_total_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FrameTelemetrySample {
    pub frame_index: u32,
    pub frame_time_ms: f32,
    pub sim_time_ms: f32,
    pub render_time_ms: f32,
    pub draw_count: u32,
    pub entity_count: u32,
    pub memory: FrameMemorySnapshot,
}

impl FrameTelemetrySample {
    pub fn from_durations(
        frame_index: u32,
        frame_time: Duration,
        sim_time: Duration,
        render_time: Duration,
        draw_count: u32,
        entity_count: u32,
        memory: FrameMemorySnapshot,
    ) -> Self {
        Self {
            frame_index,
            frame_time_ms: duration_ms(frame_time),
            sim_time_ms: duration_ms(sim_time),
            render_time_ms: duration_ms(render_time),
            draw_count,
            entity_count,
            memory,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, Default)]
pub struct TimingSummary {
    pub min_ms: f32,
    pub max_ms: f32,
    pub average_ms: f32,
    pub p95_ms: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, Default)]
pub struct CountSummary {
    pub min: u32,
    pub max: u32,
    pub average: f32,
    pub last: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ScenarioTelemetrySummary {
    pub tracing_enabled: bool,
    pub metrics_enabled: bool,
    pub profiler_backend: String,
    pub frame_count: u32,
    pub frame_time_ms: TimingSummary,
    pub sim_time_ms: TimingSummary,
    pub render_time_ms: TimingSummary,
    pub draw_count: CountSummary,
    pub entity_count: CountSummary,
    pub memory_bytes: CountSummary,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub frame_samples: Vec<FrameTelemetrySample>,
}

pub struct ScenarioTelemetryRecorder {
    config: TelemetryConfig,
    frame_samples: Vec<FrameTelemetrySample>,
}

impl ScenarioTelemetryRecorder {
    pub fn new(config: TelemetryConfig) -> Self {
        Self { config, frame_samples: Vec::new() }
    }

    pub fn record_frame(&mut self, sample: FrameTelemetrySample) {
        if self.config.enable_metrics {
            self.frame_samples.push(sample);
        }
    }

    pub fn finish(self) -> ScenarioTelemetrySummary {
        ScenarioTelemetrySummary {
            tracing_enabled: self.config.enable_tracing,
            metrics_enabled: self.config.enable_metrics,
            profiler_backend: format!("{:?}", self.config.profiler_backend).to_ascii_lowercase(),
            frame_count: self.frame_samples.len() as u32,
            frame_time_ms: summarize_f32(
                self.frame_samples.iter().map(|sample| sample.frame_time_ms),
            ),
            sim_time_ms: summarize_f32(self.frame_samples.iter().map(|sample| sample.sim_time_ms)),
            render_time_ms: summarize_f32(
                self.frame_samples.iter().map(|sample| sample.render_time_ms),
            ),
            draw_count: summarize_u32(self.frame_samples.iter().map(|sample| sample.draw_count)),
            entity_count: summarize_u32(
                self.frame_samples.iter().map(|sample| sample.entity_count),
            ),
            memory_bytes: summarize_u32(self.frame_samples.iter().map(|sample| {
                u32::try_from(sample.memory.estimated_total_bytes).unwrap_or(u32::MAX)
            })),
            frame_samples: self.frame_samples,
        }
    }
}

fn duration_ms(duration: Duration) -> f32 {
    duration.as_secs_f32() * 1000.0
}

fn summarize_f32(values: impl IntoIterator<Item = f32>) -> TimingSummary {
    let mut values = values.into_iter().collect::<Vec<_>>();
    if values.is_empty() {
        return TimingSummary::default();
    }

    values.sort_by(|left, right| left.total_cmp(right));
    let sum = values.iter().copied().sum::<f32>();
    let p95_index = percentile_index(values.len(), 95);

    TimingSummary {
        min_ms: values[0],
        max_ms: values[values.len() - 1],
        average_ms: sum / values.len() as f32,
        p95_ms: values[p95_index],
    }
}

fn summarize_u32(values: impl IntoIterator<Item = u32>) -> CountSummary {
    let values = values.into_iter().collect::<Vec<_>>();
    if values.is_empty() {
        return CountSummary::default();
    }

    let min = values.iter().copied().min().unwrap_or_default();
    let max = values.iter().copied().max().unwrap_or_default();
    let last = *values.last().unwrap_or(&0);
    let sum = values.iter().copied().map(f64::from).sum::<f64>();

    CountSummary { min, max, average: (sum / values.len() as f64) as f32, last }
}

fn percentile_index(len: usize, percentile: usize) -> usize {
    if len <= 1 {
        return 0;
    }

    ((len - 1) * percentile) / 100
}

#[cfg(test)]
mod tests {
    use super::*;
    use wr_core::{ProfilerBackend, TelemetryConfig};

    #[test]
    fn recorder_summarizes_frame_samples() {
        let mut recorder = ScenarioTelemetryRecorder::new(TelemetryConfig {
            enable_metrics: true,
            enable_tracing: true,
            profiler_backend: ProfilerBackend::Disabled,
        });

        recorder.record_frame(FrameTelemetrySample {
            frame_index: 0,
            frame_time_ms: 1.0,
            sim_time_ms: 0.6,
            render_time_ms: 0.0,
            draw_count: 0,
            entity_count: 2,
            memory: FrameMemorySnapshot { estimated_total_bytes: 256 },
        });
        recorder.record_frame(FrameTelemetrySample {
            frame_index: 1,
            frame_time_ms: 2.0,
            sim_time_ms: 1.4,
            render_time_ms: 0.0,
            draw_count: 0,
            entity_count: 2,
            memory: FrameMemorySnapshot { estimated_total_bytes: 384 },
        });

        let summary = recorder.finish();

        assert_eq!(summary.frame_count, 2);
        assert_eq!(summary.frame_time_ms.min_ms, 1.0);
        assert_eq!(summary.frame_time_ms.max_ms, 2.0);
        assert_eq!(summary.entity_count.last, 2);
        assert_eq!(summary.memory_bytes.max, 384);
    }
}
