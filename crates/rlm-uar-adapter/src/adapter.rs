#[cfg(test)]
mod tests {
    use super::*;
    use rlm_core::types::{ReplResult, RlmEvent, RlmEventData};
    use std::time::SystemTime;
    use tokio_stream::StreamExt;

    fn create_test_event(data: RlmEventData) -> RlmEvent {
        RlmEvent {
            event_id: "evt-1".to_string(),
            request_id: "req-1".to_string(),
            timestamp: SystemTime::now(),
            data,
        }
    }

    #[tokio::test]
    async fn test_adapt_chunk() {
        let events = vec![create_test_event(RlmEventData::Chunk {
            content: "Hello".to_string(),
            chunk_index: 0,
            is_final: false,
        })];

        let stream = tokio_stream::iter(events);
        let mut adapted = RlmUarAdapter::adapt_stream(stream);

        if let Some(event) = adapted.next().await {
            match event {
                NormalizedEvent::MessageDelta { text } => assert_eq!(text, "Hello"),
                _ => panic!("Expected MessageDelta"),
            }
        } else {
            panic!("No event produced");
        }
    }

    #[tokio::test]
    async fn test_adapt_repl_op() {
        let events = vec![create_test_event(RlmEventData::ReplOp {
            iteration: 1,
            code: "2+2".to_string(),
            result: ReplResult::Success {
                value: "4".to_string(),
            },
        })];

        let stream = tokio_stream::iter(events);
        let mut adapted = RlmUarAdapter::adapt_stream(stream);

        if let Some(event) = adapted.next().await {
            match event {
                NormalizedEvent::ThinkingDelta { text } => {
                    assert!(text.contains("Executing Rhai code"));
                    assert!(text.contains("2+2"));
                    assert!(text.contains("4"));
                }
                _ => panic!("Expected ThinkingDelta"),
            }
        }
    }
}
