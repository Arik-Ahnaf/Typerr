//! Native modal; PageHost owns keyboard navigation and app-side input isolation.
use super::theme;
use crate::app::Message;
use iced::{
    Border, Color, Element, Fill,
    widget::{button, column, container, progress_bar, row, scrollable, text},
};
use typerr::update::{Phase, Updater};
pub fn view(updater: &Updater) -> Element<'_, Message> {
    let (title,description)=match &updater.phase {
        Phase::Available => ("Update available", format!("Typerr {} is ready. You have {}.\nUpgrade will download, install, and restart Typerr.",updater.release.as_ref().map(|r|r.version.to_string()).unwrap_or_default(),env!("CARGO_PKG_VERSION"))),
        Phase::Downloading{..}=>("Downloading update","Your current installation stays intact until verification finishes.".into()),
        Phase::Verifying=>("Verifying update","Checking the download and preparing installation…".into()),
        Phase::Installing|Phase::Restarting=>("Installing update","Typerr will restart when the update is ready. Arch packages may request system authentication.".into()),
        Phase::Failed(error)=>("Update could not finish",error.clone()),
        _=>("Checking for updates",String::new()),
    };
    let (title, description) = if updater.cancelling() {
        (
            "Cancelling update",
            "Waiting for the current network operation to stop. Installed files are unchanged."
                .into(),
        )
    } else {
        (title, description)
    };
    let mut content = column![
        text(title)
            .size(23)
            .font(theme::SEMIBOLD_FONT)
            .color(theme::palette().correct),
        scrollable(text(description).size(14).color(theme::palette().correct))
            .height(iced::Length::Shrink)
    ]
    .spacing(18);
    if let Phase::Downloading { received, total } = updater.phase {
        content = content
            .push(
                progress_bar(0.0..=1.0, received as f32 / total.max(1) as f32)
                    .girth(8)
                    .style(|_| progress_bar::Style {
                        background: theme::palette().control_background.into(),
                        bar: theme::palette().blue.into(),
                        border: Border {
                            radius: 4.into(),
                            ..Default::default()
                        },
                    }),
            )
            .push(
                text(format!(
                    "{:.1} / {:.1} MB",
                    received as f64 / 1_048_576.,
                    total as f64 / 1_048_576.
                ))
                .size(12)
                .color(theme::palette().pending),
            );
    }
    let action = |label: &'static str, message, focused| {
        button(text(label).size(14))
            .padding([10, 14])
            .on_press(message)
            .style(move |_, status| {
                let hover = matches!(status, button::Status::Hovered | button::Status::Pressed);
                button::Style {
                    background: Some(theme::palette().control_background.into()),
                    text_color: theme::palette().correct,
                    border: Border {
                        color: if focused || hover {
                            theme::palette().blue
                        } else {
                            theme::palette().pending
                        },
                        width: if focused || hover { 2. } else { 1. },
                        radius: 8.into(),
                    },
                    ..Default::default()
                }
            })
    };
    match updater.phase {
        Phase::Available | Phase::Failed(_) => {
            let failed = matches!(updater.phase, Phase::Failed(_));
            content = content.push(
                row![
                    action(
                        if failed {
                            "Retry"
                        } else {
                            "Upgrade and restart"
                        },
                        Message::Upgrade,
                        updater.focus_primary
                    ),
                    action(
                        if failed { "Close" } else { "Later" },
                        Message::DismissUpdate,
                        !updater.focus_primary
                    )
                ]
                .spacing(12),
            );
        }
        Phase::Downloading { .. } | Phase::Verifying if !updater.cancelling() => {
            content = content.push(action("Cancel", Message::DismissUpdate, true));
        }
        _ => {}
    }
    let card = container(content)
        .padding(26)
        .width(490)
        .max_height(340)
        .style(|_| container::Style {
            background: Some(theme::palette().background.into()),
            border: Border {
                color: theme::palette().pending,
                width: 1.,
                radius: 12.into(),
            },
            ..Default::default()
        });
    container(card)
        .center_x(Fill)
        .center_y(Fill)
        .style(|_| container::Style {
            background: Some(Color::from_rgba(0., 0., 0., 0.65).into()),
            ..Default::default()
        })
        .into()
}
