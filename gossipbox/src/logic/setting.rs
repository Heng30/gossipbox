use crate::config;
use crate::slint_generatedAppWindow::{AppWindow, Logic, Store};
use crate::util::translator::tr;
use slint::{ComponentHandle, Weak};

pub fn init(ui: &AppWindow) {
    let ui_cancel = ui.as_weak();
    let ui_ok = ui.as_weak();

    init_setting_dialog(ui.as_weak());

    ui.global::<Logic>().on_setting_cancel(move || {
        init_setting_dialog(ui_cancel.clone());
    });

    ui.global::<Logic>().on_setting_ok(move |setting_config| {
        let ui = ui_ok.unwrap();
        let mut config = config::config();

        config.ui.font_size = setting_config
            .ui
            .font_size
            .to_string()
            .parse()
            .unwrap_or(18);
        config.ui.font_family = setting_config.ui.font_family.to_string();
        config.ui.win_width = setting_config
            .ui
            .win_width
            .to_string()
            .parse()
            .unwrap_or(1200);
        config.ui.win_height = setting_config
            .ui
            .win_height
            .to_string()
            .parse()
            .unwrap_or(800);

        config.ui.language = setting_config.ui.language.to_string();

        config.chat.user_name = setting_config.chat.user_name.to_string();
        config.chat.user_status = setting_config.chat.user_status.to_string();

        match config::save(config) {
            Err(e) => {
                ui.global::<Logic>().invoke_show_message(
                    slint::format!("{}, {}: {:?}", tr("保存失败"), tr("原因"), e),
                    "warning".into(),
                );
            }
            _ => {
                init_setting_dialog(ui.as_weak());
                ui.global::<Logic>()
                    .invoke_show_message(tr("保存成功").into(), "success".into());
            }
        }
    });
}

fn init_setting_dialog(ui: Weak<AppWindow>) {
    let ui = ui.unwrap();
    let ui_config = config::ui();
    let chat_config = config::chat();

    let mut setting_dialog = ui.global::<Store>().get_setting_dialog_config();
    setting_dialog.ui.font_size = slint::format!("{}", ui_config.font_size);
    setting_dialog.ui.font_family = ui_config.font_family.into();
    setting_dialog.ui.win_width = slint::format!("{}", ui_config.win_width);
    setting_dialog.ui.win_height = slint::format!("{}", ui_config.win_height);
    setting_dialog.ui.language = ui_config.language.into();

    setting_dialog.chat.user_name = chat_config.user_name.into();
    setting_dialog.chat.user_status = chat_config.user_status.into();

    ui.global::<Store>()
        .set_setting_dialog_config(setting_dialog);
}
