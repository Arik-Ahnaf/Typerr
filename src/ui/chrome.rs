//! Native page chrome; all artwork is compiled into the executable.
use super::theme;
use crate::app::{Message, Mode};
use iced::widget::{Space, button, container, row, svg, text};
use iced::{Element, Fill, Font};
use typerr::words;

fn icon(bytes: &'static [u8], width: f32, height: f32) -> svg::Svg<'static> {
    svg(svg::Handle::from_memory(bytes))
        .width(width)
        .height(height)
        .style(|_, _| svg::Style {
            color: Some(theme::CORRECT),
        })
}

fn control(
    bytes: &'static [u8],
    active: bool,
    action: Option<Message>,
    opacity: f32,
) -> Element<'static, Message> {
    let glyph = icon(bytes, 20.0, 20.0)
        .opacity(opacity)
        .style(move |_, _| svg::Style {
            color: Some(if active { theme::BLUE } else { theme::CORRECT }),
        });
    button(container(glyph).center_x(35).center_y(30))
        .padding(0)
        .on_press_maybe(action)
        .style(|_, _| button::Style::default())
        .into()
}

fn group(
    left: Element<'static, Message>,
    right: Element<'static, Message>,
    opacity: f32,
) -> Element<'static, Message> {
    container(row![left, right])
        .padding([0, 5])
        .style(move |_| container::Style {
            background: Some(theme::alpha(theme::CONTROL_BACKGROUND, opacity).into()),
            border: iced::Border::default().rounded(8),
            ..Default::default()
        })
        .into()
}

pub fn brand() -> Element<'static, Message> {
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
    container(brand)
        .padding(iced::Padding::ZERO.top(30).left(50))
        .into()
}

pub fn header(
    mode: Mode,
    options: words::Options,
    opacity: f32,
    enabled: bool,
) -> Element<'static, Message> {
    let controls = row![
        group(
            control(
                include_bytes!("../../assets/icons/punctuation.svg"),
                options.punctuation,
                enabled.then_some(Message::TogglePunctuation),
                opacity
            ),
            control(
                include_bytes!("../../assets/icons/hash.svg"),
                options.numbers,
                enabled.then_some(Message::ToggleNumbers),
                opacity
            ),
            opacity,
        ),
        group(
            control(
                include_bytes!("../../assets/icons/time-mode.svg"),
                mode == Mode::Time,
                enabled.then_some(Message::SelectMode(Mode::Time)),
                opacity
            ),
            control(
                include_bytes!("../../assets/icons/word-mode.svg"),
                mode == Mode::Words,
                enabled.then_some(Message::SelectMode(Mode::Words)),
                opacity
            ),
            opacity,
        ),
    ]
    .spacing(10);
    container(row![Space::new().width(Fill), controls])
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

fn connection(
    bytes: &'static [u8],
    width: f32,
    name: &'static str,
    url: &'static str,
    opacity: f32,
    enabled: bool,
) -> Element<'static, Message> {
    button(
        row![
            icon(bytes, width, 16.0)
                .opacity(opacity)
                .style(move |_, _| svg::Style {
                    color: Some(if name == "Github" {
                        theme::CORRECT
                    } else {
                        theme::BLUE
                    })
                }),
            label(name)
        ]
        .spacing(5)
        .align_y(iced::Center),
    )
    .padding(0)
    .on_press_maybe(enabled.then_some(Message::OpenConnection(url)))
    .style(move |_, status| button::Style {
        text_color: theme::alpha(
            if matches!(status, button::Status::Hovered) {
                theme::BLUE
            } else {
                theme::CORRECT
            },
            opacity,
        ),
        ..Default::default()
    })
    .into()
}

pub fn footer(opacity: f32, enabled: bool) -> Element<'static, Message> {
    let links = row![
        connection(
            include_bytes!("../../assets/icons/github-logo.svg"),
            16.0,
            "Github",
            "https://github.com/Arik-Ahnaf/Typerr",
            opacity,
            enabled
        ),
        connection(
            include_bytes!("../../assets/icons/twitter-logo.svg"),
            19.0,
            "Twitter",
            "https://x.com/ArikAhnaf1",
            opacity,
            enabled
        ),
        connection(
            include_bytes!("../../assets/icons/discord-logo.svg"),
            15.0,
            "Discord",
            "https://discord.gg/FYehF2KCw4",
            opacity,
            enabled
        ),
    ]
    .spacing(10);
    let version = row![
        icon(
            include_bytes!("../../assets/icons/fa-branch.svg"),
            12.0,
            16.0
        )
        .opacity(opacity)
        .style(|_, _| svg::Style {
            color: Some(theme::ORANGE)
        }),
        label("V0.1").color(theme::alpha(theme::CORRECT, opacity))
    ]
    .spacing(5)
    .align_y(iced::Center);
    let palette = row![
        icon(
            include_bytes!("../../assets/icons/fa-pallet.svg"),
            16.0,
            16.0
        )
        .opacity(opacity)
        .style(|_, _| svg::Style {
            color: Some(theme::ORANGE)
        }),
        label(theme::NAME).color(theme::alpha(theme::CORRECT, opacity))
    ]
    .spacing(5)
    .align_y(iced::Center);
    container(row![links, Space::new().width(Fill), version, palette].spacing(10))
        .padding(iced::Padding::ZERO.left(20).right(20).bottom(22))
        .width(Fill)
        .into()
}
