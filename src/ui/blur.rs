//! Cached native offscreen rendering and separable box blur for an inert backdrop.
use crate::app::Message;
use iced::advanced::renderer::{Headless, Renderer as _};
use iced::advanced::{
    Layout, Widget, image, layout, mouse, renderer,
    widget::{Tree, tree},
};
use iced::{Element, Length, Rectangle, Renderer, Size, Theme};
use std::{
    cell::{Cell, RefCell},
    future::Future,
    task::{Context, Poll, Waker},
};

pub struct Blur<'a> {
    content: Element<'a, Message>,
}
struct State {
    size: Cell<Size>,
    renderer: RefCell<Renderer>,
    image: RefCell<Option<image::Handle>>,
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
            *state.image.borrow_mut() = None;
        }
        node
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
        if cached.is_none() {
            let mut offscreen = state.renderer.borrow_mut();
            let bounds = layout.bounds();
            offscreen.reset(bounds);
            self.content.as_widget().draw(
                &tree.children[0],
                &mut offscreen,
                theme,
                style,
                layout,
                mouse::Cursor::Unavailable,
                viewport,
            );
            let size = Size::new(bounds.width.ceil() as u32, bounds.height.ceil() as u32);
            let mut pixels = offscreen.screenshot(size, 1., super::theme::BACKGROUND);
            blur(&mut pixels, size.width as usize, size.height as usize);
            *cached = Some(image::Handle::from_rgba(size.width, size.height, pixels));
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

fn blur(pixels: &mut [u8], width: usize, height: usize) {
    if width == 0 || height == 0 {
        return;
    }
    let mut scratch = vec![0; pixels.len()];
    // Three separable radius-two passes approximate a Gaussian blur (sigma ~= 2.45).
    for _ in 0..3 {
        for y in 0..height {
            for x in 0..width {
                for c in 0..4 {
                    let sum: u32 = (-2isize..=2)
                        .map(|dx| {
                            pixels[(y * width + x.saturating_add_signed(dx).min(width - 1)) * 4 + c]
                                as u32
                        })
                        .sum();
                    scratch[(y * width + x) * 4 + c] = (sum / 5) as u8;
                }
            }
        }
        for y in 0..height {
            for x in 0..width {
                for c in 0..4 {
                    let sum: u32 = (-2isize..=2)
                        .map(|dy| {
                            scratch
                                [(y.saturating_add_signed(dy).min(height - 1) * width + x) * 4 + c]
                                as u32
                        })
                        .sum();
                    pixels[(y * width + x) * 4 + c] = (sum / 5) as u8;
                }
            }
        }
    }
}
