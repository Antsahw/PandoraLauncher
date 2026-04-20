// GUI Components for Java Runtime Configuration
// Place in crates/frontend/src/pages/settings/ or similar

// Example implementation for GPUI (Pandora uses GPUI for UI)

use gpui::*;
use schema::backend_config::{JavaRuntime, JavaRuntimesConfig};
use std::sync::Arc;

/// Global Java Runtimes Configuration Panel
pub struct JavaRuntimesPanel {
    runtimes: Arc<Vec<(String, JavaRuntime)>>,
    selected_runtime_index: Option<usize>,
    default_runtime: String,
    is_editing: bool,
    edit_name: String,
    edit_path: String,
    edit_version: String,
}

impl JavaRuntimesPanel {
    pub fn new(config: &JavaRuntimesConfig) -> Self {
        let mut runtimes = Vec::new();
        if let Some(rt_map) = &config.runtimes {
            for (name, runtime) in rt_map.iter() {
                runtimes.push((name.clone(), runtime.clone()));
            }
        }

        Self {
            runtimes: Arc::new(runtimes),
            selected_runtime_index: None,
            default_runtime: config.default_runtime.clone(),
            is_editing: false,
            edit_name: String::new(),
            edit_path: String::new(),
            edit_version: String::new(),
        }
    }

    pub fn render_content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let runtime_list = self.runtimes.iter().enumerate().map(|(idx, (name, runtime))| {
            let is_default = name == &self.default_runtime;
            let is_selected = self.selected_runtime_index == Some(idx);
            
            div()
                .bg_color(if is_selected { cx.theme_colors().highlight } else { cx.theme_colors().surface })
                .border(if is_default { 2 } else { 1 })
                .border_color(if is_default { cx.theme_colors().accent } else { cx.theme_colors().border })
                .p_2()
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .child(
                            div()
                                .child(Label::new(name.clone()).size(LabelSize::Large))
                                .child(Label::new(format!("{} (v{})", runtime.name, runtime.version)))
                                .child(Label::new(&runtime.path).text_grey())
                        )
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .child(if is_default { "Default ✓" } else { "" })
                                .child(if runtime.available { "✓ Available" } else { "✗ Not Found" })
                        )
                )
        });

        div()
            .child(h2().text("Java Runtimes"))
            .child(p().text("Installed Java runtimes that can be used for instances and servers"))
            .child(div().child("Runtime List:").gap_2().child(div().child(runtime_list)))
            .child(
                div()
                    .border(1)
                    .p_2()
                    .child(h3().text("Add Runtime"))
                    .child(input().placeholder("Runtime name (e.g., java21)"))
                    .child(input().placeholder("Path to java executable"))
                    .child(input().placeholder("Version number (21, 17, etc.)"))
                    .child(button().label("Add Runtime"))
            )
    }
}

/// Instance Java Runtime Settings Panel
pub struct InstanceJavaRuntimePanel {
    available_runtimes: Arc<Vec<String>>,
    use_custom: bool,
    selected_runtime: Option<String>,
}

impl InstanceJavaRuntimePanel {
    pub fn new(available_runtimes: Vec<String>, config: Option<&schema::instance::InstanceJavaRuntimeConfiguration>) -> Self {
        let (use_custom, selected_runtime) = if let Some(cfg) = config {
            (cfg.enabled, if cfg.enabled { Some(cfg.runtime_name.clone()) } else { None })
        } else {
            (false, None)
        };

        Self {
            available_runtimes: Arc::new(available_runtimes),
            use_custom,
            selected_runtime,
        }
    }

    pub fn render_content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .child(h3().text("Java Runtime"))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .child(checkbox().checked(self.use_custom))
                            .child(label().text("Use Custom Java Runtime for this Instance"))
                    )
                    .child(
                        if self.use_custom {
                            div()
                                .child(label().text("Select Runtime:"))
                                .child(
                                    select()
                                        .options(self.available_runtimes.iter().map(|r| (r.clone(), r.clone())))
                                        .selected(self.selected_runtime.clone())
                                )
                        } else {
                            div().child(label().text_grey().text("Will use global default runtime"))
                        }
                    )
            )
    }
}

/// Server Java Runtime & Memory Configuration Panel
pub struct ServerJavaConfigPanel {
    available_runtimes: Arc<Vec<String>>,
    use_custom: bool,
    selected_runtime: Option<String>,
    memory_min: u32,
    memory_max: u32,
}

impl ServerJavaConfigPanel {
    pub fn new(
        available_runtimes: Vec<String>,
        config: Option<&schema::server_config::ServerConfiguration>,
    ) -> Self {
        let (use_custom, selected_runtime, memory_min, memory_max) = if let Some(cfg) = config {
            if let Some(java_cfg) = &cfg.java {
                let (min, max) = if let Some(mem) = &java_cfg.memory {
                    (mem.min, mem.max)
                } else {
                    (512, 1024)
                };
                (java_cfg.enabled, if java_cfg.enabled { Some(java_cfg.runtime_name.clone()) } else { None }, min, max)
            } else {
                (false, None, 512, 1024)
            }
        } else {
            (false, None, 512, 1024)
        };

        Self {
            available_runtimes: Arc::new(available_runtimes),
            use_custom,
            selected_runtime,
            memory_min,
            memory_max,
        }
    }

    pub fn render_content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .child(h3().text("Java Configuration"))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .child(checkbox().checked(self.use_custom))
                            .child(label().text("Use Custom Java Runtime for this Server"))
                    )
                    .child(
                        if self.use_custom {
                            div()
                                .child(label().text("Select Runtime:"))
                                .child(
                                    select()
                                        .options(self.available_runtimes.iter().map(|r| (r.clone(), r.clone())))
                                        .selected(self.selected_runtime.clone())
                                )
                        } else {
                            div().child(label().text_grey().text("Will use global default runtime"))
                        }
                    )
                    .child(
                        div()
                            .child(label().text("Minimum Memory (MB):"))
                            .child(slider().min(256).max(8192).value(self.memory_min as f32))
                    )
                    .child(
                        div()
                            .child(label().text("Maximum Memory (MB):"))
                            .child(slider().min(256).max(8192).value(self.memory_max as f32))
                    )
            )
    }
}

// --- Example Status Display ---

pub struct JavaRuntimeStatus;

impl JavaRuntimeStatus {
    pub fn render(runtime_name: &str, java_path: &str, is_available: bool) -> impl IntoElement {
        div()
            .border(1)
            .rounded(4)
            .p_2()
            .child(
                if is_available {
                    div().text_green().text(format!("✓ {} - {}", runtime_name, java_path))
                } else {
                    div().text_red().text(format!("✗ {} - NOT FOUND", runtime_name))
                }
            )
    }
}
