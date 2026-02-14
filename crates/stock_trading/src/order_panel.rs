use anyhow::Result;
use gpui::{
    div, App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable,
    IntoElement, ParentElement, Pixels, Render, Styled, Window, px, Subscription,
};
use ui::{prelude::*, v_flex, h_flex, Button, Label};
use util::ResultExt;
use workspace::{
    dock::{DockPosition, Panel, PanelEvent},
    Workspace,
};

use crate::{GlobalTradingManager, TradingEvent, OrderSide, OrderType};

/// Order panel - for placing buy/sell orders
pub struct OrderPanel {
    focus_handle: FocusHandle,
    width: Option<f32>,
    active_symbol: Option<String>,
    current_price: Option<f64>,
    order_side: OrderSide,
    order_type: OrderType,
    quantity: String,
    limit_price: String,
    _subscriptions: Vec<Subscription>,
}

impl OrderPanel {
    pub fn new(cx: &mut Context<Workspace>) -> Entity<Self> {
        cx.new(|cx| {
            let mut panel = Self {
                focus_handle: cx.focus_handle(),
                width: Some(300.0),
                active_symbol: None,
                current_price: None,
                order_side: OrderSide::Buy,
                order_type: OrderType::Market,
                quantity: "100".to_string(),
                limit_price: String::new(),
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
                cx.notify();
            }
            TradingEvent::MarketDataUpdated(data) => {
                if let Some(active) = &self.active_symbol
                    && &data.symbol == active
                {
                    self.current_price = Some(data.current_price);
                    if self.limit_price.is_empty() {
                        self.limit_price = format!("{:.2}", data.current_price);
                    }
                    cx.notify();
                }
            }
            _ => {}
        }
    }
    
    #[allow(dead_code)]
    fn toggle_order_side(&mut self, cx: &mut Context<Self>) {
        self.order_side = match self.order_side {
            OrderSide::Buy => OrderSide::Sell,
            OrderSide::Sell => OrderSide::Buy,
            OrderSide::SellShort => OrderSide::Buy,
        };
        cx.notify();
    }
    
    #[allow(dead_code)]
    fn toggle_order_type(&mut self, cx: &mut Context<Self>) {
        self.order_type = match self.order_type {
            OrderType::Market => OrderType::Limit,
            OrderType::Limit => OrderType::Market,
            _ => OrderType::Market,
        };
        cx.notify();
    }
    
    fn calculate_total(&self) -> Option<f64> {
        let quantity: u64 = self.quantity.parse().ok()?;
        let price = match self.order_type {
            OrderType::Market => self.current_price?,
            OrderType::Limit => self.limit_price.parse().ok()?,
            OrderType::StopLoss | OrderType::StopLimit | OrderType::TrailingStop | OrderType::TrailingStopLimit => return None,
        };
        Some(quantity as f64 * price)
    }
    
    fn submit_order(&mut self, cx: &mut Context<Self>) {
        // TODO: Implement actual order submission
        let side_str = match self.order_side {
            OrderSide::Buy => "Buy",
            OrderSide::Sell => "Sell",
            OrderSide::SellShort => "SellShort",
        };
        
        let type_str = match self.order_type {
            OrderType::Market => "Market".to_string(),
            OrderType::Limit => format!("${}", self.limit_price),
            OrderType::StopLoss => "Stop Loss".to_string(),
            OrderType::StopLimit => "Stop Limit".to_string(),
            OrderType::TrailingStop => "Trailing Stop".to_string(),
            OrderType::TrailingStopLimit => "Trailing Stop Limit".to_string(),
        };
        
        log::info!(
            "Order submitted: {} {} {} @ {}",
            side_str,
            self.quantity,
            self.active_symbol.as_ref().unwrap_or(&"N/A".to_string()),
            type_str
        );
        cx.notify();
    }
    
    fn clear_form(&mut self, cx: &mut Context<Self>) {
        self.quantity = "100".to_string();
        self.limit_price = String::new();
        cx.notify();
    }
}

impl EventEmitter<PanelEvent> for OrderPanel {}

impl Focusable for OrderPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for OrderPanel {
    fn persistent_name() -> &'static str {
        "Order"
    }

    fn panel_key() -> &'static str {
        "Order"
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
        self.width = size.map(f32::from);
    }

    fn icon(&self, _window: &Window, _cx: &App) -> Option<ui::IconName> {
        Some(ui::IconName::FileCode) // TODO: Use better icon when available
    }

    fn icon_tooltip(&self, _window: &Window, _cx: &App) -> Option<&'static str> {
        Some("Order Entry")
    }

    fn toggle_action(&self) -> Box<dyn gpui::Action> {
        Box::new(crate::ToggleOrderPanel)
    }

    fn activation_priority(&self) -> u32 {
        3
    }
}

impl Render for OrderPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let symbol = self.active_symbol.clone().unwrap_or_else(|| "Select a stock".to_string());
        let has_symbol = self.active_symbol.is_some();
        let total = self.calculate_total();
        
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
                        Label::new("Order Entry")
                            .size(LabelSize::Large)
                    )
            )
            .child(
                // Order form
                v_flex()
                    .flex_1()
                    .p_4()
                    .gap_4()
                    .child(
                        // Symbol display
                        v_flex()
                            .gap_1()
                            .child(
                                Label::new("Symbol")
                                    .size(LabelSize::Small)
                                    .color(Color::Muted)
                            )
                            .child(
                                Label::new(&symbol)
                                    .size(LabelSize::Large)
                            )
                            .when(self.current_price.is_some(), |this| {
                                this.child(
                                    Label::new(format!("Current: ${:.2}", self.current_price.unwrap()))
                                        .size(LabelSize::Small)
                                        .color(Color::Muted)
                                )
                            })
                    )
                    .child(
                        // Order side toggle
                        v_flex()
                            .gap_1()
                            .child(
                                Label::new("Side")
                                    .size(LabelSize::Small)
                                    .color(Color::Muted)
                            )
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(
                                        Button::new("buy", "Buy")
                                            .style(if matches!(self.order_side, OrderSide::Buy) {
                                                ButtonStyle::Filled
                                            } else {
                                                ButtonStyle::Subtle
                                            })
                                            .on_click(cx.listener(|this, _event, _window, cx| {
                                                this.order_side = OrderSide::Buy;
                                                cx.notify();
                                            }))
                                    )
                                    .child(
                                        Button::new("sell", "Sell")
                                            .style(if matches!(self.order_side, OrderSide::Sell) {
                                                ButtonStyle::Filled
                                            } else {
                                                ButtonStyle::Subtle
                                            })
                                            .on_click(cx.listener(|this, _event, _window, cx| {
                                                this.order_side = OrderSide::Sell;
                                                cx.notify();
                                            }))
                                    )
                            )
                    )
                    .child(
                        // Order type toggle
                        v_flex()
                            .gap_1()
                            .child(
                                Label::new("Type")
                                    .size(LabelSize::Small)
                                    .color(Color::Muted)
                            )
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(
                                        Button::new("market", "Market")
                                            .style(if matches!(self.order_type, OrderType::Market) {
                                                ButtonStyle::Filled
                                            } else {
                                                ButtonStyle::Subtle
                                            })
                                            .on_click(cx.listener(|this, _event, _window, cx| {
                                                this.order_type = OrderType::Market;
                                                cx.notify();
                                            }))
                                    )
                                    .child(
                                        Button::new("limit", "Limit")
                                            .style(if matches!(self.order_type, OrderType::Limit) {
                                                ButtonStyle::Filled
                                            } else {
                                                ButtonStyle::Subtle
                                            })
                                            .on_click(cx.listener(|this, _event, _window, cx| {
                                                this.order_type = OrderType::Limit;
                                                cx.notify();
                                            }))
                                    )
                            )
                    )
                    .child(
                        // Quantity input (simplified - just display)
                        v_flex()
                            .gap_1()
                            .child(
                                Label::new("Quantity")
                                    .size(LabelSize::Small)
                                    .color(Color::Muted)
                            )
                            .child(
                                div()
                                    .p_2()
                                    .border_1()
                                    .border_color(cx.theme().colors().border)
                                    .rounded_md()
                                    .child(
                                        Label::new(self.quantity.clone())
                                    )
                            )
                    )
                    .when(matches!(self.order_type, OrderType::Limit), |this| {
                        this.child(
                            // Limit price input (simplified - just display)
                            v_flex()
                                .gap_1()
                                .child(
                                    Label::new("Limit Price")
                                        .size(LabelSize::Small)
                                        .color(Color::Muted)
                                )
                                .child(
                                    div()
                                        .p_2()
                                        .border_1()
                                        .border_color(cx.theme().colors().border)
                                        .rounded_md()
                                        .child(
                                            Label::new(
                                                if self.limit_price.is_empty() {
                                                    "--".to_string()
                                                } else {
                                                    format!("${}", self.limit_price)
                                                }
                                            )
                                        )
                                )
                        )
                    })
                    .child(
                        // Total calculation
                        v_flex()
                            .gap_1()
                            .child(
                                Label::new("Estimated Total")
                                    .size(LabelSize::Small)
                                    .color(Color::Muted)
                            )
                            .child(
                                Label::new(
                                    total.map(|t| format!("${:.2}", t))
                                        .unwrap_or_else(|| "--".to_string())
                                )
                                .size(LabelSize::Large)
                            )
                    )
            )
            .child(
                // Action buttons
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_4()
                    .border_t_1()
                    .border_color(cx.theme().colors().border)
                    .child(
                        Button::new("submit", format!("{} {}", 
                            match self.order_side {
                                OrderSide::Buy => "Buy",
                                OrderSide::Sell => "Sell",
                                OrderSide::SellShort => "Sell Short",
                            },
                            &symbol
                        ))
                        .full_width()
                        .disabled(!has_symbol)
                        .on_click(cx.listener(|this, _event, _window, cx| {
                            this.submit_order(cx);
                        }))
                    )
                    .child(
                        Button::new("clear", "Clear")
                            .full_width()
                            .on_click(cx.listener(|this, _event, _window, cx| {
                                this.clear_form(cx);
                            }))
                    )
            )
    }
}
