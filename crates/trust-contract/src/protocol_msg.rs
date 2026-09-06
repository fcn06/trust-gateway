use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Standard wire envelope for the Negotiated Interaction Contract Protocol (spec §33).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct NicpMessage<T> {
    pub protocol: String,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub message_id: String,
    pub thread_id: String,
    pub sender: String,
    pub recipient: String,
    pub payload: T,
    pub timestamp: DateTime<Utc>,
}

impl<T> NicpMessage<T> {
    pub fn new(
        msg_type: impl Into<String>,
        thread_id: impl Into<String>,
        sender: impl Into<String>,
        recipient: impl Into<String>,
        payload: T,
    ) -> Self {
        Self {
            protocol: "nicp/1.0".to_string(),
            msg_type: msg_type.into(),
            message_id: format!("msg_{}", uuid::Uuid::new_v4()),
            thread_id: thread_id.into(),
            sender: sender.into(),
            recipient: recipient.into(),
            payload,
            timestamp: Utc::now(),
        }
    }
}
