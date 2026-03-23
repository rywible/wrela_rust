#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use bevy_ecs::component::Component;
use bevy_ecs::prelude::{IntoScheduleConfigs, Resource, SystemSet, World};
use bevy_ecs::schedule::{ExecutorKind, LogLevel, Schedule, ScheduleBuildSettings, ScheduleLabel};
use bevy_ecs::system::ScheduleSystem;
use wr_core::{CrateBoundary, CrateEntryPoint, TweakPack, TweakRegistry, TweakValue};
use wr_world_seed::RootSeed;

pub const fn init_entrypoint() -> CrateEntryPoint {
    CrateEntryPoint::new("wr_ecs", CrateBoundary::Subsystem, false)
}

#[derive(ScheduleLabel, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorldSchedule {
    Startup,
    FixedPrePhysics,
    FixedPhysics,
    FixedGameplay,
    FixedPostGameplay,
    Extract,
    RenderPrep,
    Shutdown,
}

impl WorldSchedule {
    pub const ORDERED: [Self; 8] = [
        Self::Startup,
        Self::FixedPrePhysics,
        Self::FixedPhysics,
        Self::FixedGameplay,
        Self::FixedPostGameplay,
        Self::Extract,
        Self::RenderPrep,
        Self::Shutdown,
    ];

    pub const FIXED_FRAME: [Self; 4] =
        [Self::FixedPrePhysics, Self::FixedPhysics, Self::FixedGameplay, Self::FixedPostGameplay];

    pub const RENDER_FRAME: [Self; 2] = [Self::Extract, Self::RenderPrep];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Startup => "Startup",
            Self::FixedPrePhysics => "FixedPrePhysics",
            Self::FixedPhysics => "FixedPhysics",
            Self::FixedGameplay => "FixedGameplay",
            Self::FixedPostGameplay => "FixedPostGameplay",
            Self::Extract => "Extract",
            Self::RenderPrep => "RenderPrep",
            Self::Shutdown => "Shutdown",
        }
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorldSystemSet {
    Input,
    WorldGen,
    Combat,
    Ai,
    RenderExtract,
    Tooling,
}

pub trait GamePlugin: Send + Sync {
    fn name(&self) -> &'static str;

    fn build(&self, app: &mut EcsRuntime);
}

pub struct EcsRuntime {
    world: World,
    schedules: BTreeMap<WorldSchedule, Schedule>,
    registered_plugins: BTreeSet<&'static str>,
    ambiguity_detection_enabled: bool,
}

impl Default for EcsRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl EcsRuntime {
    pub fn new() -> Self {
        let ambiguity_detection_enabled = cfg!(debug_assertions);
        let schedules = WorldSchedule::ORDERED
            .into_iter()
            .map(|label| {
                let mut schedule = Schedule::new(label);
                // Follow-up before replay-sensitive systems land: switch headless execution to a
                // single-threaded executor or add explicit intra-set ordering constraints.
                schedule.set_executor_kind(ExecutorKind::MultiThreaded);
                schedule.set_build_settings(build_settings(ambiguity_detection_enabled));
                configure_builtin_sets(&mut schedule);
                (label, schedule)
            })
            .collect();

        Self {
            world: World::new(),
            schedules,
            registered_plugins: BTreeSet::new(),
            ambiguity_detection_enabled,
        }
    }

    pub fn ambiguity_detection_enabled(&self) -> bool {
        self.ambiguity_detection_enabled
    }

    pub fn schedule_order(&self) -> &'static [WorldSchedule; 8] {
        &WorldSchedule::ORDERED
    }

    pub fn fixed_frame_order(&self) -> &'static [WorldSchedule; 4] {
        &WorldSchedule::FIXED_FRAME
    }

    pub fn render_frame_order(&self) -> &'static [WorldSchedule; 2] {
        &WorldSchedule::RENDER_FRAME
    }

    pub fn registered_plugins(&self) -> &BTreeSet<&'static str> {
        &self.registered_plugins
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn insert_resource<R>(&mut self, resource: R) -> &mut Self
    where
        R: Resource,
    {
        self.world.insert_resource(resource);
        self
    }

    pub fn add_systems<M>(
        &mut self,
        schedule: WorldSchedule,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.schedules
            .get_mut(&schedule)
            .expect("all world schedules are pre-created")
            .add_systems(systems);
        self
    }

    pub fn add_plugin<P>(&mut self, plugin: P) -> Result<&mut Self, String>
    where
        P: GamePlugin + 'static,
    {
        self.add_boxed_plugin(Box::new(plugin))
    }

    pub fn add_boxed_plugin(&mut self, plugin: Box<dyn GamePlugin>) -> Result<&mut Self, String> {
        let name = plugin.name();
        if !self.registered_plugins.insert(name) {
            return Err(format!("plugin `{name}` is already registered"));
        }

        plugin.build(self);
        Ok(self)
    }

    pub fn run_schedule(&mut self, schedule: WorldSchedule) {
        self.schedules
            .get_mut(&schedule)
            .expect("requested world schedule should exist")
            .run(&mut self.world);
    }

    pub fn run_fixed_frame(&mut self, frame: u32) {
        if self.world.contains_resource::<CurrentFrame>() {
            self.world.resource_mut::<CurrentFrame>().0 = frame;
        } else {
            self.world.insert_resource(CurrentFrame(frame));
        }
        for schedule in WorldSchedule::FIXED_FRAME {
            self.run_schedule(schedule);
        }
    }

    pub fn run_render_frame(&mut self) {
        for schedule in WorldSchedule::RENDER_FRAME {
            self.run_schedule(schedule);
        }
    }
}

fn configure_builtin_sets(schedule: &mut Schedule) {
    schedule.configure_sets(
        (
            WorldSystemSet::Input,
            WorldSystemSet::WorldGen,
            WorldSystemSet::Combat,
            WorldSystemSet::Ai,
            WorldSystemSet::RenderExtract,
            WorldSystemSet::Tooling,
        )
            .chain(),
    );
}

fn build_settings(ambiguity_detection_enabled: bool) -> ScheduleBuildSettings {
    ScheduleBuildSettings {
        ambiguity_detection: if ambiguity_detection_enabled {
            LogLevel::Warn
        } else {
            LogLevel::Ignore
        },
        ..Default::default()
    }
}

#[derive(Debug, Clone, Copy, Resource, PartialEq, Eq)]
struct CurrentFrame(u32);

#[derive(Debug, Clone, Copy, Resource, PartialEq, Eq)]
struct SimulationRateHz(u32);

#[derive(Debug, Clone, Copy, Resource, Default, PartialEq, Eq)]
struct FramesSimulated(u32);

#[derive(Debug, Clone, Copy, Resource, Default, PartialEq, Eq)]
struct AppliedInputCount(u32);

#[derive(Debug, Clone, Default, Resource, PartialEq, Eq)]
struct ActiveActions(BTreeSet<String>);

#[derive(Debug, Clone, Default, Resource, PartialEq, Eq)]
struct EventLog(Vec<String>);

#[derive(Debug, Clone, Default, Resource, PartialEq, Eq)]
struct PendingInputs(Vec<HeadlessScriptedInput>);

#[derive(Debug, Clone, Resource, PartialEq)]
struct TweakRegistryResource(TweakRegistry);

#[derive(Debug, Clone, Component, PartialEq, Eq)]
struct ScenarioActorComponent {
    actor_id: String,
    actor_kind: String,
    seed_stream: Option<String>,
    stream_signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioActorState {
    pub actor_id: String,
    pub actor_kind: String,
    pub seed_stream: Option<String>,
    pub stream_signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadlessActorSpawn {
    pub actor_id: String,
    pub actor_kind: String,
    pub seed_stream: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadlessScriptedInput {
    pub frame: u32,
    pub action: String,
    pub state: String,
}

pub struct HeadlessScenarioWorld {
    runtime: EcsRuntime,
    actors: Vec<ScenarioActorState>,
}

impl HeadlessScenarioWorld {
    pub fn new(simulation_rate_hz: u32, seed: RootSeed, actors: &[HeadlessActorSpawn]) -> Self {
        let mut runtime = EcsRuntime::new();
        runtime
            .insert_resource(CurrentFrame(0))
            .insert_resource(SimulationRateHz(simulation_rate_hz))
            .insert_resource(FramesSimulated::default())
            .insert_resource(AppliedInputCount::default())
            .insert_resource(ActiveActions::default())
            .insert_resource(EventLog::default())
            .insert_resource(PendingInputs::default())
            .insert_resource(TweakRegistryResource(TweakRegistry::default()))
            .add_systems(
                WorldSchedule::FixedPrePhysics,
                apply_pending_inputs.in_set(WorldSystemSet::Input),
            )
            .add_systems(
                WorldSchedule::FixedGameplay,
                advance_fixed_step.in_set(WorldSystemSet::Tooling),
            );

        let mut actor_records = actors
            .iter()
            .map(|actor| {
                let stream_label = actor.seed_stream.as_deref().unwrap_or(actor.actor_id.as_str());
                let state = ScenarioActorState {
                    actor_id: actor.actor_id.clone(),
                    actor_kind: actor.actor_kind.clone(),
                    seed_stream: actor.seed_stream.clone(),
                    stream_signature: seed.derive_stream_hex(stream_label),
                };

                runtime.world_mut().spawn(ScenarioActorComponent {
                    actor_id: state.actor_id.clone(),
                    actor_kind: state.actor_kind.clone(),
                    seed_stream: state.seed_stream.clone(),
                    stream_signature: state.stream_signature.clone(),
                });

                state
            })
            .collect::<Vec<_>>();
        actor_records.sort_by(|left, right| left.actor_id.cmp(&right.actor_id));

        runtime.run_schedule(WorldSchedule::Startup);

        Self { runtime, actors: actor_records }
    }

    pub fn apply_tweak_pack(&mut self, pack: &TweakPack) -> Result<(), String> {
        let dirty_namespaces = {
            let mut tweaks = self.runtime.world_mut().resource_mut::<TweakRegistryResource>();
            tweaks.0.apply_pack(pack).map_err(|error| error.to_string())?;
            tweaks
                .0
                .dirty_namespaces()
                .iter()
                .map(|namespace| namespace.as_str())
                .collect::<Vec<_>>()
                .join(",")
        };

        self.runtime.world_mut().resource_mut::<EventLog>().0.push(format!(
            "tweaks:applied:{}:{}",
            pack.entries.len(),
            if dirty_namespaces.is_empty() { "none" } else { &dirty_namespaces }
        ));
        Ok(())
    }

    pub fn apply_inputs<'a>(
        &mut self,
        inputs: impl IntoIterator<Item = &'a HeadlessScriptedInput>,
    ) {
        let pending_inputs = inputs.into_iter().cloned().collect::<Vec<_>>();
        self.runtime.world_mut().resource_mut::<PendingInputs>().0.extend(pending_inputs);
    }

    pub fn step(&mut self, frame: u32) {
        self.runtime.run_fixed_frame(frame);
    }

    pub fn metric_value(&self, metric: &str) -> Option<f32> {
        match metric {
            "world.actor_count" => Some(self.actors.len() as f32),
            "world.frames_simulated" | "startup.frame_count" => {
                Some(self.frames_simulated() as f32)
            }
            "world.active_action_count" => Some(self.active_actions().len() as f32),
            "world.applied_input_count" => Some(self.applied_input_count() as f32),
            "tweaks.dirty_namespace_count" => {
                Some(self.tweak_registry().dirty_namespace_count() as f32)
            }
            metric => metric
                .strip_prefix("tweak.")
                .and_then(|key| self.tweak_registry().value(key))
                .map(tweak_value_as_metric),
        }
    }

    pub fn frames_simulated(&self) -> u32 {
        self.runtime.world().resource::<FramesSimulated>().0
    }

    pub fn simulation_rate_hz(&self) -> u32 {
        self.runtime.world().resource::<SimulationRateHz>().0
    }

    pub fn actor_count(&self) -> u32 {
        self.actors.len() as u32
    }

    pub fn entity_count(&self) -> u32 {
        self.actor_count()
    }

    pub fn applied_input_count(&self) -> u32 {
        self.runtime.world().resource::<AppliedInputCount>().0
    }

    pub fn active_action_count(&self) -> u32 {
        self.active_actions().len() as u32
    }

    pub fn estimated_memory_bytes(&self) -> u64 {
        let actor_bytes =
            (self.actors.capacity() * std::mem::size_of::<ScenarioActorState>()) as u64;
        let active_action_bytes =
            self.active_actions().iter().map(|action| action.capacity() as u64).sum::<u64>();
        let event_log = &self.runtime.world().resource::<EventLog>().0;
        let event_log_bytes = event_log.iter().map(|event| event.capacity() as u64).sum::<u64>();
        let tweak_entries = self.tweak_registry().entries();
        let tweak_bytes = tweak_entries
            .iter()
            .map(|entry| entry.key.len() as u64 + std::mem::size_of::<TweakValue>() as u64)
            .sum::<u64>();

        actor_bytes + active_action_bytes + event_log_bytes + tweak_bytes
    }

    pub fn tweak_registry(&self) -> &TweakRegistry {
        &self.runtime.world().resource::<TweakRegistryResource>().0
    }

    pub fn deterministic_records(&self) -> Vec<String> {
        let mut records = vec![
            format!("frames={}", self.frames_simulated()),
            format!("rate_hz={}", self.simulation_rate_hz()),
            format!("applied_inputs={}", self.applied_input_count()),
        ];

        records.extend(self.actors.iter().map(|actor| {
            format!("actor:{}:{}:{}", actor.actor_id, actor.actor_kind, actor.stream_signature)
        }));
        records.extend(
            self.tweak_registry()
                .entries()
                .into_iter()
                .map(|entry| format!("tweak:{}={}", entry.key, tweak_value_as_record(entry.value))),
        );
        records.extend(
            self.tweak_registry()
                .dirty_namespaces()
                .iter()
                .map(|namespace| format!("tweak_dirty:{}", namespace.as_str())),
        );
        records
            .extend(self.active_actions().iter().map(|action| format!("active_action:{action}")));
        records.extend(self.runtime.world().resource::<EventLog>().0.iter().cloned());
        records
    }

    fn active_actions(&self) -> &BTreeSet<String> {
        &self.runtime.world().resource::<ActiveActions>().0
    }
}

fn apply_pending_inputs(
    frame: bevy_ecs::system::Res<'_, CurrentFrame>,
    mut pending_inputs: bevy_ecs::system::ResMut<'_, PendingInputs>,
    mut active_actions: bevy_ecs::system::ResMut<'_, ActiveActions>,
    mut applied_input_count: bevy_ecs::system::ResMut<'_, AppliedInputCount>,
    mut event_log: bevy_ecs::system::ResMut<'_, EventLog>,
) {
    let pending = std::mem::take(&mut pending_inputs.0);
    for input in pending {
        match input.state.as_str() {
            "pressed" => {
                active_actions.0.insert(input.action.clone());
                applied_input_count.0 += 1;
                event_log.0.push(format!("frame={}:input:{}:pressed", frame.0, input.action));
            }
            "released" => {
                active_actions.0.remove(&input.action);
                applied_input_count.0 += 1;
                event_log.0.push(format!("frame={}:input:{}:released", frame.0, input.action));
            }
            other => {
                event_log
                    .0
                    .push(format!("frame={}:ignored_input:{}:{other}", frame.0, input.action));
            }
        }
    }
}

fn advance_fixed_step(
    frame: bevy_ecs::system::Res<'_, CurrentFrame>,
    simulation_rate: bevy_ecs::system::Res<'_, SimulationRateHz>,
    active_actions: bevy_ecs::system::Res<'_, ActiveActions>,
    mut frames_simulated: bevy_ecs::system::ResMut<'_, FramesSimulated>,
    mut event_log: bevy_ecs::system::ResMut<'_, EventLog>,
) {
    frames_simulated.0 += 1;
    event_log.0.push(format!(
        "frame={}:step:{}:{}hz:{}actions",
        frame.0,
        frames_simulated.0,
        simulation_rate.0,
        active_actions.0.len()
    ));
}

fn tweak_value_as_metric(value: TweakValue) -> f32 {
    match value {
        TweakValue::Scalar(value) => value,
        TweakValue::Toggle(value) => {
            if value {
                1.0
            } else {
                0.0
            }
        }
    }
}

fn tweak_value_as_record(value: TweakValue) -> String {
    match value {
        TweakValue::Scalar(value) => format!("{value:.6}"),
        TweakValue::Toggle(value) => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::component::Component;
    use bevy_ecs::entity::Entity;
    use bevy_ecs::prelude::Resource;
    use bevy_ecs::query::With;

    use super::*;

    #[derive(Debug, Clone, Default, Resource, PartialEq, Eq)]
    struct TraceLog(Vec<&'static str>);

    #[derive(Debug, Clone, Copy, Default, Resource, PartialEq, Eq)]
    struct FrameRuns(u32);

    #[derive(Debug, Clone, Copy, Default, Component, PartialEq, Eq)]
    struct TestMarker;

    struct TestLifecyclePlugin;

    impl GamePlugin for TestLifecyclePlugin {
        fn name(&self) -> &'static str {
            "test_lifecycle"
        }

        fn build(&self, app: &mut EcsRuntime) {
            app.insert_resource(TraceLog::default())
                .insert_resource(FrameRuns::default())
                .add_systems(
                    WorldSchedule::Startup,
                    startup_spawn_marker.in_set(WorldSystemSet::WorldGen),
                )
                .add_systems(
                    WorldSchedule::FixedPrePhysics,
                    record_fixed_pre.in_set(WorldSystemSet::Tooling),
                )
                .add_systems(
                    WorldSchedule::FixedPhysics,
                    record_fixed_physics.in_set(WorldSystemSet::Combat),
                )
                .add_systems(
                    WorldSchedule::FixedGameplay,
                    (
                        increment_frame_runs.in_set(WorldSystemSet::Ai),
                        record_fixed_gameplay.in_set(WorldSystemSet::Ai),
                    ),
                )
                .add_systems(
                    WorldSchedule::FixedPostGameplay,
                    record_fixed_post.in_set(WorldSystemSet::Tooling),
                )
                .add_systems(
                    WorldSchedule::Extract,
                    record_extract.in_set(WorldSystemSet::RenderExtract),
                )
                .add_systems(
                    WorldSchedule::RenderPrep,
                    record_render_prep.in_set(WorldSystemSet::RenderExtract),
                )
                .add_systems(
                    WorldSchedule::Shutdown,
                    shutdown_cleanup.in_set(WorldSystemSet::Tooling),
                );
        }
    }

    fn startup_spawn_marker(
        mut commands: bevy_ecs::system::Commands<'_, '_>,
        mut trace: bevy_ecs::system::ResMut<'_, TraceLog>,
    ) {
        trace.0.push("startup");
        commands.spawn(TestMarker);
    }

    fn record_fixed_pre(mut trace: bevy_ecs::system::ResMut<'_, TraceLog>) {
        trace.0.push("fixed_pre");
    }

    fn record_fixed_physics(mut trace: bevy_ecs::system::ResMut<'_, TraceLog>) {
        trace.0.push("fixed_physics");
    }

    fn increment_frame_runs(mut frame_runs: bevy_ecs::system::ResMut<'_, FrameRuns>) {
        frame_runs.0 += 1;
    }

    fn record_fixed_gameplay(mut trace: bevy_ecs::system::ResMut<'_, TraceLog>) {
        trace.0.push("fixed_gameplay");
    }

    fn record_fixed_post(mut trace: bevy_ecs::system::ResMut<'_, TraceLog>) {
        trace.0.push("fixed_post");
    }

    fn record_extract(mut trace: bevy_ecs::system::ResMut<'_, TraceLog>) {
        trace.0.push("extract");
    }

    fn record_render_prep(mut trace: bevy_ecs::system::ResMut<'_, TraceLog>) {
        trace.0.push("render_prep");
    }

    fn shutdown_cleanup(
        entities: bevy_ecs::system::Query<'_, '_, Entity, With<TestMarker>>,
        mut commands: bevy_ecs::system::Commands<'_, '_>,
        mut trace: bevy_ecs::system::ResMut<'_, TraceLog>,
    ) {
        for entity in &entities {
            commands.entity(entity).despawn();
        }
        trace.0.push("shutdown");
    }

    fn count_test_markers(world: &mut World) -> usize {
        let mut query = world.query_filtered::<Entity, With<TestMarker>>();
        query.iter(world).count()
    }

    fn test_world() -> HeadlessScenarioWorld {
        HeadlessScenarioWorld::new(
            60,
            RootSeed::parse_hex("0xDEADBEEF").expect("seed should parse"),
            &[HeadlessActorSpawn {
                actor_id: "player".to_owned(),
                actor_kind: "player_sword".to_owned(),
                seed_stream: Some("player".to_owned()),
            }],
        )
    }

    #[test]
    fn sample_plugin_registers_resources_and_systems_without_global_registry() {
        let mut runtime = EcsRuntime::new();

        runtime.add_plugin(TestLifecyclePlugin).expect("plugin should register");
        runtime.run_schedule(WorldSchedule::Startup);

        assert_eq!(count_test_markers(runtime.world_mut()), 1);

        runtime.run_fixed_frame(7);
        runtime.run_render_frame();
        runtime.run_schedule(WorldSchedule::Shutdown);

        assert_eq!(count_test_markers(runtime.world_mut()), 0);
        assert_eq!(runtime.world().resource::<FrameRuns>().0, 1);
        assert_eq!(
            runtime.world().resource::<TraceLog>().0,
            vec![
                "startup",
                "fixed_pre",
                "fixed_physics",
                "fixed_gameplay",
                "fixed_post",
                "extract",
                "render_prep",
                "shutdown",
            ]
        );
    }

    #[test]
    fn schedule_order_and_builtin_sets_are_stable() {
        let runtime = EcsRuntime::new();

        assert_eq!(
            runtime.schedule_order(),
            &[
                WorldSchedule::Startup,
                WorldSchedule::FixedPrePhysics,
                WorldSchedule::FixedPhysics,
                WorldSchedule::FixedGameplay,
                WorldSchedule::FixedPostGameplay,
                WorldSchedule::Extract,
                WorldSchedule::RenderPrep,
                WorldSchedule::Shutdown,
            ]
        );
        assert_eq!(
            runtime.fixed_frame_order(),
            &[
                WorldSchedule::FixedPrePhysics,
                WorldSchedule::FixedPhysics,
                WorldSchedule::FixedGameplay,
                WorldSchedule::FixedPostGameplay,
            ]
        );
        assert!(runtime.ambiguity_detection_enabled() == cfg!(debug_assertions));
    }

    #[test]
    fn duplicate_plugin_names_are_rejected() {
        let mut runtime = EcsRuntime::new();

        runtime.add_plugin(TestLifecyclePlugin).expect("first plugin should register");
        let error = match runtime.add_plugin(TestLifecyclePlugin) {
            Ok(_) => panic!("duplicate plugin names should be rejected"),
            Err(error) => error,
        };

        assert!(error.contains("test_lifecycle"));
    }

    #[test]
    fn ignored_inputs_do_not_increment_applied_input_count() {
        let mut world = test_world();
        let inputs = [HeadlessScriptedInput {
            frame: 0,
            action: "dash".to_owned(),
            state: "unknown_state".to_owned(),
        }];

        world.apply_inputs(&inputs);
        world.step(0);

        assert_eq!(world.applied_input_count(), 0);
        assert_eq!(world.metric_value("world.applied_input_count"), Some(0.0));
        assert!(
            world
                .deterministic_records()
                .iter()
                .any(|record| record.contains("ignored_input:dash:unknown_state"))
        );
        assert!(
            !world
                .deterministic_records()
                .iter()
                .any(|record| record == "frame=0:input:dash:unknown_state")
        );
    }

    #[test]
    fn unknown_metrics_return_none() {
        let world = test_world();

        assert_eq!(world.metric_value("world.unknown_metric"), None);
    }

    #[test]
    fn deterministic_records_are_stable_for_identical_input_sequences() {
        let mut first = test_world();
        let mut second = test_world();
        let inputs = [
            HeadlessScriptedInput {
                frame: 0,
                action: "dash".to_owned(),
                state: "pressed".to_owned(),
            },
            HeadlessScriptedInput {
                frame: 1,
                action: "dash".to_owned(),
                state: "released".to_owned(),
            },
        ];

        for frame in 0..2 {
            first.apply_inputs(inputs.iter().filter(|input| input.frame == frame));
            second.apply_inputs(inputs.iter().filter(|input| input.frame == frame));
            first.step(frame);
            second.step(frame);
        }

        assert_eq!(first.deterministic_records(), second.deterministic_records());
    }

    #[test]
    fn tweak_packs_update_metrics_and_dirty_namespaces() {
        let mut world = test_world();
        let pack = TweakPack::new(BTreeMap::from([
            ("combat.hitstop_scale".to_owned(), TweakValue::Scalar(0.35)),
            ("player.camera_bob_enabled".to_owned(), TweakValue::Toggle(false)),
        ]));

        world.apply_tweak_pack(&pack).expect("tweak pack should apply");

        assert_eq!(world.metric_value("tweak.combat.hitstop_scale"), Some(0.35));
        assert_eq!(world.metric_value("tweak.player.camera_bob_enabled"), Some(0.0));
        assert_eq!(world.metric_value("tweaks.dirty_namespace_count"), Some(2.0));
        assert!(world.deterministic_records().iter().any(|record| record == "tweak_dirty:combat"));
        assert!(world.deterministic_records().iter().any(|record| record == "tweak_dirty:player"));
    }
}
