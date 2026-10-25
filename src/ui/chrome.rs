//! Native page chrome; all artwork is compiled into the executable.
use super::theme;
use crate::app::Message;
use iced::widget::{Space, button, container, row, svg, text};
use iced::{Element, Fill, Font};

fn icon(bytes: &'static [u8], width: f32, height: f32) -> svg::Svg<'static> {
    svg(svg::Handle::from_memory(bytes))
        .width(width)
        .height(height)
}

fn control(bytes: &'static [u8], active: bool) -> Element<'static, Message> {
    // These four controls are intentionally inert until their modes exist.
    let glyph = icon(bytes, 20.0, 20.0).style(move |_, _| svg::Style {
        color: Some(if active { theme::BLUE } else { theme::CORRECT }),
    });
    button(container(glyph).center_x(35).center_y(30))
        .padding(0)
        .style(|_, _| button::Style::default())
        .into()
}

fn group(
    left: Element<'static, Message>,
    right: Element<'static, Message>,
) -> Element<'static, Message> {
    container(row![left, right])
        .padding([0, 5])
        .style(|_| container::Style {
            background: Some(theme::CONTROL_BACKGROUND.into()),
            border: iced::Border::default().rounded(8),
            ..Default::default()
        })
        .into()
}

pub fn header() -> Element<'static, Message> {
    let brand = row![
        container(icon(
            include_bytes!("../../assets/icons/keyboard.svg"),
            37.0,
            25.0
        ))
        .padding(iced::Padding::ZERO.top(7)),
        text("Typerr")
            .font(theme::BRAND_FONT)
            .size(48)
            .line_height(1.0),
    ]
    .spacing(8);
    let controls = row![
        group(
            control(include_bytes!("../../assets/icons/punctuation.svg"), false),
            control(include_bytes!("../../assets/icons/Vector-1.svg"), false),
        ),
        group(
            control(include_bytes!("../../assets/icons/time-mode.svg"), false),
            control(include_bytes!("../../assets/icons/word-mode.svg"), true),
        ),
    ]
    .spacing(10);
    container(row![brand, Space::new().width(Fill), controls])
        .padding(iced::Padding::ZERO.top(30).left(50).right(80))
        .width(Fill)
        .into()
}

fn label(value: &'static str) -> iced::widget::Text<'static, iced::Theme, iced::Renderer> {
    text(value)
        .font(Font::with_name("Google Sans Code"))
        .size(12)
        .line_height(iced::widget::text::LineHeight::Absolute(15.0.into()))
}

fn connection(bytes: &'static [u8], width: f32, name: &'static str) -> Element<'static, Message> {
    button(
        row![icon(bytes, width, 16.0), label(name)]
            .spacing(5)
            .align_y(iced::Center),
    )
    .padding(0)
    .on_press(Message::OpenConnection)
    .style(|_, status| button::Style {
        text_color: if matches!(status, button::Status::Hovered) {
            theme::BLUE
        } else {
            theme::CORRECT
        },
        ..Default::default()
    })
    .into()
}

pub fn footer() -> Element<'static, Message> {
    let links = row![
        connection(
            include_bytes!("../../assets/icons/github-logo.svg"),
            16.0,
            "Github"
        ),
        connection(
            include_bytes!("../../assets/icons/twitter-logo.svg"),
            19.0,
            "Twitter"
        ),
        connection(
            include_bytes!("../../assets/icons/discord-logo.svg"),
            15.0,
            "Discord"
        ),
    ]
    .spacing(10);
    let version = row![
        icon(
            include_bytes!("../../assets/icons/fa-branch.svg"),
            12.0,
            16.0
        ),
        label("V0.1")
    ]
    .spacing(5)
    .align_y(iced::Center);
    let palette = row![
        icon(
            include_bytes!("../../assets/icons/fa-pallet.svg"),
            16.0,
            16.0
        ),
        label("Default")
    ]
    .spacing(5)
    .align_y(iced::Center);
    container(row![links, Space::new().width(Fill), version, palette].spacing(10))
        .padding(iced::Padding::ZERO.left(20).right(20).bottom(22))
        .width(Fill)
        .into()
}
