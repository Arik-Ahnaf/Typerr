//! Cached native offscreen rendering and separable box blur for an inert backdrop.
use crate::app::Message;
use iced::advanced::renderer::{Headless, Renderer as _};
use iced::advanced::{
    Clipboard, Layout, Shell, Widget, image, layout, mouse, renderer,
    widget::{Tree, tree},
};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme};
use std::{
    cell::{Cell, RefCell},
    future::Future,
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};

pub struct Blur<'a> {
    content: Element<'a, Message>,
}
struct State {
    size: Cell<Size>,
    renderer: RefCell<Renderer>,
    image: RefCell<Option<image::Handle>>,
    refresh_at: Cell<Option<Instant>>,
}
impl State {
    fn new() -> Self {
        // The bundled software backend initializes synchronously; never create a GPU here.
        let mut future = std::pin::pin!(<Renderer as Headless>::new(
            super::theme::FONT,
            16.into(),
            Some("tiny-skia")
        ));
        let Poll::Ready(Some(renderer)) = future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        else {
            panic!("bundled tiny-skia renderer must initialize synchronously");
        };
        Self {
            size: Cell::new(Size::ZERO),
            renderer: RefCell::new(renderer),
            image: RefCell::new(None),
            refresh_at: Cell::new(None),
        }
    }
}
impl<'a> Blur<'a> {
    pub fn new(content: Element<'a, Message>) -> Self {
        Self { content }
    }
}
impl Widget<Message, Theme, Renderer> for Blur<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }
    fn state(&self) -> tree::State {
        tree::State::new(State::new())
    }
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }
    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }
    fn layout(&mut self, tree: &mut Tree, _: &Renderer, limits: &layout::Limits) -> layout::Node {
        let state = tree.state.downcast_ref::<State>();
        let node = self.content.as_widget_mut().layout(
            &mut tree.children[0],
            &state.renderer.borrow(),
            limits,
        );
        if state.size.replace(node.size()) != node.size() {
            // Reuse the backdrop during a live resize; refresh after it settles.
            state
                .refresh_at
                .set(Some(Instant::now() + Duration::from_millis(120)));
        }
        node
    }
    fn update(
        &mut self,
        tree: &mut Tree,
        _: &Event,
        _: Layout<'_>,
        _: mouse::Cursor,
        _: &Renderer,
        _: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _: &Rectangle,
    ) {
        if let Some(deadline) = tree.state.downcast_ref::<State>().refresh_at.get() {
            shell.request_redraw_at(deadline);
        }
        // The backdrop stays inert: never forward events to the underlying page.
    }
    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        _: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let mut cached = state.image.borrow_mut();
        if cached.is_none()
            || state
                .refresh_at
                .get()
                .is_some_and(|at| Instant::now() >= at)
        {
            let mut offscreen = state.renderer.borrow_mut();
            let bounds = layout.bounds();
            // A blurred backdrop needs far fewer pixels than the sharp foreground.
            // Bound the work even on a maximized/high-DPI window.
            let scale = 0.5_f32.min(512. / bounds.width.max(bounds.height).max(1.));
            let size = Size::new(
                (bounds.width * scale).ceil().max(1.) as u32,
                (bounds.height * scale).ceil().max(1.) as u32,
            );
            offscreen.reset(Rectangle::with_size(Size::new(
                size.width as f32,
                size.height as f32,
            )));
            // Render in reduced pixel coordinates with a 1x screenshot viewport.
            // tiny-skia's screenshot damage region is physical, so using a
            // sub-1x viewport scale would incorrectly clip the bottom/right.
            offscreen.with_transformation(iced::Transformation::scale(scale), |offscreen| {
                self.content.as_widget().draw(
                    &tree.children[0],
                    offscreen,
                    theme,
                    style,
                    layout,
                    mouse::Cursor::Unavailable,
                    viewport,
                );
            });
            let pixels = offscreen.screenshot(size, 1., super::theme::BACKGROUND);
            let pixels = blur(pixels, size.width, size.height);
            *cached = Some(image::Handle::from_rgba(size.width, size.height, pixels));
            state.refresh_at.set(None);
        }
        image::Renderer::draw_image(
            renderer,
            image::Image::new(cached.as_ref().unwrap().clone()),
            layout.bounds(),
            *viewport,
        );
    }
}
impl<'a> From<Blur<'a>> for Element<'a, Message> {
    fn from(value: Blur<'a>) -> Self {
        Self::new(value)
    }
}

fn blur(pixels: Vec<u8>, width: u32, height: u32) -> Vec<u8> {
    if width == 0 || height == 0 {
        return pixels;
    }
    // Reuse the optimized linear-time three-pass box blur in our image dependency.
    // Sigma 1.2 at half resolution corresponds to about 2.4 logical pixels.
    let source = ::image::RgbaImage::from_raw(width, height, pixels)
        .expect("renderer supplies a complete RGBA buffer");
    ::image::imageops::fast_blur(&source, 1.2).into_raw()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::{
        Color,
        widget::{Space, container, row},
    };

    #[test]
    fn backdrop_covers_full_viewport_and_refreshes_only_after_resize_settles() {
        let mut renderer = State::new().renderer.into_inner();
        let panel = |color: Color| {
            container(Space::new().width(Length::Fill).height(Length::Fill)).style(move |_| {
                container::Style {
                    background: Some(color.into()),
                    ..Default::default()
                }
            })
        };
        let mut view = Blur::new(
            row![
                panel(Color::from_rgb(1., 0., 0.)),
                panel(Color::from_rgb(0., 0., 1.))
            ]
            .into(),
        );
        let mut tree = Tree::new(&view as &dyn Widget<Message, Theme, Renderer>);
        let small = Size::new(700., 400.);
        let node = view.layout(&mut tree, &renderer, &layout::Limits::new(small, small));
        let style = renderer::Style {
            text_color: Color::WHITE,
        };
        let draw = |view: &Blur<'_>, tree: &Tree, renderer: &mut Renderer, node: &layout::Node| {
            renderer.reset(Rectangle::with_size(node.size()));
            view.draw(
                tree,
                renderer,
                &Theme::Dark,
                &style,
                Layout::new(node),
                mouse::Cursor::Unavailable,
                &Rectangle::with_size(node.size()),
            );
        };
        draw(&view, &tree, &mut renderer, &node);
        let pixels = renderer.screenshot(Size::new(700, 400), 1., Color::BLACK);
        for (x, channel) in [(20usize, 0usize), (680, 2)] {
            let pixel = (380 * 700 + x) * 4;
            assert!(
                pixels[pixel + channel] > 240,
                "backdrop cropped at bottom edge"
            );
            assert_eq!(pixels[pixel + 3], 255);
        }
        let before = tree
            .state
            .downcast_ref::<State>()
            .image
            .borrow()
            .as_ref()
            .unwrap()
            .id();
        let large = Size::new(1000., 650.);
        let node = view.layout(&mut tree, &renderer, &layout::Limits::new(large, large));
        draw(&view, &tree, &mut renderer, &node);
        let state = tree.state.downcast_ref::<State>();
        assert_eq!(state.image.borrow().as_ref().unwrap().id(), before);
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        view.update(
            &mut tree,
            &Event::Window(iced::window::Event::RedrawRequested(Instant::now())),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &renderer,
            &mut iced::advanced::clipboard::Null,
            &mut shell,
            &Rectangle::with_size(large),
        );
        assert!(matches!(
            shell.redraw_request(),
            iced::window::RedrawRequest::At(_)
        ));
        tree.state
            .downcast_ref::<State>()
            .refresh_at
            .set(Some(Instant::now()));
        draw(&view, &tree, &mut renderer, &node);
        let state = tree.state.downcast_ref::<State>();
        assert_ne!(state.image.borrow().as_ref().unwrap().id(), before);
        assert!(state.refresh_at.get().is_none());
    }
}
