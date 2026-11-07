//! First-launch profile prompt matching Figma 122:10.
use super::theme;
use crate::{app::Message, settings::valid_name};
use iced::{
    Border, Color, Element, Fill,
    widget::{Space, button, column, container, text, text_input},
};

pub fn view<'a>(name: &'a str, error: Option<&'a str>) -> Element<'a, Message> {
    let input = text_input("Type here...", name)
        .id("name-input")
        .on_input(Message::NameChanged)
        .on_submit(Message::SaveName)
        .font(theme::ITALIC_FONT)
        .size(12)
        .padding([7, 10])
        .width(200)
        .style(|_, _| text_input::Style {
            background: theme::palette().control_background.into(),
            border: Border::default().rounded(8),
            icon: theme::palette().pending,
            placeholder: theme::palette().pending,
            value: theme::palette().correct,
            selection: theme::palette().pending,
        });
    let save = button(
        text("Save")
            .font(theme::MEDIUM_FONT)
            .size(12)
            .line_height(1.25),
    )
    .width(60)
    .height(20)
    .padding([2, 15])
    .on_press_maybe(valid_name(name).map(|_| Message::SaveName))
    .style(|_, status| button::Style {
        background: Some(theme::palette().pending.into()),
        text_color: if status == button::Status::Hovered {
            Color::WHITE
        } else {
            theme::palette().correct
        },
        border: Border::default().rounded(8),
        ..Default::default()
    });
    let card = container(
        column![
            text("What is your name?")
                .font(theme::SEMIBOLD_FONT)
                .size(16)
                .line_height(1.25)
                .color(theme::palette().correct),
            Space::new().height(15),
            input,
            Space::new().height(20),
            save,
        ]
        .align_x(iced::Center),
    )
    .padding(iced::Padding::ZERO.top(25))
    .width(300)
    .height(150)
    .align_x(iced::Center)
    .style(|_| container::Style {
        background: Some(theme::palette().background.into()),
        border: Border::default().rounded(8),
        ..Default::default()
    });
    let mut contents = column![card].align_x(iced::Center).spacing(12);
    if let Some(error) = error {
        contents = contents
            .push(container(text(error).size(12).color(theme::palette().incorrect)).max_width(460));
    }
    container(contents)
        .center_x(Fill)
        .center_y(Fill)
        .style(|_| container::Style {
            background: Some(Color::from_rgba(0., 0., 0., 0.25).into()),
            ..Default::default()
        })
        .into()
}
