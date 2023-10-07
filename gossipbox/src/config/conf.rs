use super::data::{self, Config};
use crate::CResult;
use log::debug;
use platform_dirs::AppDirs;
use std::cell::RefCell;
use std::sync::Mutex;
use std::fs;

lazy_static! {
    pub static ref CONFIG: Mutex<RefCell<Config>> = Mutex::new(RefCell::new(Config::default()));
}

pub fn init() {
    if let Err(e) = CONFIG.lock().unwrap().borrow_mut().init() {
        panic!("{:?}", e);
    }
}

pub fn ui() -> data::UI {
    CONFIG.lock().unwrap().borrow().ui.clone()
}

pub fn chat() -> data::Chat {
    CONFIG.lock().unwrap().borrow().chat.clone()
}

pub fn swarm() -> data::Swarm {
    CONFIG.lock().unwrap().borrow().swarm.clone()
}

pub fn app_uuid() -> String {
    CONFIG.lock().unwrap().borrow().app_uuid.clone()
}

pub fn net() -> String {
    CONFIG.lock().unwrap().borrow().net.clone()
}

#[allow(dead_code)]
pub fn path() -> (String, String) {
    let conf = CONFIG.lock().unwrap();
    let conf = conf.borrow();

    (conf.config_path.clone(), conf.db_path.clone())
}

pub fn config() -> data::Config {
    CONFIG.lock().unwrap().borrow().clone()
}

pub fn save(conf: data::Config) -> CResult {
    let config = CONFIG.lock().unwrap();
    let mut config = config.borrow_mut();
    *config = conf;
    config.save()
}

impl Config {
    pub fn init(&mut self) -> CResult {
        let app_dirs = AppDirs::new(Some("gossipbox"), true).unwrap();
        Self::init_app_dir(&app_dirs)?;
        self.init_config(&app_dirs)?;
        self.init_path()?;
        self.load()?;
        debug!("{:?}", self);
        Ok(())
    }

    fn init_app_dir(app_dirs: &AppDirs) -> CResult {
        fs::create_dir_all(&app_dirs.data_dir)?;
        fs::create_dir_all(&app_dirs.config_dir)?;
        Ok(())
    }

    fn init_path(&self) -> CResult {
        fs::create_dir_all(&self.cache_dir)?;
        Ok(())
    }

    fn init_config(&mut self, app_dirs: &AppDirs) -> CResult {
        self.config_path = app_dirs
            .config_dir
            .join("gossipbox.conf")
            .to_str()
            .unwrap()
            .to_string();

        self.db_path = app_dirs
            .data_dir
            .join("gossipbox.db")
            .to_str()
            .unwrap()
            .to_string();

        self.cache_dir = app_dirs
            .data_dir
            .join("cache")
            .to_str()
            .unwrap()
            .to_string();

        Ok(())
    }

    fn load(&mut self) -> CResult {
        match fs::read_to_string(&self.config_path) {
            Ok(text) => match serde_json::from_str::<Config>(&text) {
                Ok(c) => {
                    self.app_uuid = c.app_uuid;
                    self.net = c.net;
                    self.swarm = c.swarm;
                    self.ui = c.ui;
                    self.chat = c.chat;
                    Ok(())
                }
                Err(e) => Err(anyhow::anyhow!("{}", e.to_string()).into()),
            },

            Err(_) => match serde_json::to_string_pretty(self) {
                Ok(text) => Ok(fs::write(&self.config_path, text)?),
                Err(e) => Err(anyhow::anyhow!("{}", e.to_string()).into()),
            },
        }
    }

    pub fn save(&self) -> CResult {
        match serde_json::to_string_pretty(self) {
            Ok(text) => Ok(fs::write(&self.config_path, text)?),
            Err(e) => Err(anyhow::anyhow!("{}", e.to_string()).into()),
        }
    }
}
