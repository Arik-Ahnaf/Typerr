#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod ui;

fn main() -> iced::Result {
    iced::application(app::App::default, app::App::update, app::App::view)
        .title("Typerr — Alpha 0.1")
        .font(
            include_bytes!("../assets/fonts/Google_Sans_Code/static/GoogleSansCode-Regular.ttf")
                .as_slice(),
        )
        .default_font(ui::theme::FONT)
        .font(include_bytes!("../assets/fonts/Jomhuria/Jomhuria-Regular.ttf").as_slice())
        .window(iced::window::Settings {
            size: iced::Size::new(700.0, 400.0),
            min_size: Some(iced::Size::new(700.0, 400.0)),
            ..Default::default()
        })
        .style(|_, _| iced::theme::Style {
            background_color: ui::theme::BACKGROUND,
            text_color: ui::theme::CORRECT,
        })
        .run()
}
