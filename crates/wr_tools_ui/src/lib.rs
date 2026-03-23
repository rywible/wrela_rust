#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use egui::{self, RichText};
use wr_core::{
    CrateBoundary, CrateEntryPoint, TweakError, TweakNamespace, TweakRegistry, TweakRegistryEntry,
    TweakValue,
};

pub const fn init_entrypoint() -> CrateEntryPoint {
    CrateEntryPoint::new("wr_tools_ui", CrateBoundary::Subsystem, false)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespaceSummary {
    pub namespace: TweakNamespace,
    pub label: &'static str,
    pub entry_count: usize,
    pub dirty: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TweakUiFrameOutput {
    pub changed_keys: Vec<String>,
    pub saved_path: Option<PathBuf>,
    pub reloaded_path: Option<PathBuf>,
}

impl TweakUiFrameOutput {
    pub fn is_changed(&self) -> bool {
        !self.changed_keys.is_empty() || self.saved_path.is_some() || self.reloaded_path.is_some()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TweakUiShell {
    registry: TweakRegistry,
    selected_namespace: TweakNamespace,
    pack_target: Option<PathBuf>,
    status_message: Option<String>,
}

impl Default for TweakUiShell {
    fn default() -> Self {
        Self::new(TweakRegistry::default())
    }
}

impl TweakUiShell {
    pub fn new(registry: TweakRegistry) -> Self {
        Self {
            registry,
            selected_namespace: TweakNamespace::World,
            pack_target: None,
            status_message: None,
        }
    }

    pub fn with_pack_target(registry: TweakRegistry, pack_target: impl Into<PathBuf>) -> Self {
        let mut shell = Self::new(registry);
        shell.set_pack_target(pack_target);
        shell
    }

    pub fn registry(&self) -> &TweakRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut TweakRegistry {
        &mut self.registry
    }

    pub fn selected_namespace(&self) -> TweakNamespace {
        self.selected_namespace
    }

    pub fn set_selected_namespace(&mut self, namespace: TweakNamespace) {
        self.selected_namespace = namespace;
    }

    pub fn pack_target(&self) -> Option<&Path> {
        self.pack_target.as_deref()
    }

    pub fn set_pack_target(&mut self, pack_target: impl Into<PathBuf>) {
        self.pack_target = Some(pack_target.into());
    }

    pub fn status_message(&self) -> Option<&str> {
        self.status_message.as_deref()
    }

    pub fn namespace_summaries(&self) -> Vec<NamespaceSummary> {
        TweakNamespace::ALL
            .into_iter()
            .map(|namespace| NamespaceSummary {
                namespace,
                label: namespace.label(),
                entry_count: self.registry.entries_in_namespace(namespace).len(),
                dirty: self.registry.is_namespace_dirty(namespace),
            })
            .collect()
    }

    pub fn visible_entries(&self) -> Vec<TweakRegistryEntry> {
        self.registry.entries_in_namespace(self.selected_namespace)
    }

    pub fn set_value(&mut self, key: &str, value: TweakValue) -> Result<bool, TweakError> {
        let changed = self.registry.set_value(key, value)?;
        if changed {
            self.status_message = Some(format!("Updated `{key}` live."));
        }
        Ok(changed)
    }

    pub fn apply_pack_from_path(&mut self, path: impl AsRef<Path>) -> Result<PathBuf, TweakError> {
        let path = self.registry.load_pack_from_path(path)?;
        self.pack_target = Some(path.clone());
        self.status_message = Some(format!("Loaded tweak pack from {}.", path.display()));
        Ok(path)
    }

    pub fn save_pack_to_path(&mut self, path: impl AsRef<Path>) -> Result<PathBuf, TweakError> {
        let path = self.registry.save_pack_to_path(path)?;
        self.pack_target = Some(path.clone());
        self.status_message = Some(format!("Saved tweak pack to {}.", path.display()));
        Ok(path)
    }

    pub fn save_pack_to_target(&mut self) -> Result<PathBuf, TweakError> {
        let path = self
            .pack_target
            .clone()
            .ok_or_else(|| TweakError::Io("no tweak pack target configured".to_owned()))?;
        self.save_pack_to_path(path)
    }

    pub fn reload_pack_from_target(&mut self) -> Result<PathBuf, TweakError> {
        let path = self
            .pack_target
            .clone()
            .ok_or_else(|| TweakError::Io("no tweak pack target configured".to_owned()))?;
        self.apply_pack_from_path(path)
    }

    pub fn show(&mut self, ctx: &egui::Context) -> TweakUiFrameOutput {
        let mut output =
            TweakUiFrameOutput { changed_keys: Vec::new(), saved_path: None, reloaded_path: None };

        egui::Window::new("Wrela Developer Tools").default_width(520.0).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                for summary in self.namespace_summaries() {
                    let mut label = summary.label.to_owned();
                    if summary.dirty {
                        label.push('*');
                    }
                    let tooltip = format!(
                        "{} tweaks, {}",
                        summary.entry_count,
                        if summary.dirty { "dirty" } else { "clean" }
                    );
                    let response =
                        ui.selectable_label(self.selected_namespace == summary.namespace, label);
                    if response.clicked() {
                        self.selected_namespace = summary.namespace;
                    }
                    response.on_hover_text(tooltip);
                }
            });

            ui.separator();

            ui.horizontal(|ui| {
                let can_persist = self.pack_target.is_some();
                if ui.add_enabled(can_persist, egui::Button::new("Save Pack")).clicked() {
                    match self.save_pack_to_target() {
                        Ok(path) => output.saved_path = Some(path),
                        Err(error) => {
                            self.status_message = Some(error.to_string());
                        }
                    }
                }

                if ui.add_enabled(can_persist, egui::Button::new("Reload Pack")).clicked() {
                    match self.reload_pack_from_target() {
                        Ok(path) => output.reloaded_path = Some(path),
                        Err(error) => {
                            self.status_message = Some(error.to_string());
                        }
                    }
                }

                if ui.button("Clear Dirty").clicked() {
                    self.registry.clear_dirty_namespaces();
                    self.status_message = Some("Cleared namespace dirty flags.".to_owned());
                }

                if let Some(path) = self.pack_target() {
                    ui.monospace(path.display().to_string());
                } else {
                    ui.weak("Pack target not configured");
                }
            });

            if let Some(message) = self.status_message() {
                ui.label(message);
            }

            ui.separator();

            for entry in self.visible_entries() {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        let title = if entry.namespace_dirty {
                            RichText::new(entry.label).strong()
                        } else {
                            RichText::new(entry.label)
                        };
                        ui.label(title);
                        ui.monospace(entry.key);
                    });

                    ui.small(entry.description);
                    ui.horizontal(|ui| match entry.value {
                        TweakValue::Scalar(current) => {
                            let mut candidate = current;
                            let response = ui.add(egui::DragValue::new(&mut candidate).speed(0.01));
                            if response.changed()
                                && self
                                    .set_value(entry.key, TweakValue::Scalar(candidate))
                                    .unwrap_or(false)
                            {
                                output.changed_keys.push(entry.key.to_owned());
                            }
                            ui.weak(format!("default {}", format_tweak_value(entry.default_value)));
                        }
                        TweakValue::Toggle(current) => {
                            let mut candidate = current;
                            let response = ui.checkbox(&mut candidate, "enabled");
                            if response.changed()
                                && self
                                    .set_value(entry.key, TweakValue::Toggle(candidate))
                                    .unwrap_or(false)
                            {
                                output.changed_keys.push(entry.key.to_owned());
                            }
                            ui.weak(format!("default {}", format_tweak_value(entry.default_value)));
                        }
                    });
                });
                ui.separator();
            }
        });

        output
    }
}

fn format_tweak_value(value: TweakValue) -> String {
    match value {
        TweakValue::Scalar(value) => format!("{value:.2}"),
        TweakValue::Toggle(value) => {
            if value {
                "on".to_owned()
            } else {
                "off".to_owned()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn namespace_summaries_and_visible_entries_reflect_dirty_state() {
        let mut shell = TweakUiShell::default();
        shell.set_selected_namespace(TweakNamespace::Combat);
        shell
            .set_value("combat.hitstop_scale", TweakValue::Scalar(1.3))
            .expect("combat tweak should update");

        let summaries = shell.namespace_summaries();
        let combat_summary = summaries
            .iter()
            .find(|summary| summary.namespace == TweakNamespace::Combat)
            .expect("combat summary should exist");

        assert!(combat_summary.dirty);
        assert_eq!(combat_summary.entry_count, 1);
        assert_eq!(shell.visible_entries().len(), 1);
        assert!(shell.visible_entries()[0].namespace_dirty);
    }

    #[test]
    fn shell_persists_and_reloads_pack_targets() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        let pack_path = temp.path().join("hero_forest.ron");
        let mut shell = TweakUiShell::with_pack_target(TweakRegistry::default(), &pack_path);
        shell
            .set_value("lighting.sun_warmth", TweakValue::Scalar(0.82))
            .expect("lighting tweak should update");

        let saved_path = shell.save_pack_to_target().expect("pack should save");
        let serialized = fs::read_to_string(&saved_path).expect("saved pack should be readable");

        assert!(serialized.contains("lighting.sun_warmth"));
        assert!(!serialized.contains("world.wind_strength"));

        let mut reloaded = TweakUiShell::with_pack_target(TweakRegistry::default(), &pack_path);
        let loaded_path = reloaded.reload_pack_from_target().expect("pack should reload");

        assert_eq!(loaded_path, pack_path);
        assert_eq!(
            reloaded.registry().value("lighting.sun_warmth"),
            Some(TweakValue::Scalar(0.82))
        );
        assert!(reloaded.registry().is_namespace_dirty(TweakNamespace::Lighting));
    }

    #[test]
    fn show_renders_an_egui_window() {
        let mut shell = TweakUiShell::default();
        let ctx = egui::Context::default();
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            shell.show(ctx);
        });

        assert!(!output.shapes.is_empty(), "the overlay should emit renderable egui shapes");
    }
}
