mod auth_profiles;
mod auth_settings;
mod backend;
mod branch_dialog;
mod commit_dialog;
mod credentials;
mod form_controls;
mod settings;
mod settings_config;
mod settings_git;
mod settings_native;
mod settings_theme;
mod split_button;
backend::install!();
mod alert;
mod avatar;
mod context_menu;
mod dialog;
mod diff_model;
mod diff_viewer;
mod encoding_dialog;
mod file_history_dialog;
mod git_action_dialog;
mod graph;
mod history;
mod history_graph;
mod history_grid;
mod history_temp;
#[cfg(test)]
mod ime_tests;
mod line_endings;
mod loading;
mod panel_layout;
mod path_display;
mod progress;
mod recent_repository_dialog;
mod repository_dialog;
mod repository_search;
mod search_panel;
mod scrolling;
mod text_encoding;
mod text_input;
#[cfg(test)]
#[path = "../vendor/gpui/src/elements/text_layout_cache.rs"]
mod text_layout_cache;
mod text_selection;
mod theme;
mod ui;
mod virtual_list;
mod workspace_backend;
use gpui::{
    App, AppContext, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size,
};
fn main() {
    let options = match backend::Options::parse(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("{error}\n\n{}", backend::HELP);
            std::process::exit(2);
        }
    };
    if options.help {
        print!("{}", backend::HELP);
        return;
    }
    backend::initialize(options);
    let session_writer = workspace_backend::shared_writer();
    Application::new().run(|cx: &mut App| {
        cx.set_global(text_encoding::TextEncoding::default());
        match avatar::AvatarClient::new() {
            Ok(client) => cx.set_http_client(std::sync::Arc::new(client)),
            Err(_) => eprintln!("프로필 사진 조회를 초기화하지 못했습니다."),
        }
        text_input::init(cx);
        let bounds = Bounds::centered(None, size(px(1440.), px(900.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(1100.), px(700.))),
                titlebar: Some(TitlebarOptions {
                    title: Some("Feather Git".into()),
                    appears_transparent: cfg!(target_os = "windows"),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| ui::Workspace::load(window, cx)),
        )
        .expect("창을 열 수 없습니다");
        cx.activate(true);
    });
    history_temp::shutdown();
    session_writer.flush();
}
