use crate::ui::typing_panel::TypingPanel;
use iced::{
    Element, Event, keyboard,
    widget::{Space, container, mouse_area, stack},
    window,
};
use std::time::{Duration, Instant};
use typerr::{
    engine::TypingSession,
    statistics::{Score, Statistics},
    storage::{ScoreWriter, TestMode},
    words,
};

const TIME_LIMIT_SECONDS: u64 = 30;
const WORD_TARGET: usize = 50;
const FADE_DURATION: Duration = Duration::from_millis(300);

#[derive(Debug, Clone, Copy, PartialEq)]
enum Page {
    Typing,
    Transition { started: Instant, now: Instant },
    Score,
    Dashboard,
    Settings,
}
impl Page {
    fn score_opacity(self) -> f32 {
        match self {
            Self::Typing | Self::Dashboard | Self::Settings => 0.0,
            Self::Score => 1.0,
            Self::Transition { started, now } => {
                let t = (now.saturating_duration_since(started).as_secs_f32()
                    / FADE_DURATION.as_secs_f32())
                .clamp(0.0, 1.0);
                t * t * (3.0 - 2.0 * t)
            }
        }
    }
}

pub use crate::settings::Mode;

pub struct App {
    profile: Option<crate::settings::Profile>,
    picture: crate::profile_picture::Picture,
    name_draft: String,
    name_error: Option<String>,
    score_writer: Option<ScoreWriter>,
    updater: typerr::update::Updater,
    dashboard: Option<typerr::storage::DashboardData>,
    dashboard_reply:
        Option<std::sync::mpsc::Receiver<typerr::storage::Result<typerr::storage::DashboardData>>>,
    main: MainTyping,
    page: Page,
    focused: bool,
    settings: crate::settings::Settings,
    settings_page: crate::ui::settings_page::State,
    settings_task: iced::Task<Message>,
}

struct MainTyping {
    session: TypingSession,
    mode: Mode,
    options: words::Options,
    word_target: usize,
    time_limit_seconds: u64,
    amount_picker: Option<Mode>,
    deadline: Option<Instant>,
    started: Option<Instant>,
    statistics: Statistics,
    score: Score,
    seconds_remaining: u64,
    /// Changes on accepted edits or explicit session resets, restoring the viewport.
    revision: u64,
    /// Changes only when a session is replaced, even if its text is identical.
    generation: u64,
}

impl MainTyping {
    fn course(&self) -> crate::settings::Course {
        crate::settings::Course {
            mode: self.mode,
            word_target: self.word_target as u16,
            time_limit_seconds: self.time_limit_seconds as u16,
            options: self.options,
        }
    }

    fn restart(&mut self) {
        self.replace(words::generate(&mut rand::rng(), self.options));
    }

    fn replace(&mut self, target: String) {
        self.session = TypingSession::new(target);
        self.deadline = None;
        self.started = None;
        self.statistics = Statistics::default();
        self.score = Score::default();
        self.seconds_remaining = self.time_limit_seconds;
        self.amount_picker = None;
        self.revision = self.revision.wrapping_add(1);
        self.generation = self.generation.wrapping_add(1);
    }

    fn elapsed(&self, now: Instant) -> Duration {
        self.started
            .map_or(Duration::ZERO, |start| now.saturating_duration_since(start))
    }

    fn tick(&mut self, now: Instant) {
        if let Some(deadline) = self.deadline {
            let remaining = deadline.saturating_duration_since(now);
            self.seconds_remaining = remaining.as_secs() + u64::from(remaining.subsec_nanos() > 0);
        }
    }

    fn next_tick(&self) -> Option<Instant> {
        self.seconds_remaining.checked_sub(1).and_then(|seconds| {
            self.deadline
                .map(|deadline| deadline - Duration::from_secs(seconds))
        })
    }

    fn monitor(&self) -> String {
        match self.mode {
            Mode::Words => format!(
                "{}/{}",
                self.session.completed_words().min(self.word_target),
                self.word_target
            ),
            Mode::Time => format!("{}s", self.seconds_remaining),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    OpenSettings,
    Settings(crate::ui::settings_page::Event),
    ChoosePicture,
    ZoomPicture(f32),
    PanPicture(iced::Vector),
    SavePicture,
    DiscardPicture,
    NameChanged(String),
    SaveName,
    FocusName,
    Upgrade,
    DismissUpdate,
    Native(Event),
    Dashboard,
    BackToTyping,
    OpenConnection(&'static str),
    SelectMode(Mode),
    ToggleAmountPicker(Mode),
    SelectAmount(Mode, u16),
    DismissAmountPicker,
    TogglePunctuation,
    ToggleNumbers,
    Next,
    Retry,
}

impl Default for App {
    fn default() -> Self {
        Self {
            profile: None,
            picture: crate::profile_picture::Picture::default(),
            name_draft: String::new(),
            name_error: None,
            score_writer: None,
            updater: typerr::update::Updater::default(),
            dashboard: None,
            dashboard_reply: None,
            main: MainTyping {
                session: TypingSession::new(words::generate(
                    &mut rand::rng(),
                    words::Options::default(),
                )),
                mode: Mode::default(),
                options: words::Options::default(),
                word_target: WORD_TARGET,
                time_limit_seconds: TIME_LIMIT_SECONDS,
                amount_picker: None,
                deadline: None,
                started: None,
                statistics: Statistics::default(),
                score: Score::default(),
                seconds_remaining: TIME_LIMIT_SECONDS,
                revision: 0,
                generation: 0,
            },
            page: Page::Typing,
            focused: true,
            settings: crate::settings::Settings::default(),
            settings_page: crate::ui::settings_page::State::default(),
            settings_task: iced::Task::none(),
        }
    }
}

impl App {
    pub fn new(
        settings: crate::settings::Settings,
        startup: typerr::update::install::Startup,
    ) -> Self {
        let profile = crate::settings::Profile::load();
        let settings_page = match profile.parameters() {
            Ok(parameters) => crate::ui::settings_page::State::from_parameters(parameters),
            Err(error) => {
                let mut state = crate::ui::settings_page::State::default();
                state.save_result = Some(Err(error));
                state
            }
        };
        let mut app = Self {
            profile: Some(profile),
            settings_page,
            picture: crate::profile_picture::Picture::load(
                crate::settings::user_settings_path().map(|p| p.with_file_name("avatar.png")),
            ),
            score_writer: ScoreWriter::start_default(),
            updater: typerr::update::Updater::start(startup),
            settings,
            ..Self::default()
        };
        app.restore_course();
        app
    }

    fn restore_course(&mut self) {
        let Some(profile) = &self.profile else { return };
        match profile.course() {
            Ok(course) => {
                self.main.mode = course.mode;
                self.main.word_target = usize::from(course.word_target);
                self.main.time_limit_seconds = u64::from(course.time_limit_seconds);
                self.main.options = course.options;
                self.main.restart();
            }
            Err(error) => eprintln!("Could not load course preferences: {error}"),
        }
    }

    pub fn update(&mut self, message: Message) {
        self.update_at(message, Instant::now());
    }

    pub fn update_task(&mut self, message: Message) -> iced::Task<Message> {
        if self.needs_name() && matches!(message, Message::FocusName) {
            return iced::widget::operation::focus("name-input");
        }
        let close = matches!(
            message,
            Message::Native(Event::Window(window::Event::CloseRequested))
        );
        self.update(message);
        if self.updater.phase == typerr::update::Phase::Restarting
            || (close && !self.updater.busy() && !self.picture.saving())
        {
            // Drain queued score writes before telling the helper it can replace files.
            self.score_writer.take();
            self.updater.shutdown();
            return iced::exit();
        }
        std::mem::replace(&mut self.settings_task, iced::Task::none())
    }

    fn advance(&mut self, now: Instant) {
        self.picture.poll();
        let safe = matches!(self.page, Page::Score | Page::Dashboard | Page::Settings)
            || (self.page == Page::Typing && self.main.started.is_none());
        self.updater
            .poll(safe && !self.needs_name() && !self.picture.modal());
        if self.updater.visible {
            self.main.amount_picker = None;
        }
        if let Some(reply) = &self.dashboard_reply {
            match reply.try_recv() {
                Ok(result) => {
                    self.dashboard = result.ok();
                    self.dashboard_reply = None;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => self.dashboard_reply = None,
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }
        if self.page == Page::Typing {
            self.main.tick(now);
            if self.main.mode == Mode::Time
                && self.main.deadline.is_some_and(|deadline| now >= deadline)
            {
                self.finish(self.main.deadline.expect("expired deadline"));
            }
        }
        if let Page::Transition { started, .. } = self.page {
            self.page = if now.saturating_duration_since(started) >= FADE_DURATION {
                Page::Score
            } else {
                Page::Transition { started, now }
            };
        }
    }

    fn finish(&mut self, now: Instant) {
        if self.page == Page::Typing {
            self.main.amount_picker = None;
            self.main.score = self.main.statistics.finish(self.main.elapsed(now));
            if let Some(writer) = &self.score_writer {
                let (mode, amount) = match self.main.mode {
                    Mode::Words => (TestMode::Words, self.main.word_target as u16),
                    Mode::Time => (TestMode::Time, self.main.time_limit_seconds as u16),
                };
                writer.save(&self.main.score, mode, amount, self.main.options);
            }
            self.page = Page::Transition { started: now, now };
        }
    }

    fn finished(&self) -> bool {
        self.main.session.status() == typerr::engine::SessionStatus::Complete
            || (self.main.mode == Mode::Words
                && self.main.session.completed_words() >= self.main.word_target)
    }

    fn update_at(&mut self, message: Message, now: Instant) {
        let previous_course = self.main.course();
        self.handle_at(message, now);
        let course = self.main.course();
        if course != previous_course
            && let Some(profile) = &mut self.profile
            && let Err(error) = profile.save_course(course)
        {
            eprintln!("Could not save course preferences: {error}");
            self.settings_page.save_result =
                Some(Err(format!("Could not save course preferences: {error}")));
        }
        let safe = matches!(self.page, Page::Score | Page::Dashboard | Page::Settings)
            || (self.page == Page::Typing && self.main.started.is_none());
        self.updater
            .poll(safe && !self.needs_name() && !self.picture.modal());
        if self.updater.visible {
            self.main.amount_picker = None;
        }
    }

    fn handle_at(&mut self, message: Message, now: Instant) {
        let now = if let Message::Native(Event::Window(window::Event::RedrawRequested(timestamp))) =
            &message
        {
            *timestamp
        } else {
            now
        };
        // Enforce the time limit even if an input arrives before a delayed redraw.
        self.advance(now);
        if matches!(
            &message,
            Message::Native(Event::Window(window::Event::RedrawRequested(_)))
        ) {
            self.updater.confirm_startup();
        }
        if self.needs_name() {
            match message {
                Message::NameChanged(value) => {
                    self.name_draft = value.chars().filter(|c| !c.is_control()).take(16).collect();
                    self.name_error = None;
                }
                Message::SaveName => {
                    if let Some(profile) = &mut self.profile {
                        match profile.save(&self.name_draft) {
                            Ok(()) => self.name_error = None,
                            Err(error) => self.name_error = Some(error),
                        }
                    }
                }
                Message::Native(Event::Window(window::Event::Focused)) => self.focused = true,
                Message::Native(Event::Window(window::Event::Unfocused)) => self.focused = false,
                _ => {}
            }
            return;
        }
        if self.picture.modal() {
            match message {
                Message::ZoomPicture(zoom) if !self.picture.busy() => {
                    if let Some(crop) = &mut self.picture.crop {
                        crop.set_zoom(zoom);
                    }
                }
                Message::PanPicture(offset) if !self.picture.busy() => {
                    if let Some(crop) = &mut self.picture.crop {
                        crop.pan(offset);
                    }
                }
                Message::SavePicture => self.picture.save(),
                Message::DiscardPicture => self.picture.discard(),
                Message::Native(Event::Window(window::Event::Focused)) => self.focused = true,
                Message::Native(Event::Window(window::Event::Unfocused)) => self.focused = false,
                Message::Native(Event::Keyboard(keyboard::Event::KeyPressed {
                    key,
                    modifiers,
                    repeat,
                    ..
                })) if self.focused
                    && !self.picture.busy()
                    && !modifiers.control()
                    && !modifiers.alt()
                    && !modifiers.logo() =>
                {
                    use keyboard::{Key, key::Named};
                    match key {
                        Key::Named(Named::Escape) if !repeat => self.picture.discard(),
                        Key::Named(Named::Tab) if !repeat => {
                            self.picture.focus = if self.picture.focus >= 3 {
                                if modifiers.shift() { 2 } else { 0 }
                            } else {
                                (self.picture.focus + if modifiers.shift() { 2 } else { 1 }) % 3
                            }
                        }
                        Key::Named(Named::Enter | Named::Space) if !repeat => {
                            if self.picture.focus == 1 {
                                self.picture.discard();
                            } else if self.picture.focus == 2 {
                                self.picture.save();
                            }
                        }
                        Key::Named(
                            direction @ (Named::ArrowLeft
                            | Named::ArrowRight
                            | Named::ArrowUp
                            | Named::ArrowDown),
                        ) => {
                            if let Some(crop) = &mut self.picture.crop {
                                if self.picture.focus == 0 && !modifiers.shift() {
                                    crop.set_zoom(
                                        crop.zoom
                                            + if matches!(
                                                direction,
                                                Named::ArrowRight | Named::ArrowUp
                                            ) {
                                                0.05
                                            } else {
                                                -0.05
                                            },
                                    );
                                } else if modifiers.shift() {
                                    let delta = match direction {
                                        Named::ArrowLeft => iced::Vector::new(-2., 0.),
                                        Named::ArrowRight => iced::Vector::new(2., 0.),
                                        Named::ArrowUp => iced::Vector::new(0., -2.),
                                        _ => iced::Vector::new(0., 2.),
                                    };
                                    crop.pan(crop.offset + delta);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
            return;
        }
        if self.updater.visible {
            match message {
                Message::Upgrade => self.updater.accept(),
                Message::DismissUpdate => self.updater.dismiss(),
                Message::Native(Event::Window(window::Event::Focused)) => self.focused = true,
                Message::Native(Event::Window(window::Event::Unfocused)) => self.focused = false,
                Message::Native(Event::Keyboard(keyboard::Event::KeyPressed {
                    key,
                    modifiers,
                    repeat: false,
                    ..
                })) if self.focused
                    && !modifiers.control()
                    && !modifiers.alt()
                    && !modifiers.logo() =>
                {
                    match key {
                        keyboard::Key::Named(keyboard::key::Named::Escape) => {
                            self.updater.dismiss()
                        }
                        keyboard::Key::Named(keyboard::key::Named::Tab) => {
                            self.updater.focus_primary = !self.updater.focus_primary
                        }
                        keyboard::Key::Named(
                            keyboard::key::Named::Enter | keyboard::key::Named::Space,
                        ) => {
                            if self.updater.focus_primary {
                                self.updater.accept();
                            } else {
                                self.updater.dismiss();
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
            return;
        }
        if let Message::Native(Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Character(key),
            modifiers,
            repeat: false,
            ..
        })) = &message
            && self.focused
            && key.as_str() == ","
            && *modifiers == keyboard::Modifiers::CTRL
        {
            self.handle_at(Message::OpenSettings, now);
            return;
        }
        match message {
            Message::OpenSettings
                if matches!(self.page, Page::Typing | Page::Score | Page::Dashboard) =>
            {
                self.main.amount_picker = None;
                self.main.deadline = None;
                self.dashboard_reply = None;
                self.settings_task = self.settings_page.open();
                self.page = Page::Settings;
            }
            Message::Settings(crate::ui::settings_page::Event::Save)
                if self.page == Page::Settings =>
            {
                let parameters = self.settings_page.parameters();
                self.settings_page.save_result = Some(match self.profile.as_mut() {
                    Some(profile) => profile.save_parameters(&parameters),
                    None => Err("Settings storage is unavailable.".into()),
                });
            }
            Message::Settings(event) if self.page == Page::Settings => {
                self.settings_task = self.settings_page.update(event);
            }
            Message::ChoosePicture if self.page == Page::Dashboard => self.picture.choose(),
            Message::Dashboard if matches!(self.page, Page::Typing | Page::Score) => {
                self.main.amount_picker = None;
                self.dashboard = None;
                self.dashboard_reply = self.score_writer.as_ref().map(ScoreWriter::dashboard);
                self.page = Page::Dashboard;
            }
            Message::BackToTyping if matches!(self.page, Page::Dashboard | Page::Settings) => {
                self.settings_page.commit();
                self.main.restart();
                self.dashboard_reply = None;
                self.page = Page::Typing;
            }
            Message::Next if self.page == Page::Score => {
                self.main.restart();
                self.page = Page::Typing;
            }
            Message::Retry if self.page == Page::Score => {
                self.main
                    .replace(self.main.session.target().iter().collect());
                self.page = Page::Typing;
            }
            Message::TogglePunctuation if self.page == Page::Typing => {
                self.main.options.punctuation = !self.main.options.punctuation;
                self.main.restart();
            }
            Message::ToggleNumbers if self.page == Page::Typing => {
                self.main.options.numbers = !self.main.options.numbers;
                self.main.restart();
            }
            Message::SelectMode(mode) if self.page == Page::Typing => {
                self.main.amount_picker = None;
                if self.main.mode != mode {
                    self.main.mode = mode;
                    self.main.restart();
                }
            }
            Message::ToggleAmountPicker(mode) if self.page == Page::Typing => {
                self.main.amount_picker = (self.main.amount_picker != Some(mode)).then_some(mode);
            }
            Message::SelectAmount(mode, amount)
                if self.page == Page::Typing
                    && self.main.amount_picker == Some(mode)
                    && mode.amounts().contains(&amount) =>
            {
                match mode {
                    Mode::Words => self.main.word_target = usize::from(amount),
                    Mode::Time => self.main.time_limit_seconds = u64::from(amount),
                }
                self.main.mode = mode;
                self.main.restart();
            }
            Message::DismissAmountPicker => self.main.amount_picker = None,
            Message::OpenConnection(url) if self.page == Page::Typing => open_connection(url),
            Message::Native(Event::Window(window::Event::Focused)) => self.focused = true,
            Message::Native(Event::Window(window::Event::Unfocused)) => {
                self.focused = false;
                self.main.amount_picker = None;
            }
            Message::Native(Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                modifiers,
                text,
                repeat,
                ..
            })) if self.focused => {
                if self.page == Page::Settings {
                    return;
                }
                if modifiers.control() || modifiers.alt() || modifiers.logo() {
                    return;
                }
                if key == keyboard::Key::Named(keyboard::key::Named::Tab) {
                    if !repeat && !modifiers.shift() {
                        self.main.restart();
                        self.page = Page::Typing;
                        self.dashboard_reply = None;
                    }
                    return;
                }
                if self.page != Page::Typing {
                    return;
                }
                if key == keyboard::Key::Named(keyboard::key::Named::Escape) {
                    self.main.amount_picker = None;
                    return;
                }
                let changed = if key == keyboard::Key::Named(keyboard::key::Named::Backspace) {
                    let correct = self
                        .main
                        .session
                        .position()
                        .checked_sub(1)
                        .and_then(|i| self.main.session.character_state(i))
                        == Some(typerr::engine::CharacterState::Correct);
                    let changed = self.main.session.backspace();
                    if changed {
                        self.main
                            .statistics
                            .backspace(self.main.elapsed(now), correct);
                    }
                    changed
                } else {
                    let mut changed = false;
                    if let Some(text) = text {
                        for c in text.chars() {
                            let correct =
                                self.main.session.target().get(self.main.session.position())
                                    == Some(&c);
                            if self.main.session.input(c) {
                                self.main.started.get_or_insert(now);
                                self.main.statistics.input(self.main.elapsed(now), correct);
                                if self.main.mode == Mode::Time && self.main.deadline.is_none() {
                                    self.main.deadline = Some(
                                        now + Duration::from_secs(self.main.time_limit_seconds),
                                    );
                                }
                                changed = true;
                            }
                            if self.finished() {
                                self.finish(now);
                                break;
                            }
                        }
                    }
                    changed
                };
                if changed {
                    self.main.amount_picker = None;
                    self.main.revision = self.main.revision.wrapping_add(1);
                }
            }
            _ => {}
        }
    }

    fn typing_view(&self, opacity: f32, enabled: bool) -> Element<'_, Message> {
        let panel = container(
            TypingPanel::new(
                &self.main.session,
                self.main.revision,
                self.main.generation,
                self.main.monitor(),
                self.settings,
            )
            .presentation(opacity, enabled),
        )
        .max_width(crate::ui::theme::TYPING_MAX_WIDTH)
        .max_height(crate::ui::theme::typing_height(self.settings.line_height()));
        let typing = container(panel)
            .padding(iced::Padding {
                top: crate::ui::theme::VERTICAL_CLEARANCE,
                bottom: crate::ui::theme::VERTICAL_CLEARANCE,
                left: 30.0,
                right: 30.0,
            })
            .width(iced::Fill)
            .align_x(iced::Center)
            .align_y(iced::Center)
            .height(iced::Fill);
        let dismiss: Element<'_, Message> = if enabled && self.main.amount_picker.is_some() {
            mouse_area(Space::new().width(iced::Fill).height(iced::Fill))
                .on_press(Message::DismissAmountPicker)
                .on_right_press(Message::DismissAmountPicker)
                .into()
        } else {
            Space::new().into()
        };
        stack![
            typing,
            container(crate::ui::chrome::footer(opacity, enabled))
                .height(iced::Fill)
                .align_y(iced::Bottom),
            dismiss,
            crate::ui::chrome::header(
                self.main.mode,
                self.main.options,
                self.main.amount_picker,
                opacity,
                enabled
            ),
        ]
        .into()
    }

    fn page_content(&self) -> Element<'_, Message> {
        if self.page == Page::Settings {
            return crate::ui::settings_page::view(&self.settings_page);
        }
        if self.page == Page::Dashboard {
            return crate::ui::dashboard::view(
                self.dashboard.as_ref(),
                self.user_name(),
                &self.picture.avatar,
            );
        }
        let opacity = self.page.score_opacity();
        let enabled = !self.updater.visible && !self.needs_name();
        let typing: Element<'_, Message> = if self.page == Page::Score {
            Space::new().width(iced::Fill).height(iced::Fill).into()
        } else {
            self.typing_view(1.0 - opacity, self.page == Page::Typing && enabled)
        };
        let score: Element<'_, Message> = if self.page == Page::Typing {
            Space::new().into()
        } else {
            container(crate::ui::score::view(
                &self.main.score,
                opacity,
                self.page == Page::Score && enabled,
            ))
            .center_x(iced::Fill)
            .center_y(iced::Fill)
            .into()
        };
        stack![
            typing,
            score,
            crate::ui::chrome::brand(),
            crate::ui::chrome::user_name(
                self.user_name(),
                enabled && !matches!(self.page, Page::Transition { .. })
            )
        ]
        .into()
    }

    pub fn needs_name(&self) -> bool {
        self.profile.as_ref().is_some_and(|p| p.name.is_none())
    }

    fn user_name(&self) -> &str {
        self.profile
            .as_ref()
            .and_then(|p| p.name.as_deref())
            .unwrap_or("User Name")
    }

    pub fn view(&self) -> Element<'_, Message> {
        let page = self.page_content();
        let content = if self.needs_name() {
            stack![
                crate::ui::blur::Blur::new(page),
                iced::widget::opaque(crate::ui::name_prompt::view(
                    &self.name_draft,
                    self.name_error.as_deref()
                ))
            ]
            .into()
        } else if self.picture.modal() {
            stack![
                crate::ui::blur::Blur::new(page),
                iced::widget::opaque(crate::ui::picture_prompt::view(&self.picture))
            ]
            .into()
        } else if self.updater.visible {
            stack![
                page,
                iced::widget::opaque(crate::ui::update_prompt::view(&self.updater))
            ]
            .into()
        } else {
            page
        };
        let poll =
            (self.updater.pending() || self.dashboard_reply.is_some() || self.picture.busy())
                .then(|| Instant::now() + Duration::from_millis(50));
        let timer = if self.page == Page::Typing {
            self.main.next_tick()
        } else {
            None
        };
        let next = poll.into_iter().chain(timer).min();
        crate::ui::page_host::PageHost::new(
            content,
            next,
            matches!(self.page, Page::Transition { .. }),
        )
        .text_input(self.needs_name())
        .settings_input(
            self.page == Page::Settings
                && !self.needs_name()
                && !self.updater.visible
                && !self.picture.modal(),
        )
        .into()
    }
}

fn open_connection(url: &'static str) {
    // Hand off to the OS browser off the input thread and reap the launcher.
    std::thread::spawn(move || {
        #[cfg(target_os = "windows")]
        let result = {
            use std::os::windows::process::CommandExt;
            std::process::Command::new("cmd")
                .args(["/C", "start", "", url])
                .creation_flags(0x0800_0000)
                .status()
        };
        #[cfg(not(target_os = "windows"))]
        let result = std::process::Command::new("xdg-open").arg(url).status();
        if let Err(error) = result {
            eprintln!("Could not open connection: {error}");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::keyboard::{
        Key, Location, Modifiers,
        key::{Named, NativeCode, Physical},
    };

    #[test]
    fn course_choices_survive_relaunch_with_fresh_progress_and_generated_options() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(
            &path,
            r#"{"name":"Alice","caret-style":"Block","future":42}"#,
        )
        .unwrap();
        let launch = || {
            let mut app = App {
                profile: Some(crate::settings::Profile::at(path.clone(), None)),
                ..App::default()
            };
            app.restore_course();
            app
        };
        let mut app = launch();
        select_amount(&mut app, Mode::Words, 25);
        select_amount(&mut app, Mode::Time, 120);
        app.update(Message::ToggleNumbers);
        app.update(Message::TogglePunctuation);
        app.update(key(
            Key::Character("a".into()),
            Some("a"),
            Modifiers::empty(),
        ));
        assert!(app.main.started.is_some());
        let expected = app.main.course();
        drop(app);
        let mut app = launch();
        assert_eq!(app.main.course(), expected);
        assert_eq!(app.main.monitor(), "120s");
        assert_eq!(app.main.session.position(), 0);
        assert!(app.main.started.is_none() && app.main.deadline.is_none());
        assert!(app.main.session.target().iter().any(char::is_ascii_digit));
        assert!(
            app.main
                .session
                .target()
                .iter()
                .any(|c| ",.!?;:".contains(*c))
        );
        app.update(Message::SelectMode(Mode::Words));
        drop(app);
        let mut app = launch();
        assert_eq!(app.main.monitor(), "0/25");
        assert_eq!(app.main.time_limit_seconds, 120);
        app.update(Message::OpenSettings);
        app.update(Message::Settings(crate::ui::settings_page::Event::Save));
        assert_eq!(launch().main.course(), app.main.course());
        let saved: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(saved["name"], "Alice");
        assert_eq!(saved["future"], 42);
    }

    #[test]
    fn course_persistence_ignores_navigation_typing_and_rejected_choices() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let source = r#"{"name":"Alice"}"#;
        std::fs::write(&path, source).unwrap();
        let mut app = App {
            profile: Some(crate::settings::Profile::at(path.clone(), None)),
            ..App::default()
        };
        app.restore_course();
        for message in [
            Message::SelectMode(Mode::Words),
            Message::ToggleAmountPicker(Mode::Time),
            Message::SelectAmount(Mode::Time, 999),
            Message::DismissAmountPicker,
            key(Key::Character("a".into()), Some("a"), Modifiers::empty()),
            Message::Dashboard,
            Message::SelectMode(Mode::Time),
            Message::BackToTyping,
        ] {
            app.update(message);
            assert_eq!(std::fs::read_to_string(&path).unwrap(), source);
        }
        // A file damaged after startup must not be overwritten by automatic saves.
        std::fs::write(&path, "{").unwrap();
        app.update(Message::ToggleNumbers);
        assert!(app.main.options.numbers);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{");
        assert!(matches!(app.settings_page.save_result, Some(Err(_))));
    }

    fn key(key: Key, text: Option<&str>, modifiers: Modifiers) -> Message {
        Message::Native(Event::Keyboard(keyboard::Event::KeyPressed {
            modified_key: key.clone(),
            key,
            physical_key: Physical::Unidentified(NativeCode::Unidentified),
            location: Location::Standard,
            modifiers,
            text: text.map(Into::into),
            repeat: false,
        }))
    }

    fn app(target: &str) -> App {
        App {
            profile: None,
            picture: crate::profile_picture::Picture::default(),
            name_draft: String::new(),
            name_error: None,
            score_writer: None,
            updater: typerr::update::Updater::default(),
            dashboard: None,
            dashboard_reply: None,
            main: MainTyping {
                session: TypingSession::new(target),
                mode: Mode::default(),
                options: words::Options::default(),
                word_target: WORD_TARGET,
                time_limit_seconds: TIME_LIMIT_SECONDS,
                amount_picker: None,
                deadline: None,
                started: None,
                statistics: Statistics::default(),
                score: Score::default(),
                seconds_remaining: TIME_LIMIT_SECONDS,
                revision: 0,
                generation: 0,
            },
            page: Page::Typing,
            focused: true,
            settings: crate::settings::Settings::default(),
            settings_page: crate::ui::settings_page::State::default(),
            settings_task: iced::Task::none(),
        }
    }

    #[test]
    fn native_settings_save_hit_target_persists_without_applying_and_reports_errors() {
        use crate::ui::settings_page::Event as SettingsEvent;
        use iced::advanced::{
            Layout, Shell, clipboard, layout, mouse, renderer::Headless, widget::Tree,
        };
        let renderer = iced::futures::executor::block_on(iced::Renderer::new(
            crate::ui::theme::FONT,
            16.0.into(),
            Some("tiny-skia"),
        ))
        .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, r#"{"name":"Alice","custom":123}"#).unwrap();
        let mut app = App {
            profile: Some(crate::settings::Profile::at(path.clone(), None)),
            ..App::default()
        };
        app.update(Message::OpenSettings);
        app.update(Message::Settings(SettingsEvent::FontSize("30".into())));
        app.update(Message::Settings(SettingsEvent::Caret("Block".into())));
        app.update(Message::Settings(SettingsEvent::Sound("On".into())));
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            r#"{"name":"Alice","custom":123}"#
        );
        for size in [iced::Size::new(700., 400.), iced::Size::new(1100., 650.)] {
            let mut view = app.view();
            let mut tree = Tree::new(&view);
            let node = view.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(iced::Size::ZERO, size),
            );
            let mut messages = Vec::new();
            for event in [
                mouse::Event::ButtonPressed(mouse::Button::Left),
                mouse::Event::ButtonReleased(mouse::Button::Left),
            ] {
                view.as_widget_mut().update(
                    &mut tree,
                    &Event::Mouse(event),
                    Layout::new(&node),
                    mouse::Cursor::Available(iced::Point::new(
                        620. * size.width / 700.,
                        353. * size.height / 400.,
                    )),
                    &renderer,
                    &mut clipboard::Null,
                    &mut Shell::new(&mut messages),
                    &iced::Rectangle::with_size(size),
                );
            }
            drop(view);
            assert!(
                messages
                    .iter()
                    .any(|m| matches!(m, Message::Settings(SettingsEvent::Save)))
            );
            for message in messages {
                app.update(message);
            }
            assert!(matches!(app.settings_page.save_result, Some(Ok(()))));
        }
        let parameters = crate::settings::Profile::at(path.clone(), None)
            .parameters()
            .unwrap();
        assert_eq!(parameters.font_size, 30.);
        assert_eq!(parameters.caret_style, "Block");
        assert!(parameters.sound);
        assert_eq!(app.settings.font_size, 16.);
        assert_eq!(app.page, Page::Settings);
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(json["name"], "Alice");
        assert_eq!(json["custom"], 123);
        std::fs::write(&path, "{").unwrap();
        app.update(Message::Settings(SettingsEvent::Save));
        assert!(matches!(app.settings_page.save_result, Some(Err(_))));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{");
        assert_eq!(app.settings_page.font_size, "30");
        app.updater.visible = true;
        app.settings_page.save_result = None;
        app.update(Message::Settings(SettingsEvent::Save));
        assert!(app.settings_page.save_result.is_none());
    }

    #[test]
    fn settings_shortcut_guards_and_return_preserve_typing_options() {
        use crate::ui::settings_page::Event as SettingsEvent;
        let shortcut = || key(Key::Character(",".into()), Some(","), Modifiers::CTRL);
        let mut app = app("abcd ");
        app.update(Message::SelectMode(Mode::Time));
        app.update(Message::ToggleNumbers);
        app.update(key(
            Key::Character("a".into()),
            Some("a"),
            Modifiers::empty(),
        ));
        assert!(app.main.deadline.is_some());
        app.update(shortcut());
        assert_eq!(app.page, Page::Settings);
        assert!(app.main.deadline.is_none());
        let position = app.main.session.position();
        let generation = app.main.generation;
        for event in [
            key(Key::Named(Named::Tab), None, Modifiers::empty()),
            key(Key::Named(Named::Backspace), None, Modifiers::empty()),
            key(Key::Character("x".into()), Some("x"), Modifiers::empty()),
        ] {
            app.update(event);
        }
        assert_eq!(
            (app.page, app.main.session.position(), app.main.generation),
            (Page::Settings, position, generation)
        );
        app.update(Message::Settings(SettingsEvent::FontSize("32".into())));
        app.update(Message::Settings(SettingsEvent::CommitFontSize));
        app.update(Message::Settings(SettingsEvent::Search("caret".into())));
        app.update(shortcut());
        assert_eq!(app.settings_page.query, "caret");
        assert_eq!(app.settings.font_size, 16.);
        app.update(Message::BackToTyping);
        assert_eq!(app.page, Page::Typing);
        assert_eq!(app.main.session.position(), 0);
        assert_eq!(app.main.mode, Mode::Time);
        assert!(app.main.options.numbers);
        assert_eq!(app.main.generation, generation + 1);
        app.update(shortcut());
        assert_eq!(app.settings_page.font_size, "32");
        assert!(app.settings_page.query.is_empty());

        for page in [Page::Typing, Page::Score, Page::Dashboard] {
            app.page = page;
            for modifiers in [
                Modifiers::CTRL | Modifiers::SHIFT,
                Modifiers::CTRL | Modifiers::ALT,
                Modifiers::CTRL | Modifiers::LOGO,
            ] {
                app.update(key(Key::Character(",".into()), Some(","), modifiers));
                assert_eq!(app.page, page);
            }
            let mut repeated = shortcut();
            if let Message::Native(Event::Keyboard(keyboard::Event::KeyPressed {
                repeat, ..
            })) = &mut repeated
            {
                *repeat = true;
            }
            app.update(repeated);
            assert_eq!(app.page, page);
            app.focused = false;
            app.update(shortcut());
            assert_eq!(app.page, page);
            app.focused = true;
            app.update(shortcut());
            assert_eq!(app.page, Page::Settings);
        }
        let now = Instant::now();
        app.page = Page::Transition { started: now, now };
        app.update_at(shortcut(), now);
        assert!(matches!(app.page, Page::Transition { .. }));
        app.page = Page::Dashboard;
        app.picture.crop = Some(crate::profile_picture::Crop::new(
            crate::profile_picture::placeholder(),
        ));
        app.update(shortcut());
        assert_eq!(app.page, Page::Dashboard);
        app.picture.crop = None;
        app.updater.visible = true;
        app.update(shortcut());
        assert_eq!(app.page, Page::Dashboard);
    }

    #[test]
    fn picture_modal_blocks_navigation_and_keeps_draft_until_save_or_discard() {
        let mut app = App::default();
        app.update(Message::Dashboard);
        app.picture.crop = Some(crate::profile_picture::Crop::new(
            crate::profile_picture::placeholder(),
        ));
        let avatar = app.picture.avatar.clone();
        for message in [
            Message::BackToTyping,
            Message::Next,
            Message::Retry,
            Message::SelectMode(Mode::Time),
            key(Key::Character("x".into()), Some("x"), Modifiers::empty()),
            key(Key::Named(Named::Backspace), None, Modifiers::empty()),
        ] {
            app.update(message);
            assert_eq!(app.page, Page::Dashboard);
            assert_eq!(app.main.generation, 0);
            assert_eq!(app.main.session.position(), 0);
        }
        app.update(key(Key::Named(Named::Tab), None, Modifiers::empty()));
        assert_eq!(app.picture.focus, 0);
        app.update(key(Key::Named(Named::ArrowRight), None, Modifiers::empty()));
        assert!(app.picture.crop.as_ref().unwrap().zoom > 1.);
        app.update(Message::ZoomPicture(3.));
        app.update(Message::PanPicture(iced::Vector::new(10., 20.)));
        assert_eq!(
            app.picture.crop.as_ref().unwrap().offset,
            iced::Vector::new(10., 20.)
        );
        app.update(Message::SavePicture); // no user path in Default: show a retryable error
        assert!(app.picture.error.is_some());
        assert_eq!(app.picture.avatar, avatar);
        app.update(key(Key::Named(Named::Escape), None, Modifiers::empty()));
        assert!(!app.picture.modal());
        assert_eq!(app.picture.avatar, avatar);
        app.update(key(Key::Named(Named::Tab), None, Modifiers::empty()));
        assert_eq!(app.page, Page::Typing);
        assert_eq!(app.main.generation, 1);
        app.update(Message::ChoosePicture); // never open an OS dialog on Typing
        assert!(!app.picture.busy());
    }

    #[test]
    fn update_prompt_waits_for_completion_and_isolates_input() {
        use typerr::update::Phase;
        let mut app = app("ab");
        let start = Instant::now();
        app.update_at(
            key(Key::Character("a".into()), Some("a"), Modifiers::empty()),
            start,
        );
        app.updater.phase = Phase::Available;
        app.update_at(
            key(Key::Character("b".into()), Some("b"), Modifiers::empty()),
            start + Duration::from_secs(1),
        );
        assert!(!app.updater.visible);
        app.update_at(
            Message::Native(Event::Window(window::Event::RedrawRequested(
                start + Duration::from_secs(2),
            ))),
            start + Duration::from_secs(2),
        );
        assert_eq!(app.page, Page::Score);
        assert!(app.updater.visible);
        let generation = app.main.generation;
        app.update_at(Message::Next, start + Duration::from_secs(2));
        app.update_at(Message::Dashboard, start + Duration::from_secs(2));
        app.update_at(
            key(Key::Named(Named::Tab), None, Modifiers::empty()),
            start + Duration::from_secs(2),
        );
        assert_eq!(app.page, Page::Score);
        assert_eq!(app.main.generation, generation);
        assert!(app.updater.focus_primary);
        app.update_at(
            key(Key::Named(Named::Escape), None, Modifiers::empty()),
            start + Duration::from_secs(2),
        );
        assert!(!app.updater.visible);
        app.update_at(Message::Next, start + Duration::from_secs(2));
        assert_eq!(app.page, Page::Typing);
    }

    #[test]
    fn startup_prompt_and_cancel_preserve_session_and_timer() {
        use typerr::update::Phase;
        let mut app = app("ab cd");
        let now = Instant::now();
        app.main.mode = Mode::Time;
        app.updater.phase = Phase::Available;
        app.update_at(
            key(Key::Character("a".into()), Some("a"), Modifiers::empty()),
            now,
        );
        assert_eq!(app.main.session.position(), 0);
        assert!(app.main.started.is_none());
        assert!(app.main.deadline.is_none());
        app.update_at(
            key(Key::Named(Named::Backspace), None, Modifiers::empty()),
            now,
        );
        app.update_at(Message::SelectMode(Mode::Words), now);
        assert_eq!(app.main.mode, Mode::Time);
        assert_eq!(app.main.revision, 0);
        app.update_at(Message::DismissUpdate, now);
        app.update_at(
            key(Key::Character("a".into()), Some("a"), Modifiers::empty()),
            now,
        );
        assert_eq!(app.main.session.position(), 1);
        assert_eq!(app.main.deadline, Some(now + Duration::from_secs(30)));
    }

    #[test]
    fn deferred_prompt_opens_on_dashboard_or_fresh_tab() {
        use typerr::update::Phase;
        for dashboard in [true, false] {
            let mut app = app("abc");
            let now = Instant::now();
            app.update_at(
                key(Key::Character("a".into()), Some("a"), Modifiers::empty()),
                now,
            );
            app.updater.phase = Phase::Available;
            let message = if dashboard {
                Message::Dashboard
            } else {
                key(Key::Named(Named::Tab), None, Modifiers::empty())
            };
            app.update_at(message, now);
            assert!(app.updater.visible);
        }
    }

    #[test]
    fn score_tracks_corrections_unicode_spaces_and_freezes_at_completion() {
        let mut app = app("ab c");
        let start = Instant::now();
        let send = |app: &mut App, text: &str, seconds| {
            app.update_at(
                key(Key::Character("x".into()), Some(text), Modifiers::empty()),
                start + Duration::from_secs(seconds),
            )
        };
        send(&mut app, "a", 0);
        send(&mut app, "é", 1);
        app.update_at(
            key(Key::Named(Named::Backspace), None, Modifiers::empty()),
            start + Duration::from_secs(2),
        );
        send(&mut app, "b", 3);
        send(&mut app, "x", 4); // Wrong expected separator counts as a mistake.
        send(&mut app, "cSURPLUS", 6);
        let score = app.main.score.clone();
        assert_eq!(score.elapsed, Duration::from_secs(6));
        assert_eq!(score.wpm, 6.); // Three correct retained scalars.
        assert_eq!(score.raw, 8.);
        assert_eq!(score.accuracy, 60.); // Three correct attempts out of five.
        assert_eq!(score.samples.iter().map(|s| s.errors).sum::<usize>(), 2);
        assert_eq!(score.samples.last().unwrap().wpm, score.wpm);
        send(&mut app, "late", 20);
        assert_eq!(app.main.score, score);
        app.update_at(Message::Retry, start + Duration::from_secs(21));
        assert_eq!(app.main.score, Score::default());
        assert!(app.main.started.is_none());
        send(&mut app, "ab", 22);
        send(&mut app, " c", 24);
        assert_eq!(app.main.score.accuracy, 100.);
        assert_eq!(app.main.score.wpm, 24.);
        assert_eq!(app.main.score.elapsed, Duration::from_secs(2));
    }

    #[test]
    fn timed_score_uses_exact_deadline_including_idle_unfocused_time() {
        for seconds in Mode::Time.amounts() {
            let mut app = app("abc def");
            app.main.mode = Mode::Time;
            app.main.time_limit_seconds = u64::from(seconds);
            let start = Instant::now();
            app.update_at(
                key(Key::Named(Named::Backspace), None, Modifiers::empty()),
                start,
            );
            app.update_at(
                key(Key::Character("x".into()), Some("\n"), Modifiers::empty()),
                start,
            );
            assert!(app.main.started.is_none());
            app.update_at(
                key(Key::Character("a".into()), Some("a"), Modifiers::empty()),
                start,
            );
            app.update_at(
                Message::Native(Event::Window(window::Event::Unfocused)),
                start + Duration::from_secs(1),
            );
            tick(
                &mut app,
                start + Duration::from_secs(u64::from(seconds) + 10),
            );
            assert_eq!(
                app.main.score.elapsed,
                Duration::from_secs(u64::from(seconds))
            );
            assert_eq!(app.main.score.wpm, 12. / f64::from(seconds));
            assert_eq!(app.main.score.accuracy, 100.);
            assert_eq!(app.main.score.consistency, 0.);
            assert_eq!(app.main.score.samples.len(), usize::from(seconds));
        }
    }

    #[test]
    fn all_session_replacements_clear_measurements() {
        for message in [
            Message::Next,
            Message::Retry,
            key(Key::Named(Named::Tab), None, Modifiers::empty()),
            Message::SelectMode(Mode::Time),
            Message::ToggleNumbers,
            Message::TogglePunctuation,
            Message::SelectAmount(Mode::Words, 10),
        ] {
            let mut app = app("abc def");
            let start = Instant::now();
            app.update_at(
                key(Key::Character("x".into()), Some("x"), Modifiers::empty()),
                start,
            );
            if matches!(message, Message::Next | Message::Retry) {
                app.finish(start + Duration::from_secs(1));
                tick(&mut app, start + Duration::from_secs(2));
            }
            if matches!(message, Message::SelectAmount(..)) {
                app.main.amount_picker = Some(Mode::Words);
            }
            app.update_at(message, start + Duration::from_secs(3));
            assert!(app.main.started.is_none());
            assert_eq!(app.main.score, Score::default());
            assert_eq!(app.main.statistics.finish(Duration::ZERO), Score::default());
        }
    }

    fn select_amount(app: &mut App, mode: Mode, amount: u16) {
        app.update(Message::ToggleAmountPicker(mode));
        app.update(Message::SelectAmount(mode, amount));
    }

    #[test]
    fn completed_tests_save_once_with_frozen_metrics_and_selected_settings() {
        use typerr::storage::{HistoryFilter, ScoreStore};
        for (mode, amounts) in [
            (Mode::Words, Mode::Words.amounts()),
            (Mode::Time, Mode::Time.amounts()),
        ] {
            for amount in amounts {
                let dir = tempfile::tempdir().unwrap();
                let path = dir.path().join("scores.db");
                let mut app = App {
                    score_writer: Some(ScoreWriter::start(path.clone()).unwrap()),
                    ..App::default()
                };
                select_amount(&mut app, mode, amount);
                app.main.options = words::Options {
                    punctuation: true,
                    numbers: true,
                };
                app.main.replace(vec!["ab,"; 500].join(" "));
                let start = Instant::now();
                app.update_at(
                    key(Key::Character("a".into()), Some("a"), Modifiers::empty()),
                    start,
                );
                if mode == Mode::Words {
                    let rest = format!("b,{}", " ab,".repeat(usize::from(amount) - 1));
                    app.update_at(
                        key(Key::Character("x".into()), Some(&rest), Modifiers::empty()),
                        start + Duration::from_secs(7),
                    );
                } else {
                    // Late input must expire the test before changing the saved score.
                    app.update_at(
                        key(Key::Character("x".into()), Some("late"), Modifiers::empty()),
                        start + Duration::from_secs(u64::from(amount) + 5),
                    );
                }
                let frozen = app.main.score.clone();
                for _ in 0..3 {
                    tick(&mut app, start + Duration::from_secs(200));
                }
                app.update_at(Message::Next, start + Duration::from_secs(201));
                app.update_at(
                    key(Key::Named(Named::Tab), None, Modifiers::empty()),
                    start + Duration::from_secs(202),
                );
                drop(app); // Drain the real queue before reading.
                let rows = ScoreStore::open(&path)
                    .unwrap()
                    .history(HistoryFilter::default())
                    .unwrap();
                assert_eq!(rows.len(), 1);
                let saved = &rows[0].test;
                assert_eq!(
                    saved.mode,
                    if mode == Mode::Words {
                        TestMode::Words
                    } else {
                        TestMode::Time
                    }
                );
                assert_eq!(saved.amount, amount);
                assert!(saved.punctuation && saved.numbers && saved.completed_at_ms > 0);
                assert_eq!(saved.wpm, frozen.wpm);
                assert_eq!(saved.raw_wpm, frozen.raw);
                assert_eq!(saved.accuracy, frozen.accuracy);
                assert_eq!(saved.consistency, frozen.consistency);
                assert_eq!(
                    saved.elapsed_seconds,
                    if mode == Mode::Words {
                        7.
                    } else {
                        f64::from(amount)
                    }
                );
            }
        }
    }

    #[test]
    fn abandoned_tests_do_not_save_but_retry_and_cancelled_fade_keep_completed_results() {
        use typerr::storage::{HistoryFilter, ScoreStore};
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("scores.db");
        let mut app = app("ab");
        app.score_writer = Some(ScoreWriter::start(path.clone()).unwrap());
        let start = Instant::now();
        let send = |app: &mut App, text, seconds| {
            app.update_at(
                key(Key::Character("x".into()), Some(text), Modifiers::empty()),
                start + Duration::from_secs(seconds),
            )
        };
        send(&mut app, "a", 0);
        app.update_at(
            key(Key::Named(Named::Tab), None, Modifiers::empty()),
            start + Duration::from_secs(1),
        );
        app.main.replace("ab".into());
        send(&mut app, "ab", 2); // Buffer exhaustion; valid zero-duration score.
        app.update_at(
            key(Key::Named(Named::Tab), None, Modifiers::empty()),
            start + Duration::from_secs(2),
        );
        assert_eq!(app.page, Page::Typing); // Cancels fade, not the completed record.
        app.main.replace("ab".into());
        send(&mut app, "a", 3);
        send(&mut app, "b", 4);
        tick(&mut app, start + Duration::from_secs(5));
        app.update_at(Message::Retry, start + Duration::from_secs(5));
        send(&mut app, "a", 6);
        send(&mut app, "b", 8);
        drop(app);
        let rows = ScoreStore::open(&path)
            .unwrap()
            .history(HistoryFilter::default())
            .unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(
            rows.iter()
                .map(|r| r.test.elapsed_seconds)
                .collect::<Vec<_>>(),
            [0., 1., 2.]
        );
    }

    #[test]
    fn every_word_amount_finishes_at_its_final_character_including_punctuation() {
        for amount in Mode::Words.amounts() {
            let mut app = App::default();
            select_amount(&mut app, Mode::Words, amount);
            assert_eq!(app.main.session.word_count(), 500);
            app.main.replace(vec!["ab,"; 500].join(" "));
            let now = Instant::now();
            let prefix = format!("{}xx", "xxx ".repeat(usize::from(amount) - 1));
            app.update_at(
                key(
                    Key::Character("x".into()),
                    Some(&prefix),
                    Modifiers::empty(),
                ),
                now,
            );
            assert_eq!(app.page, Page::Typing);
            assert_eq!(app.main.monitor(), format!("{}/{amount}", amount - 1));
            app.update_at(
                key(
                    Key::Character("x".into()),
                    Some("x surplus"),
                    Modifiers::empty(),
                ),
                now,
            );
            assert!(matches!(app.page, Page::Transition { .. }));
            assert_eq!(app.main.monitor(), format!("{amount}/{amount}"));
            assert_eq!(app.main.session.position(), usize::from(amount) * 4 - 1);
        }
    }

    #[test]
    fn every_time_amount_starts_on_input_and_expires_before_late_edits() {
        for amount in Mode::Time.amounts() {
            let mut app = App::default();
            select_amount(&mut app, Mode::Time, amount);
            assert_eq!(app.main.monitor(), format!("{amount}s"));
            assert_eq!(app.main.deadline, None);
            let now = Instant::now();
            app.update_at(
                key(Key::Character("x".into()), Some("x"), Modifiers::empty()),
                now,
            );
            assert_eq!(app.main.next_tick(), Some(now + Duration::from_secs(1)));
            let deadline = now + Duration::from_secs(u64::from(amount));
            assert_eq!(app.main.deadline, Some(deadline));
            tick(&mut app, deadline - Duration::from_millis(1));
            assert_eq!(app.main.monitor(), "1s");
            app.update_at(
                key(Key::Character("x".into()), Some("x"), Modifiers::empty()),
                deadline,
            );
            assert_eq!(app.main.session.position(), 1);
            assert_eq!(app.main.monitor(), "0s");
            assert!(matches!(app.page, Page::Transition { .. }));
        }
    }

    #[test]
    fn amounts_are_remembered_independently_across_modes_toggles_tab_next_and_retry() {
        let mut app = App::default();
        select_amount(&mut app, Mode::Words, 25);
        select_amount(&mut app, Mode::Time, 120);
        app.update(Message::SelectMode(Mode::Words));
        assert_eq!(app.main.monitor(), "0/25");
        app.update(Message::SelectMode(Mode::Time));
        assert_eq!(app.main.monitor(), "120s");
        for action in [
            Message::ToggleNumbers,
            Message::TogglePunctuation,
            key(Key::Named(Named::Tab), None, Modifiers::empty()),
            Message::Next,
            Message::Retry,
        ] {
            let target = app.main.session.target().to_vec();
            let retry = matches!(action, Message::Retry);
            if matches!(action, Message::Next | Message::Retry) {
                app.page = Page::Score;
            }
            app.update(action);
            assert_eq!(app.main.word_target, 25);
            assert_eq!(app.main.time_limit_seconds, 120);
            assert_eq!(app.main.monitor(), "120s");
            assert_eq!(app.main.deadline, None);
            assert_eq!(app.main.session.position(), 0);
            if retry {
                assert_eq!(app.main.session.target(), target);
            }
        }
    }

    #[test]
    fn picker_opening_and_dismissal_preserve_session_and_deadline() {
        let mut app = App::default();
        app.update(Message::SelectMode(Mode::Time));
        app.update(key(
            Key::Character("x".into()),
            Some("x"),
            Modifiers::empty(),
        ));
        let generation = app.main.generation;
        let deadline = app.main.deadline;
        for dismiss in [
            Message::DismissAmountPicker,
            Message::ToggleAmountPicker(Mode::Words),
            key(Key::Named(Named::Escape), None, Modifiers::empty()),
            Message::SelectMode(Mode::Time),
            Message::Native(Event::Window(window::Event::Unfocused)),
        ] {
            app.update(Message::ToggleAmountPicker(Mode::Words));
            assert_eq!(app.main.amount_picker, Some(Mode::Words));
            assert_eq!(app.main.mode, Mode::Time);
            app.update(dismiss);
            assert_eq!(app.main.amount_picker, None);
            assert_eq!(app.main.generation, generation);
            assert_eq!(app.main.deadline, deadline);
            assert_eq!(app.main.session.position(), 1);
        }
    }

    #[test]
    fn choosing_amount_resets_once_and_ignores_invalid_or_stale_choices() {
        let mut app = App::default();
        app.update(Message::ToggleAmountPicker(Mode::Time));
        for action in [
            Message::SelectAmount(Mode::Words, 25),
            Message::SelectAmount(Mode::Time, 0),
        ] {
            app.update(action);
            assert_eq!(app.main.generation, 0);
        }
        app.update(Message::SelectAmount(Mode::Time, 60));
        assert_eq!(app.main.generation, 1);
        assert_eq!(app.main.amount_picker, None);
        assert_eq!(app.main.mode, Mode::Time);
        assert_eq!(app.main.monitor(), "60s");
        app.update(Message::SelectAmount(Mode::Time, 60));
        assert_eq!(app.main.generation, 1);
        let now = Instant::now();
        app.finish(now);
        for action in [
            Message::ToggleAmountPicker(Mode::Words),
            Message::SelectAmount(Mode::Words, 10),
        ] {
            app.update_at(action, now);
            assert_eq!(app.main.amount_picker, None);
            assert_eq!(app.main.generation, 1);
        }
    }

    #[test]
    fn native_pointer_events_open_select_dismiss_and_reuse_mode_amounts() {
        use iced::advanced::{
            Layout, Shell, clipboard, layout, mouse, renderer::Headless, widget::Tree,
        };
        let renderer = iced::futures::executor::block_on(iced::Renderer::new(
            crate::ui::theme::FONT,
            16.0.into(),
            Some("tiny-skia"),
        ))
        .expect("software renderer");
        let mut app = App::default();
        let click_at_size = |app: &mut App, x, y, button, size| {
            let mut view = app.view();
            let mut tree = Tree::new(&view);
            let node = view.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(iced::Size::ZERO, size),
            );
            let mut messages = Vec::new();
            for event in [
                mouse::Event::ButtonPressed(button),
                mouse::Event::ButtonReleased(button),
            ] {
                view.as_widget_mut().update(
                    &mut tree,
                    &Event::Mouse(event),
                    Layout::new(&node),
                    mouse::Cursor::Available(iced::Point::new(x, y)),
                    &renderer,
                    &mut clipboard::Null,
                    &mut Shell::new(&mut messages),
                    &iced::Rectangle::with_size(size),
                );
            }
            drop(view);
            for message in messages {
                app.update(message);
            }
        };
        let click = |app: &mut App, x, y, button| {
            click_at_size(app, x, y, button, iced::Size::new(700., 400.))
        };
        let profile_dir = tempfile::tempdir().unwrap();
        let profile_path = profile_dir.path().join("settings.json");
        app.profile = Some(crate::settings::Profile::at(profile_path.clone(), None));
        for (x, y, button) in [
            (615., 45., mouse::Button::Left),
            (377., 45., mouse::Button::Right),
            (350., 245., mouse::Button::Left),
        ] {
            click(&mut app, x, y, button);
            assert!(app.needs_name());
            assert_eq!(app.page, Page::Typing);
            assert_eq!(app.main.amount_picker, None);
        }
        for message in [
            Message::Dashboard,
            Message::OpenSettings,
            key(Key::Character(",".into()), Some(","), Modifiers::CTRL),
            Message::SelectMode(Mode::Time),
            Message::ToggleNumbers,
            key(Key::Named(Named::Escape), None, Modifiers::empty()),
            key(Key::Named(Named::Tab), None, Modifiers::empty()),
            key(Key::Character("x".into()), Some("x"), Modifiers::empty()),
        ] {
            app.update(message);
        }
        assert_eq!(app.main.session.position(), 0);
        assert_eq!(app.main.generation, 0);
        assert_eq!(app.main.mode, Mode::Words);
        assert!(app.main.started.is_none());
        // Route committed keyboard text through the real native text input.
        let mut view = app.view();
        let mut tree = Tree::new(&view);
        let size = iced::Size::new(700., 400.);
        let node = view.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(iced::Size::ZERO, size),
        );
        let Message::Native(text_event) =
            key(Key::Character("ë".into()), Some("ë"), Modifiers::empty())
        else {
            unreachable!()
        };
        let mut messages = Vec::new();
        for event in [
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            text_event,
        ] {
            view.as_widget_mut().update(
                &mut tree,
                &event,
                Layout::new(&node),
                mouse::Cursor::Available(iced::Point::new(300., 200.)),
                &renderer,
                &mut clipboard::Null,
                &mut Shell::new(&mut messages),
                &iced::Rectangle::with_size(size),
            );
        }
        drop(view);
        for message in messages {
            app.update(message);
        }
        assert_eq!(app.name_draft, "ë");
        assert_eq!(app.main.session.position(), 0);
        app.update(Message::NameChanged("abcdefghijklmnopq".into()));
        assert_eq!(app.name_draft, "abcdefghijklmnop");
        click(&mut app, 350., 245., mouse::Button::Left);
        assert!(!app.needs_name());
        assert_eq!(app.user_name(), "abcdefghijklmnop");
        assert_eq!(
            crate::settings::Profile::at(profile_path, None)
                .name
                .as_deref(),
            Some("abcdefghijklmnop")
        );
        app.profile = None;
        app.updater.phase = typerr::update::Phase::Available;
        app.updater.visible = true;
        click(&mut app, 615., 45., mouse::Button::Left);
        assert_eq!(app.page, Page::Typing);
        click(&mut app, 377., 45., mouse::Button::Right);
        assert_eq!(app.main.amount_picker, None);
        click(&mut app, 365., 250., mouse::Button::Left);
        assert!(!app.updater.visible, "Later dismisses the native modal");
        let initial_target = app.main.session.target().to_vec();
        click(&mut app, 615.0, 45.0, mouse::Button::Left);
        assert_eq!(app.page, Page::Dashboard);
        assert_eq!(app.main.generation, 0);
        assert_eq!(app.main.session.target(), initial_target);
        click(&mut app, 100.0, 45.0, mouse::Button::Left);
        assert_eq!(app.page, Page::Typing);
        assert_eq!(app.main.generation, 1);
        // The remaining assertions exercise mode menus independently.
        app.main.generation = 0;
        click(&mut app, 412.0, 45.0, mouse::Button::Right);
        assert_eq!(app.main.amount_picker, Some(Mode::Words));
        assert_eq!(app.main.generation, 0);
        click(&mut app, 385.0, 80.0, mouse::Button::Left);
        assert_eq!(app.main.monitor(), "0/25");
        assert_eq!(app.main.generation, 1);
        assert_eq!(app.main.amount_picker, None);
        click(&mut app, 377.0, 45.0, mouse::Button::Right);
        assert_eq!(app.main.amount_picker, Some(Mode::Time));
        click(&mut app, 440.0, 80.0, mouse::Button::Left);
        assert_eq!(app.main.monitor(), "120s");
        click(&mut app, 412.0, 45.0, mouse::Button::Left);
        assert_eq!(app.main.monitor(), "0/25");
        assert_eq!(app.main.amount_picker, None);
        let generation = app.main.generation;
        click(&mut app, 377.0, 45.0, mouse::Button::Right);
        click(&mut app, 300.0, 200.0, mouse::Button::Left);
        assert_eq!(app.main.amount_picker, None);
        assert_eq!(app.main.generation, generation);
        click(&mut app, 377.0, 45.0, mouse::Button::Left);
        assert_eq!(app.main.monitor(), "120s");
        app.page = Page::Score;
        let generation = app.main.generation;
        click(&mut app, 615.0, 45.0, mouse::Button::Left);
        assert_eq!(app.page, Page::Dashboard);
        assert_eq!(app.main.generation, generation);
        for (size, point) in [
            (iced::Size::new(1400., 800.), iced::Point::new(310., 110.)),
            (iced::Size::new(1920., 1080.), iced::Point::new(440., 160.)),
            (iced::Size::new(700., 900.), iced::Point::new(150., 95.)),
        ] {
            app.page = Page::Dashboard;
            // The formerly scaled logo location must no longer activate Home.
            click_at_size(&mut app, point.x, point.y, mouse::Button::Left, size);
            assert_eq!(app.page, Page::Dashboard);
            click_at_size(&mut app, 100., 45., mouse::Button::Left, size);
            assert_eq!(app.page, Page::Typing);
        }
    }

    #[test]
    fn dashboard_blocks_typing_and_timer_completion_and_returns_to_fresh_test() {
        let mut app = app("ab cd");
        let now = Instant::now();
        app.main.mode = Mode::Time;
        app.main.time_limit_seconds = 15;
        app.main.options.numbers = true;
        app.update_at(
            key(Key::Character("a".into()), Some("a"), Modifiers::empty()),
            now,
        );
        app.main.amount_picker = Some(Mode::Words);
        app.update_at(Message::Dashboard, now);
        assert_eq!(app.page, Page::Dashboard);
        assert_eq!(app.main.amount_picker, None);
        app.update_at(
            key(Key::Character("b".into()), Some("b"), Modifiers::empty()),
            now + Duration::from_secs(20),
        );
        assert_eq!(app.main.session.position(), 1);
        assert_eq!(app.page, Page::Dashboard);
        app.update_at(Message::BackToTyping, now + Duration::from_secs(20));
        assert_eq!(app.page, Page::Typing);
        assert_eq!(app.main.session.position(), 0);
        assert_eq!(app.main.generation, 1);
        assert!(app.main.started.is_none());
        assert!(app.main.deadline.is_none());
        assert_eq!(app.main.mode, Mode::Time);
        assert_eq!(app.main.time_limit_seconds, 15);
        assert!(app.main.options.numbers);
        app.update_at(Message::Dashboard, now);
        app.update_at(key(Key::Named(Named::Tab), None, Modifiers::empty()), now);
        assert_eq!(app.page, Page::Typing);
        assert_eq!(app.main.generation, 2);
        app.page = Page::Transition { started: now, now };
        app.update_at(Message::Dashboard, now);
        assert!(matches!(app.page, Page::Transition { .. }));
    }

    #[test]
    fn dashboard_replies_are_nonblocking_and_failures_clear_stale_data() {
        let mut app = App::default();
        app.update(Message::Dashboard);
        let (sender, receiver) = std::sync::mpsc::channel();
        app.dashboard_reply = Some(receiver);
        app.advance(Instant::now());
        assert!(app.dashboard_reply.is_some());
        sender
            .send(Ok(typerr::storage::DashboardData::default()))
            .unwrap();
        app.advance(Instant::now());
        assert!(app.dashboard.is_some());
        assert!(app.dashboard_reply.is_none());
        app.update(Message::BackToTyping);
        app.update(Message::Dashboard);
        assert!(app.dashboard.is_none());
        let (sender, receiver) = std::sync::mpsc::channel();
        app.dashboard_reply = Some(receiver);
        sender
            .send(Err(typerr::storage::Error::Invalid("unavailable")))
            .unwrap();
        app.advance(Instant::now());
        assert!(app.dashboard.is_none());
        assert!(app.dashboard_reply.is_none());
    }

    #[test]
    fn burst_edits_preserve_generation_and_only_session_resets_change_it() {
        let mut app = app(&"a".repeat(2000));
        for _ in 0..1000 {
            app.update(key(
                Key::Character("a".into()),
                Some("a"),
                Modifiers::empty(),
            ));
        }
        assert_eq!(app.main.session.position(), 1000);
        assert_eq!(app.main.generation, 0);
        for _ in 0..1000 {
            app.update(key(Key::Named(Named::Backspace), None, Modifiers::empty()));
        }
        assert_eq!(app.main.session.position(), 0);
        assert_eq!(app.main.generation, 0);
        for (index, message) in [
            key(Key::Named(Named::Tab), None, Modifiers::empty()),
            Message::SelectMode(Mode::Time),
            Message::TogglePunctuation,
            Message::ToggleNumbers,
        ]
        .into_iter()
        .enumerate()
        {
            app.update(message);
            assert_eq!(app.main.generation, index as u64 + 1);
            let generation = app.main.generation;
            app.update(Message::SelectMode(app.main.mode));
            app.update(key(Key::Named(Named::Backspace), None, Modifiers::empty()));
            tick(&mut app, Instant::now());
            assert_eq!(app.main.generation, generation);
        }
    }

    #[test]
    fn native_text_is_consumed_once_and_shortcuts_controls_are_ignored() {
        let mut app = app("ab cd");
        app.update(key(
            Key::Character("a".into()),
            Some("a"),
            Modifiers::empty(),
        ));
        assert_eq!(app.main.session.position(), 1);
        for modifier in [Modifiers::CTRL, Modifiers::ALT, Modifiers::LOGO] {
            app.update(key(Key::Character("b".into()), Some("b"), modifier));
        }
        app.update(key(
            Key::Named(Named::Enter),
            Some("\r"),
            Modifiers::empty(),
        ));
        assert_eq!(app.main.revision, 1);
        app.update(key(Key::Character("B".into()), Some("B"), Modifiers::SHIFT));
        assert_eq!(app.main.session.entered(), &['a', 'B']);
    }

    #[test]
    fn focus_and_terminal_state_gate_edits_and_noops_preserve_revision() {
        let mut app = app("ab");
        app.update(key(Key::Named(Named::Backspace), None, Modifiers::empty()));
        assert_eq!(app.main.revision, 0);
        app.update(Message::Native(Event::Window(window::Event::Unfocused)));
        app.update(key(
            Key::Character("a".into()),
            Some("a"),
            Modifiers::empty(),
        ));
        assert_eq!(app.main.session.position(), 0);
        app.update(Message::Native(Event::Window(window::Event::Focused)));
        app.update(key(
            Key::Character("a".into()),
            Some("abextra"),
            Modifiers::empty(),
        ));
        assert_eq!(app.main.session.entered(), &['a', 'b']);
        assert_eq!(app.main.revision, 1);
        app.update(key(Key::Named(Named::Backspace), None, Modifiers::empty()));
        assert_eq!(app.main.revision, 1);
    }

    #[test]
    fn tab_restarts_active_and_completed_races() {
        for input in ["a", "ab"] {
            let mut app = app("ab");
            app.update(key(
                Key::Character(input.into()),
                Some(input),
                Modifiers::empty(),
            ));
            app.update(key(Key::Named(Named::Tab), Some("\t"), Modifiers::empty()));
            assert_eq!(app.main.session.position(), 0);
            assert_eq!(
                app.main
                    .session
                    .target()
                    .iter()
                    .filter(|c| **c == ' ')
                    .count(),
                499
            );
            assert_eq!(app.main.revision, 2);
            app.update(key(
                Key::Character("a".into()),
                Some("a"),
                Modifiers::empty(),
            ));
            assert_eq!(app.main.session.position(), 1);
        }
    }

    #[test]
    fn tab_ignores_shortcuts_repeat_and_unfocused_windows() {
        let mut app = app("original");
        for modifiers in [
            Modifiers::CTRL,
            Modifiers::ALT,
            Modifiers::LOGO,
            Modifiers::SHIFT,
        ] {
            app.update(key(Key::Named(Named::Tab), None, modifiers));
        }
        let mut held = key(Key::Named(Named::Tab), None, Modifiers::empty());
        if let Message::Native(Event::Keyboard(keyboard::Event::KeyPressed { repeat, .. })) =
            &mut held
        {
            *repeat = true;
        }
        app.update(held);
        app.update(Message::Native(Event::Window(window::Event::Unfocused)));
        app.update(key(Key::Named(Named::Tab), None, Modifiers::empty()));
        assert_eq!(
            app.main.session.target(),
            &"original".chars().collect::<Vec<_>>()
        );
        assert_eq!(app.main.revision, 0);
    }
    fn tick(app: &mut App, now: Instant) {
        app.update_at(
            Message::Native(Event::Window(window::Event::RedrawRequested(now))),
            now,
        );
    }

    #[test]
    fn time_mode_waits_for_printable_input_and_uses_elapsed_time() {
        let mut app = app("abcdef");
        let start = Instant::now();
        app.update_at(Message::SelectMode(Mode::Time), start);
        assert_eq!(app.main.monitor(), "30s");
        assert_eq!(app.main.next_tick(), None);
        for message in [
            key(Key::Named(Named::Backspace), None, Modifiers::empty()),
            key(Key::Named(Named::Enter), Some("\r"), Modifiers::empty()),
            key(Key::Character("a".into()), Some("a"), Modifiers::CTRL),
        ] {
            app.update_at(message, start);
        }
        tick(&mut app, start + Duration::from_secs(60));
        assert_eq!(app.main.monitor(), "30s");
        assert_eq!(app.main.deadline, None);
        app.update_at(
            key(Key::Character("x".into()), Some("x"), Modifiers::empty()),
            start,
        );
        assert_eq!(app.main.next_tick(), Some(start + Duration::from_secs(1)));
        tick(&mut app, start + Duration::from_millis(999));
        assert_eq!(app.main.monitor(), "30s");
        tick(&mut app, start + Duration::from_secs(1));
        assert_eq!(app.main.monitor(), "29s");
        app.update_at(
            key(Key::Named(Named::Backspace), None, Modifiers::empty()),
            start + Duration::from_secs(2),
        );
        assert_eq!(app.main.session.position(), 0);
        app.update(Message::Native(Event::Window(window::Event::Unfocused)));
        tick(&mut app, start + Duration::from_millis(12_500));
        assert_eq!(app.main.monitor(), "18s");
        assert_eq!(app.main.next_tick(), Some(start + Duration::from_secs(13)));
        assert_eq!(app.main.deadline, Some(start + Duration::from_secs(30)));
    }

    #[test]
    fn timer_expiry_freezes_session_and_opens_score() {
        let mut app = app("abcdef");
        app.main.mode = Mode::Time;
        let start = Instant::now();
        app.update_at(
            key(Key::Character("a".into()), Some("a"), Modifiers::empty()),
            start,
        );
        let revision = app.main.revision;
        tick(&mut app, start + Duration::from_secs(30));
        assert_eq!(app.main.monitor(), "0s");
        assert_eq!(app.main.next_tick(), None);
        assert_eq!(app.main.session.entered(), &['a']);
        assert_eq!(app.main.revision, revision);
        tick(&mut app, start + Duration::from_secs(100));
        assert_eq!(app.main.monitor(), "0s");
        app.update_at(
            key(Key::Character("b".into()), Some("b"), Modifiers::empty()),
            start + Duration::from_secs(101),
        );
        assert_eq!(app.main.session.entered(), &['a']);
        assert_eq!(app.page, Page::Score);
        assert_eq!(app.main.monitor(), "0s");
        app.update(key(Key::Named(Named::Backspace), None, Modifiers::empty()));
        assert_eq!(app.main.session.entered(), &['a']);
    }

    #[test]
    fn mode_changes_and_tab_reset_sessions_but_active_mode_click_is_noop() {
        let mut app = App::default();
        assert_eq!(app.main.mode, Mode::Words);
        assert_eq!(app.main.monitor(), "0/50");
        for mode in [Mode::Time, Mode::Words, Mode::Time] {
            app.update(Message::SelectMode(mode));
            assert_eq!(app.main.mode, mode);
            assert_eq!(app.main.session.word_count(), 500);
            assert_eq!(app.main.session.position(), 0);
            assert_eq!(app.main.deadline, None);
            let start = Instant::now();
            app.update_at(
                key(Key::Character("a".into()), Some("a"), Modifiers::empty()),
                start,
            );
            tick(&mut app, start + Duration::from_secs(10));
            assert_eq!(
                app.main.seconds_remaining,
                if mode == Mode::Time { 20 } else { 30 }
            );
            let revision = app.main.revision;
            app.update(Message::SelectMode(mode));
            assert_eq!(app.main.session.position(), 1);
            assert_eq!(app.main.revision, revision);
            app.update(key(Key::Named(Named::Tab), None, Modifiers::empty()));
            assert_eq!(app.main.mode, mode);
            assert_eq!(app.main.session.word_count(), 500);
            assert_eq!(app.main.session.position(), 0);
            assert_eq!(app.main.seconds_remaining, 30);
            assert_eq!(app.main.deadline, None);
            assert_eq!(app.main.revision, revision + 1);
            // Leave progress before changing modes on the next iteration.
            app.update_at(
                key(Key::Character("a".into()), Some("a"), Modifiers::empty()),
                start,
            );
        }
    }
    #[test]
    fn word_target_finishes_on_fiftieth_final_character_and_discards_remaining_text() {
        let mut app = app(&vec!["ab,"; 500].join(" "));
        let now = Instant::now();
        app.update_at(
            key(
                Key::Character("x".into()),
                Some(&"xxx ".repeat(49)),
                Modifiers::empty(),
            ),
            now,
        );
        assert_eq!(app.main.session.completed_words(), 49);
        app.update_at(
            key(Key::Character("x".into()), Some("xx"), Modifiers::empty()),
            now,
        );
        assert_eq!(app.page, Page::Typing);
        assert_eq!(app.main.monitor(), "49/50");
        app.update_at(
            key(
                Key::Character("x".into()),
                Some("x ignored extra"),
                Modifiers::empty(),
            ),
            now,
        );
        assert_eq!(app.main.session.completed_words(), 50);
        assert_eq!(app.main.session.position(), 199);
        assert!(matches!(app.page, Page::Transition { .. }));
        let revision = app.main.revision;
        app.update_at(
            key(Key::Named(Named::Backspace), None, Modifiers::empty()),
            now,
        );
        app.update_at(Message::ToggleNumbers, now);
        app.update_at(Message::Next, now);
        assert_eq!(app.main.revision, revision);
        assert_eq!(app.main.session.position(), 199);
    }

    #[test]
    fn crossfade_uses_elapsed_time_and_tab_can_cancel() {
        let mut app = app("a");
        let now = Instant::now();
        app.update_at(
            key(Key::Character("a".into()), Some("a"), Modifiers::empty()),
            now,
        );
        assert_eq!(app.page.score_opacity(), 0.);
        tick(&mut app, now + FADE_DURATION / 2);
        assert!((app.page.score_opacity() - 0.5).abs() < 0.0001);
        app.update_at(
            key(Key::Named(Named::Tab), None, Modifiers::empty()),
            now + FADE_DURATION / 2,
        );
        assert_eq!(app.page, Page::Typing);
        assert_eq!(app.main.generation, 1);
        app.finish(now);
        tick(&mut app, now + FADE_DURATION);
        assert_eq!(app.page, Page::Score);
        assert_eq!(app.page.score_opacity(), 1.);
        tick(&mut app, now + Duration::from_secs(60));
        assert_eq!(app.page, Page::Score);
    }

    #[test]
    fn expiry_precedes_input_and_catches_up_after_focus_loss() {
        for elapsed in [Duration::from_secs(30), Duration::from_secs(60)] {
            let mut app = app("abcdef");
            app.main.mode = Mode::Time;
            let now = Instant::now();
            app.update_at(
                key(Key::Character("a".into()), Some("a"), Modifiers::empty()),
                now,
            );
            app.update_at(
                key(Key::Character("b".into()), Some("b"), Modifiers::empty()),
                now + elapsed,
            );
            assert_eq!(app.main.session.entered(), &['a']);
            assert_ne!(app.page, Page::Typing);
            app.update_at(
                Message::Native(Event::Window(window::Event::Unfocused)),
                now + elapsed,
            );
            tick(&mut app, now + elapsed + FADE_DURATION);
            assert_eq!(app.page, Page::Score);
        }
    }

    #[test]
    fn score_actions_preserve_options_and_reset_all_session_state() {
        for action in [
            Message::Next,
            Message::Retry,
            key(Key::Named(Named::Tab), None, Modifiers::empty()),
        ] {
            let mut app = app("same text,");
            app.main.mode = Mode::Time;
            app.main.options = words::Options {
                punctuation: true,
                numbers: true,
            };
            let now = Instant::now();
            app.update_at(
                key(Key::Character("x".into()), Some("x"), Modifiers::empty()),
                now,
            );
            tick(&mut app, now + Duration::from_secs(31));
            let revision = app.main.revision;
            let retry = matches!(action, Message::Retry);
            app.update_at(action, now + Duration::from_secs(32));
            assert_eq!(app.page, Page::Typing);
            assert_eq!(app.main.mode, Mode::Time);
            assert!(app.main.options.punctuation && app.main.options.numbers);
            assert_eq!(app.main.session.position(), 0);
            assert_eq!(app.main.deadline, None);
            assert_eq!(app.main.seconds_remaining, 30);
            assert_eq!(app.main.revision, revision + 1);
            assert_eq!(app.main.generation, 1);
            if retry {
                assert_eq!(
                    app.main.session.target().iter().collect::<String>(),
                    "same text,"
                );
            } else {
                assert_eq!(app.main.session.word_count(), 500);
            }
        }
    }
    #[test]
    fn feature_toggles_are_independent_reset_progress_and_survive_restarts_and_modes() {
        for mode in [Mode::Words, Mode::Time] {
            let mut app = App::default();
            app.update(Message::SelectMode(mode));
            for (toggle, punctuation, numbers) in [
                (Message::TogglePunctuation, true, false),
                (Message::ToggleNumbers, true, true),
                (Message::TogglePunctuation, false, true),
                (Message::ToggleNumbers, false, false),
            ] {
                let now = Instant::now();
                app.update_at(
                    key(Key::Character("x".into()), Some("x"), Modifiers::empty()),
                    now,
                );
                tick(&mut app, now + Duration::from_secs(5));
                let revision = app.main.revision;
                app.update(toggle);
                let expected = words::Options {
                    punctuation,
                    numbers,
                };
                assert_eq!(app.main.options, expected);
                assert_eq!(app.main.mode, mode);
                assert_eq!(app.main.revision, revision + 1);
                assert_eq!(app.main.session.position(), 0);
                assert_eq!(app.main.deadline, None);
                assert_eq!(app.main.seconds_remaining, 30);
                assert_eq!(
                    app.main.monitor(),
                    if mode == Mode::Words { "0/50" } else { "30s" }
                );
                for action in [
                    key(Key::Named(Named::Tab), None, Modifiers::empty()),
                    Message::SelectMode(if mode == Mode::Words {
                        Mode::Time
                    } else {
                        Mode::Words
                    }),
                    Message::SelectMode(mode),
                ] {
                    app.update(action);
                    assert_eq!(app.main.options, expected);
                    assert_eq!(app.main.session.word_count(), 500);
                    let target = app.main.session.target();
                    assert_eq!(target.iter().any(|c| c.is_ascii_punctuation()), punctuation);
                    assert_eq!(target.iter().any(|c| c.is_ascii_digit()), numbers);
                }
            }
        }
    }

    #[test]
    fn punctuation_and_numbers_use_committed_text_and_word_boundaries() {
        let mut app = app("word, 42! next");
        app.update(key(
            Key::Character("word".into()),
            Some("word"),
            Modifiers::empty(),
        ));
        assert_eq!(app.main.monitor(), "0/50");
        app.update(key(
            Key::Character(",".into()),
            Some(","),
            Modifiers::empty(),
        ));
        assert_eq!(app.main.monitor(), "1/50");
        app.update(key(
            Key::Character(" 42".into()),
            Some(" 42"),
            Modifiers::empty(),
        ));
        app.update(key(Key::Character("!".into()), Some("!"), Modifiers::SHIFT));
        assert_eq!(app.main.monitor(), "2/50");
        for index in 0..app.main.session.position() {
            assert_eq!(
                app.main.session.character_state(index),
                Some(typerr::engine::CharacterState::Correct)
            );
        }
        app.update(key(Key::Named(Named::Backspace), None, Modifiers::empty()));
        assert_eq!(app.main.monitor(), "1/50");
    }
}
