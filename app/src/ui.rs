use chrono::{DateTime, Local};
use egui::{self, Align, Color32, Frame, Layout, Margin, RichText, ScrollArea, Sense, Vec2};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use patina_core::state::{ChatMessage, Conversation, ConversationSummary, MessageRole};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeMode {
    pub const ALL: [ThemeMode; 3] = [ThemeMode::System, ThemeMode::Light, ThemeMode::Dark];

    pub fn label(self) -> &'static str {
        match self {
            ThemeMode::System => "System",
            ThemeMode::Light => "Light",
            ThemeMode::Dark => "Dark",
        }
    }

    pub fn fallback_theme(self) -> eframe::Theme {
        match self {
            ThemeMode::Light => eframe::Theme::Light,
            ThemeMode::System | ThemeMode::Dark => eframe::Theme::Dark,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemePalette {
    pub background: Color32,
    pub sidebar_background: Color32,
    pub surface: Color32,
    pub user_bubble: Color32,
    pub assistant_bubble: Color32,
    pub accent: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub border: Color32,
    pub warning: Color32,
    pub elevated_shadow: Color32,
}

impl ThemePalette {
    pub fn for_dark() -> Self {
        Self {
            background: color_from_hex("#1E1E1E"),
            sidebar_background: color_from_hex("#252526"),
            surface: color_from_hex("#2D2D30"),
            user_bubble: color_from_hex("#3A3D41"),
            assistant_bubble: color_from_hex("#2D2D30"),
            accent: color_from_hex("#0078D7"),
            text_primary: color_from_hex("#E6E6E6"),
            text_secondary: color_from_hex("#B0B0B0"),
            border: color_from_hex("#3B3B3B"),
            warning: color_from_hex("#C63C3C"),
            elevated_shadow: Color32::from_rgba_unmultiplied(0, 0, 0, 120),
        }
    }

    pub fn for_light() -> Self {
        Self {
            background: color_from_hex("#FFFFFF"),
            sidebar_background: color_from_hex("#F3F3F3"),
            surface: color_from_hex("#F9F9F9"),
            user_bubble: color_from_hex("#E5E5E5"),
            assistant_bubble: color_from_hex("#F9F9F9"),
            accent: color_from_hex("#0063B1"),
            text_primary: color_from_hex("#202020"),
            text_secondary: color_from_hex("#5F5F5F"),
            border: color_from_hex("#D0D0D0"),
            warning: color_from_hex("#B02020"),
            elevated_shadow: Color32::from_rgba_unmultiplied(0, 0, 0, 60),
        }
    }

    pub fn visuals(&self, dark_mode: bool) -> egui::Visuals {
        let mut visuals = if dark_mode {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        visuals.window_fill = self.surface;
        visuals.panel_fill = self.background;
        visuals.extreme_bg_color = self.surface;
        visuals.widgets.noninteractive.bg_fill = self.surface;
        visuals.widgets.noninteractive.fg_stroke.color = self.text_primary;
        visuals.widgets.active.fg_stroke.color = self.text_primary;
        visuals.widgets.inactive.fg_stroke.color = self.text_primary;
        visuals.dark_mode = dark_mode;
        visuals
    }
}

fn color_from_hex(hex: &str) -> Color32 {
    let trimmed = hex.trim_start_matches('#');
    if trimmed.len() == 6 {
        if let Ok(value) = u32::from_str_radix(trimmed, 16) {
            let r = ((value >> 16) & 0xFF) as u8;
            let g = ((value >> 8) & 0xFF) as u8;
            let b = (value & 0xFF) as u8;
            return Color32::from_rgb(r, g, b);
        }
    }
    Color32::WHITE
}

#[derive(Debug, Default)]
pub struct MenuBarState {
    pub theme_mode: ThemeMode,
}

#[derive(Default)]
pub struct MenuBarOutput {
    pub new_project: bool,
    pub open_project: bool,
    pub new_chat: bool,
    pub toggle_sidebar: bool,
    pub focus_search: bool,
    pub clear_input: bool,
    pub exit: bool,
    pub show_about: bool,
    pub show_settings: bool,
    pub theme_changed: Option<ThemeMode>,
}

pub struct MenuBar;

impl MenuBar {
    pub fn show(
        ui: &mut egui::Ui,
        state: &mut MenuBarState,
        logo_texture: Option<&egui::TextureHandle>,
        project_available: bool,
        project_name: Option<&str>,
    ) -> MenuBarOutput {
        let mut output = MenuBarOutput::default();
        egui::menu::bar(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 18.0;
                if let Some(texture) = logo_texture {
                    ui.image((texture.id(), egui::vec2(18.0, 18.0)));
                }
                if let Some(name) = project_name {
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new(format!("Workspace: {name}"))
                            .small()
                            .color(ui.visuals().text_color()),
                    );
                }
                ui.menu_button("File", |ui| {
                    if ui.button("New Project…").clicked() {
                        output.new_project = true;
                        ui.close_menu();
                    }
                    if ui.button("Open Project…").clicked() {
                        output.open_project = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Settings…").clicked() {
                        output.show_settings = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .add_enabled(project_available, egui::Button::new("New chat\tCtrl+N"))
                        .clicked()
                    {
                        output.new_chat = true;
                        ui.close_menu();
                    }
                    if ui.button("Exit").clicked() {
                        output.exit = true;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Edit", |ui| {
                    if ui
                        .add_enabled(project_available, egui::Button::new("Clear input"))
                        .clicked()
                    {
                        output.clear_input = true;
                        ui.close_menu();
                    }
                });
                ui.menu_button("View", |ui| {
                    if ui
                        .add_enabled(
                            project_available,
                            egui::Button::new("Toggle sidebar\tCtrl+M"),
                        )
                        .clicked()
                    {
                        output.toggle_sidebar = true;
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(project_available, egui::Button::new("Focus search\tCtrl+K"))
                        .clicked()
                    {
                        output.focus_search = true;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        output.show_about = true;
                        ui.close_menu();
                    }
                });
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::ComboBox::from_id_source("theme_selector")
                        .selected_text(state.theme_mode.label())
                        .show_ui(ui, |ui| {
                            for mode in ThemeMode::ALL {
                                if ui
                                    .selectable_label(state.theme_mode == mode, mode.label())
                                    .clicked()
                                {
                                    if state.theme_mode != mode {
                                        state.theme_mode = mode;
                                        output.theme_changed = Some(mode);
                                    }
                                    ui.close_menu();
                                }
                            }
                        });
                });
            });
        });
        output
    }
}

#[derive(Default)]
pub struct SidebarState {
    pub collapsed: bool,
    pub search_query: String,
    pub search_focus_requested: bool,
    pub mcp_collapsed: bool,
    pub chats_collapsed: bool,
    rename_editor: Option<RenameEditor>,
    pub dragging_chat: Option<Uuid>,
    pub hovered_chat: Option<Uuid>,
    pub active_mcp_popup: Option<String>,
}

impl SidebarState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn request_search_focus(&mut self) {
        self.search_focus_requested = true;
    }
}

#[derive(Clone)]
struct RenameEditor {
    id: Uuid,
    buffer: String,
}

impl RenameEditor {
    fn new(id: Uuid, current: &str) -> Self {
        Self {
            id,
            buffer: current.to_string(),
        }
    }
}

#[derive(Default)]
pub struct SidebarOutput {
    pub selected_chat: Option<Uuid>,
    pub rename: Option<(Uuid, String)>,
    pub delete: Option<Uuid>,
    pub pin: Option<Uuid>,
    pub unpin: Option<Uuid>,
    pub reorder: Option<(Uuid, Uuid)>,
}

pub struct Sidebar;

impl Sidebar {
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        ui: &mut egui::Ui,
        state: &mut SidebarState,
        palette: &ThemePalette,
        summaries: &[ConversationSummary],
        pinned_lookup: &HashSet<Uuid>,
        pinned_order: &[Uuid],
        mcp_entries: &mut [McpSidebarEntry],
        active_chat: Option<Uuid>,
    ) -> SidebarOutput {
        let mut output = SidebarOutput::default();
        let search_frame = Frame::none()
            .fill(palette.surface)
            .inner_margin(Margin::same(6.0))
            .rounding(6.0)
            .stroke(egui::Stroke::new(1.0, palette.border));

        search_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("🔍");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut state.search_query)
                        .hint_text("Search MCPs & chats"),
                );
                if state.search_focus_requested {
                    response.request_focus();
                    state.search_focus_requested = false;
                }
            });
        });

        ui.add_space(12.0);
        let query = state.search_query.clone();
        Self::mcp_section(ui, state, palette, mcp_entries, &query);
        ui.add_space(10.0);
        Self::chats_section(
            ui,
            state,
            palette,
            summaries,
            pinned_lookup,
            pinned_order,
            &query,
            &mut output,
            active_chat,
        );
        output
    }

    fn mcp_section(
        ui: &mut egui::Ui,
        state: &mut SidebarState,
        palette: &ThemePalette,
        entries: &mut [McpSidebarEntry],
        query: &str,
    ) {
        let filtered_query = query.trim().to_lowercase();
        ui.collapsing("MCP", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            for entry in entries.iter_mut().filter(|entry| {
                filtered_query.is_empty()
                    || entry.name.to_lowercase().contains(&filtered_query)
                    || entry.description.to_lowercase().contains(&filtered_query)
            }) {
                let card = Frame::none()
                    .fill(palette.surface)
                    .inner_margin(Margin::symmetric(8.0, 6.0))
                    .rounding(6.0)
                    .stroke(egui::Stroke::new(1.0, palette.border));
                let popup_id = ui.make_persistent_id(format!("mcp_popup_{}", entry.id));
                let response = card.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let indicator_color = entry.status.color(palette);
                        let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                        ui.painter()
                            .circle_filled(rect.center(), 5.0, indicator_color);
                        ui.vertical(|ui| {
                            ui.label(RichText::new(&entry.name).strong());
                            ui.label(
                                RichText::new(format!(
                                    "{} • {}",
                                    entry.description,
                                    entry.status.label()
                                ))
                                .color(palette.text_secondary)
                                .small(),
                            );
                        });
                    });
                });
                if response.response.clicked() {
                    state.active_mcp_popup = Some(entry.id.clone());
                    ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                }
                if let Some(active) = &state.active_mcp_popup {
                    if *active == entry.id {
                        egui::popup::popup_above_or_below_widget(
                            ui,
                            popup_id,
                            &response.response,
                            egui::AboveOrBelow::Below,
                            |popup_ui| {
                                popup_ui.set_min_width(220.0);
                                popup_ui.label(RichText::new(&entry.name).strong());
                                popup_ui.separator();
                                popup_ui.label("Status");
                                popup_ui.label(entry.status.label());
                                popup_ui.separator();
                                popup_ui.horizontal(|ui| {
                                    if ui.button("Reconnect").clicked() {
                                        entry.status = McpStatus::Connecting;
                                        ui.close_menu();
                                    }
                                    if ui.button("Close").clicked() {
                                        ui.close_menu();
                                    }
                                });
                            },
                        );
                    }
                }
            }
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn chats_section(
        ui: &mut egui::Ui,
        state: &mut SidebarState,
        palette: &ThemePalette,
        summaries: &[ConversationSummary],
        pinned_lookup: &HashSet<Uuid>,
        pinned_order: &[Uuid],
        query: &str,
        output: &mut SidebarOutput,
        active_chat: Option<Uuid>,
    ) {
        let lower_query = query.trim().to_lowercase();
        ui.collapsing("Chats", |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            state.hovered_chat = None;
            let pinned: Vec<_> = pinned_order
                .iter()
                .filter_map(|id| summaries.iter().find(|s| &s.id == id))
                .filter(|summary| {
                    lower_query.is_empty() || summary.title.to_lowercase().contains(&lower_query)
                })
                .collect();
            let others: Vec<_> = summaries
                .iter()
                .filter(|summary| !pinned_lookup.contains(&summary.id))
                .filter(|summary| {
                    lower_query.is_empty() || summary.title.to_lowercase().contains(&lower_query)
                })
                .collect();

            ScrollArea::vertical()
                .id_source("sidebar_chats")
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    if !pinned.is_empty() {
                        ui.label(RichText::new("Pinned").color(palette.text_secondary));
                        for summary in pinned {
                            Self::chat_entry(
                                ui,
                                state,
                                palette,
                                summary,
                                true,
                                output,
                                active_chat,
                            );
                        }
                        ui.separator();
                    }
                    for summary in others {
                        Self::chat_entry(ui, state, palette, summary, false, output, active_chat);
                    }
                });
        });

        let pointer_down = ui.ctx().input(|i| i.pointer.primary_down());
        if !pointer_down {
            if let Some(dragged) = state.dragging_chat.take() {
                if let Some(target) = state.hovered_chat {
                    if dragged != target {
                        output.reorder = Some((dragged, target));
                    }
                }
            }
        }
    }

    fn chat_entry(
        ui: &mut egui::Ui,
        state: &mut SidebarState,
        palette: &ThemePalette,
        summary: &ConversationSummary,
        pinned: bool,
        output: &mut SidebarOutput,
        active_chat: Option<Uuid>,
    ) {
        let available = ui.available_width();
        let desired = Vec2::new(available, 52.0);
        let (rect, response) = ui.allocate_exact_size(desired, Sense::click_and_drag());
        let mut frame = Frame::none()
            .rounding(6.0)
            .stroke(egui::Stroke::new(1.0, palette.border));
        let fill = if Some(summary.id) == active_chat {
            palette.surface.linear_multiply(1.08)
        } else {
            palette.surface
        };
        frame = frame.fill(fill);
        let mut child_ui = ui.child_ui(rect, Layout::top_down(Align::Min));
        frame.show(&mut child_ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&summary.title).strong());
                if pinned {
                    ui.label(RichText::new("📌").color(palette.accent));
                }
            });
            let timestamp = format_timestamp(summary.updated_at);
            ui.label(
                RichText::new(format!(
                    "{} · {} messages",
                    timestamp, summary.message_count
                ))
                .color(palette.text_secondary)
                .small(),
            );
        });

        if response.clicked() {
            output.selected_chat = Some(summary.id);
        }
        if response.hovered() {
            state.hovered_chat = Some(summary.id);
        }
        if response.drag_started() {
            state.dragging_chat = Some(summary.id);
        }
        if response.double_clicked() {
            state.rename_editor = Some(RenameEditor::new(summary.id, &summary.title));
        }

        response.context_menu(|ui| {
            if ui.button("Rename").clicked() {
                state.rename_editor = Some(RenameEditor::new(summary.id, &summary.title));
                ui.close_menu();
            }
            if pinned {
                if ui.button("Unpin").clicked() {
                    output.unpin = Some(summary.id);
                    ui.close_menu();
                }
            } else if ui.button("Pin").clicked() {
                output.pin = Some(summary.id);
                ui.close_menu();
            }
            if ui.button("Delete").clicked() {
                output.delete = Some(summary.id);
                ui.close_menu();
            }
        });

        if matches!(
            state.rename_editor.as_ref().map(|e| e.id),
            Some(edit_id) if edit_id == summary.id
        ) {
            ui.add_space(4.0);
            if let Some(editor) = state.rename_editor.as_mut() {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut editor.buffer)
                        .desired_width(f32::INFINITY)
                        .hint_text("Chat name"),
                );
                let mut commit = false;
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    commit = true;
                }
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        commit = true;
                    }
                    if ui.button("Cancel").clicked() {
                        state.rename_editor = None;
                    }
                });
                if commit {
                    if let Some(editor) = state.rename_editor.take() {
                        let trimmed = editor.buffer.trim().to_string();
                        if !trimmed.is_empty() {
                            output.rename = Some((summary.id, trimmed));
                        }
                    }
                }
            }
        }
    }
}

fn format_timestamp(time: DateTime<chrono::Utc>) -> String {
    let local: DateTime<Local> = DateTime::from(time);
    local.format("%b %e, %H:%M").to_string()
}

#[derive(Clone, Debug)]
pub struct McpSidebarEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: McpStatus,
}

impl McpSidebarEntry {
    pub fn matches(&self, query: &str) -> bool {
        let q = query.trim().to_lowercase();
        q.is_empty()
            || self.name.to_lowercase().contains(&q)
            || self.description.to_lowercase().contains(&q)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McpStatus {
    Connected,
    Connecting,
    Disconnected,
}

impl McpStatus {
    pub fn label(&self) -> &'static str {
        match self {
            McpStatus::Connected => "Connected",
            McpStatus::Connecting => "Connecting",
            McpStatus::Disconnected => "Disconnected",
        }
    }

    pub fn color(&self, palette: &ThemePalette) -> Color32 {
        match self {
            McpStatus::Connected => Color32::from_rgb(28, 185, 96),
            McpStatus::Connecting => palette.accent,
            McpStatus::Disconnected => palette.text_secondary,
        }
    }
}

#[derive(Clone)]
pub struct ChatPanelState {
    pub visible_limit: usize,
    pub last_conversation_id: Option<Uuid>,
}

impl Default for ChatPanelState {
    fn default() -> Self {
        Self {
            visible_limit: 80,
            last_conversation_id: None,
        }
    }
}

impl ChatPanelState {
    pub fn reset_if_needed(&mut self, conversation_id: Uuid) {
        if self.last_conversation_id != Some(conversation_id) {
            self.last_conversation_id = Some(conversation_id);
            self.visible_limit = 80;
        }
    }

    pub fn request_more(&mut self, total: usize) {
        if self.visible_limit < total {
            self.visible_limit = (self.visible_limit + 40).min(total);
        }
    }
}

#[derive(Default)]
pub struct ChatPanelOutput {
    pub load_older: bool,
}

pub struct ChatPanel;

impl ChatPanel {
    pub fn show(
        ui: &mut egui::Ui,
        palette: &ThemePalette,
        state: &mut ChatPanelState,
        conversation: &Conversation,
        markdown_cache: &mut CommonMarkCache,
    ) -> ChatPanelOutput {
        let mut output = ChatPanelOutput::default();
        state.reset_if_needed(conversation.id);
        let total = conversation.messages.len();
        let start = total.saturating_sub(state.visible_limit);
        let messages = &conversation.messages[start..];
        let scroll = ScrollArea::vertical()
            .id_source("chat_history")
            .stick_to_bottom(true)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for message in messages {
                    Self::chat_bubble(ui, palette, markdown_cache, message);
                    ui.add_space(8.0);
                }
            });
        if scroll.state.offset.y <= 4.0 && start > 0 {
            output.load_older = true;
        }
        output
    }

    fn chat_bubble(
        ui: &mut egui::Ui,
        palette: &ThemePalette,
        markdown_cache: &mut CommonMarkCache,
        message: &ChatMessage,
    ) {
        let is_user = matches!(message.role, MessageRole::User);
        let bubble_color = if is_user {
            palette.user_bubble
        } else {
            palette.assistant_bubble
        };
        let total_width = ui.available_width().max(0.0);
        let min_width = 240.0;
        let max_user_width = 640.0;
        let (bubble_width, leading_pad, trailing_pad) = if is_user {
            let width = total_width
                .min(max_user_width)
                .max(min_width)
                .min(total_width);
            let pad = (total_width - width).max(0.0);
            (width, pad, 0.0)
        } else {
            (total_width, 0.0, 0.0)
        };
        ui.horizontal(|ui| {
            if leading_pad > 0.0 {
                ui.add_space(leading_pad);
            }
            ui.allocate_ui_with_layout(
                Vec2::new(bubble_width, 0.0),
                Layout::top_down(Align::Min),
                |ui| {
                    ui.set_width(bubble_width);
                    Frame::none()
                        .fill(bubble_color)
                        .stroke(egui::Stroke::new(1.0, palette.border))
                        .rounding(egui::Rounding::same(10.0))
                        .inner_margin(Margin::symmetric(12.0, 8.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(message.role_label()).strong());
                                ui.label(
                                    RichText::new(message.created_at.to_rfc2822())
                                        .color(palette.text_secondary)
                                        .small(),
                                );
                            });
                            CommonMarkViewer::new(format!("msg_{}", message.id)).show(
                                ui,
                                markdown_cache,
                                &message.content,
                            );
                            let code_blocks = extract_code_blocks(&message.content);
                            for block in code_blocks {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(format!("Code ({})", block.language))
                                            .color(palette.text_secondary)
                                            .small(),
                                    );
                                    if ui
                                        .button("Copy")
                                        .on_hover_text("Copy code block to clipboard")
                                        .clicked()
                                    {
                                        ui.output_mut(|out| {
                                            out.copied_text = block.content.clone()
                                        });
                                    }
                                });
                            }
                            if !message.tool_calls.is_empty() {
                                ui.collapsing("Tool calls", |ui| {
                                    for call in &message.tool_calls {
                                        ui.label(RichText::new(&call.name).strong());
                                        if let Ok(pretty) =
                                            serde_json::to_string_pretty(&call.arguments)
                                        {
                                            ui.code(pretty);
                                        }
                                    }
                                });
                            }
                            let token_guess = (message.content.chars().count() / 4).max(1);
                            ui.label(
                                RichText::new(format!("~{} tokens", token_guess))
                                    .color(palette.text_secondary)
                                    .small(),
                            );
                        });
                },
            );
            if trailing_pad > 0.0 {
                ui.add_space(trailing_pad);
            }
        });
    }
}

trait RoleLabel {
    fn role_label(&self) -> &'static str;
}

impl RoleLabel for ChatMessage {
    fn role_label(&self) -> &'static str {
        match self.role {
            MessageRole::System => "System",
            MessageRole::User => "You",
            MessageRole::Assistant => "Patina",
            MessageRole::Tool => "Tool",
        }
    }
}

struct CodeBlock {
    language: String,
    content: String,
}

fn extract_code_blocks(content: &str) -> Vec<CodeBlock> {
    let mut blocks = Vec::new();
    let mut current_language = String::new();
    let mut current = String::new();
    let mut in_block = false;
    for line in content.lines() {
        if line.trim_start().starts_with("```") {
            if in_block {
                blocks.push(CodeBlock {
                    language: current_language.clone(),
                    content: current.trim().to_string(),
                });
                current.clear();
                current_language.clear();
                in_block = false;
            } else {
                in_block = true;
                current_language = line.trim_matches('`').trim().to_string();
                if current_language.is_empty() {
                    current_language = "text".to_string();
                }
            }
            continue;
        }
        if in_block {
            current.push_str(line);
            current.push('\n');
        }
    }
    blocks
}

#[derive(Clone)]
pub struct InputBarState {
    pub draft: String,
    pub selected_model: String,
    pub temperature: f32,
    pub retain_input: bool,
    active_tools: HashSet<InputTool>,
}

impl InputBarState {
    pub fn new(model: impl Into<String>, temperature: f32, retain_input: bool) -> Self {
        let mut active_tools = HashSet::new();
        active_tools.insert(InputTool::Tools);
        active_tools.insert(InputTool::Mcps);
        Self {
            draft: String::new(),
            selected_model: model.into(),
            temperature,
            retain_input,
            active_tools,
        }
    }

    pub fn toggle_tool(&mut self, tool: InputTool) {
        if !self.active_tools.insert(tool) {
            self.active_tools.remove(&tool);
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputTool {
    Tools,
    Mcps,
    Files,
}

impl InputTool {
    const ALL: [InputTool; 3] = [InputTool::Tools, InputTool::Mcps, InputTool::Files];

    fn label(self) -> &'static str {
        match self {
            InputTool::Tools => "Tools",
            InputTool::Mcps => "MCPs",
            InputTool::Files => "Files",
        }
    }
}

#[derive(Default)]
pub struct InputBarOutput {
    pub send: bool,
    pub clear: bool,
    pub model_changed: Option<String>,
    pub temperature_changed: Option<f32>,
}

pub struct InputBar;

impl InputBar {
    pub fn show(
        ui: &mut egui::Ui,
        state: &mut InputBarState,
        palette: &ThemePalette,
        available_models: &[String],
        selection_valid: bool,
    ) -> InputBarOutput {
        let mut output = InputBarOutput::default();
        Frame::none()
            .fill(palette.surface)
            .rounding(6.0)
            .stroke(egui::Stroke::new(1.0, palette.border))
            .inner_margin(Margin::symmetric(10.0, 8.0))
            .show(ui, |ui| {
                let textarea = egui::TextEdit::multiline(&mut state.draft)
                    .desired_rows(4)
                    .hint_text("Message Patina…")
                    .lock_focus(true)
                    .frame(false);
                let response = ui.add(textarea);
                let send_shortcut = ui.input(|i| {
                    i.key_pressed(egui::Key::Enter) && i.modifiers.command && !i.modifiers.shift
                });
                if send_shortcut && response.has_focus() {
                    output.send = true;
                }
                ui.horizontal(|ui| {
                    if ui.button("✈ Send").clicked() {
                        output.send = true;
                    }
                    if ui.button("Clear").clicked() {
                        output.clear = true;
                    }
                    ui.checkbox(&mut state.retain_input, "Retain input");
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        let display_text = if state.selected_model.is_empty() {
                            "Select model"
                        } else {
                            state.selected_model.as_str()
                        };
                        egui::ComboBox::from_id_source("model_selector")
                            .selected_text(display_text)
                            .show_ui(ui, |ui| {
                                for model in available_models {
                                    if ui
                                        .selectable_label(state.selected_model == *model, model)
                                        .clicked()
                                        && state.selected_model != *model
                                    {
                                        state.selected_model = model.clone();
                                        output.model_changed = Some(model.clone());
                                    }
                                }
                            });
                        if available_models.is_empty() {
                            ui.label(
                                RichText::new("No models configured")
                                    .color(palette.warning)
                                    .small(),
                            );
                        } else if !selection_valid {
                            ui.label(
                                RichText::new("Model not in patina.yaml")
                                    .color(palette.warning)
                                    .small(),
                            );
                        }
                    });
                    let slider =
                        egui::Slider::new(&mut state.temperature, 0.0..=2.0).text("Temperature");
                    if ui.add(slider).drag_released() {
                        output.temperature_changed = Some(state.temperature);
                    }
                    for tool in InputTool::ALL {
                        let active = state.active_tools.contains(&tool);
                        let label = RichText::new(tool.label()).color(if active {
                            palette.text_primary
                        } else {
                            palette.text_secondary
                        });
                        if ui.selectable_label(active, label).clicked() {
                            state.toggle_tool(tool);
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let chars = state.draft.chars().count();
                        let tokens = (chars as f32 / 4.0).ceil() as usize;
                        ui.label(
                            RichText::new(format!("{chars} chars · ~{tokens} tokens"))
                                .color(palette.text_secondary)
                                .small(),
                        );
                    });
                });
            });
        output
    }
}
