#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod settings;
mod ui;

fn main() -> iced::Result {
    let settings = settings::Settings::load();
    let window_size = iced::Size::new(
        700.0,
        ui::theme::minimum_window_height(settings.line_height()),
    );
    iced::application(
        move || app::App::new(settings),
        app::App::update,
        app::App::view,
    )
    .title("Typerr — Alpha 0.1")
    .font(
        include_bytes!("../assets/fonts/Google_Sans_Code/static/GoogleSansCode-Regular.ttf")
            .as_slice(),
    )
    .default_font(ui::theme::FONT)
    .font(include_bytes!("../assets/fonts/Jomhuria/Jomhuria-Regular.ttf").as_slice())
    .window(iced::window::Settings {
        size: window_size,
        min_size: Some(window_size),
        ..Default::default()
    })
    .style(|_, _| iced::theme::Style {
        background_color: ui::theme::BACKGROUND,
        text_color: ui::theme::CORRECT,
    })
    .run()
}
