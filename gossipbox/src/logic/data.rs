use crate::config;
use uuid::Uuid;

pub const CHUNK_ITEM_SIZE: usize = 64 * 1024;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MsgItem {
    pub r#type: String,
    pub msg_id: String,
    pub from_uuid: String,
    pub to_uuid: String,
    pub name: String,
    pub status: String,
    pub text: String,
}

impl From<&str> for MsgItem {
    fn from(text: &str) -> Self {
        match serde_json::from_str::<MsgItem>(&text) {
            Ok(item) => item,
            _ => MsgItem::default(),
        }
    }
}

impl Default for MsgItem {
    fn default() -> Self {
        Self {
            r#type: String::default(),
            msg_id: Uuid::new_v4().to_string(),
            from_uuid: config::app_uuid(),
            to_uuid: String::default(),
            name: config::chat().user_name,
            status: config::chat().user_status,
            text: String::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct ChunkItem {
    pub id: String,
    pub total_chunk: u32,
    pub current_chunk: u32,
    pub data: String,
}

impl From<&str> for ChunkItem {
    fn from(text: &str) -> Self {
        match serde_json::from_str::<ChunkItem>(&text) {
            Ok(item) => item,
            _ => ChunkItem::default(),
        }
    }
}
