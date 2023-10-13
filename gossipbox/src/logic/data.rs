use crate::config;
use uuid::Uuid;

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
pub struct FileInfo {
    pub id: String,
    pub name: String,
    pub total_size: u64,
}

impl From<&str> for FileInfo {
    fn from(text: &str) -> Self {
        match serde_json::from_str::<FileInfo>(&text) {
            Ok(item) => item,
            _ => FileInfo::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct DynFileSvrInfo {
    pub ips: Vec<String>,
    pub port: u16,
}

impl From<&str> for DynFileSvrInfo {
    fn from(text: &str) -> Self {
        match serde_json::from_str::<DynFileSvrInfo>(&text) {
            Ok(item) => item,
            _ => DynFileSvrInfo::default(),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct ChatImgArgs {
    pub dfi: DynFileSvrInfo,
}

#[derive(Default, Debug, Clone)]
pub struct ChatFileArgs {
    pub uuid: String,
    pub dfi: DynFileSvrInfo,
}

#[derive(Debug, Clone)]
pub enum RecvFileCBArgs {
    Image(ChatImgArgs),
    File(ChatFileArgs),
}
