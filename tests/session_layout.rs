use typerr::{
    engine::{SessionStatus, TypingSession},
    layout::{TextLayout, Viewport},
};

#[test]
fn separator_consumption_and_correction_move_the_viewport() {
    for separator_input in [' ', 'x'] {
        let mut session = TypingSession::new("one two three");
        let layout = TextLayout::new(session.target(), 82.0, 10.0, 25.0);
        let mut viewport = Viewport::default();
        viewport.resize(&layout, 50.0, 0);
        for c in "one two".chars() {
            session.input(c);
            viewport.follow(&layout, session.position());
        }
        assert_eq!(viewport.top_line, 0);
        assert_eq!(layout.caret(session.position()).x, 70.0);
        session.input(separator_input);
        viewport.follow(&layout, session.position());
        assert_eq!(viewport.top_line, 1);
        assert_eq!(layout.caret(session.position()).x, 0.0);
        assert!(session.backspace());
        viewport.follow(&layout, session.position());
        assert_eq!(viewport.top_line, 0);
        session.input(' ');
        for c in "three".chars() {
            session.input(c);
            viewport.follow(&layout, session.position());
        }
        assert_eq!(session.status(), SessionStatus::Complete);
        assert_eq!(viewport.top_line, 1);
        assert_eq!(layout.caret(session.position()).x, 50.0);
        assert_eq!(layout.caret(session.position()).y, 25.0);
        viewport.scroll(-1, layout.lines.len());
        assert!(!session.input('a'));
        assert!(!session.backspace());
        assert_eq!(viewport.top_line, 0);
    }
}

#[test]
fn every_character_and_caret_fits_supported_panel_widths() {
    let target: Vec<_> = "one extraordinarilylongword two three four five six seven eight nine ten"
        .chars()
        .collect();
    for width in [600.0, 640.0, 800.0, 1000.0] {
        // Measured metrics enter this pure layer; this value is a synthetic test fixture.
        let layout = TextLayout::new(&target, width, 12.25, 25.0);
        assert_eq!(
            layout
                .lines
                .iter()
                .flat_map(Clone::clone)
                .collect::<Vec<_>>(),
            (0..target.len()).collect::<Vec<_>>()
        );
        for position in 0..=target.len() {
            let caret = layout.caret(position);
            assert!(caret.x >= 0.0 && caret.x + 2.0 <= width);
            assert_eq!(caret.y, layout.active_line(position) as f32 * 25.0);
        }
    }
}
