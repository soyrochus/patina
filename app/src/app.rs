use crate::app::panels::Sidebar;
use anyhow::Result;
use egui::{self, RichText};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use patina_core::state::{AppState, Conversation};
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tracing::error;

mod panels {
    use egui::Ui;
    use patina_core::state::{ConversationSummary, MessageRole};
    use uuid::Uuid;

    pub struct Sidebar;

    impl Sidebar {
        pub fn show<F>(
            ui: &mut Ui,
            sessions: &[ConversationSummary],
            active: Option<Uuid>,
            mut on_select: F,
        ) where
            F: FnMut(Uuid),
        {
            ui.heading("Chats");
            for summary in sessions {
                let label = format!("{} ({})", summary.title, summary.message_count);
                let selected = Some(summary.id) == active;
                if ui.selectable_label(selected, label).clicked() {
                    on_select(summary.id);
                }
            }
        }
    }

    pub fn role_badge(role: &MessageRole) -> &'static str {
        match role {
            MessageRole::System => "System",
            MessageRole::User => "You",
            MessageRole::Assistant => "Patina",
            MessageRole::Tool => "Tool",
        }
    }
}

pub struct PatinaEguiApp {
    state: Arc<AppState>,
    runtime: Arc<Runtime>,
    input: String,
    error: Option<String>,
    tx: UnboundedSender<Result<()>>,
    rx: UnboundedReceiver<Result<()>>,
    markdown_cache: CommonMarkCache,
}

impl PatinaEguiApp {
    pub fn new(state: Arc<AppState>, runtime: Arc<Runtime>) -> Self {
        let (tx, rx) = unbounded_channel();
        Self {
            state,
            runtime,
            input: String::new(),
            error: None,
            tx,
            rx,
            markdown_cache: CommonMarkCache::default(),
        }
    }

    fn submit_message(&mut self) {
        let content = self.input.trim().to_owned();
        if content.is_empty() {
            return;
        }
        self.input.clear();
        let state = self.state.clone();
        let tx = self.tx.clone();
        self.runtime.spawn(async move {
            let result = state.send_user_message(content).await;
            if tx.send(result).is_err() {
                tracing::warn!("UI has been dropped before message completion");
            }
        });
    }

    fn process_background_results(&mut self) {
        while let Ok(result) = self.rx.try_recv() {
            if let Err(err) = result {
                error!("error" = %err, "Failed to send message");
                self.error = Some(err.to_string());
            } else {
                self.error = None;
            }
        }
    }

    fn show_conversation(&mut self, ui: &mut egui::Ui, conversation: &Conversation) {
        for message in &conversation.messages {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(panels::role_badge(&message.role)).strong());
                    ui.small(message.created_at.to_rfc3339());
                });
                self.render_markdown(ui, &message.content);
                if !message.tool_calls.is_empty() {
                    ui.collapsing("Tool calls", |ui| {
                        for call in &message.tool_calls {
                            ui.monospace(format!(
                                "{} • {:?}\n{}",
                                call.name,
                                call.status,
                                serde_json::to_string_pretty(&call.arguments).unwrap_or_default()
                            ));
                            if let Some(response) = &call.response {
                                ui.separator();
                                ui.monospace(
                                    serde_json::to_string_pretty(response)
                                        .unwrap_or_else(|_| "<invalid>".into()),
                                );
                            }
                        }
                    });
                }
            });
            ui.add_space(6.0);
        }
    }

    fn render_markdown(&mut self, ui: &mut egui::Ui, text: &str) {
        CommonMarkViewer::new("patina_markdown").show(ui, &mut self.markdown_cache, text);
    }
}

impl eframe::App for PatinaEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_background_results();

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.heading("Patina");
            if ui.button("New chat").clicked() {
                self.state.start_new_conversation();
            }
            if let Some(error) = &self.error {
                ui.colored_label(egui::Color32::RED, error);
            }
        });

        egui::SidePanel::left("sidebar").show(ctx, |ui| {
            let sessions = self.state.conversation_summaries();
            let active = self.state.active_conversation().map(|c| c.id);
            Sidebar::show(ui, &sessions, active, |id| {
                self.state.select_conversation(id)
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(conversation) = self.state.active_conversation() {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.show_conversation(ui, &conversation);
                });
            } else {
                ui.label("Start a conversation by sending a message.");
            }
            ui.separator();
            let input =
                ui.add(egui::TextEdit::multiline(&mut self.input).hint_text("Send a message"));
            if input.lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift)
            {
                self.submit_message();
            }
            if ui.button("Send").clicked() {
                self.submit_message();
            }
        });
    }
}
