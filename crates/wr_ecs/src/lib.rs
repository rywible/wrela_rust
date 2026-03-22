#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use wr_core::{CrateBoundary, CrateEntryPoint};
use wr_tools_harness::{ScenarioActorSpawn, ScriptedInput};
use wr_world_seed::RootSeed;

pub const fn init_entrypoint() -> CrateEntryPoint {
    CrateEntryPoint::new("wr_ecs", CrateBoundary::Subsystem, false)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioActorState {
    pub actor_id: String,
    pub actor_kind: String,
    pub seed_stream: Option<String>,
    pub stream_signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadlessScenarioWorld {
    simulation_rate_hz: u32,
    frames_simulated: u32,
    applied_input_count: u32,
    actors: Vec<ScenarioActorState>,
    active_actions: BTreeSet<String>,
    event_log: Vec<String>,
}

impl HeadlessScenarioWorld {
    pub fn new(simulation_rate_hz: u32, seed: RootSeed, actors: &[ScenarioActorSpawn]) -> Self {
        let actors = actors
            .iter()
            .map(|actor| {
                let stream_label = actor.seed_stream.as_deref().unwrap_or(actor.actor_id.as_str());

                ScenarioActorState {
                    actor_id: actor.actor_id.clone(),
                    actor_kind: actor.actor_kind.clone(),
                    seed_stream: actor.seed_stream.clone(),
                    stream_signature: seed.derive_stream_hex(stream_label),
                }
            })
            .collect();

        Self {
            simulation_rate_hz,
            frames_simulated: 0,
            applied_input_count: 0,
            actors,
            active_actions: BTreeSet::new(),
            event_log: Vec::new(),
        }
    }

    pub fn apply_inputs<'a>(
        &mut self,
        frame: u32,
        inputs: impl IntoIterator<Item = &'a ScriptedInput>,
    ) {
        for input in inputs {
            match input.state.as_str() {
                "pressed" => {
                    self.active_actions.insert(input.action.clone());
                }
                "released" => {
                    self.active_actions.remove(&input.action);
                }
                other => {
                    self.event_log
                        .push(format!("frame={frame}:ignored_input:{}:{other}", input.action));
                }
            }

            self.applied_input_count += 1;
            self.event_log.push(format!("frame={frame}:input:{}:{}", input.action, input.state));
        }
    }

    pub fn step(&mut self, frame: u32) {
        self.frames_simulated += 1;
        self.event_log.push(format!(
            "frame={frame}:step:{}:{}hz:{}actions",
            self.frames_simulated,
            self.simulation_rate_hz,
            self.active_actions.len()
        ));
    }

    pub fn metric_value(&self, metric: &str) -> Option<f32> {
        match metric {
            "world.actor_count" => Some(self.actors.len() as f32),
            "world.frames_simulated" | "startup.frame_count" => Some(self.frames_simulated as f32),
            "world.active_action_count" => Some(self.active_actions.len() as f32),
            "world.applied_input_count" => Some(self.applied_input_count as f32),
            _ => None,
        }
    }

    pub fn frames_simulated(&self) -> u32 {
        self.frames_simulated
    }

    pub fn simulation_rate_hz(&self) -> u32 {
        self.simulation_rate_hz
    }

    pub fn actor_count(&self) -> u32 {
        self.actors.len() as u32
    }

    pub fn applied_input_count(&self) -> u32 {
        self.applied_input_count
    }

    pub fn deterministic_records(&self) -> Vec<String> {
        let mut records = vec![
            format!("frames={}", self.frames_simulated),
            format!("rate_hz={}", self.simulation_rate_hz),
            format!("applied_inputs={}", self.applied_input_count),
        ];

        records.extend(self.actors.iter().map(|actor| {
            format!("actor:{}:{}:{}", actor.actor_id, actor.actor_kind, actor.stream_signature)
        }));
        records.extend(self.active_actions.iter().map(|action| format!("active_action:{action}")));
        records.extend(self.event_log.iter().cloned());
        records
    }
}
