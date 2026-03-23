#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Fullscreen, Window, WindowAttributes, WindowId};
use wr_core::{CrateBoundary, CrateEntryPoint};
use wr_render_api::{ColorRgba8, GraphicsAdapterInfo, RenderSize};
use wr_render_wgpu::SurfaceRenderer;

const DEFAULT_RENDER_RATE_HZ: u32 = 60;
const DEFAULT_CLEAR_COLOR: ColorRgba8 = ColorRgba8::new(46, 78, 126, 255);

pub const fn init_entrypoint() -> CrateEntryPoint {
    CrateEntryPoint::new("wr_platform", CrateBoundary::Subsystem, false)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EngineAction {
    MoveForward,
    MoveBackward,
    MoveLeft,
    MoveRight,
    Jump,
    Dash,
    LightAttack,
    HeavyAttack,
    Parry,
    ToggleDeveloperOverlay,
}

impl EngineAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MoveForward => "move_forward",
            Self::MoveBackward => "move_backward",
            Self::MoveLeft => "move_left",
            Self::MoveRight => "move_right",
            Self::Jump => "jump",
            Self::Dash => "dash",
            Self::LightAttack => "light_attack",
            Self::HeavyAttack => "heavy_attack",
            Self::Parry => "parry",
            Self::ToggleDeveloperOverlay => "toggle_developer_overlay",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActionState {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KeyboardControl {
    KeyW,
    KeyA,
    KeyS,
    KeyD,
    Space,
    ShiftLeft,
    KeyF,
    F1,
}

impl KeyboardControl {
    pub fn from_key_code(code: KeyCode) -> Option<Self> {
        match code {
            KeyCode::KeyW => Some(Self::KeyW),
            KeyCode::KeyA => Some(Self::KeyA),
            KeyCode::KeyS => Some(Self::KeyS),
            KeyCode::KeyD => Some(Self::KeyD),
            KeyCode::Space => Some(Self::Space),
            KeyCode::ShiftLeft => Some(Self::ShiftLeft),
            KeyCode::KeyF => Some(Self::KeyF),
            KeyCode::F1 => Some(Self::F1),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MouseControl {
    Left,
    Right,
    Middle,
}

impl MouseControl {
    pub fn from_mouse_button(button: MouseButton) -> Option<Self> {
        match button {
            MouseButton::Left => Some(Self::Left),
            MouseButton::Right => Some(Self::Right),
            MouseButton::Middle => Some(Self::Middle),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GamepadControl {
    South,
    East,
    West,
    North,
    LeftShoulder,
    RightShoulder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RawInput {
    Keyboard(KeyboardControl),
    Mouse(MouseControl),
    Gamepad(GamepadControl),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputSignal {
    pub input: RawInput,
    pub state: ActionState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionEvent {
    pub action: EngineAction,
    pub state: ActionState,
    pub input: RawInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionMap {
    bindings: BTreeMap<RawInput, EngineAction>,
}

impl Default for ActionMap {
    fn default() -> Self {
        Self {
            bindings: BTreeMap::from([
                (RawInput::Keyboard(KeyboardControl::KeyW), EngineAction::MoveForward),
                (RawInput::Keyboard(KeyboardControl::KeyS), EngineAction::MoveBackward),
                (RawInput::Keyboard(KeyboardControl::KeyA), EngineAction::MoveLeft),
                (RawInput::Keyboard(KeyboardControl::KeyD), EngineAction::MoveRight),
                (RawInput::Keyboard(KeyboardControl::Space), EngineAction::Jump),
                (RawInput::Keyboard(KeyboardControl::ShiftLeft), EngineAction::Dash),
                (RawInput::Keyboard(KeyboardControl::KeyF), EngineAction::Parry),
                (RawInput::Keyboard(KeyboardControl::F1), EngineAction::ToggleDeveloperOverlay),
                (RawInput::Mouse(MouseControl::Left), EngineAction::LightAttack),
                (RawInput::Mouse(MouseControl::Right), EngineAction::HeavyAttack),
                (RawInput::Gamepad(GamepadControl::South), EngineAction::Jump),
                (RawInput::Gamepad(GamepadControl::East), EngineAction::Dash),
                (RawInput::Gamepad(GamepadControl::RightShoulder), EngineAction::LightAttack),
                (RawInput::Gamepad(GamepadControl::LeftShoulder), EngineAction::HeavyAttack),
                (RawInput::Gamepad(GamepadControl::West), EngineAction::Parry),
            ]),
        }
    }
}

impl ActionMap {
    pub fn bindings(&self) -> &BTreeMap<RawInput, EngineAction> {
        &self.bindings
    }

    pub fn bind(&mut self, input: RawInput, action: EngineAction) -> Option<EngineAction> {
        self.bindings.insert(input, action)
    }

    pub fn action_for(&self, input: RawInput) -> Option<EngineAction> {
        self.bindings.get(&input).copied()
    }

    pub fn translate_signal(
        &self,
        tracker: &mut ActionTracker,
        signal: InputSignal,
    ) -> Option<ActionEvent> {
        let action = self.action_for(signal.input)?;
        tracker.transition(action, signal.state).then_some(ActionEvent {
            action,
            state: signal.state,
            input: signal.input,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActionTracker {
    active_actions: BTreeSet<EngineAction>,
}

impl ActionTracker {
    pub fn is_active(&self, action: EngineAction) -> bool {
        self.active_actions.contains(&action)
    }

    pub fn active_actions(&self) -> &BTreeSet<EngineAction> {
        &self.active_actions
    }

    pub fn transition(&mut self, action: EngineAction, state: ActionState) -> bool {
        match state {
            ActionState::Pressed => self.active_actions.insert(action),
            ActionState::Released => self.active_actions.remove(&action),
        }
    }
}

pub fn input_signal_from_window_event(event: &WindowEvent) -> Option<InputSignal> {
    match event {
        WindowEvent::KeyboardInput { event, .. } => {
            input_signal_from_keyboard(event.physical_key, event.state, event.repeat)
        }
        WindowEvent::MouseInput { state, button, .. } => {
            input_signal_from_mouse_button(*button, *state)
        }
        _ => None,
    }
}

pub fn input_signal_from_keyboard(
    physical_key: PhysicalKey,
    state: ElementState,
    repeat: bool,
) -> Option<InputSignal> {
    if repeat {
        return None;
    }

    let PhysicalKey::Code(code) = physical_key else {
        return None;
    };
    let control = KeyboardControl::from_key_code(code)?;
    Some(InputSignal {
        input: RawInput::Keyboard(control),
        state: element_state_to_action_state(state),
    })
}

pub fn input_signal_from_mouse_button(
    button: MouseButton,
    state: ElementState,
) -> Option<InputSignal> {
    let control = MouseControl::from_mouse_button(button)?;
    Some(InputSignal {
        input: RawInput::Mouse(control),
        state: element_state_to_action_state(state),
    })
}

fn element_state_to_action_state(state: ElementState) -> ActionState {
    match state {
        ElementState::Pressed => ActionState::Pressed,
        ElementState::Released => ActionState::Released,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedStepConfig {
    pub simulation_rate_hz: u32,
    pub max_catch_up_steps: u32,
}

impl Default for FixedStepConfig {
    fn default() -> Self {
        Self { simulation_rate_hz: 120, max_catch_up_steps: 5 }
    }
}

impl FixedStepConfig {
    pub fn validate(self) -> Result<Self, String> {
        if self.simulation_rate_hz == 0 {
            return Err(String::from("simulation_rate_hz must be greater than zero"));
        }
        if self.max_catch_up_steps == 0 {
            return Err(String::from("max_catch_up_steps must be greater than zero"));
        }
        Ok(self)
    }

    pub fn step_duration(self) -> Duration {
        Duration::from_secs_f64(1.0 / f64::from(self.simulation_rate_hz))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedStepDecision {
    pub steps_to_run: u32,
    pub backlog_saturated: bool,
    pub residual_time: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedStepClock {
    config: FixedStepConfig,
    step_duration: Duration,
    accumulator: Duration,
}

impl FixedStepClock {
    pub fn new(config: FixedStepConfig) -> Result<Self, String> {
        let config = config.validate()?;
        Ok(Self { step_duration: config.step_duration(), config, accumulator: Duration::ZERO })
    }

    pub fn config(&self) -> FixedStepConfig {
        self.config
    }

    pub fn step_duration(&self) -> Duration {
        self.step_duration
    }

    pub fn advance(&mut self, elapsed: Duration) -> FixedStepDecision {
        let max_backlog =
            self.step_duration.checked_mul(self.config.max_catch_up_steps).unwrap_or(Duration::MAX);

        let mut backlog_saturated = false;
        self.accumulator = self.accumulator.saturating_add(elapsed);
        if self.accumulator > max_backlog {
            self.accumulator = max_backlog;
            backlog_saturated = true;
        }

        let mut steps_to_run = 0;
        while self.accumulator >= self.step_duration
            && steps_to_run < self.config.max_catch_up_steps
        {
            self.accumulator = self.accumulator.saturating_sub(self.step_duration);
            steps_to_run += 1;
        }

        FixedStepDecision { steps_to_run, backlog_saturated, residual_time: self.accumulator }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowMode {
    Windowed,
    Borderless,
}

impl WindowMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Windowed => "windowed",
            Self::Borderless => "borderless",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub mode: WindowMode,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: String::from("Wrela v0 Bootstrap Client"),
            width: 1280,
            height: 720,
            mode: WindowMode::Windowed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClientRuntimeConfig {
    pub window: WindowConfig,
    pub fixed_step: FixedStepConfig,
    pub auto_close_after_fixed_updates: Option<u64>,
    pub auto_close_after_redraws: Option<u64>,
}

impl ClientRuntimeConfig {
    pub fn validate(&self) -> Result<(), String> {
        self.fixed_step.validate()?;
        if self.window.width == 0 || self.window.height == 0 {
            return Err(String::from("window dimensions must be greater than zero"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ClientDiagnostics {
    pub fixed_updates: u64,
    pub rendered_frames: u64,
    pub raw_input_events: u64,
    pub action_events: u64,
    pub resize_events: u64,
    pub focus_change_events: u64,
    pub backlog_saturation_events: u64,
    pub max_fixed_updates_per_pump: u32,
    pub average_render_interval: Option<Duration>,
    pub last_render_interval: Option<Duration>,
    pub last_window_size: (u32, u32),
    pub focused: bool,
    pub overlay_visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientExitReason {
    CloseRequested,
    AutoCloseAfterFixedUpdates(u64),
    AutoCloseAfterRedraws(u64),
}

impl fmt::Display for ClientExitReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CloseRequested => write!(f, "close_requested"),
            Self::AutoCloseAfterFixedUpdates(limit) => {
                write!(f, "auto_close_after_fixed_updates({limit})")
            }
            Self::AutoCloseAfterRedraws(limit) => {
                write!(f, "auto_close_after_redraws({limit})")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientRunSummary {
    pub exit_reason: ClientExitReason,
    pub window_mode: WindowMode,
    pub simulation_rate_hz: u32,
    pub diagnostics: ClientDiagnostics,
    pub graphics_adapter: Option<GraphicsAdapterInfo>,
}

impl ClientRunSummary {
    pub fn summary_line(&self) -> String {
        let average_render_ms = self
            .diagnostics
            .average_render_interval
            .map(|duration| format!("{:.2}", duration.as_secs_f64() * 1000.0))
            .unwrap_or_else(|| String::from("n/a"));

        format!(
            "exit={} window_mode={} fixed_updates={} rendered_frames={} resize_events={} focus_changes={} avg_render_ms={} backlog_saturations={} adapter={}",
            self.exit_reason,
            self.window_mode.as_str(),
            self.diagnostics.fixed_updates,
            self.diagnostics.rendered_frames,
            self.diagnostics.resize_events,
            self.diagnostics.focus_change_events,
            average_render_ms,
            self.diagnostics.backlog_saturation_events,
            self.graphics_adapter
                .as_ref()
                .map(|adapter| format!("{}:{}", adapter.backend, adapter.name))
                .unwrap_or_else(|| String::from("unavailable"))
        )
    }
}

pub fn run_client(config: ClientRuntimeConfig) -> Result<ClientRunSummary, String> {
    config.validate()?;

    let event_loop =
        EventLoop::new().map_err(|error| format!("failed to create event loop: {error}"))?;

    let mut app = PlatformClientApp::new(config)?;
    event_loop.run_app(&mut app).map_err(|error| format!("client event loop failed: {error}"))?;

    app.finish()
}

struct PlatformClientApp {
    config: ClientRuntimeConfig,
    window: Option<Arc<Window>>,
    window_id: Option<WindowId>,
    renderer: Option<SurfaceRenderer>,
    action_map: ActionMap,
    action_tracker: ActionTracker,
    fixed_step_clock: FixedStepClock,
    diagnostics: ClientDiagnostics,
    last_step_instant: Option<Instant>,
    last_render_instant: Option<Instant>,
    last_redraw_request_instant: Option<Instant>,
    total_render_interval: Duration,
    render_interval_samples: u64,
    exit_reason: Option<ClientExitReason>,
    fatal_error: Option<String>,
}

impl PlatformClientApp {
    fn new(config: ClientRuntimeConfig) -> Result<Self, String> {
        Ok(Self {
            fixed_step_clock: FixedStepClock::new(config.fixed_step)?,
            config,
            window: None,
            window_id: None,
            renderer: None,
            action_map: ActionMap::default(),
            action_tracker: ActionTracker::default(),
            diagnostics: ClientDiagnostics::default(),
            last_step_instant: None,
            last_render_instant: None,
            last_redraw_request_instant: None,
            total_render_interval: Duration::ZERO,
            render_interval_samples: 0,
            exit_reason: None,
            fatal_error: None,
        })
    }

    fn finish(self) -> Result<ClientRunSummary, String> {
        if let Some(error) = self.fatal_error {
            return Err(error);
        }

        let exit_reason = self
            .exit_reason
            .ok_or_else(|| String::from("client exited without recording an exit reason"))?;

        Ok(ClientRunSummary {
            exit_reason,
            window_mode: self.config.window.mode,
            simulation_rate_hz: self.config.fixed_step.simulation_rate_hz,
            diagnostics: self.diagnostics,
            graphics_adapter: self.renderer.as_ref().map(|renderer| renderer.adapter().clone()),
        })
    }

    fn request_exit(&mut self, event_loop: &ActiveEventLoop, reason: ClientExitReason) {
        self.exit_reason.get_or_insert(reason);
        event_loop.exit();
    }

    fn handle_input_signal(&mut self, signal: InputSignal) {
        self.diagnostics.raw_input_events += 1;
        if let Some(event) = self.action_map.translate_signal(&mut self.action_tracker, signal) {
            self.diagnostics.action_events += 1;
            if event.action == EngineAction::ToggleDeveloperOverlay
                && event.state == ActionState::Pressed
            {
                self.diagnostics.overlay_visible = !self.diagnostics.overlay_visible;
            }
        }
    }
}

impl ApplicationHandler for PlatformClientApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let mut attributes = WindowAttributes::default()
            .with_title(self.config.window.title.clone())
            .with_resizable(true)
            .with_inner_size(LogicalSize::new(
                f64::from(self.config.window.width),
                f64::from(self.config.window.height),
            ));

        if self.config.window.mode == WindowMode::Borderless {
            attributes = attributes.with_fullscreen(Some(Fullscreen::Borderless(None)));
        }

        match event_loop.create_window(attributes) {
            Ok(window) => {
                let window = Arc::new(window);
                let inner_size = window.inner_size();
                self.diagnostics.last_window_size = (inner_size.width, inner_size.height);
                self.window_id = Some(window.id());
                self.renderer = match SurfaceRenderer::new(
                    Arc::clone(&window),
                    RenderSize::new(inner_size.width, inner_size.height),
                ) {
                    Ok(renderer) => Some(renderer),
                    Err(error) => {
                        self.fatal_error =
                            Some(format!("failed to initialize wgpu surface renderer: {error}"));
                        event_loop.exit();
                        None
                    }
                };
                self.window = Some(window);
            }
            Err(error) => {
                self.fatal_error = Some(format!("failed to create client window: {error}"));
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.window_id != Some(window_id) {
            return;
        }

        if let Some(signal) = input_signal_from_window_event(&event) {
            self.handle_input_signal(signal);
        }

        match event {
            WindowEvent::CloseRequested => {
                self.request_exit(event_loop, ClientExitReason::CloseRequested);
            }
            WindowEvent::Focused(focused) => {
                if self.diagnostics.focused != focused {
                    self.diagnostics.focus_change_events += 1;
                    self.diagnostics.focused = focused;
                }
            }
            WindowEvent::Resized(size) => {
                self.diagnostics.resize_events += 1;
                self.diagnostics.last_window_size = (size.width, size.height);
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(RenderSize::new(size.width, size.height));
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = &mut self.renderer
                    && let Err(error) = renderer.render(DEFAULT_CLEAR_COLOR)
                {
                    self.fatal_error = Some(format!("failed to render a client frame: {error}"));
                    event_loop.exit();
                    return;
                }
                self.diagnostics.rendered_frames += 1;

                // Bootstrap shell only: platform wall-clock sampling remains local to the shell.
                // Replay-oriented time-source injection should land before client-side replay work.
                let now = Instant::now();
                if let Some(last_render_instant) = self.last_render_instant.replace(now) {
                    let render_interval = now.saturating_duration_since(last_render_instant);
                    self.diagnostics.last_render_interval = Some(render_interval);
                    self.total_render_interval =
                        self.total_render_interval.saturating_add(render_interval);
                    self.render_interval_samples += 1;
                    self.diagnostics.average_render_interval = Some(
                        self.total_render_interval.div_f64(self.render_interval_samples as f64),
                    );
                }

                if let Some(limit) = self.config.auto_close_after_redraws
                    && self.diagnostics.rendered_frames >= limit
                {
                    self.request_exit(event_loop, ClientExitReason::AutoCloseAfterRedraws(limit));
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Bootstrap shell only: platform wall-clock sampling remains local to the shell.
        // Replay-oriented time-source injection should land before client-side replay work.
        let now = Instant::now();
        let elapsed = match self.last_step_instant.replace(now) {
            Some(previous) => now.saturating_duration_since(previous),
            None => Duration::ZERO,
        };

        let decision = self.fixed_step_clock.advance(elapsed);
        if decision.backlog_saturated {
            self.diagnostics.backlog_saturation_events += 1;
        }
        self.diagnostics.max_fixed_updates_per_pump =
            self.diagnostics.max_fixed_updates_per_pump.max(decision.steps_to_run);
        self.diagnostics.fixed_updates += u64::from(decision.steps_to_run);

        if let Some(limit) = self.config.auto_close_after_fixed_updates
            && self.diagnostics.fixed_updates >= limit
        {
            self.request_exit(event_loop, ClientExitReason::AutoCloseAfterFixedUpdates(limit));
            return;
        }

        let render_interval = render_interval();
        if let Some(window) = &self.window {
            let should_redraw = self.last_redraw_request_instant.is_none_or(|last_request| {
                now.saturating_duration_since(last_request) >= render_interval
            });
            if should_redraw {
                self.last_redraw_request_instant = Some(now);
                window.request_redraw();
            }
        }

        let next_fixed_update =
            now + self.fixed_step_clock.step_duration().saturating_sub(decision.residual_time);
        let next_redraw = self
            .last_redraw_request_instant
            .map(|last_request| last_request + render_interval)
            .unwrap_or(now);
        let next_deadline =
            if next_fixed_update < next_redraw { next_fixed_update } else { next_redraw };
        event_loop.set_control_flow(ControlFlow::WaitUntil(next_deadline));
    }
}

fn render_interval() -> Duration {
    Duration::from_secs_f64(1.0 / f64::from(DEFAULT_RENDER_RATE_HZ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::dpi::PhysicalSize;
    use winit::event::DeviceId;

    #[test]
    fn action_map_translates_keyboard_mouse_and_gamepad_inputs() {
        let action_map = ActionMap::default();

        assert_eq!(
            action_map.action_for(RawInput::Keyboard(KeyboardControl::KeyW)),
            Some(EngineAction::MoveForward)
        );
        assert_eq!(
            action_map.action_for(RawInput::Mouse(MouseControl::Left)),
            Some(EngineAction::LightAttack)
        );
        assert_eq!(
            action_map.action_for(RawInput::Gamepad(GamepadControl::West)),
            Some(EngineAction::Parry)
        );
    }

    #[test]
    fn action_tracker_only_emits_edge_transitions() {
        let action_map = ActionMap::default();
        let mut tracker = ActionTracker::default();
        let press = InputSignal {
            input: RawInput::Keyboard(KeyboardControl::Space),
            state: ActionState::Pressed,
        };
        let release = InputSignal {
            input: RawInput::Keyboard(KeyboardControl::Space),
            state: ActionState::Released,
        };

        let first_press = action_map.translate_signal(&mut tracker, press);
        let repeated_press = action_map.translate_signal(&mut tracker, press);
        let release_event = action_map.translate_signal(&mut tracker, release);
        let repeated_release = action_map.translate_signal(&mut tracker, release);

        assert_eq!(
            first_press,
            Some(ActionEvent {
                action: EngineAction::Jump,
                state: ActionState::Pressed,
                input: RawInput::Keyboard(KeyboardControl::Space),
            })
        );
        assert_eq!(repeated_press, None);
        assert_eq!(
            release_event,
            Some(ActionEvent {
                action: EngineAction::Jump,
                state: ActionState::Released,
                input: RawInput::Keyboard(KeyboardControl::Space),
            })
        );
        assert_eq!(repeated_release, None);
        assert!(!tracker.is_active(EngineAction::Jump));
    }

    #[test]
    fn keyboard_and_mouse_helpers_translate_supported_inputs() {
        let keyboard_signal = input_signal_from_keyboard(
            PhysicalKey::Code(KeyCode::KeyW),
            ElementState::Pressed,
            false,
        );
        let repeated_keyboard_signal = input_signal_from_keyboard(
            PhysicalKey::Code(KeyCode::KeyW),
            ElementState::Pressed,
            true,
        );
        let mouse_signal =
            input_signal_from_mouse_button(MouseButton::Left, ElementState::Released);

        assert_eq!(
            keyboard_signal,
            Some(InputSignal {
                input: RawInput::Keyboard(KeyboardControl::KeyW),
                state: ActionState::Pressed,
            })
        );
        assert_eq!(repeated_keyboard_signal, None);
        assert_eq!(
            mouse_signal,
            Some(InputSignal {
                input: RawInput::Mouse(MouseControl::Left),
                state: ActionState::Released,
            })
        );
    }

    #[test]
    fn window_event_bridge_translates_mouse_input_and_ignores_unmapped_events() {
        let mouse_event = WindowEvent::MouseInput {
            device_id: DeviceId::dummy(),
            state: ElementState::Pressed,
            button: MouseButton::Right,
        };
        let resize_event = WindowEvent::Resized(PhysicalSize::new(1280, 720));

        assert_eq!(
            input_signal_from_window_event(&mouse_event),
            Some(InputSignal {
                input: RawInput::Mouse(MouseControl::Right),
                state: ActionState::Pressed,
            })
        );
        assert_eq!(input_signal_from_window_event(&resize_event), None);
    }

    #[test]
    fn fixed_step_clock_emits_expected_steps_and_carry() {
        let config = FixedStepConfig { simulation_rate_hz: 60, max_catch_up_steps: 5 };
        let mut clock = FixedStepClock::new(config).expect("clock should build");
        let step = clock.step_duration();

        let first = clock.advance(step + step);
        let second = clock.advance(step);

        assert_eq!(first.steps_to_run, 2);
        assert!(!first.backlog_saturated);
        assert_eq!(second.steps_to_run, 1);
    }

    #[test]
    fn fixed_step_clock_caps_large_backlogs() {
        let config = FixedStepConfig { simulation_rate_hz: 60, max_catch_up_steps: 3 };
        let mut clock = FixedStepClock::new(config).expect("clock should build");
        let long_frame = clock
            .step_duration()
            .checked_mul(10)
            .expect("ten fixed steps should fit in a Duration");
        let decision = clock.advance(long_frame);

        assert_eq!(decision.steps_to_run, 3);
        assert!(decision.backlog_saturated);
        assert_eq!(decision.residual_time, Duration::ZERO);
    }

    #[test]
    fn config_validation_rejects_zero_values() {
        let invalid_window = ClientRuntimeConfig {
            window: WindowConfig { width: 0, ..WindowConfig::default() },
            ..ClientRuntimeConfig::default()
        };
        let invalid_height = ClientRuntimeConfig {
            window: WindowConfig { height: 0, ..WindowConfig::default() },
            ..ClientRuntimeConfig::default()
        };
        let invalid_clock = FixedStepConfig { simulation_rate_hz: 0, max_catch_up_steps: 5 };

        assert_eq!(
            invalid_window.validate().expect_err("zero width should fail"),
            "window dimensions must be greater than zero"
        );
        assert_eq!(
            invalid_height.validate().expect_err("zero height should fail"),
            "window dimensions must be greater than zero"
        );
        assert_eq!(
            invalid_clock.validate().expect_err("zero rate should fail"),
            "simulation_rate_hz must be greater than zero"
        );
    }

    #[test]
    fn summary_line_reports_the_core_diagnostics() {
        let summary = ClientRunSummary {
            exit_reason: ClientExitReason::AutoCloseAfterFixedUpdates(30),
            window_mode: WindowMode::Windowed,
            simulation_rate_hz: 120,
            diagnostics: ClientDiagnostics {
                fixed_updates: 30,
                rendered_frames: 15,
                resize_events: 2,
                focus_change_events: 1,
                backlog_saturation_events: 0,
                average_render_interval: Some(Duration::from_millis(16)),
                ..ClientDiagnostics::default()
            },
            graphics_adapter: Some(GraphicsAdapterInfo {
                backend: "metal".to_owned(),
                name: "test-adapter".to_owned(),
                device_type: "integrated_gpu".to_owned(),
                driver: Some("test-driver".to_owned()),
                driver_info: Some("test-driver-info".to_owned()),
                shading_language: "wgsl".to_owned(),
            }),
        };

        let line = summary.summary_line();

        assert!(line.contains("exit=auto_close_after_fixed_updates(30)"));
        assert!(line.contains("window_mode=windowed"));
        assert!(line.contains("fixed_updates=30"));
        assert!(line.contains("rendered_frames=15"));
        assert!(line.contains("resize_events=2"));
        assert!(line.contains("focus_changes=1"));
        assert!(line.contains("avg_render_ms=16.00"));
        assert!(line.contains("backlog_saturations=0"));
        assert!(line.contains("adapter=metal:test-adapter"));
    }
}
