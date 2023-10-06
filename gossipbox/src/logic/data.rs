#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct SendItem {
    pub r#type: String,
    pub from_uuid: String,
    pub to_uuid: String,
    pub name: String,
    pub text: String,
    pub timestamp: u128,
}

impl From<&str> for SendItem {
    fn from(text: &str) -> Self {
        match serde_json::from_str::<SendItem>(&text) {
            Ok(item) => item,
            _ => SendItem::default(),
        }
    }
}
