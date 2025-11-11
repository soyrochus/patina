use patina_core::project::ProjectHandle;
use patina_core::state::AppState;
use patina_core::{llm::LlmDriver, state::MessageRole};
use std::sync::Arc;
use tempfile::TempDir;

fn test_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime")
}

#[test]
fn app_state_records_messages() {
    let runtime = test_runtime();
    let temp_dir = TempDir::new().expect("temp dir");
    let project = ProjectHandle::create(temp_dir.path(), "TestProject").expect("project");
    let store = project.transcript_store();
    let driver = runtime.block_on(LlmDriver::fake());
    let state = Arc::new(AppState::with_store(project, store, driver));

    runtime
        .block_on(state.send_user_message("hello world", "mock", 0.6))
        .expect("send message");

    let conversation = state.active_conversation().expect("conversation");
    assert!(conversation
        .messages
        .iter()
        .any(|msg| msg.role == MessageRole::Assistant));
    assert!(conversation
        .messages
        .iter()
        .any(|msg| msg.role == MessageRole::User));
}
