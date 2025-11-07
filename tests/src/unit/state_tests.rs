use patina_core::state::AppState;
use patina_core::store::TranscriptStore;
use patina_core::{llm::LlmDriver, state::MessageRole};
use std::sync::Arc;

fn test_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime")
}

#[test]
fn app_state_records_messages() {
    let runtime = test_runtime();
    let store = TranscriptStore::in_memory();
    let driver = runtime.block_on(LlmDriver::fake());
    let state = Arc::new(AppState::new(store, driver));

    runtime
        .block_on(state.send_user_message("hello world"))
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
