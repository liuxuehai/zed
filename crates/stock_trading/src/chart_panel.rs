use anyhow::Result;
use gpui::{
    div, App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable,
    IntoElement, ParentElement, Pixels, Render, Styled, Window, px, Subscription,
};
use ui::{prelude::*, v_flex, h_flex, Button, Label, IconButton, IconName};
use util::ResultExt;
use workspace::{
    dock::{DockPosition, Panel, PanelEvent},
    Workspace,
};

use crate::{GlobalTradingManager, TradingEvent, MarketData, TimeFrame};

/// Chart panel - displays price chart and technical indicators
pub struct ChartPanel {
    focus_handle: FocusHandle,
    height: Option<f32>,
    active_symbol: Option<String>,
    market_data: Option<MarketData>,
    timeframe: TimeFrame,
    _subscriptions: Vec<Subscription>,
}

impl ChartPanel {
    pub fn new(cx: &mut Context<Workspace>) -> Entity<Self> {
        cx.new(|cx| {
            let mut panel = Self {
                focus_handle: cx.focus_handle(),
                height: Some(400.0),
                active_symbol: None,
                market_data: None,
                timeframe: TimeFrame::OneDay,
                _subscriptions: Vec::new(),
            };
            
            // Subscribe to trading manager events
            if let Some(global_manager) = cx.try_global::<GlobalTradingManager>() {
                let manager = global_manager.0.clone();
                let subscription = cx.subscribe(&manager, Self::handle_trading_event);
                panel._subscriptions.push(subscription);
            }
            
            panel
        })
    }

    pub fn load(
        _workspace: &mut Workspace,
        _window: &mut Window,
        cx: &mut Context<Workspace>,
    ) -> Result<Entity<Self>> {
        let panel = Self::new(cx);
        Ok(panel)
    }

    pub fn register(workspace: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>) {
        let panel = Self::load(workspace, window, cx).log_err();
        if let Some(panel) = panel {
            workspace.add_panel(panel, window, cx);
        }
    }
    
    fn handle_trading_event(
        &mut self,
        _manager: Entity<crate::TradingManager>,
        event: &TradingEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            TradingEvent::SymbolSelected(symbol) => {
                self.active_symbol = Some(symbol.clone());
                self.market_data = None; // Clear old data
                cx.notify();
            }
            TradingEvent::MarketDataUpdated(data) => {
                if let Some(active) = &self.active_symbol
                    && &data.symbol == active
                {
                    self.market_data = Some(data.clone());
                    cx.notify();
                }
            }
            _ => {}
        }
    }
    
    fn cycle_timeframe(&mut self, cx: &mut Context<Self>) {
        self.timeframe = match self.timeframe {
            TimeFrame::OneMinute => TimeFrame::FiveMinutes,
            TimeFrame::FiveMinutes => TimeFrame::FifteenMinutes,
            TimeFrame::FifteenMinutes => TimeFrame::ThirtyMinutes,
            TimeFrame::ThirtyMinutes => TimeFrame::OneHour,
            TimeFrame::OneHour => TimeFrame::FourHours,
            TimeFrame::FourHours => TimeFrame::OneDay,
            TimeFrame::OneDay => TimeFrame::OneWeek,
            TimeFrame::OneWeek => TimeFrame::OneMonth,
            TimeFrame::OneMonth => TimeFrame::OneMinute,
        };
        cx.notify();
    }
    
    fn timeframe_label(&self) -> &'static str {
        match self.timeframe {
            TimeFrame::OneMinute => "1m",
            TimeFrame::FiveMinutes => "5m",
            TimeFrame::FifteenMinutes => "15m",
            TimeFrame::ThirtyMinutes => "30m",
            TimeFrame::OneHour => "1h",
            TimeFrame::FourHours => "4h",
            TimeFrame::OneDay => "1d",
            TimeFrame::OneWeek => "1w",
            TimeFrame::OneMonth => "1M",
        }
    }
}

impl EventEmitter<PanelEvent> for ChartPanel {}

impl Focusable for ChartPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for ChartPanel {
    fn persistent_name() -> &'static str {
        "Chart"
    }

    fn panel_key() -> &'static str {
        "Chart"
    }

    fn position(&self, _window: &Window, _cx: &App) -> DockPosition {
        DockPosition::Bottom
    }

    fn position_is_valid(&self, _position: DockPosition) -> bool {
        true
    }

    fn set_position(&mut self, _position: DockPosition, _window: &mut Window, _cx: &mut Context<Self>) {
        // Position can be changed
    }

    fn size(&self, _window: &Window, _cx: &App) -> Pixels {
        px(self.height.unwrap_or(400.0))
    }

    fn set_size(&mut self, size: Option<Pixels>, _window: &mut Window, _cx: &mut Context<Self>) {
        self.height = size.map(f32::from);
    }

    fn icon(&self, _window: &Window, _cx: &App) -> Option<ui::IconName> {
        Some(ui::IconName::ChevronUp) // Using available icon
    }

    fn icon_tooltip(&self, _window: &Window, _cx: &App) -> Option<&'static str> {
        Some("Chart")
    }

    fn toggle_action(&self) -> Box<dyn gpui::Action> {
        Box::new(crate::ToggleChartPanel)
    }

    fn activation_priority(&self) -> u32 {
        2
    }
}

impl Render for ChartPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let symbol = self.active_symbol.clone().unwrap_or_else(|| "No symbol selected".to_string());
        let has_data = self.market_data.is_some();
        
        v_flex()
            .size_full()
            .child(
                // Header with controls
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .p_2()
                    .border_b_1()
                    .border_color(cx.theme().colors().border)
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                Label::new(symbol.clone())
                                    .size(LabelSize::Large)
                            )
                            .when(has_data, |this| {
                                this.child(
                                    Label::new(
                                        self.market_data.as_ref()
                                            .map(|d| format!("${:.2}", d.current_price))
                                            .unwrap_or_default()
                                    )
                                    .size(LabelSize::Large)
                                )
                            })
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                Button::new("timeframe", self.timeframe_label())
                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                        this.cycle_timeframe(cx);
                                    }))
                            )
                            .child(
                                IconButton::new("zoom_in", IconName::Plus)
                                    .icon_size(IconSize::Small)
                            )
                            .child(
                                IconButton::new("zoom_out", IconName::Dash)
                                    .icon_size(IconSize::Small)
                            )
                    )
            )
            .child(
                // Chart area (simplified - just showing price info)
                div()
                    .flex()
                    .flex_1()
                    .items_center()
                    .justify_center()
                    .bg(cx.theme().colors().editor_background)
                    .child(
                        if let Some(data) = &self.market_data {
                            v_flex()
                                .gap_4()
                                .items_center()
                                .child(
                                    Label::new(format!("${:.2}", data.current_price))
                                        .size(LabelSize::Large)
                                )
                                .child(
                                    h_flex()
                                        .gap_4()
                                        .child(
                                            v_flex()
                                                .gap_1()
                                                .child(
                                                    Label::new("Change")
                                                        .size(LabelSize::Small)
                                                        .color(Color::Muted)
                                                )
                                                .child(
                                                    Label::new(format!("{:+.2} ({:+.2}%)", data.change, data.change_percent))
                                                        .color(if data.change >= 0.0 { Color::Success } else { Color::Error })
                                                )
                                        )
                                        .child(
                                            v_flex()
                                                .gap_1()
                                                .child(
                                                    Label::new("Volume")
                                                        .size(LabelSize::Small)
                                                        .color(Color::Muted)
                                                )
                                                .child(
                                                    Label::new(format!("{:.2}M", data.volume as f64 / 1_000_000.0))
                                                )
                                        )
                                )
                                .child(
                                    h_flex()
                                        .gap_4()
                                        .child(
                                            v_flex()
                                                .gap_1()
                                                .child(
                                                    Label::new("Day High")
                                                        .size(LabelSize::Small)
                                                        .color(Color::Muted)
                                                )
                                                .child(
                                                    Label::new(format!("${:.2}", data.day_high))
                                                )
                                        )
                                        .child(
                                            v_flex()
                                                .gap_1()
                                                .child(
                                                    Label::new("Day Low")
                                                        .size(LabelSize::Small)
                                                        .color(Color::Muted)
                                                )
                                                .child(
                                                    Label::new(format!("${:.2}", data.day_low))
                                                )
                                        )
                                        .child(
                                            v_flex()
                                                .gap_1()
                                                .child(
                                                    Label::new("Prev Close")
                                                        .size(LabelSize::Small)
                                                        .color(Color::Muted)
                                                )
                                                .child(
                                                    Label::new(format!("${:.2}", data.previous_close))
                                                )
                                        )
                                )
                                .child(
                                    Label::new("📊 Full chart visualization coming soon")
                                        .size(LabelSize::Small)
                                        .color(Color::Muted)
                                )
                        } else {
                            v_flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    Label::new("No data available")
                                        .size(LabelSize::Large)
                                        .color(Color::Muted)
                                )
                                .child(
                                    Label::new("Select a stock from the watchlist")
                                        .size(LabelSize::Small)
                                        .color(Color::Muted)
                                )
                        }
                    )
            )
    }
}
