use bridge::handle::BackendHandle;
use gpui::{prelude::*, *};
use gpui_component::{h_flex, v_flex, ActiveTheme as _};
use std::path::PathBuf;

use crate::ts;

pub struct ServerStatisticsSubpage {
    server_name: SharedString,
    _backend_handle: BackendHandle,
}

impl ServerStatisticsSubpage {
    pub fn new(
        server_name: SharedString,
        backend_handle: BackendHandle,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> Self {
        Self {
            server_name,
            _backend_handle: backend_handle,
        }
    }

    /// Load server stats from disk
    fn load_server_stats(&self) -> Option<serde_json::Value> {
        let pandora_dir = if let Ok(dir) = std::env::var("PANDORA_DIR") {
            PathBuf::from(dir)
        } else {
            let base_dirs = directories::BaseDirs::new()?;
            let data_dir = base_dirs.data_dir();
            data_dir.join("PandoraLauncher")
        };

        let stats_path = pandora_dir
            .join("servers")
            .join(self.server_name.as_str())
            .join("stats.json");

        if stats_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&stats_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    return Some(json);
                }
            }
        }

        None
    }

    /// Load server metadata from disk
    fn load_server_metadata(&self) -> Option<serde_json::Value> {
        let pandora_dir = if let Ok(dir) = std::env::var("PANDORA_DIR") {
            PathBuf::from(dir)
        } else {
            let base_dirs = directories::BaseDirs::new()?;
            let data_dir = base_dirs.data_dir();
            data_dir.join("PandoraLauncher")
        };

        let metadata_path = pandora_dir
            .join("servers")
            .join(self.server_name.as_str())
            .join("server_metadata.json");

        if metadata_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&metadata_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    return Some(json);
                }
            }
        }

        None
    }

    /// Get the server directory path
    fn get_server_path(&self) -> Option<PathBuf> {
        let pandora_dir = if let Ok(dir) = std::env::var("PANDORA_DIR") {
            PathBuf::from(dir)
        } else {
            let base_dirs = directories::BaseDirs::new()?;
            let data_dir = base_dirs.data_dir();
            data_dir.join("PandoraLauncher")
        };

        Some(
            pandora_dir
                .join("servers")
                .join(self.server_name.as_str()),
        )
    }

    /// Calculate directory size in bytes
    fn calculate_dir_size(path: &PathBuf) -> u64 {
        let mut size = 0u64;
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_dir() {
                        if let Some(path) = entry.path().to_str() {
                            size += Self::calculate_dir_size(&PathBuf::from(path));
                        }
                    } else {
                        size += metadata.len();
                    }
                }
            }
        }
        size
    }

    /// Format bytes to human readable format
    fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size > 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        format!("{:.2} {}", size, UNITS[unit_index])
    }

    /// Format seconds to human readable uptime format
    fn format_uptime(total_secs: u64) -> String {
        let days = total_secs / 86400;
        let hours = (total_secs % 86400) / 3600;
        let minutes = (total_secs % 3600) / 60;
        let seconds = total_secs % 60;

        if days > 0 {
            format!("{}d {}h {}m", days, hours, minutes)
        } else if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        }
    }
}

impl Render for ServerStatisticsSubpage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let metadata = self.load_server_metadata();
        let stats = self.load_server_stats();

        let software = metadata
            .as_ref()
            .and_then(|m| m.get("server_software"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let version = metadata
            .as_ref()
            .and_then(|m| m.get("minecraft_version"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let port = metadata
            .as_ref()
            .and_then(|m| m.get("server_port"))
            .and_then(|v| v.as_i64())
            .map(|p| p.to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let size_str = if let Some(path) = self.get_server_path() {
            Self::format_bytes(Self::calculate_dir_size(&path))
        } else {
            "Unknown".to_string()
        };

        let total_uptime_str = stats
            .as_ref()
            .and_then(|s| s.get("total_uptime_secs"))
            .and_then(|v| v.as_u64())
            .map(Self::format_uptime)
            .unwrap_or_else(|| "0s".to_string());

        let start_count = stats
            .as_ref()
            .and_then(|s| s.get("start_count"))
            .and_then(|v| v.as_u64())
            .map(|c| c.to_string())
            .unwrap_or_else(|| "0".to_string());

        let stat_card = |label: SharedString, value: SharedString| -> Div {
            v_flex()
                .gap_2()
                .px_3()
                .py_2()
                .min_w_64()
                .border_1()
                .border_color(theme.border)
                .rounded(theme.radius)
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.foreground)
                        .child(label),
                )
                .child(div().text_lg().child(value))
        };

        v_flex()
            .size_full()
            .p_4()
            .gap_4()
            .child(
                div()
                    .text_lg()
                    .font_weight(gpui::FontWeight::BOLD)
                    .child(ts!("server.statistics")),
            )
            .child(
                h_flex()
                    .gap_4()
                    .flex_wrap()
                    .child(stat_card(ts!("server.statistics.software"), SharedString::from(software)))
                    .child(stat_card(ts!("server.statistics.version"), SharedString::from(version)))
                    .child(stat_card(ts!("server.statistics.port"), SharedString::from(port)))
                    .child(stat_card(ts!("server.statistics.size"), SharedString::from(size_str))),
            )
            .child(
                h_flex()
                    .gap_4()
                    .flex_wrap()
                    .child(stat_card(ts!("server.statistics.total_uptime"), SharedString::from(total_uptime_str)))
                    .child(stat_card(ts!("server.statistics.start_count"), SharedString::from(start_count))),
            )
    }
}
