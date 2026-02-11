use anyhow::Result;
use gpui::{
    div, App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable,
    IntoElement, ParentElement, Pixels, Render, Styled, Window, px,
};
use ui::{prelude::*, v_flex, Button, Label};
use util::ResultExt;
use workspace::{
    dock::{DockPosition, Panel, PanelEvent},
    Workspace,
};

use crate::GlobalTradingManager;

/// Simple demo panel to show stock trading system is running
/// Uses Zed's built-in UI components (no gpui-component dependency)
pub struct StockTradingDemoPanel {
    focus_handle: FocusHandle,
    width: Option<f32>,
}

impl StockTradingDemoPanel {
    pub fn new(cx: &mut Context<Workspace>) -> Entity<Self> {
        cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
            width: Some(300.0),
        })
    }

    pub fn load(
        _workspace: &mut Workspace,
        _window: &mut Window,
        cx: &mut Context<Workspace>,
    ) -> Result<Entity<Self>> {
        let panel = Self::new(cx);
        
        // Try to get trading manager if available
        if let Some(global_manager) = cx.try_global::<GlobalTradingManager>() {
            let _manager = global_manager.0.clone();
            // Future: subscribe to trading manager events
        }
        
        Ok(panel)
    }

    pub fn register(workspace: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>) {
        let panel = Self::load(workspace, window, cx).log_err();
        if let Some(panel) = panel {
            workspace.add_panel(panel, window, cx);
        }
    }
}

impl EventEmitter<PanelEvent> for StockTradingDemoPanel {}

impl Focusable for StockTradingDemoPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for StockTradingDemoPanel {
    fn persistent_name() -> &'static str {
        "StockTradingDemo"
    }

    fn panel_key() -> &'static str {
        "StockTradingDemo"
    }

    fn position(&self, _window: &Window, _cx: &App) -> DockPosition {
        DockPosition::Right
    }

    fn position_is_valid(&self, _position: DockPosition) -> bool {
        true
    }

    fn set_position(&mut self, _position: DockPosition, _window: &mut Window, _cx: &mut Context<Self>) {
        // Position can be changed
    }

    fn size(&self, _window: &Window, _cx: &App) -> Pixels {
        px(self.width.unwrap_or(300.0))
    }

    fn set_size(&mut self, size: Option<Pixels>, _window: &mut Window, _cx: &mut Context<Self>) {
        self.width = size.map(|p| f32::from(p));
    }

    fn icon(&self, _window: &Window, _cx: &App) -> Option<ui::IconName> {
        Some(ui::IconName::FileCode)
    }

    fn icon_tooltip(&self, _window: &Window, _cx: &App) -> Option<&'static str> {
        Some("Stock Trading")
    }

    fn toggle_action(&self) -> Box<dyn gpui::Action> {
        Box::new(crate::ToggleStockTradingDemoPanel)
    }

    fn activation_priority(&self) -> u32 {
        1
    }
}

impl Render for StockTradingDemoPanel {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .p_4()
            .gap_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        Label::new("Stock Trading System")
                            .size(LabelSize::Large)
                    )
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Label::new("Status: Running"))
                    .child(Label::new("Version: 0.1.0"))
                    .child(Label::new("Mode: Demo"))
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .mt_4()
                    .child(Label::new("Available Features:"))
                    .child(Label::new("• Market Data Service"))
                    .child(Label::new("• WebSocket Connection"))
                    .child(Label::new("• Mock Data Generation"))
                    .child(Label::new("• Panel Management"))
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .mt_4()
                    .child(
                        Button::new("refresh", "Refresh")
                            .on_click(|_event, _window, _cx| {
                                // Future: refresh data
                            })
                    )
            )
    }
}
