//! Native Figma 143:15 crop card and Dashboard avatar interaction.
use super::theme;
use crate::{
    app::Message,
    profile_picture::{Crop, Picture},
};
use iced::{
    Border, Color, Element, Event, Fill, Point, Rectangle, Renderer, Theme, Vector, mouse,
    widget::{
        Space, button,
        canvas::{self, Frame, Path},
        column, container, row, slider, stack, svg, text,
    },
};

pub fn avatar(handle: &iced::widget::image::Handle, size: f32) -> Element<'_, Message> {
    stack![
        iced::widget::image(handle.clone()).width(size).height(size),
        canvas::Canvas::new(Avatar).width(size).height(size)
    ]
    .width(size)
    .height(size)
    .into()
}
struct Avatar;
impl canvas::Program<Message> for Avatar {
    type State = ();
    fn update(
        &self,
        _: &mut (),
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        if matches!(
            event,
            Event::Mouse(mouse::Event::CursorMoved { .. } | mouse::Event::CursorLeft)
        ) {
            return Some(canvas::Action::request_redraw());
        }
        if !over_circle(cursor, bounds) {
            return None;
        }
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                Some(canvas::Action::publish(Message::ChoosePicture).and_capture())
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                Some(canvas::Action::request_redraw())
            }
            _ => None,
        }
    }
    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        _: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        if over_circle(cursor, bounds) {
            frame.fill(
                &Path::circle(frame.center(), bounds.width / 2.),
                Color::from_rgba(0., 0., 0., 0.45),
            );
            let size = bounds.width * (25. / 80.);
            frame.draw_svg(
                Rectangle {
                    x: (bounds.width - size) / 2.,
                    y: (bounds.height - size) / 2.,
                    width: size,
                    height: size,
                },
                iced::advanced::svg::Svg::new(svg::Handle::from_memory(
                    include_bytes!("../../assets/icons/fa-pencil.svg").as_slice(),
                ))
                .color(theme::CORRECT),
            );
        }
        vec![frame.into_geometry()]
    }
    fn mouse_interaction(
        &self,
        _: &(),
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if over_circle(cursor, bounds) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::None
        }
    }
}
fn over_circle(cursor: mouse::Cursor, bounds: Rectangle) -> bool {
    cursor.position_in(bounds).is_some_and(|p| {
        p.distance(Point::new(bounds.width / 2., bounds.height / 2.)) <= bounds.width / 2.
    })
}

#[derive(Default)]
pub struct Drag {
    start: Option<(Point, Vector)>,
}
struct Preview<'a>(&'a Crop, bool);
impl canvas::Program<Message> for Preview<'_> {
    type State = Drag;
    fn update(
        &self,
        state: &mut Drag,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        if !self.1 {
            state.start = None;
            return None;
        }
        match event {
            Event::Window(iced::window::Event::Unfocused | iced::window::Event::Resized(_)) => {
                state.start = None;
                None
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if cursor.is_over(bounds) =>
            {
                state.start = cursor.position().map(|p| (p, self.0.offset));
                Some(canvas::Action::request_redraw().and_capture())
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => state
                .start
                .take()
                .map(|_| canvas::Action::request_redraw().and_capture()),
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                state.start.map(|(start, offset)| {
                    canvas::Action::publish(Message::PanPicture(offset + (*position - start)))
                        .and_capture()
                })
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) if cursor.is_over(bounds) => {
                let amount = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y * 0.1,
                    mouse::ScrollDelta::Pixels { y, .. } => *y * 0.005,
                };
                Some(
                    canvas::Action::publish(Message::ZoomPicture(self.0.zoom + amount))
                        .and_capture(),
                )
            }
            _ => None,
        }
    }
    fn draw(
        &self,
        _: &Drag,
        renderer: &Renderer,
        _: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill(
            &Path::rectangle(Point::ORIGIN, bounds.size()),
            theme::CONTROL_BACKGROUND,
        );
        frame.with_clip(Rectangle::with_size(bounds.size()), |frame| {
            frame.draw_image(self.0.bounds(Point::new(85., 65.)), &self.0.source.handle);
        });
        vec![frame.into_geometry()]
    }
    fn mouse_interaction(
        &self,
        state: &Drag,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if !self.1 {
            mouse::Interaction::None
        } else if state.start.is_some() {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(bounds) {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::None
        }
    }
}
// Separate layers keep raster images below vector masks on every Iced backend.
struct Mask;
impl canvas::Program<Message> for Mask {
    type State = ();
    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        _: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        // Dim the rejected image area; the circular selection remains clear.
        let mask = Path::new(|p| {
            p.rectangle(Point::ORIGIN, bounds.size());
            p.circle(Point::new(85., 65.), 48.);
        });
        frame.fill(
            &mask,
            canvas::Fill {
                style: Color::from_rgba(0., 0., 0., 0.35).into(),
                rule: canvas::fill::Rule::EvenOdd,
            },
        );
        // Cover just the rounded outer corners, retaining the exact preview slot.
        let corners = Path::new(|p| {
            p.rectangle(Point::ORIGIN, bounds.size());
            p.rounded_rectangle(Point::ORIGIN, bounds.size(), 8.into());
        });
        frame.fill(
            &corners,
            canvas::Fill {
                style: theme::BACKGROUND.into(),
                rule: canvas::fill::Rule::EvenOdd,
            },
        );
        vec![frame.into_geometry()]
    }
}
fn at<'a>(content: impl Into<Element<'a, Message>>, x: f32, y: f32) -> Element<'a, Message> {
    container(content)
        .padding(iced::Padding::ZERO.left(x).top(y))
        .into()
}
fn action(
    label: &'static str,
    message: Message,
    primary: bool,
    enabled: bool,
    focused: bool,
) -> Element<'static, Message> {
    button(
        text(label)
            .font(theme::MEDIUM_FONT)
            .size(12)
            .line_height(1.25)
            .center(),
    )
    .width(70)
    .height(20)
    .padding(0)
    .on_press_maybe(enabled.then_some(message))
    .style(move |_, status| button::Style {
        background: Some(
            if primary {
                theme::PENDING
            } else {
                theme::CONTROL_BACKGROUND
            }
            .into(),
        ),
        text_color: theme::CORRECT,
        border: Border {
            radius: 8.into(),
            width: if focused || status == button::Status::Hovered {
                1.
            } else {
                0.
            },
            color: theme::CORRECT,
        },
        ..Default::default()
    })
    .into()
}

pub fn view(picture: &Picture) -> Element<'_, Message> {
    let busy = picture.busy();
    // The crop card appears only after the OS picker returns an image.
    if busy && picture.crop.is_none() {
        return Space::new().width(Fill).height(Fill).into();
    }
    let preview: Element<'_, Message> = if let Some(crop) = &picture.crop {
        stack![
            canvas::Canvas::new(Preview(crop, !busy))
                .width(170)
                .height(130),
            canvas::Canvas::new(Mask).width(170).height(130),
            at(
                svg(svg::Handle::from_memory(
                    include_bytes!("../../assets/icons/pfp-selection.svg").as_slice()
                ))
                .width(100)
                .height(100),
                35.,
                15.
            )
        ]
        .width(170)
        .height(130)
        .into()
    } else {
        container(
            text(if busy {
                "Choose an image…"
            } else {
                "Cannot open image"
            })
            .size(12)
            .center(),
        )
        .center_x(170)
        .center_y(130)
        .into()
    };
    let close = button(
        svg(svg::Handle::from_memory(
            include_bytes!("../../assets/icons/pfp-close.svg").as_slice(),
        ))
        .width(13)
        .height(12.2),
    )
    .width(20)
    .height(20)
    .padding(3)
    .on_press_maybe((!busy).then_some(Message::DiscardPicture))
    .style(|_, status| button::Style {
        background: (status == button::Status::Hovered).then_some(theme::CONTROL_BACKGROUND.into()),
        ..Default::default()
    });
    let zoom: Element<'_, Message> = if let Some(crop) = &picture.crop {
        slider(1.0..=4.0, crop.zoom, Message::ZoomPicture)
            .step(0.01_f32)
            .width(155)
            .height(12)
            .style(move |_, _| slider::Style {
                rail: slider::Rail {
                    backgrounds: (theme::CORRECT.into(), theme::CORRECT.into()),
                    width: 4.,
                    border: Border::default().rounded(2),
                },
                handle: slider::Handle {
                    shape: slider::HandleShape::Circle { radius: 6. },
                    background: theme::PENDING.into(),
                    border_width: if picture.focus == 0 { 1. } else { 0. },
                    border_color: theme::CORRECT,
                },
            })
            .into()
    } else {
        Space::new().into()
    };
    let card = container(
        stack![
            at(
                text("Edit Image")
                    .font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..theme::FONT
                    })
                    .size(12)
                    .line_height(1.25),
                15.,
                10.
            ),
            at(close, 170., 6.5),
            at(preview, 15., 30.),
            at(zoom, 22.5, 170.),
            at(
                row![
                    action(
                        "Discard",
                        Message::DiscardPicture,
                        false,
                        !busy,
                        picture.focus == 1
                    ),
                    action(
                        if picture.saving() {
                            "Saving…"
                        } else {
                            "Save"
                        },
                        Message::SavePicture,
                        true,
                        !busy && picture.crop.is_some(),
                        picture.focus == 2
                    )
                ]
                .spacing(10),
                25.,
                190.
            ),
        ]
        .width(200)
        .height(230),
    )
    .style(|_| container::Style {
        background: Some(theme::BACKGROUND.into()),
        border: Border::default().rounded(8),
        text_color: Some(theme::CORRECT),
        ..Default::default()
    });
    let mut contents = column![card].align_x(iced::Center).spacing(10);
    if let Some(error) = &picture.error {
        contents = contents
            .push(container(text(error).size(12).color(theme::INCORRECT).center()).max_width(440));
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

#[cfg(test)]
mod tests {
    use super::*;
    use canvas::Program;
    #[test]
    fn avatar_hit_target_is_circular_and_hover_exit_redraws() {
        let avatar = Avatar;
        let bounds = Rectangle::new(Point::new(55., 85.), iced::Size::new(80., 80.));
        let press = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        let center = mouse::Cursor::Available(bounds.center());
        assert!(matches!(
            avatar
                .update(&mut (), &press, bounds, center)
                .unwrap()
                .into_inner()
                .0,
            Some(Message::ChoosePicture)
        ));
        assert!(
            avatar
                .update(
                    &mut (),
                    &press,
                    bounds,
                    mouse::Cursor::Available(bounds.position())
                )
                .is_none()
        );
        assert!(
            avatar
                .update(
                    &mut (),
                    &Event::Mouse(mouse::Event::CursorLeft),
                    bounds,
                    mouse::Cursor::Unavailable
                )
                .is_some()
        );
    }
    #[test]
    fn crop_drag_keeps_origin_and_release_or_focus_loss_ends_it() {
        let crop = Crop::new(crate::profile_picture::placeholder());
        let preview = Preview(&crop, true);
        let mut state = Drag::default();
        let bounds = Rectangle::new(Point::new(100., 100.), iced::Size::new(170., 130.));
        let cursor = mouse::Cursor::Available(Point::new(150., 150.));
        preview
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                bounds,
                cursor,
            )
            .unwrap();
        let action = preview
            .update(
                &mut state,
                &Event::Mouse(mouse::Event::CursorMoved {
                    position: Point::new(190., 170.),
                }),
                bounds,
                cursor,
            )
            .unwrap()
            .into_inner();
        assert!(matches!(action.0,Some(Message::PanPicture(v)) if v == Vector::new(40.,20.)));
        preview.update(
            &mut state,
            &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            bounds,
            cursor,
        );
        assert!(state.start.is_none());
        preview.update(
            &mut state,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            bounds,
            cursor,
        );
        preview.update(
            &mut state,
            &Event::Window(iced::window::Event::Unfocused),
            bounds,
            cursor,
        );
        assert!(state.start.is_none());
        assert!(
            Preview(&crop, false)
                .update(
                    &mut state,
                    &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                    bounds,
                    cursor
                )
                .is_none()
        );
    }
}

#[cfg(test)]
mod rendering_tests {
    use super::*;
    use iced::advanced::{
        Layout, layout,
        renderer::{Headless, Renderer as _},
        widget::Tree,
    };
    fn render(mut view: Element<'_, Message>, size: iced::Size, cursor: mouse::Cursor) -> Vec<u8> {
        let mut renderer = iced::futures::executor::block_on(Renderer::new(
            theme::FONT,
            16.into(),
            Some("tiny-skia"),
        ))
        .unwrap();
        let mut tree = Tree::new(&view);
        let node = view.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(iced::Size::ZERO, size),
        );
        let bounds = Rectangle::with_size(size);
        renderer.reset(bounds);
        view.as_widget().draw(
            &tree,
            &mut renderer,
            &Theme::Dark,
            &iced::advanced::renderer::Style {
                text_color: theme::CORRECT,
            },
            Layout::new(&node),
            cursor,
            &bounds,
        );
        renderer.screenshot(
            iced::Size::new(size.width as u32, size.height as u32),
            1.,
            theme::BACKGROUND,
        )
    }
    #[test]
    fn image_cannot_cover_crop_ring_or_hover_overlay() {
        let picture = Picture::default();
        let normal = render(
            avatar(&picture.avatar, 80.),
            iced::Size::new(80., 80.),
            mouse::Cursor::Unavailable,
        );
        let hover = render(
            avatar(&picture.avatar, 80.),
            iced::Size::new(80., 80.),
            mouse::Cursor::Available(Point::new(40., 40.)),
        );
        // Clear of the pencil: hovering dims the original image. Center includes the pencil.
        let edge = (40 * 80 + 10) * 4;
        assert!(hover[edge] < normal[edge]);
        assert!(
            hover
                .as_chunks::<4>()
                .0
                .iter()
                .any(|p| p[0] > 230 && p[1] > 230)
        );
        let mut picture = Picture::default();
        picture.crop = Some(Crop::new(crate::profile_picture::placeholder()));
        let pixels = render(
            view(&picture),
            iced::Size::new(200., 230.),
            mouse::Cursor::Unavailable,
        );
        let ring = (95 * 200 + 51) * 4;
        assert!(
            pixels[ring] > 230 && pixels[ring + 1] > 230,
            "crop ring must stay above the photograph"
        );
    }
}
