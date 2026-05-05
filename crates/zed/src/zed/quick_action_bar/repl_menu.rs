use gpui::AnyElement;
use super::QuickActionBar;

impl QuickActionBar {
    pub fn render_repl_menu(&self, _cx: &mut gpui::Context<Self>) -> Option<AnyElement> {
        None
    }
}
