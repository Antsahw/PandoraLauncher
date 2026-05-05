use bridge::handle::BackendHandle;
use gpui::{prelude::*, *};
use gpui_component::{h_flex, v_flex, ActiveTheme as _};
use std::time::SystemTime;

use crate::{entity::instance::InstanceEntry, ts};

pub struct InstanceStatisticsSubpage {
    instance: Entity<InstanceEntry>,
}

impl InstanceStatisticsSubpage {
    pub fn new(
        instance: &Entity<InstanceEntry>,
        _backend_handle: BackendHandle,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> Self {
        Self {
            instance: instance.clone(),
        }
    }
}

impl Render for InstanceStatisticsSubpage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let instance = self.instance.read(cx);
        let theme = cx.theme();

        let total_secs = instance.playtime.total_secs;
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        let seconds = total_secs % 60;

        let last_played_text = instance
            .playtime
            .last_played_unix_ms
            .map(|ms| {
                let duration_since = SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d: std::time::Duration| d.as_millis() as i64 - ms)
                    .unwrap_or(0);

                if duration_since < 60000 {
                    ts!("instance.statistics.just_now").to_string()
                } else if duration_since < 3600000 {
                    format!(
                        "{} {}",
                        duration_since / 60000,
                        ts!("instance.statistics.minutes_ago")
                    )
                } else if duration_since < 86400000 {
                    format!(
                        "{} {}",
                        duration_since / 3600000,
                        ts!("instance.statistics.hours_ago")
                    )
                } else {
                    format!(
                        "{} {}",
                        duration_since / 86400000,
                        ts!("instance.statistics.days_ago")
                    )
                }
            })
            .unwrap_or_else(|| ts!("instance.statistics.never").to_string());

        v_flex()
            .size_full()
            .p_4()
            .gap_4()
            .child(
                div()
                    .text_lg()
                    .font_weight(gpui::FontWeight::BOLD)
                    .child(ts!("instance.statistics")),
            )
            .child(
                h_flex()
                    .gap_4()
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.foreground)
                                    .child(ts!("instance.statistics.playtime")),
                            )
                            .child(
                                div()
                                    .text_base()
                                    .child(format!("{}h {}m {}s", hours, minutes, seconds)),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .gap_4()
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.foreground)
                                    .child(ts!("instance.statistics.last_played")),
                            )
                            .child(div().text_base().child(last_played_text)),
                    ),
            )
    }
}
