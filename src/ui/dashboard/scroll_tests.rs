use super::*;
use iced::advanced::{
    Layout, Shell, clipboard, layout,
    renderer::{Headless, Renderer as _},
    widget::Tree,
};

#[test]
fn body_scrolls_at_each_preset_while_header_and_hit_targets_stay_fixed() {
    let mut renderer =
        iced::futures::executor::block_on(Renderer::new(theme::FONT, 16.into(), Some("tiny-skia")))
            .unwrap();
    let picture = crate::profile_picture::Picture::default();
    for size in [
        Size::new(700., 400.),
        Size::new(1001., 501.),
        Size::new(1400., 800.),
        Size::new(1800., 1000.),
    ] {
        let scale = super::super::sizing::Preset::for_window(size).scale();
        let mut view = view(None, "Eric", &picture.avatar);
        let mut tree = Tree::new(&view);
        let node =
            view.as_widget_mut()
                .layout(&mut tree, &renderer, &layout::Limits::new(size, size));
        let bounds = Rectangle::with_size(size);
        let capture = |view: &Element<'_, Message>, tree: &Tree, renderer: &mut Renderer| {
            renderer.reset(bounds);
            view.as_widget().draw(
                tree,
                renderer,
                &Theme::Dark,
                &iced::advanced::renderer::Style {
                    text_color: theme::palette().correct,
                },
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &bounds,
            );
            renderer.screenshot(
                Size::new(size.width as u32, size.height as u32),
                1.,
                theme::palette().background,
            )
        };
        let send = |view: &mut Element<'_, Message>,
                    tree: &mut Tree,
                    renderer: &Renderer,
                    event: mouse::Event,
                    position: Point| {
            let mut messages = vec![];
            view.as_widget_mut().update(
                tree,
                &iced::Event::Mouse(event),
                Layout::new(&node),
                mouse::Cursor::Available(position),
                renderer,
                &mut clipboard::Null,
                &mut Shell::new(&mut messages),
                &bounds,
            );
            messages
        };
        let before = capture(&view, &tree, &mut renderer);
        let avatar_center = Point::new(
            (size.width - 700. * scale) / 2. + 95. * scale,
            HEADER_HEIGHT + 80. * scale,
        );
        let click = mouse::Event::ButtonPressed(mouse::Button::Left);
        assert!(
            send(&mut view, &mut tree, &renderer, click, avatar_center)
                .iter()
                .any(|m| matches!(m, Message::ChoosePicture))
        );
        send(
            &mut view,
            &mut tree,
            &renderer,
            mouse::Event::ButtonReleased(mouse::Button::Left),
            avatar_center,
        );
        let wheel = |y| mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: 0., y },
        };
        send(
            &mut view,
            &mut tree,
            &renderer,
            wheel(-3000.),
            Point::new(size.width / 2., size.height - 50.),
        );
        let after = capture(&view, &tree, &mut renderer);
        let header_bytes = size.width as usize * HEADER_HEIGHT as usize * 4;
        assert_eq!(
            &before[..header_bytes],
            &after[..header_bytes],
            "fixed header at {size:?}"
        );
        assert_ne!(
            &before[header_bytes..],
            &after[header_bytes..],
            "scrolling body at {size:?}"
        );
        assert!(
            send(&mut view, &mut tree, &renderer, click, avatar_center).is_empty(),
            "old avatar location must move with content"
        );
        send(
            &mut view,
            &mut tree,
            &renderer,
            wheel(3000.),
            Point::new(100., 45.),
        );
        assert_eq!(
            after,
            capture(&view, &tree, &mut renderer),
            "header wheel must not scroll body"
        );
        send(
            &mut view,
            &mut tree,
            &renderer,
            click,
            Point::new(100., 45.),
        );
        assert!(
            send(
                &mut view,
                &mut tree,
                &renderer,
                mouse::Event::ButtonReleased(mouse::Button::Left),
                Point::new(100., 45.)
            )
            .iter()
            .any(|m| matches!(m, Message::BackToTyping))
        );
        send(
            &mut view,
            &mut tree,
            &renderer,
            wheel(3000.),
            Point::new(size.width / 2., size.height - 50.),
        );
        assert_eq!(
            before,
            capture(&view, &tree, &mut renderer),
            "scroll returns to the original geometry"
        );
    }
}
