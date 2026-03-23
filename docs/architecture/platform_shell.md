# Platform Shell

`PR-006` introduces the first native windowed client shell for Wrela v0.

## Purpose

The bootstrap client exists to prove three contracts before rendering and gameplay integration get more complex:

1. the repo can open and close a native client window on the target Mac machine,
2. OS events are translated into engine actions instead of leaking raw key codes through the runtime,
3. fixed-step simulation pacing stays independent from redraw cadence.

This is intentionally still a bootstrap shell. It owns window lifecycle, action translation, and timing diagnostics, but it does not yet render the real scene or host the later ECS/gameplay runtime.

## Current architecture

The current path is:

`winit window events -> wr_platform action map -> fixed-step clock -> bootstrap client diagnostics`

`wr_platform` owns:

- window configuration and mode selection,
- keyboard/mouse/gamepad action bindings,
- edge-triggered action tracking,
- fixed-step catch-up logic,
- frame-pacing diagnostics and smoke-friendly auto-close support.

`wr_client` stays thin and only:

- parses CLI flags,
- selects window mode / smoke flags,
- launches the platform runtime,
- prints a concise exit summary.

## Action layer

The bootstrap action map converts supported inputs into engine actions such as movement, jump, dash, attack, parry, and developer-overlay toggle.

That keeps later gameplay tasks aligned with the repo rule that input should flow through named actions rather than direct platform key codes.

Gamepad wiring is currently contract-level only: the shared input map supports gamepad bindings for tests and future integration, while the live `winit` bridge currently feeds keyboard and mouse events.

## Fixed-step pacing

The default simulation rate is `120 Hz` with bounded catch-up, while redraw requests are paced separately at a lightweight `60 Hz` bootstrap cap.

The clock accumulates elapsed wall time, runs a whole number of fixed updates, and caps the backlog to a configured maximum step count per pump so the client cannot spiral forever after a stall.

Current diagnostics track:

- rendered frame count,
- fixed update count,
- maximum fixed updates processed in one pump,
- backlog saturation count,
- resize count,
- focus-change count,
- average and last render interval.

The bootstrap shell still samples `Instant::now()` at the platform boundary for pacing and diagnostics. That is intentionally kept local to `wr_platform`, but it should be replaced with an injectable time source before replay-oriented client work lands.

## Manual smoke checklist

Run these from the repo root on the Mac dev machine:

1. `cargo run -p wr_client -- --smoke-test`
   Expect a window to open in windowed mode, run briefly, auto-close after 30 fixed updates, and print an exit summary that includes `auto_close_after_fixed_updates(30)`.
2. `cargo run -p wr_client -- --borderless --auto-close-after-fixed-updates 30`
   Expect a borderless window to open and auto-close with the same fixed-step summary.
3. `cargo run -p wr_client -- --auto-close-after-fixed-updates 240`
   While it is open, resize the window and alt-tab away and back once.
   Expect the final summary line to report at least one resize event and focus-change event.

These checks are intentionally lightweight until the render backend lands. The important proof here is window lifecycle, action/timing wiring, and observable diagnostics.
