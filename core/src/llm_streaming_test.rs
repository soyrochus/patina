#[cfg(test)]
mod streaming_tests {
    use crate::llm::LlmDriver;
    use crate::state::{ChatMessage, MessageRole};

    #[tokio::test]
    async fn test_mock_provider_streaming() {
        let driver = LlmDriver::fake().await;
        
        let messages = vec![
            ChatMessage::new(MessageRole::User, "Hello, can you help me?"),
        ];

        let mut stream = driver
            .respond_streaming(&messages, Some("mock"), Some(0.7))
            .await
            .expect("Failed to start streaming");

        let mut accumulated = String::new();
        let mut chunk_count = 0;
        let mut done = false;

        while let Some(result) = stream.recv().await {
            match result {
                Ok(chunk) => {
                    if chunk.done {
                        done = true;
                        break;
                    }
                    accumulated.push_str(&chunk.delta);
                    chunk_count += 1;
                }
                Err(e) => panic!("Stream error: {}", e),
            }
        }

        assert!(done, "Stream should complete with done=true");
        assert!(chunk_count > 0, "Should receive at least one chunk");
        assert!(!accumulated.is_empty(), "Should accumulate content");
        assert!(accumulated.contains("Mock"), "Mock response should contain 'Mock'");
    }

    #[tokio::test]
    async fn test_streaming_with_empty_history() {
        let driver = LlmDriver::fake().await;
        
        let messages: Vec<ChatMessage> = vec![];

        let mut stream = driver
            .respond_streaming(&messages, Some("mock"), Some(0.7))
            .await
            .expect("Failed to start streaming");

        let mut done = false;
        while let Some(result) = stream.recv().await {
            match result {
                Ok(chunk) => {
                    if chunk.done {
                        done = true;
                        break;
                    }
                }
                Err(e) => panic!("Stream error: {}", e),
            }
        }

        assert!(done, "Stream should complete even with empty history");
    }
}
