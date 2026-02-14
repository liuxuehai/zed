use anyhow::Result;
use gpui::{
    div, App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable,
    IntoElement, ParentElement, Pixels, Render, Styled, Window, px, Subscription,
};
use ui::{prelude::*, v_flex, h_flex, Label, IconButton, IconName};
use util::ResultExt;
use workspace::{
    dock::{DockPosition, Panel, PanelEvent},
    Workspace,
};

use crate::{GlobalTradingManager, TradingEvent, MarketData};

/// Watchlist panel - displays a list of stocks being monitored
pub struct WatchlistPanel {
    focus_handle: FocusHandle,
    width: Option<f32>,
    watchlist: Vec<String>,
    selected_index: Option<usize>,
    market_data: std::collections::HashMap<String, MarketData>,
    _subscriptions: Vec<Subscription>,
}

impl WatchlistPanel {
    pub fn new(cx: &mut Context<Workspace>) -> Entity<Self> {
        cx.new(|cx| {
            let mut panel = Self {
                focus_handle: cx.focus_handle(),
                width: Some(250.0),
                watchlist: vec![
                    "AAPL".to_string(),
                    "GOOGL".to_string(),
                    "MSFT".to_string(),
                    "TSLA".to_string(),
                ],
                selected_index: None,
                market_data: std::collections::HashMap::new(),
                _subscriptions: Vec::new(),
            };
            
            // Subscribe to trading manager events if available
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
            TradingEvent::MarketDataUpdated(data) => {
                self.market_data.insert(data.symbol.clone(), data.clone());
                cx.notify();
            }
            TradingEvent::SymbolSelected(symbol) => {
                if let Some(index) = self.watchlist.iter().position(|s| s == symbol) {
                    self.selected_index = Some(index);
                    cx.notify();
                }
            }
            _ => {}
        }
    }
    
    fn add_stock(&mut self, symbol: String, cx: &mut Context<Self>) {
        if !self.watchlist.contains(&symbol) {
            self.watchlist.push(symbol.clone());
            
            // Subscribe to this symbol via trading manager
            if let Some(global_manager) = cx.try_global::<GlobalTradingManager>() {
                let manager = global_manager.0.clone();
                manager.update(cx, |manager, cx| {
                    manager.subscribe_to_symbol(symbol, cx).log_err();
                });
            }
            
            cx.notify();
        }
    }
    
    fn remove_stock(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.watchlist.len() {
            let symbol = self.watchlist.remove(index);
            self.market_data.remove(&symbol);
            
            if self.selected_index == Some(index) {
                self.selected_index = None;
            } else if let Some(selected) = self.selected_index
                && selected > index
            {
                self.selected_index = Some(selected - 1);
            }
            
            cx.notify();
        }
    }
    
    fn select_stock(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.watchlist.len() {
            self.selected_index = Some(index);
            let symbol = self.watchlist[index].clone();
            
            // Notify trading manager of selection
            if let Some(global_manager) = cx.try_global::<GlobalTradingManager>() {
                let manager = global_manager.0.clone();
                manager.update(cx, |manager, cx| {
                    manager.set_active_symbol(symbol, cx).log_err();
                });
            }
            
            cx.notify();
        }
    }
    
    fn format_price(price: f64) -> String {
        format!("${:.2}", price)
    }
    
    fn format_change(change: f64, change_percent: f64) -> (String, Color) {
        let color = if change >= 0.0 {
            Color::Success
        } else {
            Color::Error
        };
        
        let text = format!("{:+.2} ({:+.2}%)", change, change_percent);
        (text, color)
    }
}

impl EventEmitter<PanelEvent> for WatchlistPanel {}

impl Focusable for WatchlistPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for WatchlistPanel {
    fn persistent_name() -> &'static str {
        "Watchlist"
    }

    fn panel_key() -> &'static str {
        "Watchlist"
    }

    fn position(&self, _window: &Window, _cx: &App) -> DockPosition {
        DockPosition::Left
    }

    fn position_is_valid(&self, _position: DockPosition) -> bool {
        true
    }

    fn set_position(&mut self, _position: DockPosition, _window: &mut Window, _cx: &mut Context<Self>) {
        // Position can be changed
    }

    fn size(&self, _window: &Window, _cx: &App) -> Pixels {
        px(self.width.unwrap_or(250.0))
    }

    fn set_size(&mut self, size: Option<Pixels>, _window: &mut Window, _cx: &mut Context<Self>) {
        self.width = size.map(f32::from);
    }

    fn icon(&self, _window: &Window, _cx: &App) -> Option<ui::IconName> {
        Some(ui::IconName::ListTree)
    }

    fn icon_tooltip(&self, _window: &Window, _cx: &App) -> Option<&'static str> {
        Some("Watchlist")
    }

    fn toggle_action(&self) -> Box<dyn gpui::Action> {
        Box::new(crate::ToggleWatchlistPanel)
    }

    fn activation_priority(&self) -> u32 {
        1
    }
}

impl Render for WatchlistPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected_index = self.selected_index;
        
        v_flex()
            .size_full()
            .child(
                // Header
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .p_2()
                    .border_b_1()
                    .border_color(cx.theme().colors().border)
                    .child(
                        Label::new("Watchlist")
                            .size(LabelSize::Large)
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                IconButton::new("add_stock", IconName::Plus)
                                    .icon_size(IconSize::Small)
                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                        // TODO: Show input dialog
                                        this.add_stock("NVDA".to_string(), cx);
                                    }))
                            )
                            .child(
                                IconButton::new("refresh", IconName::ArrowCircle)
                                    .icon_size(IconSize::Small)
                                    .on_click(cx.listener(|_this, _event, _window, _cx| {
                                        // TODO: Refresh market data
                                    }))
                            )
                    )
            )
            .child(
                // Stock list
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .children(
                        self.watchlist.iter().enumerate().map(|(index, symbol)| {
                            let is_selected = selected_index == Some(index);
                            let market_data = self.market_data.get(symbol);
                            
                            let (change_text, change_color) = if let Some(data) = market_data {
                                Self::format_change(data.change, data.change_percent)
                            } else {
                                ("--".to_string(), Color::Muted)
                            };
                            
                            div()
                                .id(("watchlist_item", index))
                                .flex()
                                .flex_col()
                                .p_2()
                                .gap_1()
                                .border_b_1()
                                .border_color(cx.theme().colors().border)
                                .when(is_selected, |this| {
                                    this.bg(cx.theme().colors().element_selected)
                                })
                                .hover(|this| {
                                    this.bg(cx.theme().colors().element_hover)
                                })
                                .cursor_pointer()
                                .on_click(cx.listener(move |this, _event, _window, cx| {
                                    this.select_stock(index, cx);
                                }))
                                .child(
                                    h_flex()
                                        .justify_between()
                                        .items_center()
                                        .child(
                                            Label::new(symbol.clone())
                                                .size(LabelSize::Default)
                                        )
                                        .child(
                                            IconButton::new(("remove", index), IconName::Close)
                                                .icon_size(IconSize::Small)
                                                .on_click(cx.listener(move |this, _event, _window, cx| {
                                                    this.remove_stock(index, cx);
                                                }))
                                        )
                                )
                                .child(
                                    h_flex()
                                        .justify_between()
                                        .child(
                                            Label::new(
                                                market_data
                                                    .map(|d| Self::format_price(d.current_price))
                                                    .unwrap_or_else(|| "--".to_string())
                                            )
                                            .size(LabelSize::Default)
                                        )
                                        .child(
                                            Label::new(change_text)
                                                .size(LabelSize::Small)
                                                .color(change_color)
                                        )
                                )
                        })
                    )
            )
            .child(
                // Footer with stats
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .p_2()
                    .border_t_1()
                    .border_color(cx.theme().colors().border)
                    .child(
                        Label::new(format!("{} stocks", self.watchlist.len()))
                            .size(LabelSize::Small)
                            .color(Color::Muted)
                    )
            )
    }
}
