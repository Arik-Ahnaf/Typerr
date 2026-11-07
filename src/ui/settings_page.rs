//! Native Settings form. App owns persistence; the view never performs disk I/O.
use super::{chrome, theme};
use crate::app::Message;
use iced::{
    Border, Element, Fill, Padding, Rectangle, Size, Task,
    advanced::{
        text as native_text,
        text::Paragraph,
        widget::{Id, Operation, operation},
    },
    widget::{
        Space, button, column, combo_box, container, responsive, row, scrollable, stack, svg, text,
        text_input,
    },
};

const PANEL: &str = "settings-options";
const SEARCH: &str = "settings-search";
const FONT_SIZE: &str = "settings-font-size";
const STRIDE: f32 = 120.;
const SAVE_ROW_HEIGHT: f32 = 43.;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    General,
    Caret,
    Theme,
    Sound,
}
impl Category {
    pub const ALL: [Self; 4] = [Self::General, Self::Caret, Self::Theme, Self::Sound];
    fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Caret => "Caret",
            Self::Theme => "Theme",
            Self::Sound => "Sound",
        }
    }
    fn rows(self) -> &'static [&'static str] {
        match self {
            Self::General => &["Font size", "Font family"],
            Self::Caret => &["Caret Style"],
            Self::Theme => &["Theme"],
            Self::Sound => &["Sound"],
        }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Save,
    Search(String),
    FontSize(String),
    CommitFontSize,
    FontFamily(String),
    Caret(String),
    Theme(String),
    Sound(String),
    Category(Category, f32),
    Scrolled(scrollable::Viewport, f32),
    Tab(bool),
    InspectFocus(bool),
    Focus(FocusSnapshot, bool),
}
fn message(event: Event) -> Message {
    Message::Settings(event)
}

pub struct State {
    pub query: String,
    pub font_size: String,
    valid_font_size: f32,
    pub save_result: Option<Result<(), String>>,
    family: String,
    pub caret: String,
    selected_theme: String,
    pub sound: String,
    pub active: Option<Category>,
    families: combo_box::State<String>,
    carets: combo_box::State<String>,
    themes: combo_box::State<String>,
    sounds: combo_box::State<String>,
}
impl Default for State {
    fn default() -> Self {
        fn choices(values: &[&str]) -> combo_box::State<String> {
            combo_box::State::new(values.iter().map(|s| (*s).into()).collect())
        }
        Self {
            query: String::new(),
            font_size: "16".into(),
            valid_font_size: 16.,
            save_result: None,
            family: "Google Sans Code".into(),
            caret: "Line".into(),
            selected_theme: "Dracula".into(),
            sound: "Off".into(),
            active: Some(Category::General),
            families: choices(&["Google Sans Code"]),
            carets: choices(&["Line", "Block", "Underline"]),
            themes: choices(&crate::settings::Theme::NAMES),
            sounds: choices(&["Off", "On"]),
        }
    }
}
impl State {
    pub fn from_parameters(parameters: crate::settings::Parameters) -> Self {
        Self {
            font_size: parameters.font_size.to_string(),
            valid_font_size: parameters.font_size,
            family: parameters.font_family,
            caret: parameters.caret_style,
            selected_theme: parameters.theme,
            sound: if parameters.sound { "On" } else { "Off" }.into(),
            ..Self::default()
        }
    }
    pub fn parameters(&mut self) -> crate::settings::Parameters {
        self.commit();
        crate::settings::Parameters {
            font_size: self.valid_font_size,
            font_family: self.family.clone(),
            caret_style: self.caret.clone(),
            theme: self.selected_theme.clone(),
            sound: self.sound == "On",
        }
    }
    pub fn open(&mut self) -> Task<Message> {
        self.query.clear();
        self.active = Some(Category::General);
        iced::widget::operation::focus(SEARCH).chain(scroll_to(0.))
    }
    pub fn commit(&mut self) {
        if let Ok(value @ 8..=48) = self.font_size.trim().parse::<u8>() {
            self.valid_font_size = f32::from(value);
        }
        self.font_size = self.valid_font_size.to_string();
    }
    fn matches(&self, category: Category, label: &str) -> bool {
        let query = self.query.trim().to_lowercase();
        category.label().to_lowercase().contains(&query) || label.to_lowercase().contains(&query)
    }
    fn categories(&self) -> Vec<Category> {
        Category::ALL
            .into_iter()
            .filter(|&c| c.rows().iter().any(|label| self.matches(c, label)))
            .collect()
    }
    pub fn update(&mut self, event: Event) -> Task<Message> {
        if matches!(
            event,
            Event::FontSize(_)
                | Event::FontFamily(_)
                | Event::Caret(_)
                | Event::Theme(_)
                | Event::Sound(_)
        ) {
            self.save_result = None;
        }
        match event {
            Event::Save => {} // App handles writes after modal/page guards.
            Event::Search(query) => {
                self.commit();
                self.query = query;
                self.active = self.categories().first().copied();
                return scroll_to(0.);
            }
            Event::FontSize(value) => self.font_size = value,
            Event::CommitFontSize => self.commit(),
            Event::FontFamily(value) => self.family = value,
            Event::Caret(value) => self.caret = value,
            Event::Theme(value) => self.selected_theme = value,
            Event::Sound(value) => self.sound = value,
            Event::Category(category, scale_y) => {
                self.commit();
                self.query.clear();
                self.active = Some(category);
                return iced::widget::operation::focus("settings-no-field")
                    .chain(scroll_to(category as usize as f32 * STRIDE * scale_y));
            }
            Event::Scrolled(viewport, scale_y) => {
                let stride = STRIDE * scale_y;
                self.active = self
                    .categories()
                    .get(((viewport.absolute_offset().y + 0.5) / stride).floor() as usize)
                    .copied();
            }
            Event::Tab(backwards) => {
                self.commit();
                let focus = if backwards {
                    iced::widget::operation::focus_previous()
                } else {
                    iced::widget::operation::focus_next()
                };
                return focus.chain(inspect_focus(true));
            }
            Event::InspectFocus(reveal) => return inspect_focus(reveal),
            Event::Focus(snapshot, reveal) => {
                if !snapshot.font_size_focused {
                    self.commit();
                }
                if reveal && let Some(y) = snapshot.reveal_offset() {
                    return scroll_to(y);
                }
            }
        }
        Task::none()
    }
}
fn scroll_to(y: f32) -> Task<Message> {
    iced::widget::operation::scroll_to(PANEL, iced::widget::operation::AbsoluteOffset { x: 0., y })
}

// Native focus traversal includes combo boxes. Reveal keyboard-focused rows even
// when their section is outside the options viewport; mouse clicks never jump it.
#[derive(Debug, Clone, Default)]
pub struct FocusSnapshot {
    focused: Option<Rectangle>,
    panel: Option<(Rectangle, f32)>,
    font_size_focused: bool,
}
impl FocusSnapshot {
    fn reveal_offset(&self) -> Option<f32> {
        let (panel, offset) = self.panel?;
        let focused = self.focused?;
        if focused.y < panel.y {
            return None;
        } // Search is outside the scrollable.
        let top = focused.y - panel.y;
        let bottom = top + focused.height;
        if top < offset {
            Some((top - 10.).max(0.))
        } else if bottom > offset + panel.height {
            Some(bottom - panel.height + 10.)
        } else {
            None
        }
    }
}
impl Operation<FocusSnapshot> for FocusSnapshot {
    fn traverse(&mut self, visit: &mut dyn FnMut(&mut dyn Operation<FocusSnapshot>)) {
        visit(self);
    }
    fn focusable(
        &mut self,
        id: Option<&Id>,
        bounds: Rectangle,
        state: &mut dyn operation::Focusable,
    ) {
        if state.is_focused() {
            self.focused = Some(bounds);
            self.font_size_focused = id == Some(&Id::new(FONT_SIZE));
        }
    }
    fn scrollable(
        &mut self,
        id: Option<&Id>,
        bounds: Rectangle,
        _: Rectangle,
        translation: iced::Vector,
        _: &mut dyn operation::Scrollable,
    ) {
        if id == Some(&Id::new(PANEL)) {
            self.panel = Some((bounds, translation.y));
        }
    }
    fn finish(&self) -> operation::Outcome<FocusSnapshot> {
        operation::Outcome::Some(self.clone())
    }
}
fn inspect_focus(reveal: bool) -> Task<Message> {
    iced::advanced::widget::operate(FocusSnapshot::default())
        .map(move |snapshot| message(Event::Focus(snapshot, reveal)))
}

#[derive(Clone, Copy)]
struct Scale {
    x: f32,
    y: f32,
    text: f32,
}
impl Scale {
    // Branding is fixed, so the search should not drift down with window height.
    fn search_scale(self) -> f32 {
        (1. + (self.text - 1.) * 0.25).clamp(1., 1.25)
    }
    fn content_top(self) -> f32 {
        80. + 50. * self.search_scale()
    }
    fn panel_height(self) -> f32 {
        380. * self.y - self.content_top()
    }
    fn options_height(self) -> f32 {
        self.panel_height() - SAVE_ROW_HEIGHT * self.y
    }
    fn new(size: Size) -> Self {
        let x = size.width / 700.;
        let y = size.height / 400.;
        Self {
            x,
            y,
            text: x.min(y),
        }
    }
}
fn panel_style(_: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(theme::palette().control_background.into()),
        border: Border::default().rounded(4),
        ..Default::default()
    }
}
fn input_style(
    background: iced::Color,
    radius: f32,
    status: text_input::Status,
) -> text_input::Style {
    text_input::Style {
        background: background.into(),
        border: Border {
            color: theme::palette().pending,
            width: if matches!(status, text_input::Status::Focused { .. }) {
                1.
            } else {
                0.
            },
            radius: radius.into(),
        },
        icon: theme::palette().correct,
        placeholder: theme::palette().correct,
        value: theme::palette().correct,
        selection: theme::palette().pending,
    }
}
fn text_width(value: &str, size: f32) -> f32 {
    type Paragraph = <iced::Renderer as native_text::Renderer>::Paragraph;
    Paragraph::with_text(native_text::Text {
        content: value,
        bounds: Size::INFINITE,
        size: size.into(),
        line_height: native_text::LineHeight::Relative(1.25),
        font: theme::FONT,
        align_x: native_text::Alignment::Left,
        align_y: iced::alignment::Vertical::Top,
        shaping: native_text::Shaping::Basic,
        wrapping: native_text::Wrapping::None,
    })
    .min_width()
}
fn choice<'a>(
    options: &'a combo_box::State<String>,
    selected: &'a str,
    change: fn(String) -> Event,
    scale: Scale,
) -> Element<'a, Message> {
    let width = 161. * scale.x;
    let left = ((width - text_width(selected, 12. * scale.text)) / 2.).max(5. * scale.text);
    let selected = selected.to_owned();
    let input = combo_box(options, &selected, Some(&selected), move |value| {
        message(change(value))
    })
    .font(theme::FONT)
    .size(12. * scale.text)
    .line_height(1.25)
    .width(width)
    .padding(Padding {
        top: 2. * scale.text,
        bottom: 2. * scale.text,
        left,
        right: 15. * scale.text,
    })
    .input_style(move |_, status| input_style(theme::palette().background, 8. * scale.text, status))
    .menu_style(|_| iced::widget::overlay::menu::Style {
        background: theme::palette().background.into(),
        border: Border::default().rounded(4),
        text_color: theme::palette().correct,
        selected_text_color: theme::palette().correct,
        selected_background: theme::palette().pending.into(),
        shadow: Default::default(),
    });
    let arrow = container(
        svg(svg::Handle::from_memory(include_bytes!(
            "../../assets/icons/settings-chevron.svg"
        )))
        .width(11.0001 * scale.text)
        .height(7.33339 * scale.text),
    )
    .width(width)
    .height(19. * scale.text)
    .align_x(iced::Right)
    .align_y(iced::Center)
    .padding(Padding::ZERO.right(8. * scale.text));
    stack![super::settings_choice::wrap(input), arrow].into()
}

fn section<'a>(
    state: &'a State,
    category: Category,
    scale: Scale,
    last: bool,
) -> Element<'a, Message> {
    let heading = row![
        text(category.label())
            .font(theme::SEMIBOLD_FONT)
            .size(16. * scale.text)
            .line_height(1.25),
        container(Space::new().height(3. * scale.text))
            .width(Fill)
            .style(|_| container::Style {
                background: Some(theme::palette().correct.into()),
                border: Border::default().rounded(8),
                ..Default::default()
            }),
    ]
    .spacing(5. * scale.text)
    .align_y(iced::Center);
    let mut rows = column![heading].spacing(3. * scale.text);
    for &label in category.rows() {
        if !state.matches(category, label) {
            continue;
        }
        let field = match (category, label) {
            (Category::General, "Font size") => text_input("16", &state.font_size)
                .id(FONT_SIZE)
                .on_input(|s| message(Event::FontSize(s)))
                .on_submit(message(Event::CommitFontSize))
                .font(theme::FONT)
                .size(12. * scale.text)
                .line_height(1.25)
                .align_x(iced::alignment::Horizontal::Center)
                .padding([2. * scale.text, 5. * scale.text])
                .width(161. * scale.x)
                .style(move |_, status| {
                    input_style(theme::palette().background, 8. * scale.text, status)
                })
                .into(),
            (Category::General, _) => {
                choice(&state.families, &state.family, Event::FontFamily, scale)
            }
            (Category::Caret, _) => choice(&state.carets, &state.caret, Event::Caret, scale),
            (Category::Theme, _) => {
                choice(&state.themes, &state.selected_theme, Event::Theme, scale)
            }
            (Category::Sound, _) => choice(&state.sounds, &state.sound, Event::Sound, scale),
        };
        rows = rows.push(
            container(
                row![
                    text(format!("{label}:"))
                        .font(theme::FONT)
                        .size(12. * scale.text)
                        .line_height(1.25),
                    field
                ]
                .spacing(5. * scale.text)
                .align_y(iced::Center),
            )
            .padding(Padding::ZERO.left(20. * scale.x).bottom(3. * scale.text)),
        );
    }
    container(rows)
        .width(Fill)
        .height(if last {
            scale.options_height()
        } else {
            STRIDE * scale.y
        })
        .padding(
            Padding::ZERO
                .top(10. * scale.y)
                .left(10. * scale.x)
                .right(31. * scale.x),
        )
        .into()
}
pub fn view(state: &State) -> Element<'_, Message> {
    responsive(move |size| {
        let scale = Scale::new(size);
        let search_scale = scale.search_scale();
        let search = text_input("Search Settings...", &state.query)
            .id(SEARCH)
            .on_input(|s| message(Event::Search(s)))
            .font(theme::ITALIC_FONT)
            .size(16. * search_scale)
            .line_height(1.25)
            .padding(Padding {
                top: 10. * search_scale,
                bottom: 10. * search_scale,
                left: 20. * search_scale,
                right: 20. * search_scale,
            })
            .width(640. * scale.x)
            .style(move |_, status| {
                input_style(
                    theme::palette().control_background,
                    8. * search_scale,
                    status,
                )
            });
        let categories = container(column(Category::ALL.into_iter().map(|category| {
            button(
                container(
                    text(category.label())
                        .font(theme::SEMIBOLD_FONT)
                        .size(16. * scale.text)
                        .line_height(1.25),
                )
                .center_x(90. * scale.x)
                .center_y(30. * scale.y),
            )
            .padding(0)
            .on_press(message(Event::Category(category, scale.y)))
            .style(move |_, status| button::Style {
                text_color: if state.active == Some(category)
                    || matches!(status, button::Status::Hovered | button::Status::Pressed)
                {
                    theme::palette().correct
                } else {
                    theme::palette().pending
                },
                ..Default::default()
            })
            .into()
        })))
        .padding(Padding::ZERO.top(20. * scale.y).bottom(20. * scale.y))
        .style(panel_style);
        let visible = state.categories();
        let mut sections = column![];
        for (i, &category) in visible.iter().enumerate() {
            sections = sections.push(section(state, category, scale, i + 1 == visible.len()));
        }
        if visible.is_empty() {
            sections = sections.push(
                container(
                    text("No settings found")
                        .font(theme::FONT)
                        .size(12. * scale.text),
                )
                .padding(10. * scale.text),
            );
        }
        let options = scrollable(sections)
            .id(PANEL)
            .width(540. * scale.x)
            .height(scale.options_height())
            .on_scroll(move |viewport| message(Event::Scrolled(viewport, scale.y)))
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new()
                    .width(5. * scale.text)
                    .scroller_width(5. * scale.text),
            ))
            .style(|theme, status| {
                let mut style = scrollable::default(theme, status);
                style.container = panel_style(theme);
                style.vertical_rail.background = None;
                style.vertical_rail.scroller.background = super::theme::palette().pending.into();
                style.vertical_rail.scroller.border = Border::default().rounded(4);
                style
            });
        let save = button(
            container(
                text("Save")
                    .font(theme::SAVE_FONT)
                    .size(20. * scale.text)
                    .line_height(1.25),
            )
            .center_x(100. * scale.x)
            .center_y(33. * scale.y),
        )
        .padding(0)
        .on_press(message(Event::Save))
        .style(move |_, status| button::Style {
            background: Some(theme::palette().background.into()),
            text_color: theme::palette().correct,
            border: Border {
                color: theme::palette().pending,
                width: if matches!(status, button::Status::Hovered | button::Status::Pressed) {
                    1.
                } else {
                    0.
                },
                radius: (8. * scale.text).into(),
            },
            ..Default::default()
        });
        let status: Element<'_, Message> = match &state.save_result {
            Some(result) => {
                let label = match result {
                    Ok(()) => "Saved",
                    Err(error) => error.as_str(),
                };
                scrollable(text(label).font(theme::FONT).size(11. * scale.text).color(
                    if result.is_ok() {
                        theme::palette().correct
                    } else {
                        theme::palette().incorrect
                    },
                ))
                .width(410. * scale.x)
                .height(33. * scale.y)
                .into()
            }
            None => Space::new().width(410. * scale.x).into(),
        };
        let footer = row![status, save]
            .spacing(10. * scale.x)
            .align_y(iced::Center);
        let options = container(column![
            options,
            container(footer).padding(Padding::ZERO.left(10. * scale.x))
        ])
        .width(540. * scale.x)
        .height(scale.panel_height())
        .style(panel_style);
        let body = column![search, row![categories, options].spacing(10. * scale.x)]
            .spacing(10. * search_scale);
        let home = button(Space::new().width(124).height(40))
            .padding(0)
            .on_press(Message::BackToTyping)
            .style(|_, _| button::Style::default());
        stack![
            container(body).padding(Padding::ZERO.left(30. * scale.x).top(80.)),
            chrome::brand(),
            container(home).padding(Padding::ZERO.left(50).top(30)),
        ]
        .width(Fill)
        .height(Fill)
        .into()
    })
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filtering_keeps_matching_sections_and_rows() {
        let mut state = State::default();
        assert_eq!(state.categories(), Category::ALL);
        let _ = state.update(Event::Search("  FONT  ".into()));
        assert_eq!(state.categories(), [Category::General]);
        assert!(state.matches(Category::General, "Font family"));
        let _ = state.update(Event::Search("general".into()));
        assert!(state.matches(Category::General, "Font size"));
        let _ = state.update(Event::Search("style".into()));
        assert_eq!(state.categories(), [Category::Caret]);
        let _ = state.update(Event::Search("missing".into()));
        assert!(state.categories().is_empty());
        assert_eq!(state.active, None);
        let _ = state.update(Event::Category(Category::Sound, 2.));
        assert!(state.query.is_empty());
        assert_eq!(state.active, Some(Category::Sound));
    }
    #[test]
    fn commits_restore_invalid_values_and_open_preserves_drafts() {
        let mut state = State::default();
        for value in ["8", "48", "24"] {
            let _ = state.update(Event::FontSize(value.into()));
            state.commit();
            assert_eq!(state.font_size, value);
        }
        for value in ["", "7", "49", "256", "NaN", "16.5", "-12"] {
            let _ = state.update(Event::FontSize(value.into()));
            state.commit();
            assert_eq!(state.font_size, "24");
        }
        let _ = state.update(Event::Caret("Block".into()));
        let _ = state.update(Event::Sound("On".into()));
        let _ = state.open();
        assert_eq!(
            (&*state.caret, &*state.sound, &*state.font_size),
            ("Block", "On", "24")
        );
    }
    #[test]
    fn scaled_sections_and_focus_reveal_share_panel_geometry() {
        for size in [
            Size::new(700., 400.),
            Size::new(1400., 800.),
            Size::new(1200., 400.),
            Size::new(700., 800.),
        ] {
            let scale = Scale::new(size);
            assert_eq!(scale.text, scale.x.min(scale.y));
            // Three section strides plus a full final viewport let Sound align at the top.
            assert_eq!(
                3. * STRIDE * scale.y + scale.options_height() - scale.options_height(),
                360. * scale.y
            );
            let panel = Rectangle {
                x: 130. * scale.x,
                y: scale.content_top(),
                width: 540. * scale.x,
                height: scale.options_height(),
            };
            let mut snapshot = FocusSnapshot {
                focused: Some(Rectangle {
                    x: panel.x + 30.,
                    y: panel.y + 390. * scale.y,
                    width: 161.,
                    height: 19. * scale.text,
                }),
                panel: Some((panel, 0.)),
                font_size_focused: false,
            };
            assert!(snapshot.reveal_offset().unwrap() > 0.);
            snapshot.panel = Some((panel, 360. * scale.y));
            assert_eq!(snapshot.reveal_offset(), None);
            snapshot.focused.as_mut().unwrap().y = panel.y + 33. * scale.y;
            assert!(snapshot.reveal_offset().unwrap() < 360. * scale.y);
            snapshot.focused.as_mut().unwrap().y = 90.;
            assert_eq!(snapshot.reveal_offset(), None);
        }
    }
}
