#![forbid(unsafe_code)]

use wr_core::{CrateBoundary, CrateEntryPoint};
use wr_platform::{ClientRunSummary, ClientRuntimeConfig, WindowMode};

const HELP_TEXT: &str = "Usage: cargo run -p wr_client -- [--windowed|--borderless] [--width <px>] [--height <px>] [--title <text>] [--simulation-rate-hz <hz>] [--max-catch-up-steps <count>] [--auto-close-after-fixed-updates <count>] [--auto-close-after-redraws <count>] [--smoke-test]";

pub const fn init_entrypoint() -> CrateEntryPoint {
    CrateEntryPoint::new("wr_client", CrateBoundary::AppShell, true)
}

pub const fn target_runtime() -> CrateEntryPoint {
    wr_game::init_entrypoint()
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClientCliOptions {
    pub runtime: ClientRuntimeConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientCommand {
    Run(ClientCliOptions),
    Help,
}

impl ClientCliOptions {
    pub fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut options = Self::default();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--windowed" => {
                    options.runtime.window.mode = WindowMode::Windowed;
                }
                "--borderless" => {
                    options.runtime.window.mode = WindowMode::Borderless;
                }
                "--width" => {
                    options.runtime.window.width = parse_u32(&mut args, "--width")?;
                }
                "--height" => {
                    options.runtime.window.height = parse_u32(&mut args, "--height")?;
                }
                "--title" => {
                    options.runtime.window.title = args
                        .next()
                        .ok_or_else(|| String::from("expected a value after --title"))?;
                }
                "--simulation-rate-hz" => {
                    options.runtime.fixed_step.simulation_rate_hz =
                        parse_u32(&mut args, "--simulation-rate-hz")?;
                }
                "--max-catch-up-steps" => {
                    options.runtime.fixed_step.max_catch_up_steps =
                        parse_u32(&mut args, "--max-catch-up-steps")?;
                }
                "--auto-close-after-fixed-updates" => {
                    options.runtime.auto_close_after_fixed_updates =
                        Some(parse_u64(&mut args, "--auto-close-after-fixed-updates")?);
                }
                "--auto-close-after-redraws" => {
                    options.runtime.auto_close_after_redraws =
                        Some(parse_u64(&mut args, "--auto-close-after-redraws")?);
                }
                "--smoke-test" => {
                    options.runtime.auto_close_after_fixed_updates.get_or_insert(30);
                }
                other => {
                    return Err(format!(
                        "unsupported wr_client argument `{other}`\n\n{}",
                        help_text()
                    ));
                }
            }
        }

        options.runtime.validate()?;
        Ok(options)
    }
}

pub fn help_text() -> &'static str {
    HELP_TEXT
}

pub fn parse_command(args: impl Iterator<Item = String>) -> Result<ClientCommand, String> {
    let args = args.collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        return Ok(ClientCommand::Help);
    }

    ClientCliOptions::parse(args.into_iter()).map(ClientCommand::Run)
}

pub fn run(args: impl Iterator<Item = String>) -> Result<Option<ClientRunSummary>, String> {
    match parse_command(args)? {
        ClientCommand::Run(options) => {
            let terrain_scene = wr_render_scene::canonical_hero_terrain_debug_scene()?;
            wr_platform::run_client_with_scene(
                options.runtime,
                Some(terrain_scene.scene),
                Some(terrain_scene.graph),
            )
            .map(Some)
        }
        ClientCommand::Help => Ok(None),
    }
}

fn parse_u32(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<u32, String> {
    args.next()
        .ok_or_else(|| format!("expected a value after {flag}"))?
        .parse::<u32>()
        .map_err(|error| format!("failed to parse {flag}: {error}"))
}

fn parse_u64(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<u64, String> {
    args.next()
        .ok_or_else(|| format!("expected a value after {flag}"))?
        .parse::<u64>()
        .map_err(|error| format!("failed to parse {flag}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wr_platform::{FixedStepConfig, WindowConfig};

    #[test]
    fn smoke_test_flag_sets_an_auto_close_threshold() {
        let options =
            ClientCliOptions::parse([String::from("--smoke-test")].into_iter()).expect("parse");

        assert_eq!(options.runtime.auto_close_after_fixed_updates, Some(30));
        assert_eq!(options.runtime.window.mode, WindowMode::Windowed);
    }

    #[test]
    fn parser_accepts_borderless_and_timing_overrides() {
        let options = ClientCliOptions::parse(
            [
                String::from("--borderless"),
                String::from("--simulation-rate-hz"),
                String::from("90"),
                String::from("--max-catch-up-steps"),
                String::from("8"),
                String::from("--auto-close-after-redraws"),
                String::from("12"),
            ]
            .into_iter(),
        )
        .expect("parse");

        assert_eq!(options.runtime.window.mode, WindowMode::Borderless);
        assert_eq!(
            options.runtime.fixed_step,
            FixedStepConfig { simulation_rate_hz: 90, max_catch_up_steps: 8 }
        );
        assert_eq!(options.runtime.auto_close_after_redraws, Some(12));
    }

    #[test]
    fn parser_rejects_unknown_flags() {
        let error = ClientCliOptions::parse([String::from("--mystery")].into_iter())
            .expect_err("unknown flags should fail");

        assert!(error.contains("unsupported wr_client argument `--mystery`"));
    }

    #[test]
    fn parse_command_treats_help_as_a_non_error_path() {
        let command = parse_command([String::from("--help")].into_iter()).expect("parse");

        assert_eq!(command, ClientCommand::Help);
    }

    #[test]
    fn target_runtime_stays_on_wr_game() {
        assert_eq!(target_runtime(), wr_game::init_entrypoint());
    }

    #[test]
    fn default_window_config_matches_platform_defaults() {
        let options = ClientCliOptions::default();

        assert_eq!(options.runtime.window, WindowConfig::default());
    }
}
