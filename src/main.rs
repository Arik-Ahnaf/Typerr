#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod settings;
mod ui;

fn main() -> iced::Result {
    let startup = match typerr::update::install::bootstrap() {
        Ok(typerr::update::install::Boot::Exit) => return Ok(()),
        Ok(typerr::update::install::Boot::Run(startup)) => startup,
        Err(error) => {
            typerr::update::diagnostic(&error.to_string());
            if std::env::args_os()
                .nth(1)
                .is_some_and(|a| a == "--typerr-update-helper" || a == "--typerr-recover")
            {
                std::process::exit(1);
            }
            return iced::application::<_, (), iced::Theme, iced::Renderer>(
                move || error.to_string(),
                |_: &mut String, _: ()| iced::exit::<()>(),
                startup_error_view,
            )
            .title("Typerr update")
            .window_size((600., 250.))
            .run();
        }
    };
    let settings = settings::Settings::load();
    let window_size = iced::Size::new(
        700.0,
        ui::theme::minimum_window_height(settings.line_height()),
    );
    let startup = std::cell::RefCell::new(Some(startup));
    iced::application(
        move || {
            app::App::new(
                settings,
                startup
                    .borrow_mut()
                    .take()
                    .expect("one application startup"),
            )
        },
        app::App::update_task,
        app::App::view,
    )
    .title(concat!("Typerr — ", env!("CARGO_PKG_VERSION")))
    .font(
        include_bytes!("../assets/fonts/Google_Sans_Code/static/GoogleSansCode-Regular.ttf")
            .as_slice(),
    )
    .font(
        include_bytes!("../assets/fonts/Google_Sans_Code/static/GoogleSansCode-SemiBoldItalic.ttf")
            .as_slice(),
    )
    .default_font(ui::theme::FONT)
    .font(
        include_bytes!("../assets/fonts/Google_Sans_Code/static/GoogleSansCode-Italic.ttf")
            .as_slice(),
    )
    .font(
        include_bytes!("../assets/fonts/Google_Sans_Code/static/GoogleSansCode-Medium.ttf")
            .as_slice(),
    )
    .font(
        include_bytes!("../assets/fonts/Google_Sans_Code/static/GoogleSansCode-SemiBold.ttf")
            .as_slice(),
    )
    .font(
        include_bytes!("../assets/fonts/Google_Sans_Code/static/GoogleSansCode-Light.ttf")
            .as_slice(),
    )
    .font(include_bytes!("../assets/fonts/Jomhuria/Jomhuria-Regular.ttf").as_slice())
    .window(iced::window::Settings {
        size: window_size,
        exit_on_close_request: false,
        min_size: Some(window_size),
        #[cfg(target_os = "linux")]
        platform_specific: iced::window::settings::PlatformSpecific {
            application_id: "typerr".into(),
            ..Default::default()
        },
        icon: Some(
            iced::window::icon::from_rgba(
                include_bytes!("../assets/icons/typerr-logo.rgba").to_vec(),
                64,
                64,
            )
            .expect("valid bundled app icon"),
        ),
        ..Default::default()
    })
    .style(|_, _| iced::theme::Style {
        background_color: ui::theme::BACKGROUND,
        text_color: ui::theme::CORRECT,
    })
    .run()
}

fn startup_error_view(message: &String) -> iced::Element<'_, ()> {
    iced::widget::container(
        iced::widget::column![
            iced::widget::text("Typerr could not start").size(24),
            iced::widget::text(message).size(16),
            iced::widget::button("Close").on_press(())
        ]
        .spacing(20),
    )
    .padding(30)
    .into()
}
