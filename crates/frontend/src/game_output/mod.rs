use std::{cell::RefCell, num::NonZeroUsize, ops::Range, rc::Rc, sync::Arc};

use ftree::FenwickTree;
use gpui::{prelude::*, *};
use gpui_component::{
    button::Button, h_flex, input::{Input, InputEvent, InputState}, scroll::{Scrollbar, ScrollbarHandle}, v_flex, ActiveTheme as _, Icon, Sizable, Disableable
};
use lru::LruCache;
use rustc_hash::FxBuildHasher;
use open;

use bridge::{game_output::GameOutputLogLevel, handle::BackendHandle, instance::{InstanceID, InstanceStatus}, keep_alive::{KeepAlive, KeepAliveHandle}, message::MessageToBackend};

use crate::{CloseWindow, icon::PandoraIcon, interface_config::InterfaceConfig, ts};

struct CachedShapedLogLevels {
    fatal: Arc<ShapedLine>,
    error: Arc<ShapedLine>,
    warn: Arc<ShapedLine>,
    info: Arc<ShapedLine>,
    debug: Arc<ShapedLine>,
    trace: Arc<ShapedLine>,
    other: Arc<ShapedLine>,
}

struct CachedShapedLines {
    last_time: Option<Arc<ShapedLine>>,
    last_time_millis: i64,

    item_lines: LruCache<usize, WrappedLines, FxBuildHasher>,
}

pub struct GameOutputItemState {
    items: Vec<GameOutputItem>,
    last_scrolled_item: usize,
    item_sizes: FenwickTree<usize>,
    total_line_count: usize,
    cached_shaped_lines: CachedShapedLines,
    search_query: SharedString,
}

pub struct GameOutput {
    font: Font,
    pub scroll_state: Rc<RefCell<GameOutputScrollState>>,
    pending: Vec<(i64, GameOutputLogLevel, Arc<[Arc<str>]>)>,
    item_state: Option<GameOutputItemState>,
    time_column_width: Pixels,
    level_column_width: Pixels,
    shaped_log_levels: Option<CachedShapedLogLevels>,
}

impl Default for GameOutput {
    fn default() -> Self {
        Self {
            font: Font {
                family: SharedString::new_static("Roboto Mono"),
                features: FontFeatures::default(),
                fallbacks: None,
                weight: FontWeight::NORMAL,
                style: FontStyle::Normal,
            },
            scroll_state: Default::default(),
            pending: Default::default(),
            item_state: Some(GameOutputItemState {
                items: Vec::new(),
                last_scrolled_item: 0,
                item_sizes: FenwickTree::new(),
                total_line_count: 0,
                cached_shaped_lines: CachedShapedLines {
                    last_time: None,
                    last_time_millis: 0,
                    item_lines: LruCache::with_hasher(NonZeroUsize::new(256).unwrap(), FxBuildHasher),
                },
                search_query: SharedString::new_static(""),
            }),
            time_column_width: Default::default(),
            level_column_width: Default::default(),
            shaped_log_levels: None,
        }
    }
}

impl GameOutput {
    pub fn add(&mut self, time: i64, level: GameOutputLogLevel, text: Arc<[Arc<str>]>) {
        self.pending.push((time, level, text));
    }

    pub fn clear(&mut self) {
        self.pending.clear();
        if let Some(item_state) = &mut self.item_state {
            item_state.items.clear();
            item_state.item_sizes = FenwickTree::new();
            item_state.total_line_count = 0;
        }
    }

    pub fn collect_logs(&self) -> String {
        let mut output = String::new();
        
        if let Some(item_state) = &self.item_state {
            for item in &item_state.items {
                if !item.skip {
                    for line in item.text.iter() {
                        output.push_str(line);
                        output.push('\n');
                    }
                }
            }
        }
        
        output
    }

    fn shape_log_level(
        &self,
        level: &'static str,
        color: Hsla,
        text_system: &Arc<WindowTextSystem>,
        text_style: &TextStyle,
        font_size: Pixels,
    ) -> Arc<ShapedLine> {
        let level_run = TextRun {
            len: level.len(),
            font: self.font.clone(),
            color,
            background_color: text_style.background_color,
            underline: text_style.underline,
            strikethrough: text_style.strikethrough,
        };
        Arc::new(text_system.shape_line(SharedString::new_static(level), font_size, &[level_run], None))
    }

    pub fn apply_pending(&mut self, window: &mut Window, _cx: &mut App) {
        if self.shaped_log_levels.is_none() {
            let text_style = window.text_style();
            let font_size = text_style.font_size.to_pixels(window.rem_size());
            let text_system = window.text_system();

            let levels = CachedShapedLogLevels {
                fatal: self.shape_log_level("FATAL", hsla(0.0, 0.737, 0.418, 1.0), text_system, &text_style, font_size), // red-700
                error: self.shape_log_level("ERROR", hsla(0.0, 0.842, 0.602, 1.0), text_system, &text_style, font_size), // red-500
                warn: self.shape_log_level("WARN", hsla(24.6/360.0, 0.95, 0.531, 1.0), text_system, &text_style, font_size), // orange-500
                info: self.shape_log_level("INFO", hsla(83.7/360.0, 0.805, 0.443, 1.0), text_system, &text_style, font_size), // lime-500
                debug: self.shape_log_level("DEBUG", hsla(258.3/360.0, 0.895, 0.663, 1.0), text_system, &text_style, font_size), // violet-500
                trace: self.shape_log_level("TRACE", hsla(198.6/360.0, 0.887, 0.484, 1.0), text_system, &text_style, font_size), // sky-500
                other: self.shape_log_level("OTHER", hsla(0.0, 0.5, 0.5, 1.0), text_system, &text_style, font_size),
            };

            self.level_column_width = levels.fatal.width.max(levels.error.width).max(levels.warn.width)
                .max(levels.info.width).max(levels.debug.width).max(levels.trace.width).max(levels.other.width) + font_size/2.0;
            self.shaped_log_levels = Some(levels);
        }
        let Some(item_state) = &mut self.item_state else {
            return;
        };
        for (time, level, text) in self.pending.drain(..) {
            let shaped_level = match level {
                GameOutputLogLevel::Fatal => self.shaped_log_levels.as_ref().unwrap().fatal.clone(),
                GameOutputLogLevel::Error => self.shaped_log_levels.as_ref().unwrap().error.clone(),
                GameOutputLogLevel::Warn => self.shaped_log_levels.as_ref().unwrap().warn.clone(),
                GameOutputLogLevel::Info => self.shaped_log_levels.as_ref().unwrap().info.clone(),
                GameOutputLogLevel::Debug => self.shaped_log_levels.as_ref().unwrap().debug.clone(),
                GameOutputLogLevel::Trace => self.shaped_log_levels.as_ref().unwrap().trace.clone(),
                GameOutputLogLevel::Other => self.shaped_log_levels.as_ref().unwrap().other.clone(),
            };

            let mut highlighted_text = None;

            if !item_state.search_query.is_empty() {
                for (line_index, line) in text.iter().enumerate() {
                    if let Some(found) = line.find(item_state.search_query.as_str()) {
                        highlighted_text = Some((line_index, found..found+item_state.search_query.as_str().len()));
                        break;
                    }
                }
                if highlighted_text.is_none() {
                    // Item doesn't match search query, push skipped item
                    let backup_total_lines_while_skipped = text.len();
                    item_state.item_sizes.push(0);
                    item_state.items.push(GameOutputItem {
                        time: TimeShapedLine::Timestamp(time),
                        level: shaped_level.clone(),
                        text: text.clone(),
                        index: item_state.items.len(),
                        backup_total_lines_while_skipped,
                        total_lines: 0,
                        highlighted_text: None,
                        skip: true,
                    });
                    continue;
                }
            }

            let total_lines = text.len();
            item_state.item_sizes.push(total_lines);
            item_state.total_line_count += total_lines;
            item_state.items.push(GameOutputItem {
                time: TimeShapedLine::Timestamp(time),
                level: shaped_level.clone(),
                text: text.clone(),
                index: item_state.items.len(),
                backup_total_lines_while_skipped: total_lines,
                total_lines,
                highlighted_text,
                skip: false,
            });
        }
    }
}

pub struct GameOutputList {
    interactivity: Interactivity,
    game_output: Entity<GameOutput>,
}

enum TimeShapedLine {
    Timestamp(i64),
    Shaped(Arc<ShapedLine>),
}

struct GameOutputItem {
    time: TimeShapedLine,
    level: Arc<ShapedLine>,

    text: Arc<[Arc<str>]>,
    index: usize,
    backup_total_lines_while_skipped: usize,
    total_lines: usize,
    highlighted_text: Option<(usize, Range<usize>)>,
    skip: bool,
}

impl GameOutputItem {
    pub fn compute_wrapped_text<'a>(
        &mut self,
        wrap_width: Pixels,
        text_system: &Arc<WindowTextSystem>,
        font: &Font,
        font_size: Pixels,
        text_style: &TextStyle,
        line_wrapper: &mut LineWrapperHandle,
        cache: &'a mut CachedShapedLines,
    ) -> &'a [ShapedLine] {
        let mut recompute = true;

        if let Some(last_wrapped) = cache.item_lines.get(&self.index)
            && (last_wrapped.wrap_width == wrap_width || (last_wrapped.lines.len() == 1 && last_wrapped.lines.first().unwrap().width < wrap_width)) {
                recompute = false;
            }

        if recompute {
            let mut wrapped = Vec::new();
            for (original_line_index, line) in self.text.iter().enumerate() {
                let fragments = [LineFragment::Text { text: line }];
                let boundaries = line_wrapper.wrap_line(&fragments, wrap_width);

                let mut handle_segment = |wrapped_line: SharedString, from, to| {
                    let runs: &[TextRun] = if let Some((highlight_line, highlight_range)) = &self.highlighted_text
                        && *highlight_line == original_line_index
                        && highlight_range.start < to
                        && highlight_range.end > from
                    {
                        let highlight_start = highlight_range.start.max(from);
                        let highlight_end = highlight_range.end.min(to);

                        &[
                            TextRun {
                                len: highlight_start - from,
                                font: font.clone(),
                                color: text_style.color,
                                background_color: text_style.background_color,
                                underline: text_style.underline,
                                strikethrough: text_style.strikethrough,
                            },
                            TextRun {
                                len: highlight_end - highlight_start,
                                font: font.clone(),
                                color: gpui::black(),
                                background_color: Some(gpui::yellow()),
                                underline: text_style.underline,
                                strikethrough: text_style.strikethrough,
                            },
                            TextRun {
                                len: to - highlight_end,
                                font: font.clone(),
                                color: text_style.color,
                                background_color: text_style.background_color,
                                underline: text_style.underline,
                                strikethrough: text_style.strikethrough,
                            },
                        ]
                    } else {
                        &[TextRun {
                            len: wrapped_line.len(),
                            font: font.clone(),
                            color: text_style.color,
                            background_color: text_style.background_color,
                            underline: text_style.underline,
                            strikethrough: text_style.strikethrough,
                        }]
                    };

                    let shaped = text_system.shape_line(wrapped_line, font_size, runs, None);
                    wrapped.push(shaped);
                };

                let mut last_boundary_ix = 0;
                for boundary in boundaries {
                    let wrapped_line = &line[last_boundary_ix..boundary.ix];
                    let wrapped_line = SharedString::new(wrapped_line);
                    (handle_segment)(wrapped_line, last_boundary_ix, boundary.ix);
                    last_boundary_ix = boundary.ix;
                }

                // Push last segment
                let wrapped_line = if last_boundary_ix == 0 {
                    line.into()
                } else {
                    SharedString::new(&line[last_boundary_ix..])
                };
                (handle_segment)(wrapped_line, last_boundary_ix, line.len());
            }

            cache.item_lines.put(
                self.index,
                WrappedLines {
                    wrap_width,
                    lines: wrapped,
                },
            );
        }

        cache.item_lines.get(&self.index).unwrap().lines.as_slice()
    }
}

struct WrappedLines {
    wrap_width: Pixels,
    lines: Vec<ShapedLine>,
}

impl InteractiveElement for GameOutputList {
    fn interactivity(&mut self) -> &mut Interactivity {
        &mut self.interactivity
    }
}

impl IntoElement for GameOutputList {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for GameOutputList {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let layout_id = self.interactivity.request_layout(global_id, inspector_id, window, cx, |mut style, window, cx| {
            style.size.width = relative(1.0).into();
            style.size.height = relative(1.0).into();
            window.request_layout(style, None, cx)
        });
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.interactivity.prepaint(
            global_id,
            inspector_id,
            bounds,
            bounds.size,
            window,
            cx,
            |_, _, _, _, _| {}
        )
    }

    fn paint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            self.interactivity.paint(
                global_id,
                inspector_id,
                bounds,
                None,
                window,
                cx,
                |_, window, cx| {
                    let visible_bounds = bounds;
                    let mut bounds = bounds.inset(px(12.0));
                    bounds.size.width += px(12.0);

                    cx.update_entity(&self.game_output, |game_output, cx| {
                        game_output.apply_pending(window, cx);

                        let text_style = window.text_style();

                        let font_size = text_style.font_size.to_pixels(window.rem_size());
                        let line_height = font_size * 1.25;

                        let text_width = bounds.size.width
                            - game_output.time_column_width
                            - game_output.level_column_width;
                        let wrap_width = text_width.max(font_size * 30);

                        let mut line_wrapper = window.text_system().line_wrapper(game_output.font.clone(), font_size);

                        let scroll_render_info = game_output.update_scrolling(line_height, wrap_width,
                            font_size, &text_style, &mut line_wrapper, window.text_system());

                        if let Some(item_state) = game_output.item_state.as_mut() && !item_state.items.is_empty() {
                            if scroll_render_info.reverse {
                                paint_lines::<true>(
                                    item_state.items[..scroll_render_info.item+1].iter_mut().rev(),
                                    visible_bounds,
                                    bounds,
                                    scroll_render_info.offset,
                                    &game_output.font,
                                    &text_style,
                                    wrap_width,
                                    font_size,
                                    line_height,
                                    &mut game_output.time_column_width,
                                    game_output.level_column_width,
                                    &mut item_state.item_sizes,
                                    &mut item_state.total_line_count,
                                    &mut line_wrapper,
                                    &mut item_state.cached_shaped_lines,
                                    window,
                                    cx,
                                );
                            } else {
                                paint_lines::<false>(
                                    item_state.items[scroll_render_info.item..].iter_mut(),
                                    visible_bounds,
                                    bounds,
                                    scroll_render_info.offset,
                                    &game_output.font,
                                    &text_style,
                                    wrap_width,
                                    font_size,
                                    line_height,
                                    &mut game_output.time_column_width,
                                    game_output.level_column_width,
                                    &mut item_state.item_sizes,
                                    &mut item_state.total_line_count,
                                    &mut line_wrapper,
                                    &mut item_state.cached_shaped_lines,
                                    window,
                                    cx,
                                );
                            }
                        }

                        let mut scroll_state = game_output.scroll_state.borrow_mut();
                        scroll_state.bounds_y = bounds.size.height;
                        scroll_state.line_height = line_height;
                        scroll_state.lines = if let Some(item_state) = &game_output.item_state {
                            item_state.total_line_count
                        } else {
                            0
                        };
                    });
                });
        });
    }
}

#[derive(Debug)]
struct ScrollRenderInfo {
    item: usize,
    reverse: bool,
    offset: Pixels,
}

impl GameOutput {
    fn update_scrolling(
        &mut self,
        line_height: Pixels,
        wrap_width: Pixels,
        font_size: Pixels,
        text_style: &TextStyle,
        line_wrapper: &mut LineWrapperHandle,
        text_system: &Arc<WindowTextSystem>,
    ) -> ScrollRenderInfo {
        let mut scroll_state = self.scroll_state.borrow_mut();

        let Some(item_state) = self.item_state.as_mut() else {
            scroll_state.scrolling = GameOutputScrolling::Bottom;
            return ScrollRenderInfo {
                item: 0,
                reverse: true,
                offset: Pixels::ZERO,
            };
        };

        if item_state.items.is_empty() {
            scroll_state.scrolling = GameOutputScrolling::Bottom;
            item_state.last_scrolled_item = 0;
            return ScrollRenderInfo {
                item: 0,
                reverse: false,
                offset: Pixels::ZERO,
            };
        }

        let max_offset = (item_state.total_line_count * line_height - scroll_state.bounds_y).max(px(1.0));

        match &mut scroll_state.scrolling {
            GameOutputScrolling::Bottom => {
                if let Some(active_drag) = &mut scroll_state.active_drag {
                    active_drag.actual_offset = -max_offset;
                }
                item_state.last_scrolled_item = item_state.items.len().saturating_sub(1);
                ScrollRenderInfo {
                    item: item_state.items.len().saturating_sub(1),
                    reverse: true,
                    offset: Pixels::ZERO,
                }
            },
            GameOutputScrolling::Top { offset } => {
                let mut offset = *offset;

                for check_scrolled_items in [true, false] {
                    let mut effective_offset = offset;

                    if offset <= -max_offset {
                        scroll_state.scrolling = GameOutputScrolling::Bottom;
                        if let Some(active_drag) = &mut scroll_state.active_drag {
                            active_drag.actual_offset = -max_offset;
                        }
                        item_state.last_scrolled_item = item_state.items.len().saturating_sub(1);
                        return ScrollRenderInfo {
                            item: item_state.items.len().saturating_sub(1),
                            reverse: true,
                            offset: Pixels::ZERO,
                        };
                    }

                    if offset < px(-1.0)
                        && let Some(active_drag) = &scroll_state.active_drag
                    {
                        let drag_pivot = active_drag.drag_pivot.min(Pixels::ZERO);
                        let real_pivot = active_drag.real_pivot.min(Pixels::ZERO);
                        let new_max_offset =
                            (item_state.total_line_count * line_height - scroll_state.bounds_y).max(px(1.0));
                        let old_max_offset = (active_drag.start_content_height - scroll_state.bounds_y).max(px(1.0));

                        if offset < drag_pivot {
                            effective_offset = (offset - drag_pivot) / (-old_max_offset - drag_pivot)
                                * (-new_max_offset - real_pivot)
                                + real_pivot;
                        } else {
                            effective_offset = offset / drag_pivot * real_pivot;
                        }
                    }

                    if let Some(active_drag) = &mut scroll_state.active_drag {
                        active_drag.actual_offset = effective_offset;
                    }

                    let top = (-effective_offset).max(Pixels::ZERO);
                    let top_offset_for_inset = line_height.min(top);
                    let top = top - top_offset_for_inset;

                    let top_line = (top / line_height) as usize;
                    let line_remainder = top_line * line_height - top;

                    let (item_index, remainder_lines) = item_state.item_sizes.index_of_with_remainder(top_line + 1);

                    if check_scrolled_items && item_index < item_state.last_scrolled_item {
                        let mut resized_above = Pixels::ZERO;
                        let mut changed = false;
                        let from = item_index.max(item_state.last_scrolled_item.saturating_sub(32));
                        for item in item_state.items[from..item_state.last_scrolled_item].iter_mut() {
                            if item.skip {
                                continue;
                            }
                            let lines = item.compute_wrapped_text(
                                wrap_width,
                                text_system,
                                &self.font,
                                font_size,
                                text_style,
                                line_wrapper,
                                &mut item_state.cached_shaped_lines,
                            );
                            let line_count = lines.len().max(1);
                            if line_count != item.total_lines {
                                resized_above += line_count * line_height - item.total_lines * line_height;
                                if item.total_lines < line_count {
                                    item_state.item_sizes.add_at(item.index, line_count - item.total_lines);
                                    item_state.total_line_count += line_count - item.total_lines;
                                } else {
                                    item_state.item_sizes.sub_at(item.index, item.total_lines - line_count);
                                    item_state.total_line_count -= item.total_lines - line_count;
                                }
                                item.total_lines = line_count;
                                changed = true;
                            }
                        }
                        if changed {
                            if let Some(active_drag) = &mut scroll_state.active_drag {
                                active_drag.drag_pivot = offset;
                                active_drag.real_pivot = effective_offset - resized_above;
                            } else {
                                offset -= resized_above;
                                if let GameOutputScrolling::Top { offset } = &mut scroll_state.scrolling {
                                    *offset -= resized_above;
                                }
                            }
                            continue;
                        }
                    }

                    let render_offset = -(remainder_lines * line_height) + line_remainder + line_height - top_offset_for_inset;

                    if scroll_state.active_drag.is_some() {
                        let mut remaining_lines = ((scroll_state.bounds_y - render_offset) / line_height) as usize + 1;
                        let mut changed = false;
                        for item in item_state.items[item_index..].iter_mut() {
                            if item.skip {
                                continue;
                            }
                            let lines = item.compute_wrapped_text(
                                wrap_width,
                                text_system,
                                &self.font,
                                font_size,
                                text_style,
                                line_wrapper,
                                &mut item_state.cached_shaped_lines,
                            );
                            let line_count = lines.len().max(1);
                            if line_count != item.total_lines {
                                if item.total_lines < line_count {
                                    item_state.item_sizes.add_at(item.index, line_count - item.total_lines);
                                    item_state.total_line_count += line_count - item.total_lines;
                                } else {
                                    item_state.item_sizes.sub_at(item.index, item.total_lines - line_count);
                                    item_state.total_line_count -= item.total_lines - line_count;
                                }
                                item.total_lines = line_count;
                                changed = true;
                            }
                            remaining_lines = remaining_lines.saturating_sub(line_count);
                            if remaining_lines == 0 {
                                break;
                            }
                        }
                        if changed && let Some(active_drag) = &mut scroll_state.active_drag {
                            active_drag.drag_pivot = offset;
                            active_drag.real_pivot = effective_offset;
                        }
                    }

                    item_state.last_scrolled_item = item_index;
                    return ScrollRenderInfo {
                        item: item_index,
                        reverse: false,
                        offset: render_offset,
                    };
                }
                unreachable!();
            },
        }
    }
}

fn paint_lines<'a, const REVERSE: bool>(
    items: impl Iterator<Item = &'a mut GameOutputItem>,
    visible_bounds: Bounds<Pixels>,
    bounds: Bounds<Pixels>,
    offset: Pixels,
    font: &Font,
    text_style: &TextStyle,
    wrap_width: Pixels,
    font_size: Pixels,
    line_height: Pixels,
    time_column_width: &mut Pixels,
    level_column_width: Pixels,
    item_sizes: &mut FenwickTree<usize>,
    total_line_count: &mut usize,
    line_wrapper: &mut LineWrapperHandle,
    cache: &mut CachedShapedLines,
    window: &mut Window,
    cx: &mut App,
) {
    let mut text_origin = bounds.origin;
    if REVERSE {
        text_origin.y += bounds.size.height;
        text_origin.y -= line_height;
    }
    text_origin.y += offset;

    for item in items {
        if item.skip {
            continue;
        }
        let has_highlighted_text = item.highlighted_text.is_some();

        let lines = item.compute_wrapped_text(
            wrap_width,
            window.text_system(),
            font,
            font_size,
            text_style,
            line_wrapper,
            cache,
        );

        let line_count = lines.len().max(1);

        /*
        let item_bounds = Bounds {
            origin: if REVERSE {
                let mut item_origin = line_origin.clone();
                item_origin.y -= (line_count - 1) * line_height;
                item_origin
            } else {
                line_origin
            },
            size: Size::new(wrap_width, line_count * line_height),
        };
        let item_background_color = if item.index & 1 == 0 {
            Hsla { h: 0.0, s: 0.0, l: 0.06, a: 0.5 }
        } else {
            Hsla { h: 0.0, s: 0.0, l: 0.12, a: 0.5 }
        };
        window.paint_quad(fill(item_bounds,item_background_color));
        */

        let mut line_origin = text_origin;
        line_origin.x += *time_column_width + level_column_width;
        if REVERSE {
            for shaped in lines.iter().rev() {
                if line_origin.y <= visible_bounds.origin.y + visible_bounds.size.height {
                    if has_highlighted_text {
                        _ = shaped.paint_background(line_origin, line_height, TextAlign::Left, None, window, cx);
                    }
                    _ = shaped.paint(line_origin, line_height, TextAlign::Left, None, window, cx);
                }
                line_origin.y -= line_height;
            }
        } else {
            for shaped in lines.iter() {
                if line_origin.y >= visible_bounds.origin.y - line_height {
                    if has_highlighted_text {
                        _ = shaped.paint_background(line_origin, line_height, TextAlign::Left, None, window, cx);
                    }
                    _ = shaped.paint(line_origin, line_height, TextAlign::Left, None, window, cx);
                }
                line_origin.y += line_height;
            }
        }

        // Shape time text if needed
        if let TimeShapedLine::Timestamp(timestamp) = item.time {
            if let Some(last_shaped_time) = &cache.last_time && cache.last_time_millis == timestamp {
                item.time = TimeShapedLine::Shaped(Arc::clone(last_shaped_time));
            } else {
                let date_time = chrono::DateTime::from_timestamp_millis(timestamp).unwrap().with_timezone(&chrono::Local);
                let time = format!("{}", date_time.time().format("%H:%M:%S%.3f"));
                let time_run = TextRun {
                    len: time.len(),
                    font: font.clone(),
                    color: text_style.color,
                    background_color: text_style.background_color,
                    underline: text_style.underline,
                    strikethrough: text_style.strikethrough,
                };
                let shaped_time = Arc::new(window.text_system().shape_line(time.into(), font_size, &[time_run], None));

                item.time = TimeShapedLine::Shaped(Arc::clone(&shaped_time));

                *time_column_width = (*time_column_width).max(shaped_time.width + font_size / 2.0);

                cache.last_time = Some(shaped_time);
                cache.last_time_millis = timestamp;
            }
        }

        // Render time text
        let mut time_origin = text_origin;
        if REVERSE {
            time_origin.y -= (line_count - 1) * line_height;
        }
        if let TimeShapedLine::Shaped(shaped_time) = &item.time {
            _ = shaped_time.paint(time_origin, line_height, TextAlign::Left, None, window, cx);
        }

        let mut level_origin = time_origin;
        level_origin.x += *time_column_width + level_column_width - item.level.width - font_size/2.0;
        _ = item.level.paint(level_origin, line_height, TextAlign::Left, None, window, cx);

        if line_count != item.total_lines {
            if item.total_lines < line_count {
                item_sizes.add_at(item.index, line_count - item.total_lines);
                *total_line_count += line_count - item.total_lines;
            } else {
                item_sizes.sub_at(item.index, item.total_lines - line_count);
                *total_line_count -= item.total_lines - line_count;
            }
            item.total_lines = line_count;
        }

        if REVERSE {
            text_origin.y -= line_count * line_height;
            if text_origin.y < visible_bounds.origin.y - line_height {
                break;
            }
        } else {
            text_origin.y += line_count * line_height;
            if text_origin.y > visible_bounds.origin.y + visible_bounds.size.height {
                break;
            }
        }
    }
}

pub struct GameOutputRoot {
    pub scroll_handler: ScrollHandler,
    _keep_alive: KeepAlive,
    pub game_output: Entity<GameOutput>,
    search_state: Entity<InputState>,
    server_command_state: Option<Entity<InputState>>,
    _search_task: Task<()>,
    _search_input_subscription: Subscription,
    _server_command_subscription: Option<Subscription>,
    focus_handle: FocusHandle,
    // Tabbed mode fields
    is_tabbed: bool,
    pub active_instance_id: Option<usize>,
    pub tabs: std::collections::HashMap<usize, Entity<GameOutput>>,
    pub instance_names: std::collections::HashMap<usize, SharedString>,
    pub instance_folders: std::collections::HashMap<usize, Arc<std::path::Path>>,
    pub dot_minecraft_folders: std::collections::HashMap<usize, Arc<std::path::Path>>,
    pub instance_keep_alives: std::collections::HashMap<usize, KeepAliveHandle>,
    instance_ids: std::collections::HashMap<usize, InstanceID>,
    instance_statuses: std::collections::HashMap<InstanceID, InstanceStatus>,
    server_names: std::collections::HashMap<usize, String>,
    server_folders: std::collections::HashMap<usize, Arc<std::path::Path>>,
    server_statuses: std::collections::HashMap<String, bool>, // Track if servers are running
    backend_handle: BackendHandle,
}

#[derive(Clone)]
pub struct ScrollHandler {
    pub state: Rc<RefCell<GameOutputScrollState>>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ActiveDrag {
    start_content_height: Pixels,
    drag_pivot: Pixels,
    real_pivot: Pixels,
    actual_offset: Pixels,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GameOutputScrollState {
    lines: usize,
    line_height: Pixels,
    bounds_y: Pixels,
    scrolling: GameOutputScrolling,
    active_drag: Option<ActiveDrag>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum GameOutputScrolling {
    #[default]
    Bottom,
    Top {
        offset: Pixels,
    },
}

impl GameOutputScrollState {
    pub fn content_height_for_scrollbar(&self) -> Pixels {
        self.active_drag
            .as_ref()
            .map(|v| v.start_content_height)
            .unwrap_or(self.lines * self.line_height)
    }

    pub fn max_scroll_amount(&self) -> Pixels {
        (self.lines * self.line_height - self.bounds_y).max(Pixels::ZERO)
    }

    pub fn offset(&self) -> Pixels {
        match self.scrolling {
            GameOutputScrolling::Bottom => {
                let content_height = self.content_height_for_scrollbar();
                -(content_height - self.bounds_y)
            },
            GameOutputScrolling::Top { offset } => offset,
        }
    }

    pub fn set_offset(&mut self, new_offset: Pixels) {
        let content_height = self.content_height_for_scrollbar();
        let new_offset = new_offset.min(Pixels::ZERO);
        let total_offset = -(content_height - self.bounds_y);

        if new_offset < total_offset + self.line_height / 4.0 {
            self.scrolling = GameOutputScrolling::Bottom;
        } else {
            self.scrolling = GameOutputScrolling::Top { offset: new_offset };
        }
    }
}

impl ScrollbarHandle for ScrollHandler {
    fn offset(&self) -> Point<Pixels> {
        let state = self.state.borrow();
        Point::new(Pixels::ZERO, state.offset())
    }

    fn set_offset(&self, new_offset: Point<Pixels>) {
        let mut state = self.state.borrow_mut();
        state.set_offset(new_offset.y);
    }

    fn content_size(&self) -> Size<Pixels> {
        let state = self.state.borrow();
        let content_height = state.content_height_for_scrollbar();
        Size::new(Pixels::ZERO, content_height)
    }

    fn start_drag(&self) {
        let mut state = self.state.borrow_mut();
        state.active_drag = Some(ActiveDrag {
            start_content_height: state.lines * state.line_height,
            drag_pivot: Pixels::ZERO,
            real_pivot: Pixels::ZERO,
            actual_offset: state.offset(),
        });
    }

    fn end_drag(&self) {
        let mut state = self.state.borrow_mut();
        if let Some(drag) = state.active_drag.take() {
            state.set_offset(drag.actual_offset);
        }
    }
}

impl GameOutputRoot {
    pub fn new(
        keep_alive: KeepAlive,
        game_output: Entity<GameOutput>,
        backend_handle: BackendHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let scroll_state = Rc::clone(&game_output.read(cx).scroll_state);

        let search_state = cx.new(|cx| InputState::new(window, cx).placeholder(ts!("common.search")).clean_on_escape());

        let _search_input_subscription = cx.subscribe_in(&search_state, window, Self::on_search_input_event);

        let server_command_state = cx.new(|cx| InputState::new(window, cx).placeholder("Type a command...").clean_on_escape());
        let _server_command_subscription = cx.subscribe_in(&server_command_state, window, Self::on_server_command_input_event);

        let focus_handle = cx.focus_handle();
        focus_handle.focus(window, cx);

        // Observe window bounds changes and save them to config
        // Delay observation slightly to let OS apply initial window bounds first
        let observe_start = std::cell::RefCell::new(std::time::Instant::now());
        cx.observe_window_bounds(window, move |_, window, cx| {
            let elapsed = observe_start.borrow().elapsed().as_millis();
            // Only start tracking changes after 200ms to let OS position the window
            if elapsed < 200 {
                return;
            }

            let origin = window.bounds().origin;
            let size = window.viewport_size();
            
            let x = origin.x.to_f64() as f32;
            let y = origin.y.to_f64() as f32;
            let w = size.width.to_f64() as f32;
            let h = size.height.to_f64() as f32;

            let new_window_bounds = if window.is_fullscreen() {
                crate::interface_config::WindowBounds::Fullscreen { x, y, w, h }
            } else if window.is_maximized() {
                crate::interface_config::WindowBounds::Maximized { x, y, w, h }
            } else {
                crate::interface_config::WindowBounds::Windowed { x, y, w, h }
            };

            let old_window_bounds = InterfaceConfig::get(cx).game_output_bounds.clone();
            if new_window_bounds != old_window_bounds {
                InterfaceConfig::get_mut(cx).game_output_bounds = new_window_bounds;
            }
        }).detach();

        // Force save config when window is closed to ensure bounds are persisted
        cx.on_window_closed({
            move |cx| {
                InterfaceConfig::force_save(cx);
            }
        }).detach();

        Self {
            scroll_handler: ScrollHandler { state: scroll_state },
            _keep_alive: keep_alive,
            game_output,
            search_state,
            server_command_state: Some(server_command_state),
            _search_task: Task::ready(()),
            _search_input_subscription,
            _server_command_subscription: Some(_server_command_subscription),
            focus_handle,
            is_tabbed: false,
            active_instance_id: None,
            tabs: std::collections::HashMap::new(),
            instance_names: std::collections::HashMap::new(),
            instance_folders: std::collections::HashMap::new(),
            dot_minecraft_folders: std::collections::HashMap::new(),
            instance_keep_alives: std::collections::HashMap::new(),
            instance_ids: std::collections::HashMap::new(),
            instance_statuses: std::collections::HashMap::new(),
            server_names: std::collections::HashMap::new(),
            server_folders: std::collections::HashMap::new(),
            server_statuses: std::collections::HashMap::new(),
            backend_handle,
        }
    }

    pub fn new_tabbed(
        output_id: usize,
        instance_id: InstanceID,
        instance_name: SharedString,
        instance_folder: Arc<std::path::Path>,
        dot_minecraft_folder: Arc<std::path::Path>,
        keep_alive: KeepAlive,
        game_output: Entity<GameOutput>,
        backend_handle: BackendHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let scroll_state = Rc::clone(&game_output.read(cx).scroll_state);

        let search_state = cx.new(|cx| InputState::new(window, cx).placeholder(ts!("common.search")).clean_on_escape());

        let _search_input_subscription = cx.subscribe_in(&search_state, window, Self::on_search_input_event);

        let server_command_state = cx.new(|cx| InputState::new(window, cx).placeholder("Type a command...").clean_on_escape());
        let _server_command_subscription = cx.subscribe_in(&server_command_state, window, Self::on_server_command_input_event);

        let focus_handle = cx.focus_handle();
        focus_handle.focus(window, cx);

        // Observe window bounds changes and save them to config
        let observe_start = std::cell::RefCell::new(std::time::Instant::now());
        cx.observe_window_bounds(window, move |_, window, cx| {
            let elapsed = observe_start.borrow().elapsed().as_millis();
            if elapsed < 200 {
                return;
            }

            let origin = window.bounds().origin;
            let size = window.viewport_size();
            
            let x = origin.x.to_f64() as f32;
            let y = origin.y.to_f64() as f32;
            let w = size.width.to_f64() as f32;
            let h = size.height.to_f64() as f32;

            let new_window_bounds = if window.is_fullscreen() {
                crate::interface_config::WindowBounds::Fullscreen { x, y, w, h }
            } else if window.is_maximized() {
                crate::interface_config::WindowBounds::Maximized { x, y, w, h }
            } else {
                crate::interface_config::WindowBounds::Windowed { x, y, w, h }
            };

            let old_window_bounds = InterfaceConfig::get(cx).game_output_bounds.clone();
            if new_window_bounds != old_window_bounds {
                InterfaceConfig::get_mut(cx).game_output_bounds = new_window_bounds;
            }
        }).detach();

        cx.on_window_closed({
            move |cx| {
                InterfaceConfig::force_save(cx);
            }
        }).detach();

        let mut tabs = std::collections::HashMap::new();
        tabs.insert(output_id, game_output.clone());
        
        let mut instance_names = std::collections::HashMap::new();
        instance_names.insert(output_id, instance_name.clone());
        
        let mut instance_folders = std::collections::HashMap::new();
        instance_folders.insert(output_id, instance_folder);
        
        let mut dot_minecraft_folders = std::collections::HashMap::new();
        dot_minecraft_folders.insert(output_id, dot_minecraft_folder);
        
        let mut instance_keep_alives = std::collections::HashMap::new();
        instance_keep_alives.insert(output_id, keep_alive.create_handle());
        
        let mut instance_ids = std::collections::HashMap::new();
        instance_ids.insert(output_id, instance_id);

        let mut server_names = std::collections::HashMap::new();
        let mut server_folders = std::collections::HashMap::new();
        let mut server_statuses = std::collections::HashMap::new();
        // If instance_id is dangling, it's a server
        if instance_id == bridge::instance::InstanceID::dangling() {
            let server_name = instance_name.to_string();
            server_names.insert(output_id, server_name.clone());
            server_statuses.insert(server_name.clone(), true); // Mark as running
            // For servers, compute the correct server folder path
            // Server path is ~/.local/share/PandoraLauncher/servers/{server_name}
            if let Some(data_dir) = std::env::var("XDG_DATA_HOME")
                .ok()
                .and_then(|p| if p.is_empty() { None } else { Some(p) })
                .or_else(|| {
                    std::env::var("HOME").ok().map(|h| {
                        format!("{}/.local/share", h)
                    })
                })
            {
                let server_path = std::path::PathBuf::from(data_dir)
                    .join("PandoraLauncher")
                    .join("servers")
                    .join(&server_name);
                server_folders.insert(output_id, server_path.into());
            }
        }

        Self {
            scroll_handler: ScrollHandler { state: scroll_state },
            _keep_alive: keep_alive,
            game_output,
            search_state,
            server_command_state: Some(server_command_state),
            _search_task: Task::ready(()),
            _search_input_subscription,
            _server_command_subscription: Some(_server_command_subscription),
            focus_handle,
            is_tabbed: true,
            active_instance_id: Some(output_id),
            tabs,
            instance_names,
            instance_folders,
            dot_minecraft_folders,
            instance_keep_alives,
            instance_ids,
            instance_statuses: std::collections::HashMap::new(),
            server_names,
            server_folders,
            server_statuses,
            backend_handle,
        }
    }

    pub fn create_or_switch_tab(
        &mut self,
        output_id: usize,
        instance_id: InstanceID,
        instance_name: SharedString,
        instance_folder: Arc<std::path::Path>,
        dot_minecraft_folder: Arc<std::path::Path>,
        game_output: Entity<GameOutput>,
        keep_alive_handle: KeepAliveHandle,
        cx: &mut Context<Self>,
    ) {
        // Check if tab already exists
        if !self.tabs.contains_key(&output_id) {
            // Create new tab
            self.tabs.insert(output_id, game_output.clone());
            self.instance_names.insert(output_id, instance_name.clone());
            self.instance_folders.insert(output_id, instance_folder);
            self.dot_minecraft_folders.insert(output_id, dot_minecraft_folder);
            self.instance_keep_alives.insert(output_id, keep_alive_handle);
            self.instance_ids.insert(output_id, instance_id);
            
            // If instance_id is dangling, it's a server
            if instance_id == bridge::instance::InstanceID::dangling() {
                let server_name = instance_name.to_string();
                self.server_names.insert(output_id, server_name.clone());
                self.server_statuses.insert(server_name.clone(), true); // Mark as running
                // For servers, compute the correct server folder path
                // Server path is ~/.local/share/PandoraLauncher/servers/{server_name}
                if let Some(data_dir) = std::env::var("XDG_DATA_HOME")
                    .ok()
                    .and_then(|p| if p.is_empty() { None } else { Some(p) })
                    .or_else(|| {
                        std::env::var("HOME").ok().map(|h| {
                            format!("{}/.local/share", h)
                        })
                    })
                {
                    let server_path = std::path::PathBuf::from(data_dir)
                        .join("PandoraLauncher")
                        .join("servers")
                        .join(&server_name);
                    self.server_folders.insert(output_id, server_path.into());
                }
            }
        }
        
        // Switch to this tab
        self.active_instance_id = Some(output_id);
        if let Some(game_output_entity) = self.tabs.get(&output_id) {
            self.game_output = game_output_entity.clone();
            let scroll_state = Rc::clone(&self.game_output.read(cx).scroll_state);
            self.scroll_handler = ScrollHandler { state: scroll_state };
        }
        cx.notify();
    }

    pub fn add_output_to_tab(
        &mut self,
        instance_id: usize,
        time: i64,
        level: GameOutputLogLevel,
        text: Arc<[Arc<str>]>,
        cx: &mut Context<Self>,
    ) {
        if let Some(game_output) = self.tabs.get(&instance_id) {
            game_output.update(cx, |game_output, _| {
                game_output.add(time, level, text);
            });
        }
    }

    pub fn update_instance_status(&mut self, instance_id: InstanceID, status: InstanceStatus) {
        self.instance_statuses.insert(instance_id, status);
    }

    pub fn close_tab(&mut self, instance_id: usize, cx: &mut Context<Self>) {
        // Check if instance is still running
        if let Some(instance_id_obj) = self.instance_ids.get(&instance_id) {
            if let Some(status) = self.instance_statuses.get(instance_id_obj) {
                if *status == InstanceStatus::Running {
                    return; // Cannot close running instances
                }
            }
        }

        // Check if server is still running
        if let Some(server_name) = self.server_names.get(&instance_id) {
            if let Some(is_running) = self.server_statuses.get(server_name) {
                if *is_running {
                    return; // Cannot close running servers
                }
            }
        }

        // Tab must exist to be closed
        if !self.tabs.contains_key(&instance_id) {
            return;
        }

        // Remove from all maps
        self.tabs.remove(&instance_id);
        self.instance_names.remove(&instance_id);
        self.instance_folders.remove(&instance_id);
        self.dot_minecraft_folders.remove(&instance_id);
        self.instance_keep_alives.remove(&instance_id);
        self.instance_ids.remove(&instance_id);
        self.server_names.remove(&instance_id);
        self.server_folders.remove(&instance_id);

        // If this was the active tab being viewed, switch to another one
        if self.active_instance_id == Some(instance_id) {
            if let Some(&next_instance_id) = self.tabs.keys().next() {
                self.active_instance_id = Some(next_instance_id);
                if let Some(game_output) = self.tabs.get(&next_instance_id) {
                    self.game_output = game_output.clone();
                    let scroll_state = Rc::clone(&self.game_output.read(cx).scroll_state);
                    self.scroll_handler = ScrollHandler { state: scroll_state };
                }
            } else {
                self.active_instance_id = None;
            }
        }

        cx.notify();
    }

    fn on_search_input_event(
        &mut self,
        state: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let InputEvent::PressEnter { secondary: false } = event else {
            return;
        };

        let item_state = self.game_output.update(cx, |game_output, _| game_output.item_state.take());

        let Some(mut item_state) = item_state else {
            return; // Already searching
        };

        let search_pattern = state.read(cx).value();
        if search_pattern.trim().is_empty() {
            self._search_task = cx.spawn_in(window, async move |this, window| {
                let mut lengths = Vec::new();
                item_state.total_line_count = 0;
                for item in &mut item_state.items {
                    if item.skip {
                        item.total_lines = item.backup_total_lines_while_skipped;
                    }

                    item.skip = false;
                    item.highlighted_text = None;

                    item_state.total_line_count += item.total_lines;
                    lengths.push(item.total_lines);
                }
                item_state.item_sizes = FenwickTree::from_iter(lengths.into_iter());
                item_state.cached_shaped_lines.item_lines.clear();
                item_state.search_query = SharedString::new_static("");

                this.update_in(window, |this, window, cx| {
                    this.game_output.update(cx, |game_output, _| {
                        game_output.item_state = Some(item_state);
                    });
                    this.search_state.update(cx, |input, cx| input.set_loading(false, window, cx));
                    cx.notify();
                }).unwrap();
            });
        } else {
            self._search_task = cx.spawn_in(window, async move |this, window| {
                let mut lengths = Vec::new();
                item_state.total_line_count = 0;
                for item in &mut item_state.items {
                    let mut contains = None;
                    for (line_index, line) in item.text.iter().enumerate() {
                        if let Some(found) = line.find(search_pattern.as_str()) {
                            contains = Some((line_index, found..found+search_pattern.as_str().len()));
                            break;
                        }
                    }
                    if contains.is_some() {
                        lengths.push(item.total_lines);
                        item_state.total_line_count += item.total_lines;

                        item.highlighted_text = contains;
                        item.skip = false;
                    } else {
                        item.backup_total_lines_while_skipped = item.total_lines;
                        item.total_lines = 0;
                        lengths.push(0);

                        item.skip = true;
                    }
                }
                item_state.item_sizes = FenwickTree::from_iter(lengths.into_iter());
                item_state.cached_shaped_lines.item_lines.clear();
                item_state.search_query = search_pattern;

                this.update_in(window, |this, window, cx| {
                    this.game_output.update(cx, |game_output, _| {
                        game_output.item_state = Some(item_state);
                    });
                    this.search_state.update(cx, |input, cx| input.set_loading(false, window, cx));
                    cx.notify();
                })
                .unwrap();
            });
        }

        state.update(cx, |input, cx| input.set_loading(true, window, cx));
    }

    fn on_server_command_input_event(
        &mut self,
        state: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let InputEvent::PressEnter { secondary: false } = event else {
            return;
        };

        let command = state.read(cx).value();
        if command.trim().is_empty() {
            return;
        }

        // Check if we have an active server
        if let Some(active_id) = self.active_instance_id {
            if let Some(server_name) = self.server_names.get(&active_id).cloned() {
                // Send the command to the backend
                self.backend_handle.send(MessageToBackend::SendServerCommand {
                    name: server_name.into(),
                    command: command.as_str().into(),
                });

                // Clear the input field
                state.update(cx, |state, cx| state.set_value("", window, cx));
            }
        }
    }
}

impl Render for GameOutputRoot {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let search = Input::new(&self.search_state).prefix(Icon::new(PandoraIcon::Search).small());

        let bar = h_flex()
            .w_full()
            .id("controls")
            .gap_4()
            .px_2()
            .py_2()
            .child(search)
            .child(Button::new("top").label(ts!("common.nav.top")).on_click(cx.listener(|root, _, _, cx| {
                let mut state = root.scroll_handler.state.borrow_mut();
                state.scrolling = GameOutputScrolling::Top { offset: Pixels::ZERO };
                cx.notify();
            })))
            .child(Button::new("bottom").label(ts!("common.nav.bottom")).on_click(cx.listener(|root, _, _, cx| {
                let mut state = root.scroll_handler.state.borrow_mut();
                state.scrolling = GameOutputScrolling::Bottom;
                cx.notify();
            })))
            .child(Button::new("clear").label("Clear").on_click(cx.listener(|root, _, _, cx| {
                root.game_output.update(cx, |game_output, _| {
                    game_output.clear();
                });
                cx.notify();
            })))
            .child(Button::new("copy").label("Copy").on_click(cx.listener(|root, _, _, cx| {
                let logs = root.game_output.read(cx).collect_logs();
                cx.write_to_clipboard(logs.into());
            })));

        // Build bottom bar with kill button (only if instance is running)
        let mut bottom_bar = h_flex()
            .w_full()
            .px_2()
            .py_2()
            .gap_2()
            .items_center();

        // Always add folder, command input, and kill buttons
        if let Some(active_id) = self.active_instance_id {
            if let Some(instance_id) = self.instance_ids.get(&active_id).copied() {
                // Check if this is a server (has command input)
                let is_server = self.server_names.get(&active_id).is_some();
                
                // Create button container with appropriate layout
                let mut action_buttons = if is_server {
                    h_flex()
                        .flex_1()  // For servers, flex to fill space for the command input
                        .gap_1()
                        .items_center()
                } else {
                    h_flex()
                        .flex_1()  // Stretch to fill space
                        .justify_end()  // For instances, align buttons to the right
                        .gap_1()
                        .items_center()
                };
                
                // Add command input bar if we have a server (FIRST)
                if is_server {
                    if let Some(command_state) = &self.server_command_state {
                        let command_input = Input::new(command_state)
                            .flex_1();
                        
                        action_buttons = action_buttons.child(command_input);
                    }
                }
                
                // Add folder button - check both instance and server folders
                let folder_path = if let Some(server_path) = self.server_folders.get(&active_id) {
                    Some(server_path.clone())
                } else if let Some(instance_path) = self.instance_folders.get(&active_id) {
                    // For instances, always ensure we open the .minecraft folder
                    let dot_minecraft = if let Some(dot_minecraft_path) = self.dot_minecraft_folders.get(&active_id) {
                        dot_minecraft_path.clone()
                    } else {
                        // Fallback: append .minecraft to instance path if not already in dot_minecraft_folders
                        let mut path = instance_path.to_path_buf();
                        if !path.ends_with(".minecraft") {
                            path.push(".minecraft");
                        }
                        Arc::from(path)
                    };
                    Some(dot_minecraft)
                } else {
                    None
                };
                
                if let Some(folder_path) = folder_path {
                    let folder_btn = Button::new("open_folder")
                        .icon(PandoraIcon::Folder)
                        .on_click(cx.listener(move |_, _, _, _| {
                            let _ = open::that(folder_path.as_ref());
                        }))
                        .p_1()
                        .h(px(24.0));
                    
                    action_buttons = action_buttons.child(folder_btn);
                }
                
                // Add kill button last
                let is_instance_running = self.active_instance_id
                    .and_then(|output_id| self.instance_ids.get(&output_id))
                    .and_then(|instance_id| self.instance_statuses.get(instance_id))
                    .map(|status| *status == InstanceStatus::Running)
                    .unwrap_or(false);
                
                // Check if server is running
                let is_server_running = self.active_instance_id
                    .and_then(|output_id| self.server_names.get(&output_id))
                    .and_then(|server_name| self.server_statuses.get(server_name))
                    .copied()
                    .unwrap_or(false);
                
                let mut kill_btn = Button::new("kill_instance")
                    .icon(PandoraIcon::Close)
                    .label("Kill")
                    .p_1()
                    .h(px(24.0));
                
                if is_instance_running {
                    kill_btn = kill_btn.on_click(cx.listener(move |root, _, _, _cx| {
                        // Send kill instance message to backend
                        root.backend_handle.send(MessageToBackend::KillInstance { id: instance_id });
                    }));
                } else if is_server_running {
                    kill_btn = kill_btn.on_click(cx.listener(move |root, _, _, _cx| {
                        // Send stop server message to backend
                        if let Some(server_name) = root.active_instance_id
                            .and_then(|output_id| root.server_names.get(&output_id))
                            .cloned()
                        {
                            // Mark server as stopped immediately
                            root.server_statuses.insert(server_name.clone(), false);
                            root.backend_handle.send(MessageToBackend::StopServer { name: server_name.into() });
                        }
                    }));
                } else {
                    // Disable kill button if instance/server is not running
                    kill_btn = kill_btn.disabled(true);
                }
                
                action_buttons = action_buttons.child(kill_btn);
                bottom_bar = bottom_bar.child(action_buttons);
            }
        }

        // Build tab bar if in tabbed mode
        // No fixed limit - tabs fit naturally based on available space
        let mut tab_ids: Vec<_> = self.tabs.keys().copied().collect();
        tab_ids.sort();
        
        // Build tab bar - show all tabs, they'll shrink/wrap as needed
        let mut tab_bar = h_flex()
            .w_full()
            .gap_2()
            .px_2()
            .items_center();
        
        // Render all tabs - they'll naturally wrap/shrink based on available space
        for instance_id in tab_ids {
            let _is_viewed = self.active_instance_id == Some(instance_id);
            let instance_name = self.instance_names.get(&instance_id)
                .cloned()
                .unwrap_or_else(|| format!("Instance {}", instance_id).into());

            // Check if instance/server is running
            let is_running = {
                let mut running = false;
                if let Some(inst_id) = self.instance_ids.get(&instance_id) {
                    if let Some(status) = self.instance_statuses.get(inst_id) {
                        running = *status == InstanceStatus::Running;
                    }
                }
                if !running {
                    if let Some(server_name) = self.server_names.get(&instance_id) {
                        if let Some(is_srv_running) = self.server_statuses.get(server_name) {
                            running = *is_srv_running;
                        }
                    }
                }
                running
            };

            // Always add right-click handler if NOT running (dead)
            let tab = Button::new(format!("tab_{}", instance_id))
                .label(instance_name)
                .on_click(cx.listener(move |root, _, _, cx| {
                    root.active_instance_id = Some(instance_id);
                    if let Some(game_output) = root.tabs.get(&instance_id) {
                        root.game_output = game_output.clone();
                        let scroll_state = Rc::clone(&root.game_output.read(cx).scroll_state);
                        root.scroll_handler = ScrollHandler { state: scroll_state };
                    }
                    cx.notify();
                }));
            
            let tab = if !is_running {
                tab.on_mouse_down(MouseButton::Right, cx.listener(move |root, _, window, cx| {
                    window.prevent_default();
                    root.close_tab(instance_id, cx);
                }))
            } else {
                tab
            };

            tab_bar = tab_bar.child(tab);
        }

        // Check if we're in tabbed mode
        let has_tabs = self.is_tabbed && !self.tabs.is_empty();
        
        // Build the main content area (without tabs/borders)
        let main_output = h_flex()
            .flex_1()
            .w_full()
            .child(GameOutputList {
                interactivity: Interactivity::new(),
                game_output: self.game_output.clone(),
            })
            .child(
                div()
                    .w_3()
                    .h_full()
                    .border_y_12()
                    .child(Scrollbar::vertical(&self.scroll_handler)),
            );

        // Build layout: search/buttons > tabs > output, all in single border
        let mut inner = v_flex()
            .w_full()
            .flex_1()
            .gap_1();

        // Add control buttons first with separator
        let bar_with_separator = v_flex()
            .w_full()
            .child(bar)
            .child(
                div()
                    .w_full()
                    .border_b_1()
                    .border_color(cx.theme().border)
            );
        inner = inner.child(bar_with_separator);

        // Add tab bar below buttons if we have tabs with separator
        if has_tabs {
            let tabs_with_separator = v_flex()
                .w_full()
                .child(tab_bar)
                .child(
                    div()
                        .w_full()
                        .border_b_1()
                        .border_color(cx.theme().border)
                );
            inner = inner.child(tabs_with_separator);
        }

        // Add main content area last
        inner = inner.child(main_output);

        // Add bottom bar with kill button (only if instance is active)
        if self.active_instance_id.is_some() {
            let bottom_with_separator = v_flex()
                .w_full()
                .child(
                    div()
                        .w_full()
                        .border_t_1()
                        .border_color(cx.theme().border)
                )
                .child(bottom_bar);
            inner = inner.child(bottom_with_separator);
        }

        // Wrap everything in a single border
        let content_area = v_flex()
            .w_full()
            .flex_1()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .child(inner);

        let mut contents = v_flex()
            .size_full()
            .border_12()
            .gap_4();

        contents = contents.child(content_area);

        contents
            .on_scroll_wheel(cx.listener(|root, event: &ScrollWheelEvent, _, cx| {
                let state = root.scroll_handler.state.borrow();
                let delta = event.delta.pixel_delta(state.line_height).y;
                let max_scroll_amount = state.max_scroll_amount();
                drop(state);

                let current_offset = root.scroll_handler.offset().y;
                let new_offset = (current_offset + delta).clamp(-max_scroll_amount, Pixels::ZERO);
                if current_offset != new_offset {
                    root.scroll_handler.set_offset(Point::new(Pixels::ZERO, new_offset));
                    cx.notify();
                }
            }))
            .track_focus(&self.focus_handle)
            .on_action(|_: &CloseWindow, window, _| {
                window.remove_window();
            })
    }
}
