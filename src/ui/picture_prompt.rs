//! Native Figma 143:15 crop card and Dashboard avatar interaction.
use super::theme;
use crate::{
    app::Message,
    profile_picture::{Crop, Picture},
};
use iced::{
    Border, Color, Element, Event, Fill, Point, Rectangle, Renderer, Size, Theme, Vector, mouse,
    widget::{
        Space, button,
        canvas::{self, Frame, Path},
        column, container, responsive, row, scrollable, slider, stack, svg, text,
    },
};

fn modal_scale(size: Size, has_error: bool) -> f32 {
    let requested = super::sizing::Preset::for_window(size).scale();
    let height = if has_error { 290. } else { 230. };
    [2., 1.5, 1.25, 1.]
        .into_iter()
        .find(|scale| {
            *scale <= requested
                && 200. * scale <= size.width - 32.
                && height * scale <= size.height - 32.
        })
        .unwrap_or(1.)
}

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
                .color(theme::palette().correct),
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
                    canvas::Action::publish(Message::PanPicture(
                        offset + (*position - start) / (bounds.width / 170.),
                    ))
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
            theme::palette().control_background,
        );
        // A clipped subframe has its own transform. Keep the image rectangle and
        // its clip in actual canvas pixels instead of relying on the parent scale.
        let scale = bounds.width / 170.;
        let crop = self.0.bounds(Point::new(85., 65.));
        let image_bounds = Rectangle {
            x: crop.x * scale,
            y: crop.y * scale,
            width: crop.width * scale,
            height: crop.height * scale,
        };
        frame.with_clip(Rectangle::with_size(bounds.size()), |frame| {
            frame.draw_image(image_bounds, &self.0.source.handle);
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
        frame.scale(bounds.width / 170.);
        let size = Size::new(170., 130.);
        // Dim the rejected image area; the circular selection remains clear.
        let mask = Path::new(|p| {
            p.rectangle(Point::ORIGIN, size);
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
            p.rectangle(Point::ORIGIN, size);
            p.rounded_rectangle(Point::ORIGIN, size, 8.into());
        });
        frame.fill(
            &corners,
            canvas::Fill {
                style: theme::palette().background.into(),
                rule: canvas::fill::Rule::EvenOdd,
            },
        );
        vec![frame.into_geometry()]
    }
}
fn at<'a>(
    content: impl Into<Element<'a, Message>>,
    x: f32,
    y: f32,
    scale: f32,
) -> Element<'a, Message> {
    container(content)
        .padding(iced::Padding::ZERO.left(x * scale).top(y * scale))
        .into()
}
fn action(
    label: &'static str,
    message: Message,
    primary: bool,
    enabled: bool,
    focused: bool,
    scale: f32,
) -> Element<'static, Message> {
    let scaled = move |value| value * scale;
    button(
        text(label)
            .font(theme::MEDIUM_FONT)
            .size(scaled(12.))
            .line_height(1.25)
            .center(),
    )
    .width(scaled(70.))
    .height(scaled(20.))
    .padding(0)
    .on_press_maybe(enabled.then_some(message))
    .style(move |_, status| button::Style {
        background: Some(
            if primary {
                theme::palette().pending
            } else {
                theme::palette().control_background
            }
            .into(),
        ),
        text_color: theme::palette().correct,
        border: Border {
            radius: scaled(8.).into(),
            width: if focused || status == button::Status::Hovered {
                scaled(1.)
            } else {
                0.
            },
            color: theme::palette().correct,
        },
        ..Default::default()
    })
    .into()
}

pub fn view(picture: &Picture) -> Element<'_, Message> {
    responsive(move |size| view_scaled(picture, modal_scale(size, picture.error.is_some()))).into()
}

fn view_scaled(picture: &Picture, scale: f32) -> Element<'_, Message> {
    let scaled = move |value| value * scale;
    let busy = picture.busy();
    // The crop card appears only after the OS picker returns an image.
    if busy && picture.crop.is_none() {
        return Space::new().width(Fill).height(Fill).into();
    }
    let preview: Element<'_, Message> = if let Some(crop) = &picture.crop {
        stack![
            canvas::Canvas::new(Preview(crop, !busy))
                .width(scaled(170.))
                .height(scaled(130.)),
            canvas::Canvas::new(Mask)
                .width(scaled(170.))
                .height(scaled(130.)),
            at(
                svg(svg::Handle::from_memory(
                    include_bytes!("../../assets/icons/pfp-selection.svg").as_slice()
                ))
                .width(scaled(100.))
                .height(scaled(100.)),
                35.,
                15.,
                scale,
            )
        ]
        .width(scaled(170.))
        .height(scaled(130.))
        .into()
    } else {
        container(
            text(if busy {
                "Choose an image…"
            } else {
                "Cannot open image"
            })
            .size(scaled(12.))
            .center(),
        )
        .center_x(scaled(170.))
        .center_y(scaled(130.))
        .into()
    };
    let close = button(
        svg(svg::Handle::from_memory(
            include_bytes!("../../assets/icons/pfp-close.svg").as_slice(),
        ))
        .width(scaled(13.))
        .height(scaled(12.2)),
    )
    .width(scaled(20.))
    .height(scaled(20.))
    .padding(scaled(3.))
    .on_press_maybe((!busy).then_some(Message::DiscardPicture))
    .style(|_, status| button::Style {
        background: (status == button::Status::Hovered)
            .then_some(theme::palette().control_background.into()),
        ..Default::default()
    });
    let zoom: Element<'_, Message> = if let Some(crop) = &picture.crop {
        slider(1.0..=4.0, crop.zoom, Message::ZoomPicture)
            .step(0.01_f32)
            .width(scaled(155.))
            .height(scaled(12.))
            .style(move |_, _| slider::Style {
                rail: slider::Rail {
                    backgrounds: (
                        theme::palette().correct.into(),
                        theme::palette().correct.into(),
                    ),
                    width: scaled(4.),
                    border: Border::default().rounded(scaled(2.)),
                },
                handle: slider::Handle {
                    shape: slider::HandleShape::Circle { radius: scaled(6.) },
                    background: theme::palette().pending.into(),
                    border_width: if picture.focus == 0 { scaled(1.) } else { 0. },
                    border_color: theme::palette().correct,
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
                    .size(scaled(12.))
                    .line_height(1.25),
                15.,
                10.,
                scale,
            ),
            at(close, 170., 6.5, scale),
            at(preview, 15., 30., scale),
            at(zoom, 22.5, 170., scale),
            at(
                row![
                    action(
                        "Discard",
                        Message::DiscardPicture,
                        false,
                        !busy,
                        picture.focus == 1,
                        scale,
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
                        picture.focus == 2,
                        scale,
                    )
                ]
                .spacing(scaled(10.)),
                25.,
                190.,
                scale,
            ),
        ]
        .width(scaled(200.))
        .height(scaled(230.)),
    )
    .style(move |_| container::Style {
        background: Some(theme::palette().background.into()),
        border: Border::default().rounded(scaled(8.)),
        text_color: Some(theme::palette().correct),
        ..Default::default()
    });
    let mut contents = column![card].align_x(iced::Center).spacing(scaled(10.));
    if let Some(error) = &picture.error {
        contents = contents.push(
            scrollable(
                text(error)
                    .size(scaled(10.))
                    .color(theme::palette().incorrect)
                    .center(),
            )
            .width(scaled(200.))
            .height(scaled(50.)),
        );
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
        let bounds = Rectangle::new(Point::new(100., 100.), iced::Size::new(340., 260.));
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
                    position: Point::new(230., 190.),
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
                text_color: theme::palette().correct,
            },
            Layout::new(&node),
            cursor,
            &bounds,
        );
        renderer.screenshot(
            iced::Size::new(size.width as u32, size.height as u32),
            1.,
            theme::palette().background,
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
            view_scaled(&picture, 2.),
            iced::Size::new(400., 460.),
            mouse::Cursor::Unavailable,
        );
        let ring = (190 * 400 + 102) * 4;
        assert!(
            pixels[ring] > 230 && pixels[ring + 1] > 230,
            "crop ring must stay above the photograph"
        );
    }

    #[test]
    fn preview_matches_saved_crop_across_window_sizes_zoom_and_pan() {
        let dir = tempfile::tempdir().unwrap();
        for (width, height) in [(320, 180), (180, 320)] {
            let path = dir.path().join("gradient.png");
            image::RgbImage::from_fn(width, height, |x, y| {
                image::Rgb([(x * 220 / width) as u8, (y * 220 / height) as u8, 90])
            })
            .save(&path)
            .unwrap();
            let mut picture = Picture::default();
            let mut crop = Crop::new(crate::profile_picture::read(&path).unwrap());
            for zoom in [1., 2.5, 4.] {
                crop.set_zoom(zoom);
                crop.pan(Vector::new(15., -12.));
                let saved = crop.output();
                picture.crop = Some(crop.clone());
                for size in [Size::new(700., 400.), Size::new(1000., 650.)] {
                    let scale = modal_scale(size, false);
                    let pixels = render(view(&picture), size, mouse::Cursor::Unavailable);
                    let origin = Point::new(
                        (size.width - 200. * scale) / 2.,
                        (size.height - 230. * scale) / 2.,
                    );
                    // Sample well inside each quadrant, away from the mask/ring.
                    for (dx, dy) in [(-20., -20.), (20., -20.), (-20., 20.), (20., 20.)] {
                        let x = (origin.x + (100. + dx) * scale) as usize;
                        let y = (origin.y + (95. + dy) * scale) as usize;
                        let offset = (y * size.width as usize + x) * 4;
                        let expected = saved.get_pixel(
                            ((50. + dx) * 512. / 100.) as u32,
                            ((50. + dy) * 512. / 100.) as u32,
                        );
                        for channel in 0..3 {
                            assert!(
                                pixels[offset + channel].abs_diff(expected[channel]) <= 5,
                                "preview differs from saved crop at {size:?}, zoom {zoom}, point ({dx}, {dy})"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn bottom_of_both_buttons_remains_clickable_after_resizing() {
        use iced::advanced::{Shell, clipboard};
        let mut picture = Picture::default();
        picture.crop = Some(Crop::new(crate::profile_picture::placeholder()));
        let renderer = iced::futures::executor::block_on(Renderer::new(
            theme::FONT,
            16.into(),
            Some("tiny-skia"),
        ))
        .unwrap();
        let mut view = view(&picture);
        let mut tree = Tree::new(&view);
        for size in [
            Size::new(1000., 650.),
            Size::new(700., 400.),
            Size::new(800., 450.),
        ] {
            let scale = modal_scale(size, false);
            let node =
                view.as_widget_mut()
                    .layout(&mut tree, &renderer, &layout::Limits::new(size, size));
            for (x, save) in [(60., false), (140., true)] {
                let cursor = mouse::Cursor::Available(Point::new(
                    (size.width - 200. * scale) / 2. + x * scale,
                    (size.height - 230. * scale) / 2. + 208. * scale,
                ));
                let mut messages = Vec::new();
                for event in [
                    mouse::Event::ButtonPressed(mouse::Button::Left),
                    mouse::Event::ButtonReleased(mouse::Button::Left),
                ] {
                    view.as_widget_mut().update(
                        &mut tree,
                        &Event::Mouse(event),
                        Layout::new(&node),
                        cursor,
                        &renderer,
                        &mut clipboard::Null,
                        &mut Shell::new(&mut messages),
                        &Rectangle::with_size(size),
                    );
                }
                assert!(
                    messages.iter().any(|message| matches!(
                        (save, message),
                        (true, Message::SavePicture) | (false, Message::DiscardPicture)
                    )),
                    "button clipped at {size:?}"
                );
            }
        }
    }
}
