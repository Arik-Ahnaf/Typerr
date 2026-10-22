use typerr::layout::CaretTarget;

/// Presentation coordinates have their own lifetime, independent of engine progress.
/// A later animation can interpolate `rendered` toward `target` on frame events.
#[derive(Default)]
pub struct Caret {
    pub target: CaretTarget,
    pub rendered: CaretTarget,
}

impl Caret {
    pub fn snap_to(&mut self, target: CaretTarget) {
        self.target = target;
        self.rendered = self.target;
    }
}
