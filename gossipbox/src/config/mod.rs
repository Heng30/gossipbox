mod conf;
mod data;

pub use conf::{config, init, path, save, ui, app_uuid, net, name, set_name};
pub use data::Config;