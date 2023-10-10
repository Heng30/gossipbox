use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(skip)]
    pub config_path: String,

    #[serde(skip)]
    pub db_path: String,

    #[serde(skip)]
    pub cache_dir: String,

    pub app_uuid: String,
    pub net: String,

    pub ui: UI,
    pub chat: Chat,
    pub swarm: Swarm,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            config_path: "".to_string(),
            db_path: "".to_string(),
            cache_dir: "".to_string(),
            app_uuid: Uuid::new_v4().to_string(),
            net: "goosip-net".to_string(),
            ui: UI::default(),
            chat: Chat::default(),
            swarm: Swarm::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UI {
    pub font_size: u32,
    pub font_family: String,
    pub win_width: u32,
    pub win_height: u32,
    pub language: String,
}

impl Default for UI {
    fn default() -> Self {
        Self {
            font_size: 18,
            font_family: "SourceHanSerifCN".to_string(),
            win_width: 1200,
            win_height: 800,
            language: "cn".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Swarm {
    pub enable_ping: bool,
    pub ping_interval: u64,
    pub keepalive_interval: u64,
}

impl Default for Swarm {
    fn default() -> Self {
        Self {
            enable_ping: true,
            ping_interval: 10,
            keepalive_interval: 10,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Chat {
    pub user_name: String,
    pub user_status: String,
}

impl Default for Chat {
    fn default() -> Self {
        Self {
            user_name: "匿名用户".to_string(),
            user_status: "在线".to_string(),
        }
    }
}
