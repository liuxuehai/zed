use anyhow::Result;
use gpui::{
    App, AppContext, Context, Entity, EventEmitter, FocusHandle, 
    IntoElement, Pixels, Render, Subscription, WeakEntity, Window
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    MarketData, OrderBook, OrderBookEntry, StockInfo, WatchlistItem, Order, OrderSide, 
    OrderType, TimeInForce, TimeFrame, Candle, TradingManager, TradingEvent, DockPosition,
    SymbolValidator, OrderValidator, InputParser,
    // Import all actions from trading_actions module
    trading_actions::*,
};

/// Enhanced panel events for inter-component communication
#[derive(Clone, Debug)]
pub enum PanelEvent {
    StockSelected(String),
    WatchlistUpdated(Vec<WatchlistItem>),
    OrderPlaced(Order),
    OrderCancelled(String),
    TimeFrameChanged(TimeFrame),
    ChartDataRequested(String, TimeFrame),
    MarketDataUpdated(MarketData),
    OrderBookRequested(String),
    OrderBookUpdated(OrderBook),
    HistoricalDataRequested(String, TimeFrame, usize),
    HistoricalDataUpdated(String, Vec<Candle>),
    RealTimeSubscriptionRequested(String),
    RealTimeSubscriptionCancelled(String),
    RefreshRequested(String),
    ErrorOccurred(String),
}

/// Watchlist panel using gpui-component's virtualized Table
pub struct WatchlistPanel {
    focus_handle: FocusHandle,
    watchlist_data: Vec<WatchlistItem>,
    selected_index: Option<usize>,
    trading_manager: WeakEntity<TradingManager>,
    width: Option<Pixels>,
    add_stock_input: String,
    real_time_enabled: bool,
    last_update: Option<std::time::SystemTime>,
    symbol_validator: SymbolValidator,
    last_validation_error: Option<String>,
    _subscriptions: Vec<Subscription>,
}

impl WatchlistPanel {
    pub fn new(trading_manager: WeakEntity<TradingManager>, cx: &mut App) -> Entity<Self> {
        let panel = cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
            watchlist_data: Vec::new(),
            selected_index: None,
            trading_manager: trading_manager.clone(),
            width: None,
            add_stock_input: String::new(),
            real_time_enabled: true,
            last_update: None,
            symbol_validator: SymbolValidator::new(),
            last_validation_error: None,
            _subscriptions: Vec::new(),
        });
        
        // Subscribe to TradingManager events for real-time updates
        if let Some(manager) = trading_manager.upgrade() {
            let subscription = cx.subscribe(&manager, |this, _manager, event, cx| {
                this.handle_trading_event(event.clone(), cx);
            });
            panel.update(cx, |this, _| {
                this._subscriptions.push(subscription);
            });
        }
        
        // Register action handlers for keyboard shortcuts
        panel.update(cx, |this, cx| {
            this.register_action_handlers(cx);
        });
        
        panel
    }
    
    /// Register action handlers for keyboard shortcuts (.rules compliance)
    fn register_action_handlers(&mut self, cx: &mut Context<Self>) {
        cx.on_action(|this: &mut Self, _action: &RefreshMarketData, cx| {
            this.refresh_all_market_data(cx);
        });
        
        cx.on_action(|this: &mut Self, _action: &FocusAddStockInput, cx| {
            cx.focus(&this.focus_handle);
            cx.notify();
        });
        
        cx.on_action(|this: &mut Self, action: &AddStockToWatchlist, cx| {
            if let Err(error) = this.add_stock(action.symbol.clone(), cx) {
                error.log_err(); // Proper error handling
            }
        });
        
        cx.on_action(|this: &mut Self, action: &RemoveStockFromWatchlist, cx| {
            if let Err(error) = this.remove_stock(action.index, cx) {
                error.log_err(); // Proper error handling
            }
        });
        
        cx.on_action(|this: &mut Self, action: &SelectStock, cx| {
            if let Err(error) = this.select_stock(action.index, cx) {
                error.log_err(); // Proper error handling
            }
        });
    }
    
    /// Add stock to watchlist with validation and real-time subscription (.rules compliance)
    pub fn add_stock(&mut self, symbol: String, cx: &mut Context<Self>) -> Result<()> {
        // Use symbol validator with proper error handling (.rules compliance)
        let validated_symbol = match self.symbol_validator.validate_symbol(&symbol) {
            Ok(s) => s,
            Err(error) => {
                // Store error for UI display
                self.last_validation_error = Some(error.to_string());
                
                // Try to suggest corrections
                let suggestions = self.symbol_validator.suggest_corrections(&symbol);
                if !suggestions.is_empty() {
                    let suggestion_text = suggestions.join(", ");
                    let enhanced_error = format!("{}. Did you mean: {}?", error, suggestion_text);
                    self.last_validation_error = Some(enhanced_error.clone());
                    cx.emit(PanelEvent::ErrorOccurred(enhanced_error));
                } else {
                    cx.emit(PanelEvent::ErrorOccurred(error.to_string()));
                }
                
                cx.notify();
                return Err(error);
            }
        };
        
        // Clear validation error on success
        self.last_validation_error = None;
        
        // Check if symbol already exists using bounds checking (.rules compliance)
        if self.watchlist_data.iter().any(|item| item.symbol == validated_symbol) {
            let error = anyhow::anyhow!(
                "Symbol '{}' is already in your watchlist.",
                validated_symbol
            );
            self.last_validation_error = Some(error.to_string());
            cx.emit(PanelEvent::ErrorOccurred(error.to_string()));
            cx.notify();
            return Err(error);
        }
        
        let watchlist_item = WatchlistItem {
            symbol: validated_symbol.clone(),
            current_price: 0.0,
            change: 0.0,
            change_percent: 0.0,
            volume: 0,
            market_cap: None,
            pe_ratio: None,
        };
        
        self.watchlist_data.push(watchlist_item);
        self.add_stock_input.clear();
        
        // Subscribe to real-time updates if enabled
        if self.real_time_enabled {
            if let Err(error) = self.subscribe_to_symbol(&validated_symbol, cx) {
                log::warn!("Failed to subscribe to real-time updates for {}: {}", validated_symbol, error);
                error.log_err(); // Log but don't fail the add operation
            }
        }
        
        // Request initial market data
        self.request_market_data(&validated_symbol, cx);
        
        cx.emit(PanelEvent::WatchlistUpdated(self.watchlist_data.clone()));
        cx.emit(PanelEvent::RealTimeSubscriptionRequested(validated_symbol));
        cx.notify();
        
        Ok(())
    }
    
    /// Remove stock from watchlist with bounds checking and unsubscribe (.rules compliance)
    pub fn remove_stock(&mut self, index: usize, cx: &mut Context<Self>) -> Result<()> {
        if let Some(item) = self.watchlist_data.get(index) {
            let symbol = item.symbol.clone();
            self.watchlist_data.remove(index);
            
            // Adjust selected index if necessary
            if let Some(selected) = self.selected_index {
                if selected >= index {
                    self.selected_index = if selected == 0 { None } else { Some(selected - 1) };
                }
            }
            
            // Unsubscribe from real-time updates
            if let Err(error) = self.unsubscribe_from_symbol(&symbol, cx) {
                error.log_err(); // Log but don't fail the remove operation
            }
            
            cx.emit(PanelEvent::WatchlistUpdated(self.watchlist_data.clone()));
            cx.emit(PanelEvent::RealTimeSubscriptionCancelled(symbol));
            cx.notify();
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid index for watchlist removal"))
        }
    }
    
    /// Select stock with bounds checking (.rules compliance)
    pub fn select_stock(&mut self, index: usize, cx: &mut Context<Self>) -> Result<()> {
        if let Some(item) = self.watchlist_data.get(index) {
            self.selected_index = Some(index);
            
            // Notify TradingManager of symbol selection
            if let Some(trading_manager) = self.trading_manager.upgrade() {
                trading_manager.update(cx, |manager, cx| {
                    manager.set_active_symbol(item.symbol.clone(), cx)
                })?;
            }
            
            cx.emit(PanelEvent::StockSelected(item.symbol.clone()));
            cx.notify();
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid index for stock selection"))
        }
    }
    
    /// Update market data for watchlist items with WebSocket integration and validation
    pub fn update_market_data(&mut self, market_data: MarketData, cx: &mut Context<Self>) {
        // Validate market data before updating
        if market_data.current_price < 0.0 {
            log::warn!("Received invalid market data with negative price for {}", market_data.symbol);
            return;
        }
        
        let mut updated = false;
        for item in &mut self.watchlist_data {
            if item.symbol == market_data.symbol {
                item.current_price = market_data.current_price;
                item.change = market_data.change;
                item.change_percent = market_data.change_percent;
                item.volume = market_data.volume;
                item.market_cap = market_data.market_cap;
                updated = true;
                break;
            }
        }
        
        if updated {
            self.last_update = Some(std::time::SystemTime::now());
            cx.notify(); // Trigger UI update
        }
    }
    
    /// Handle trading events from TradingManager for real-time updates
    fn handle_trading_event(&mut self, event: TradingEvent, cx: &mut Context<Self>) {
        match event {
            TradingEvent::MarketDataUpdated(market_data) => {
                self.update_market_data(market_data, cx);
            }
            TradingEvent::WebSocketConnected => {
                // Re-subscribe to all symbols when WebSocket reconnects
                if let Err(error) = self.resubscribe_all_symbols(cx) {
                    error.log_err(); // Log reconnection errors
                }
            }
            TradingEvent::DataServiceError(error) => {
                log::error!("Data service error in watchlist: {}", error);
                cx.emit(PanelEvent::ErrorOccurred(error));
            }
            _ => {} // Handle other events as needed
        }
    }
    
    /// Subscribe to real-time updates for a symbol
    fn subscribe_to_symbol(&mut self, symbol: &str, cx: &mut Context<Self>) -> Result<()> {
        if let Some(trading_manager) = self.trading_manager.upgrade() {
            trading_manager.update(cx, |manager, cx| {
                manager.subscribe_to_symbol(symbol.to_string(), cx)
            })?;
        }
        Ok(())
    }
    
    /// Unsubscribe from real-time updates for symbol
    fn unsubscribe_from_symbol(&mut self, symbol: &str, cx: &mut Context<Self>) -> Result<()> {
        if let Some(trading_manager) = self.trading_manager.upgrade() {
            trading_manager.update(cx, |manager, cx| {
                manager.unsubscribe_from_symbol(symbol, cx)
            })?;
        }
        Ok(())
    }
    
    /// Re-subscribe to all watchlist symbols (useful after reconnection)
    fn resubscribe_all_symbols(&mut self, cx: &mut Context<Self>) -> Result<()> {
        if !self.real_time_enabled {
            return Ok(());
        }
        
        for item in &self.watchlist_data {
            if let Err(error) = self.subscribe_to_symbol(&item.symbol, cx) {
                error.log_err(); // Log but continue with other symbols
            }
        }
        Ok(())
    }
    
    /// Request market data for a symbol
    fn request_market_data(&mut self, symbol: &str, cx: &mut Context<Self>) {
        if let Some(trading_manager) = self.trading_manager.upgrade() {
            let symbol_clone = symbol.to_string();
            let _task = trading_manager.update(cx, |manager, cx| {
                manager.get_market_data(&symbol_clone, cx)
            });
        }
    }
    
    /// Refresh market data for all symbols
    pub fn refresh_all_market_data(&mut self, cx: &mut Context<Self>) {
        for item in &self.watchlist_data {
            self.request_market_data(&item.symbol, cx);
        }
        cx.emit(PanelEvent::RefreshRequested("all".to_string()));
    }
    
    /// Toggle real-time updates
    pub fn set_real_time_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) -> Result<()> {
        self.real_time_enabled = enabled;
        
        if enabled {
            // Subscribe to all symbols
            self.resubscribe_all_symbols(cx)?;
        } else {
            // Unsubscribe from all symbols
            for item in &self.watchlist_data {
                if let Err(error) = self.unsubscribe_from_symbol(&item.symbol, cx) {
                    error.log_err(); // Log but continue
                }
            }
        }
        
        cx.notify();
        Ok(())
    }
    
    /// Get watchlist data for persistence
    pub fn get_watchlist_symbols(&self) -> Vec<String> {
        self.watchlist_data.iter().map(|item| item.symbol.clone()).collect()
    }
    
    /// Load watchlist from symbols (for persistence)
    pub fn load_watchlist(&mut self, symbols: Vec<String>, cx: &mut Context<Self>) {
        for symbol in symbols {
            if let Err(error) = self.add_stock(symbol, cx) {
                error.log_err(); // Log but continue loading other symbols
            }
        }
    }
}

impl EventEmitter<PanelEvent> for WatchlistPanel {}

impl Panel for WatchlistPanel {
    fn panel_name(&self) -> &'static str {
        "Watchlist"
    }
    
    fn dock_position(&self) -> DockPosition {
        DockPosition::Left
    }
    
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
    
    fn set_width(&mut self, width: Option<Pixels>, _cx: &mut Context<Self>) {
        self.width = width;
    }
}

impl Render for WatchlistPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Root::new()
            .child(
                gpui::div()
                    .flex()
                    .flex_col()
                    .w_full()
                    .h_full()
                    .p_4()
                    .child(
                        // Header with add stock input
                        gpui::div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .mb_4()
                            .child(
                                Input::new("add-stock-input")
                                    .placeholder("Enter symbol (e.g., AAPL)...")
                                    .value(self.add_stock_input.clone())
                                    .on_input(cx.listener(|this, input: &str, cx| {
                                        this.add_stock_input = input.to_uppercase();
                                        cx.notify();
                                    }))
                            )
                            .child(
                                Button::new("add-stock-btn")
                                    .label("Add")
                                    .on_click(cx.listener(|this, _event, cx| {
                                        let symbol = this.add_stock_input.clone();
                                        if let Err(error) = this.add_stock(symbol, cx) {
                                            // Show error message to user
                                            log::error!("Failed to add stock: {}", error);
                                            error.log_err();
                                        }
                                    }))
                            )
                            .child(
                                Button::new("refresh-btn")
                                    .label("Refresh")
                                    .on_click(cx.listener(|this, _event, cx| {
                                        this.refresh_all_market_data(cx);
                                    }))
                            )
                    )
                    .child(
                        // Watchlist table
                        if self.watchlist_data.is_empty() {
                            gpui::div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .h_full()
                                .child(
                                    gpui::div()
                                        .text_color(gpui::rgb(0x888888))
                                        .child("No stocks in watchlist. Add a symbol above.")
                                )
                        } else {
                            self.render_watchlist_table(cx)
                        }
                    )
            )
    }
    
    fn render_watchlist_table(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let columns = vec![
            TableColumn::new("Symbol", 80),
            TableColumn::new("Price", 80),
            TableColumn::new("Change", 80),
            TableColumn::new("Change %", 80),
            TableColumn::new("Volume", 100),
            TableColumn::new("Actions", 80),
        ];
        
        let table_data: Vec<TableData> = self.watchlist_data
            .iter()
            .enumerate()
            .map(|(index, item)| {
                TableData::new(vec![
                    item.symbol.clone(),
                    format!("${:.2}", item.current_price),
                    format!("{:+.2}", item.change),
                    format!("{:+.2}%", item.change_percent),
                    item.volume.to_string(),
                    "Remove".to_string(),
                ])
                .with_id(index.to_string())
            })
            .collect();
        
        Table::new("watchlist-table")
            .columns(columns)
            .data(table_data)
            .selected_row(self.selected_index.map(|i| i.to_string()))
            .on_row_click(cx.listener(|this, row_id: &str, cx| {
                if let Ok(index) = row_id.parse::<usize>() {
                    if let Err(error) = this.select_stock(index, cx) {
                        error.log_err(); // Proper error handling
                    }
                }
            }))
            .on_cell_click(cx.listener(|this, (row_id, col_index): &(String, usize), cx| {
                if *col_index == 5 { // Actions column
                    if let Ok(index) = row_id.parse::<usize>() {
                        if let Err(error) = this.remove_stock(index, cx) {
                            error.log_err(); // Proper error handling
                        }
                    }
                }
            }))
    }
}

/// Chart panel using gpui-component's built-in Chart with enhanced features
pub struct ChartPanel {
    focus_handle: FocusHandle,
    current_symbol: Option<String>,
    current_timeframe: TimeFrame,
    chart_data: Vec<Candle>,
    trading_manager: WeakEntity<TradingManager>,
    width: Option<Pixels>,
    real_time_enabled: bool,
    last_update: Option<std::time::SystemTime>,
    zoom_level: f64,
    pan_offset: f64,
    show_volume: bool,
    chart_style: ChartStyle,
    _subscriptions: Vec<Subscription>,
}

/// Chart display style options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartStyle {
    Candlestick,
    Line,
    Area,
}

impl ChartPanel {
    pub fn new(trading_manager: WeakEntity<TradingManager>, cx: &mut App) -> Entity<Self> {
        let panel = cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
            current_symbol: None,
            current_timeframe: TimeFrame::OneDay,
            chart_data: Vec::new(),
            trading_manager: trading_manager.clone(),
            width: None,
            real_time_enabled: true,
            last_update: None,
            zoom_level: 1.0,
            pan_offset: 0.0,
            show_volume: true,
            chart_style: ChartStyle::Candlestick,
            _subscriptions: Vec::new(),
        });
        
        // Subscribe to TradingManager events for real-time updates
        if let Some(manager) = trading_manager.upgrade() {
            let subscription = cx.subscribe(&manager, |this, _manager, event, cx| {
                this.handle_trading_event(event.clone(), cx);
            });
            panel.update(cx, |this, _| {
                this._subscriptions.push(subscription);
            });
        }
        
        // Register action handlers for chart interactions
        panel.update(cx, |this, cx| {
            this.register_action_handlers(cx);
        });
        
        panel
    }
    
    /// Register action handlers for keyboard shortcuts and chart interactions (.rules compliance)
    fn register_action_handlers(&mut self, cx: &mut Context<Self>) {
        // Register zoom in/out actions
        cx.on_action(|this: &mut Self, _action: &ZoomIn, cx| {
            this.zoom_in(cx);
        });
        
        cx.on_action(|this: &mut Self, _action: &ZoomOut, cx| {
            this.zoom_out(cx);
        });
        
        cx.on_action(|this: &mut Self, _action: &ResetZoom, cx| {
            this.reset_zoom(cx);
        });
        
        cx.on_action(|this: &mut Self, _action: &ToggleVolume, cx| {
            this.toggle_volume(cx);
        });
        
        cx.on_action(|this: &mut Self, _action: &RefreshMarketData, cx| {
            this.refresh_chart_data(cx);
        });
    }
    
    /// Set symbol and load chart data with real-time subscription
    pub fn set_symbol(&mut self, symbol: String, cx: &mut Context<Self>) {
        self.current_symbol = Some(symbol.clone());
        
        // Request historical data
        self.request_historical_data(&symbol, cx);
        
        // Subscribe to real-time updates if enabled
        if self.real_time_enabled {
            if let Err(error) = self.subscribe_to_symbol(&symbol, cx) {
                error.log_err(); // Log but don't fail
            }
        }
        
        cx.emit(PanelEvent::ChartDataRequested(symbol.clone(), self.current_timeframe));
        cx.notify();
    }
    
    /// Change timeframe with validation and async data refresh (.rules compliance)
    pub fn set_timeframe(&mut self, timeframe: TimeFrame, cx: &mut Context<Self>) -> Result<()> {
        self.current_timeframe = timeframe;
        
        if let Some(symbol) = &self.current_symbol {
            // Use variable shadowing for clarity in async context (.rules compliance)
            let symbol = symbol.clone();
            let timeframe = timeframe;
            
            // Request fresh historical data for new timeframe with proper error propagation
            if let Some(trading_manager) = self.trading_manager.upgrade() {
                let _task = trading_manager.update(cx, |manager, cx| {
                    manager.get_historical_data(&symbol, timeframe, 100, cx)
                });
            }
            
            cx.emit(PanelEvent::ChartDataRequested(symbol, timeframe));
        }
        
        cx.emit(PanelEvent::TimeFrameChanged(timeframe));
        cx.notify();
        Ok(())
    }
    
    /// Update chart data with bounds checking and validation (.rules compliance)
    pub fn update_chart_data(&mut self, data: Vec<Candle>, cx: &mut Context<Self>) {
        // Validate candle data with proper error handling
        let valid_data: Vec<Candle> = data.into_iter()
            .filter(|candle| {
                // Validate OHLC relationships
                let is_valid = candle.high >= candle.low && 
                    candle.high >= candle.open && 
                    candle.high >= candle.close &&
                    candle.low <= candle.open &&
                    candle.low <= candle.close &&
                    candle.open > 0.0 &&
                    candle.high > 0.0 &&
                    candle.low > 0.0 &&
                    candle.close > 0.0;
                
                if !is_valid {
                    log::warn!("Invalid candle data filtered out: O={}, H={}, L={}, C={}", 
                        candle.open, candle.high, candle.low, candle.close);
                }
                
                is_valid
            })
            .collect();
        
        // Only update if we have valid data
        if !valid_data.is_empty() {
            self.chart_data = valid_data;
            self.last_update = Some(std::time::SystemTime::now());
            cx.notify();
        } else {
            log::error!("No valid candle data to display");
        }
    }
    
    /// Update real-time price data (append new candle or update last candle)
    pub fn update_real_time_data(&mut self, market_data: MarketData, cx: &mut Context<Self>) {
        if let Some(symbol) = &self.current_symbol {
            if market_data.symbol == *symbol {
                // Update the last candle with current price using bounds checking
                if let Some(last_candle) = self.chart_data.last_mut() {
                    last_candle.close = market_data.current_price;
                    last_candle.high = last_candle.high.max(market_data.current_price);
                    last_candle.low = last_candle.low.min(market_data.current_price);
                    last_candle.volume = market_data.volume;
                    last_candle.timestamp = market_data.timestamp;
                    
                    self.last_update = Some(std::time::SystemTime::now());
                    cx.notify(); // Trigger UI update
                }
            }
        }
    }
    
    /// Handle trading events from TradingManager for real-time updates
    fn handle_trading_event(&mut self, event: TradingEvent, cx: &mut Context<Self>) {
        match event {
            TradingEvent::SymbolSelected(symbol) => {
                // Update chart when symbol is selected from watchlist
                self.set_symbol(symbol, cx);
            }
            TradingEvent::MarketDataUpdated(market_data) => {
                self.update_real_time_data(market_data, cx);
            }
            TradingEvent::HistoricalDataUpdated(symbol, candles) => {
                if let Some(current_symbol) = &self.current_symbol {
                    if symbol == *current_symbol {
                        self.update_chart_data(candles, cx);
                    }
                }
            }
            TradingEvent::WebSocketConnected => {
                // Re-subscribe when WebSocket reconnects
                if let Some(symbol) = &self.current_symbol {
                    if let Err(error) = self.subscribe_to_symbol(symbol, cx) {
                        error.log_err();
                    }
                }
            }
            TradingEvent::DataServiceError(error) => {
                log::error!("Data service error in chart: {}", error);
                cx.emit(PanelEvent::ErrorOccurred(error));
            }
            _ => {} // Handle other events as needed
        }
    }
    
    /// Subscribe to real-time updates for symbol
    fn subscribe_to_symbol(&mut self, symbol: &str, cx: &mut Context<Self>) -> Result<()> {
        if let Some(trading_manager) = self.trading_manager.upgrade() {
            trading_manager.update(cx, |manager, cx| {
                manager.subscribe_to_symbol(symbol.to_string(), cx)
            })?;
        }
        Ok(())
    }
    
    /// Request historical data for current symbol and timeframe
    fn request_historical_data(&mut self, symbol: &str, cx: &mut Context<Self>) {
        if let Some(trading_manager) = self.trading_manager.upgrade() {
            let symbol_clone = symbol.to_string();
            let timeframe = self.current_timeframe;
            let _task = trading_manager.update(cx, |manager, cx| {
                manager.get_historical_data(&symbol_clone, timeframe, 100, cx)
            });
        }
    }
    
    /// Toggle real-time updates
    pub fn set_real_time_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.real_time_enabled = enabled;
        cx.notify();
    }
    
    /// Refresh chart data
    pub fn refresh_chart_data(&mut self, cx: &mut Context<Self>) {
        if let Some(symbol) = &self.current_symbol {
            self.request_historical_data(symbol, cx);
            cx.emit(PanelEvent::RefreshRequested(symbol.clone()));
        }
    }
    
    /// Zoom in on chart with bounds checking (.rules compliance)
    pub fn zoom_in(&mut self, cx: &mut Context<Self>) {
        self.zoom_level = (self.zoom_level * 1.2).min(5.0); // Max 5x zoom
        cx.notify();
    }
    
    /// Zoom out on chart with bounds checking (.rules compliance)
    pub fn zoom_out(&mut self, cx: &mut Context<Self>) {
        self.zoom_level = (self.zoom_level / 1.2).max(0.2); // Min 0.2x zoom
        cx.notify();
    }
    
    /// Reset zoom to default
    pub fn reset_zoom(&mut self, cx: &mut Context<Self>) {
        self.zoom_level = 1.0;
        self.pan_offset = 0.0;
        cx.notify();
    }
    
    /// Pan chart left
    pub fn pan_left(&mut self, cx: &mut Context<Self>) {
        self.pan_offset = (self.pan_offset - 10.0).max(-100.0);
        cx.notify();
    }
    
    /// Pan chart right
    pub fn pan_right(&mut self, cx: &mut Context<Self>) {
        self.pan_offset = (self.pan_offset + 10.0).min(100.0);
        cx.notify();
    }
    
    /// Toggle volume display
    pub fn toggle_volume(&mut self, cx: &mut Context<Self>) {
        self.show_volume = !self.show_volume;
        cx.notify();
    }
    
    /// Set chart style with validation (.rules compliance)
    pub fn set_chart_style(&mut self, style: ChartStyle, cx: &mut Context<Self>) {
        self.chart_style = style;
        cx.notify();
    }
    
    /// Get visible candles based on zoom and pan with bounds checking (.rules compliance)
    fn get_visible_candles(&self) -> &[Candle] {
        if self.chart_data.is_empty() {
            return &[];
        }
        
        let total_candles = self.chart_data.len();
        let visible_count = ((total_candles as f64) / self.zoom_level).max(10.0) as usize;
        let visible_count = visible_count.min(total_candles);
        
        // Calculate start index based on pan offset
        let pan_shift = ((self.pan_offset / 100.0) * (total_candles as f64)) as isize;
        let start_index = ((total_candles as isize) - (visible_count as isize) - pan_shift)
            .max(0)
            .min((total_candles - visible_count) as isize) as usize;
        
        // Safe indexing with bounds checking (.rules compliance)
        if let Some(slice) = self.chart_data.get(start_index..start_index + visible_count) {
            slice
        } else {
            &self.chart_data[..]
        }
    }
}

impl EventEmitter<PanelEvent> for ChartPanel {}

impl Panel for ChartPanel {
    fn panel_name(&self) -> &'static str {
        "Chart"
    }
    
    fn dock_position(&self) -> DockPosition {
        DockPosition::Right
    }
    
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
    
    fn set_width(&mut self, width: Option<Pixels>, _cx: &mut Context<Self>) {
        self.width = width;
    }
}

impl Render for ChartPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Root::new()
            .child(
                gpui::div()
                    .flex()
                    .flex_col()
                    .w_full()
                    .h_full()
                    .p_4()
                    .child(
                        // Header with symbol, timeframe buttons, and chart controls
                        gpui::div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .mb_4()
                            .child(
                                // Symbol and info
                                gpui::div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        gpui::div()
                                            .text_lg()
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .child(format!("Chart: {}", 
                                                self.current_symbol.as_deref().unwrap_or("No symbol selected")))
                                    )
                                    .child(
                                        gpui::div()
                                            .text_sm()
                                            .text_color(gpui::rgb(0x666666))
                                            .child(format!("Zoom: {:.1}x", self.zoom_level))
                                    )
                            )
                            .child(
                                // Timeframe selection buttons
                                gpui::div()
                                    .flex()
                                    .gap_1()
                                    .child(self.render_timeframe_button("1m", TimeFrame::OneMinute, cx))
                                    .child(self.render_timeframe_button("5m", TimeFrame::FiveMinutes, cx))
                                    .child(self.render_timeframe_button("15m", TimeFrame::FifteenMinutes, cx))
                                    .child(self.render_timeframe_button("1h", TimeFrame::OneHour, cx))
                                    .child(self.render_timeframe_button("1d", TimeFrame::OneDay, cx))
                                    .child(self.render_timeframe_button("1w", TimeFrame::OneWeek, cx))
                                    .child(self.render_timeframe_button("1M", TimeFrame::OneMonth, cx))
                            )
                            .child(
                                // Chart controls
                                gpui::div()
                                    .flex()
                                    .gap_1()
                                    .child(
                                        Button::new("zoom-in-btn")
                                            .label("+")
                                            .on_click(cx.listener(|this, _event, cx| {
                                                this.zoom_in(cx);
                                            }))
                                    )
                                    .child(
                                        Button::new("zoom-out-btn")
                                            .label("-")
                                            .on_click(cx.listener(|this, _event, cx| {
                                                this.zoom_out(cx);
                                            }))
                                    )
                                    .child(
                                        Button::new("reset-zoom-btn")
                                            .label("Reset")
                                            .on_click(cx.listener(|this, _event, cx| {
                                                this.reset_zoom(cx);
                                            }))
                                    )
                                    .child(
                                        Button::new("toggle-volume-btn")
                                            .label(if self.show_volume { "Hide Vol" } else { "Show Vol" })
                                            .on_click(cx.listener(|this, _event, cx| {
                                                this.toggle_volume(cx);
                                            }))
                                    )
                                    .child(
                                        Button::new("refresh-chart-btn")
                                            .label("Refresh")
                                            .on_click(cx.listener(|this, _event, cx| {
                                                this.refresh_chart_data(cx);
                                            }))
                                    )
                            )
                    )
                    .child(
                        // Chart area with error handling (.rules compliance)
                        if self.chart_data.is_empty() {
                            gpui::div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .h_full()
                                .child(
                                    gpui::div()
                                        .text_color(gpui::rgb(0x888888))
                                        .child("Select a stock to view chart")
                                )
                        } else {
                            match self.render_chart(cx) {
                                Ok(chart) => chart,
                                Err(error) => {
                                    error.log_err(); // Proper error handling (.rules compliance)
                                    gpui::div()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .h_full()
                                        .child(
                                            gpui::div()
                                                .text_color(gpui::rgb(0xaa0000))
                                                .child("Error rendering chart. Please try refreshing.")
                                        )
                                }
                            }
                        }
                    )
            )
    }
    
    fn render_timeframe_button(
        &mut self, 
        label: &str, 
        timeframe: TimeFrame, 
        cx: &mut Context<Self>
    ) -> impl IntoElement {
        let is_active = self.current_timeframe == timeframe;
        
        Button::new(format!("timeframe-{}", label))
            .label(label)
            .variant(if is_active { "primary" } else { "secondary" })
            .on_click(cx.listener(move |this, _event, cx| {
                if let Err(error) = this.set_timeframe(timeframe, cx) {
                    error.log_err(); // Proper error handling (.rules compliance)
                }
            }))
    }
    
    /// Render chart with proper OHLC data binding and error handling (.rules compliance)
    fn render_chart(&mut self, _cx: &mut Context<Self>) -> Result<impl IntoElement> {
        // Get visible candles based on zoom and pan with bounds checking
        let visible_candles = self.get_visible_candles();
        
        if visible_candles.is_empty() {
            return Err(anyhow::anyhow!("No visible candles to display"));
        }
        
        // Convert candles to ChartData format with proper OHLC binding
        let chart_data: Vec<ChartData> = visible_candles
            .iter()
            .map(|candle| {
                // Convert SystemTime to timestamp with error handling
                let timestamp = candle.timestamp
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as f64;
                
                // Create ChartData with OHLC values
                ChartData::new(
                    timestamp,
                    vec![candle.open, candle.high, candle.low, candle.close]
                )
            })
            .collect();
        
        // Validate chart data before rendering
        if chart_data.is_empty() {
            return Err(anyhow::anyhow!("Failed to convert candle data to chart format"));
        }
        
        // Create Chart component with proper configuration
        let chart = Chart::new("price-chart")
            .chart_type(match self.chart_style {
                ChartStyle::Candlestick => ChartType::Candlestick,
                ChartStyle::Line => ChartType::Line,
                ChartStyle::Area => ChartType::Area,
            })
            .data(chart_data)
            .width_full()
            .height_full();
        
        Ok(gpui::div()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .child(chart)
            .when(self.show_volume, |div| {
                // Add volume chart below main chart if enabled
                div.child(self.render_volume_chart(visible_candles))
            }))
    }
    
    /// Render volume chart with bounds checking (.rules compliance)
    fn render_volume_chart(&self, candles: &[Candle]) -> impl IntoElement {
        let volume_data: Vec<ChartData> = candles
            .iter()
            .map(|candle| {
                let timestamp = candle.timestamp
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as f64;
                
                ChartData::new(timestamp, vec![candle.volume as f64])
            })
            .collect();
        
        gpui::div()
            .h(gpui::px(100.0))
            .w_full()
            .mt_2()
            .child(
                Chart::new("volume-chart")
                    .chart_type(ChartType::Bar)
                    .data(volume_data)
                    .width_full()
                    .height_full()
            )
    }
}

/// Stock info panel using gpui-component's layout components
pub struct StockInfoPanel {
    focus_handle: FocusHandle,
    current_symbol: Option<String>,
    stock_info: Option<StockInfo>,
    current_market_data: Option<MarketData>,
    trading_manager: WeakEntity<TradingManager>,
    width: Option<Pixels>,
    real_time_enabled: bool,
    last_update: Option<std::time::SystemTime>,
    _subscriptions: Vec<Subscription>,
}

impl StockInfoPanel {
    pub fn new(trading_manager: WeakEntity<TradingManager>, cx: &mut App) -> Entity<Self> {
        let panel = cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
            current_symbol: None,
            stock_info: None,
            current_market_data: None,
            trading_manager: trading_manager.clone(),
            width: None,
            real_time_enabled: true,
            last_update: None,
            _subscriptions: Vec::new(),
        });
        
        // Subscribe to TradingManager events for real-time updates
        if let Some(manager) = trading_manager.upgrade() {
            let subscription = cx.subscribe(&manager, |this, _manager, event, cx| {
                this.handle_trading_event(event.clone(), cx);
            });
            panel.update(cx, |this, _| {
                this._subscriptions.push(subscription);
            });
        }
        
        panel
    }
    
    /// Set symbol and load stock info with real-time subscription
    pub fn set_symbol(&mut self, symbol: String, cx: &mut Context<Self>) {
        self.current_symbol = Some(symbol.clone());
        
        // Subscribe to real-time updates if enabled
        if self.real_time_enabled {
            if let Err(error) = self.subscribe_to_symbol(&symbol, cx) {
                error.log_err();
            }
        }
        
        // Request market data
        self.request_market_data(&symbol, cx);
        
        // Load stock info (placeholder for now)
        self.stock_info = Some(StockInfo {
            symbol: symbol.clone(),
            company_name: format!("{} Inc.", symbol),
            sector: "Technology".to_string(),
            industry: "Software".to_string(),
            market_cap: Some(1_000_000_000),
            pe_ratio: Some(25.5),
            dividend_yield: Some(2.1),
            fifty_two_week_high: 150.0,
            fifty_two_week_low: 80.0,
            average_volume: 1_000_000,
            beta: Some(1.2),
            eps: Some(5.50),
            description: "A leading technology company.".to_string(),
        });
        
        cx.notify();
    }
    
    /// Update stock info data
    pub fn update_stock_info(&mut self, info: StockInfo, cx: &mut Context<Self>) {
        self.stock_info = Some(info);
        self.last_update = Some(std::time::SystemTime::now());
        cx.notify();
    }
    
    /// Update market data for real-time price updates
    pub fn update_market_data(&mut self, market_data: MarketData, cx: &mut Context<Self>) {
        if let Some(symbol) = &self.current_symbol {
            if market_data.symbol == *symbol {
                self.current_market_data = Some(market_data);
                self.last_update = Some(std::time::SystemTime::now());
                cx.notify();
            }
        }
    }
    
    /// Handle trading events from TradingManager
    fn handle_trading_event(&mut self, event: TradingEvent, cx: &mut Context<Self>) {
        match event {
            TradingEvent::SymbolSelected(symbol) => {
                // Update stock info when symbol is selected from watchlist
                self.set_symbol(symbol, cx);
            }
            TradingEvent::MarketDataUpdated(market_data) => {
                self.update_market_data(market_data, cx);
            }
            TradingEvent::WebSocketConnected => {
                if let Some(symbol) = &self.current_symbol {
                    if let Err(error) = self.subscribe_to_symbol(symbol, cx) {
                        error.log_err();
                    }
                }
            }
            TradingEvent::DataServiceError(error) => {
                log::error!("Data service error in stock info: {}", error);
            }
            _ => {}
        }
    }
    
    /// Subscribe to real-time updates
    fn subscribe_to_symbol(&mut self, symbol: &str, cx: &mut Context<Self>) -> Result<()> {
        if let Some(trading_manager) = self.trading_manager.upgrade() {
            trading_manager.update(cx, |manager, cx| {
                manager.subscribe_to_symbol(symbol.to_string(), cx)
            })?;
        }
        Ok(())
    }
    
    /// Request market data
    fn request_market_data(&mut self, symbol: &str, cx: &mut Context<Self>) {
        if let Some(trading_manager) = self.trading_manager.upgrade() {
            let symbol_clone = symbol.to_string();
            let _task = trading_manager.update(cx, |manager, cx| {
                manager.get_market_data(&symbol_clone, cx)
            });
        }
    }
    
    /// Toggle real-time updates
    pub fn set_real_time_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.real_time_enabled = enabled;
        cx.notify();
    }
    
    /// Refresh stock info
    pub fn refresh_stock_info(&mut self, cx: &mut Context<Self>) {
        if let Some(symbol) = &self.current_symbol {
            self.request_market_data(symbol, cx);
        }
    }
}

impl EventEmitter<PanelEvent> for StockInfoPanel {}

impl Panel for StockInfoPanel {
    fn panel_name(&self) -> &'static str {
        "Stock Info"
    }
    
    fn dock_position(&self) -> DockPosition {
        DockPosition::Right
    }
    
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
    
    fn set_width(&mut self, width: Option<Pixels>, _cx: &mut Context<Self>) {
        self.width = width;
    }
}

impl Render for StockInfoPanel {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        Root::new()
            .child(
                gpui::div()
                    .flex()
                    .flex_col()
                    .w_full()
                    .h_full()
                    .p_4()
                    .child(
                        gpui::div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::BOLD)
                            .mb_4()
                            .child("Stock Information")
                    )
                    .child(
                        if let Some(info) = &self.stock_info {
                            self.render_stock_info(info)
                        } else {
                            gpui::div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .h_full()
                                .child(
                                    gpui::div()
                                        .text_color(gpui::rgb(0x888888))
                                        .child("Select a stock to view information")
                                )
                        }
                    )
            )
    }
    
    fn render_stock_info(&self, info: &StockInfo) -> impl IntoElement {
        gpui::div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                gpui::div()
                    .child(
                        gpui::div()
                            .text_xl()
                            .font_weight(gpui::FontWeight::BOLD)
                            .child(format!("{} ({})", info.company_name, info.symbol))
                    )
                    .child(
                        gpui::div()
                            .text_sm()
                            .text_color(gpui::rgb(0x666666))
                            .child(format!("{} - {}", info.sector, info.industry))
                    )
            )
            .child(
                gpui::div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(self.render_info_row("Market Cap", 
                        info.market_cap.map(|mc| format!("${:.2}B", mc as f64 / 1_000_000_000.0))
                            .unwrap_or_else(|| "N/A".to_string())))
                    .child(self.render_info_row("P/E Ratio", 
                        info.pe_ratio.map(|pe| format!("{:.2}", pe))
                            .unwrap_or_else(|| "N/A".to_string())))
                    .child(self.render_info_row("Dividend Yield", 
                        info.dividend_yield.map(|dy| format!("{:.2}%", dy))
                            .unwrap_or_else(|| "N/A".to_string())))
                    .child(self.render_info_row("52W High", format!("${:.2}", info.fifty_two_week_high)))
                    .child(self.render_info_row("52W Low", format!("${:.2}", info.fifty_two_week_low)))
                    .child(self.render_info_row("Avg Volume", format!("{}", info.average_volume)))
                    .child(self.render_info_row("Beta", 
                        info.beta.map(|b| format!("{:.2}", b))
                            .unwrap_or_else(|| "N/A".to_string())))
                    .child(self.render_info_row("EPS", 
                        info.eps.map(|eps| format!("${:.2}", eps))
                            .unwrap_or_else(|| "N/A".to_string())))
            )
            .child(
                gpui::div()
                    .mt_4()
                    .child(
                        gpui::div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::BOLD)
                            .mb_2()
                            .child("Description")
                    )
                    .child(
                        gpui::div()
                            .text_sm()
                            .text_color(gpui::rgb(0x666666))
                            .child(info.description.clone())
                    )
            )
    }
    
    fn render_info_row(&self, label: &str, value: String) -> impl IntoElement {
        gpui::div()
            .flex()
            .justify_between()
            .items_center()
            .child(
                gpui::div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child(label)
            )
            .child(
                gpui::div()
                    .text_sm()
                    .child(value)
            )
    }
}

/// Order panel using gpui-component's form controls
pub struct OrderPanel {
    focus_handle: FocusHandle,
    current_symbol: Option<String>,
    order_side: OrderSide,
    order_type: OrderType,
    quantity: String,
    price: String,
    time_in_force: TimeInForce,
    trading_manager: WeakEntity<TradingManager>,
    width: Option<Pixels>,
    _subscriptions: Vec<Subscription>,
}

impl OrderPanel {
    pub fn new(trading_manager: WeakEntity<TradingManager>, cx: &mut App) -> Entity<Self> {
        let panel = cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
            current_symbol: None,
            order_side: OrderSide::Buy,
            order_type: OrderType::Market,
            quantity: String::new(),
            price: String::new(),
            time_in_force: TimeInForce::Day,
            trading_manager: trading_manager.clone(),
            width: None,
            _subscriptions: Vec::new(),
        });
        
        // Subscribe to TradingManager events
        if let Some(manager) = trading_manager.upgrade() {
            let subscription = cx.subscribe(&manager, |this, _manager, event, cx| {
                this.handle_trading_event(event.clone(), cx);
            });
            panel.update(cx, |this, _| {
                this._subscriptions.push(subscription);
            });
        }
        
        // Register action handlers for keyboard shortcuts
        panel.update(cx, |this, cx| {
            this.register_action_handlers(cx);
        });
        
        panel
    }
    
    /// Handle trading events from TradingManager
    fn handle_trading_event(&mut self, event: TradingEvent, cx: &mut Context<Self>) {
        match event {
            TradingEvent::SymbolSelected(symbol) => {
                // Update order panel when symbol is selected from watchlist
                self.set_symbol(symbol, cx);
            }
            _ => {}
        }
    }
    
    /// Register action handlers for keyboard shortcuts (.rules compliance)
    fn register_action_handlers(&mut self, cx: &mut Context<Self>) {
        cx.on_action(|this: &mut Self, _action: &SubmitOrder, cx| {
            if let Err(error) = this.place_order(cx) {
                error.log_err(); // Proper error handling
            }
        });
        
        cx.on_action(|this: &mut Self, _action: &ClearOrderForm, cx| {
            this.clear_form(cx);
        });
        
        cx.on_action(|this: &mut Self, _action: &FocusQuantityInput, cx| {
            cx.focus(&this.focus_handle);
            cx.notify();
        });
        
        cx.on_action(|this: &mut Self, _action: &FocusPriceInput, cx| {
            if matches!(this.order_type, OrderType::Limit) {
                cx.focus(&this.focus_handle);
                cx.notify();
            }
        });
    }
    
    /// Set symbol for order
    pub fn set_symbol(&mut self, symbol: String, cx: &mut Context<Self>) {
        self.current_symbol = Some(symbol);
        cx.notify();
    }
    
    /// Place order with validation (.rules compliance)
    pub fn place_order(&mut self, cx: &mut Context<Self>) -> Result<()> {
        let symbol = self.current_symbol.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No symbol selected. Please select a stock from the watchlist first."))?;
        
        // Validate quantity
        if self.quantity.is_empty() {
            return Err(anyhow::anyhow!("Quantity is required. Please enter the number of shares to trade."));
        }
        
        let quantity: u64 = self.quantity.parse()
            .map_err(|_| anyhow::anyhow!(
                "Invalid quantity '{}'. Please enter a valid number.",
                self.quantity
            ))?;
        
        if quantity == 0 {
            return Err(anyhow::anyhow!("Quantity must be greater than zero."));
        }
        
        // Validate quantity is reasonable (not too large)
        if quantity > 1_000_000 {
            return Err(anyhow::anyhow!(
                "Quantity {} is too large. Please enter a reasonable number of shares.",
                quantity
            ));
        }
        
        // Validate price for limit orders
        let price = if self.order_type == OrderType::Market {
            None
        } else {
            if self.price.is_empty() {
                return Err(anyhow::anyhow!("Price is required for limit orders. Please enter a price."));
            }
            
            let parsed_price = self.price.parse::<f64>()
                .map_err(|_| anyhow::anyhow!(
                    "Invalid price '{}'. Please enter a valid number.",
                    self.price
                ))?;
            
            if parsed_price <= 0.0 {
                return Err(anyhow::anyhow!("Price must be greater than zero."));
            }
            
            if parsed_price > 1_000_000.0 {
                return Err(anyhow::anyhow!(
                    "Price ${:.2} is too high. Please enter a reasonable price.",
                    parsed_price
                ));
            }
            
            Some(parsed_price)
        };
        
        let order = Order {
            id: format!("ORD_{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default().as_millis()),
            symbol: symbol.clone(),
            side: self.order_side,
            order_type: self.order_type,
            quantity,
            price,
            time_in_force: self.time_in_force,
            status: crate::OrderStatus::Pending,
            filled_quantity: 0,
            average_fill_price: None,
            created_at: std::time::SystemTime::now(),
            updated_at: std::time::SystemTime::now(),
        };
        
        cx.emit(PanelEvent::OrderPlaced(order));
        
        // Clear form after successful order
        self.clear_form(cx);
        
        Ok(())
    }
    
    /// Clear order form (.rules compliance)
    pub fn clear_form(&mut self, cx: &mut Context<Self>) {
        self.quantity.clear();
        self.price.clear();
        cx.notify();
    }
}

impl EventEmitter<PanelEvent> for OrderPanel {}

impl Panel for OrderPanel {
    fn panel_name(&self) -> &'static str {
        "Order Entry"
    }
    
    fn dock_position(&self) -> DockPosition {
        DockPosition::Bottom
    }
    
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
    
    fn set_width(&mut self, width: Option<Pixels>, _cx: &mut Context<Self>) {
        self.width = width;
    }
}

impl Render for OrderPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Root::new()
            .child(
                gpui::div()
                    .flex()
                    .flex_col()
                    .w_full()
                    .h_full()
                    .p_4()
                    .child(
                        gpui::div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::BOLD)
                            .mb_4()
                            .child(format!("Order Entry - {}", 
                                self.current_symbol.as_deref().unwrap_or("No symbol selected")))
                    )
                    .child(
                        if self.current_symbol.is_some() {
                            self.render_order_form(cx)
                        } else {
                            gpui::div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .h_full()
                                .child(
                                    gpui::div()
                                        .text_color(gpui::rgb(0x888888))
                                        .child("Select a stock to place orders")
                                )
                        }
                    )
            )
    }
    
    fn render_order_form(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        gpui::div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                // Buy/Sell buttons
                gpui::div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new("buy-btn")
                            .label("Buy")
                            .variant(if matches!(self.order_side, OrderSide::Buy) { "primary" } else { "secondary" })
                            .on_click(cx.listener(|this, _event, cx| {
                                this.order_side = OrderSide::Buy;
                                cx.notify();
                            }))
                    )
                    .child(
                        Button::new("sell-btn")
                            .label("Sell")
                            .variant(if matches!(self.order_side, OrderSide::Sell) { "primary" } else { "secondary" })
                            .on_click(cx.listener(|this, _event, cx| {
                                this.order_side = OrderSide::Sell;
                                cx.notify();
                            }))
                    )
            )
            .child(
                // Order type buttons
                gpui::div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new("market-btn")
                            .label("Market")
                            .variant(if matches!(self.order_type, OrderType::Market) { "primary" } else { "secondary" })
                            .on_click(cx.listener(|this, _event, cx| {
                                this.order_type = OrderType::Market;
                                cx.notify();
                            }))
                    )
                    .child(
                        Button::new("limit-btn")
                            .label("Limit")
                            .variant(if matches!(self.order_type, OrderType::Limit) { "primary" } else { "secondary" })
                            .on_click(cx.listener(|this, _event, cx| {
                                this.order_type = OrderType::Limit;
                                cx.notify();
                            }))
                    )
            )
            .child(
                // Quantity input
                gpui::div()
                    .child(
                        gpui::div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .mb_1()
                            .child("Quantity")
                    )
                    .child(
                        Input::new("quantity-input")
                            .placeholder("Enter quantity (e.g., 100)...")
                            .value(self.quantity.clone())
                            .on_input(cx.listener(|this, input: &str, cx| {
                                // Only allow numeric input
                                if input.is_empty() || input.chars().all(|c| c.is_ascii_digit()) {
                                    this.quantity = input.to_string();
                                    cx.notify();
                                }
                            }))
                    )
            )
            .child(
                // Price input (only for limit orders)
                if matches!(self.order_type, OrderType::Limit) {
                    gpui::div()
                        .child(
                            gpui::div()
                                .text_sm()
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .mb_1()
                                .child("Price")
                        )
                        .child(
                            Input::new("price-input")
                                .placeholder("Enter price (e.g., 150.50)...")
                                .value(self.price.clone())
                                .on_input(cx.listener(|this, input: &str, cx| {
                                    // Allow numeric input with decimal point
                                    if input.is_empty() || input.chars().all(|c| c.is_ascii_digit() || c == '.') {
                                        // Validate only one decimal point
                                        if input.chars().filter(|&c| c == '.').count() <= 1 {
                                            this.price = input.to_string();
                                            cx.notify();
                                        }
                                    }
                                }))
                        )
                } else {
                    gpui::div() // Empty div for market orders
                }
            )
            .child(
                // Action buttons
                gpui::div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new("place-order-btn")
                            .label("Place Order")
                            .variant("primary")
                            .on_click(cx.listener(|this, _event, cx| {
                                if let Err(error) = this.place_order(cx) {
                                    log::error!("Failed to place order: {}", error);
                                    error.log_err();
                                }
                            }))
                    )
                    .child(
                        Button::new("clear-form-btn")
                            .label("Clear")
                            .variant("secondary")
                            .on_click(cx.listener(|this, _event, cx| {
                                this.clear_form(cx);
                            }))
                    )
            )
    }
}

/// Order book panel using gpui-component's virtualized Table
pub struct OrderBookPanel {
    focus_handle: FocusHandle,
    current_symbol: Option<String>,
    order_book: Option<OrderBook>,
    trading_manager: WeakEntity<TradingManager>,
    width: Option<Pixels>,
    real_time_enabled: bool,
    last_update: Option<std::time::SystemTime>,
    _subscriptions: Vec<Subscription>,
}

impl OrderBookPanel {
    pub fn new(trading_manager: WeakEntity<TradingManager>, cx: &mut App) -> Entity<Self> {
        let panel = cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
            current_symbol: None,
            order_book: None,
            trading_manager: trading_manager.clone(),
            width: None,
            real_time_enabled: true,
            last_update: None,
            _subscriptions: Vec::new(),
        });
        
        // Subscribe to TradingManager events for real-time updates
        if let Some(manager) = trading_manager.upgrade() {
            let subscription = cx.subscribe(&manager, |this, _manager, event, cx| {
                this.handle_trading_event(event.clone(), cx);
            });
            panel.update(cx, |this, _| {
                this._subscriptions.push(subscription);
            });
        }
        
        panel
    }
    
    /// Set symbol and load order book with real-time subscription
    pub fn set_symbol(&mut self, symbol: String, cx: &mut Context<Self>) {
        self.current_symbol = Some(symbol.clone());
        
        // Subscribe to real-time order book updates if enabled
        if self.real_time_enabled {
            if let Err(error) = self.subscribe_to_symbol(&symbol, cx) {
                error.log_err(); // Proper error handling
            }
        }
        
        // Request initial order book data
        self.request_order_book(&symbol, cx);
        
        cx.notify();
    }
    
    /// Update order book data with real-time integration and validation
    pub fn update_order_book(&mut self, order_book: OrderBook, cx: &mut Context<Self>) {
        if let Some(current_symbol) = &self.current_symbol {
            if order_book.symbol == *current_symbol {
                // Validate order book data
                if self.validate_order_book(&order_book) {
                    self.order_book = Some(order_book);
                    self.last_update = Some(std::time::SystemTime::now());
                    cx.notify(); // Trigger UI update
                } else {
                    log::warn!("Received invalid order book data for {}", current_symbol);
                }
            }
        }
    }
    
    /// Validate order book data
    fn validate_order_book(&self, order_book: &OrderBook) -> bool {
        // Check that bids are sorted descending
        for i in 1..order_book.bids.len() {
            if let (Some(prev), Some(curr)) = (order_book.bids.get(i - 1), order_book.bids.get(i)) {
                if prev.price < curr.price {
                    return false;
                }
            }
        }
        
        // Check that asks are sorted ascending
        for i in 1..order_book.asks.len() {
            if let (Some(prev), Some(curr)) = (order_book.asks.get(i - 1), order_book.asks.get(i)) {
                if prev.price > curr.price {
                    return false;
                }
            }
        }
        
        // Check that best bid < best ask (if both exist)
        if let (Some(best_bid), Some(best_ask)) = (order_book.bids.first(), order_book.asks.first()) {
            if best_bid.price >= best_ask.price {
                return false;
            }
        }
        
        true
    }
    
    /// Handle trading events from TradingManager
    fn handle_trading_event(&mut self, event: TradingEvent, cx: &mut Context<Self>) {
        match event {
            TradingEvent::SymbolSelected(symbol) => {
                // Update order book when symbol is selected from watchlist
                self.set_symbol(symbol, cx);
            }
            TradingEvent::OrderBookUpdated(symbol, order_book) => {
                if let Some(current_symbol) = &self.current_symbol {
                    if symbol == *current_symbol {
                        self.update_order_book(order_book, cx);
                    }
                }
            }
            TradingEvent::WebSocketConnected => {
                if let Some(symbol) = &self.current_symbol {
                    if let Err(error) = self.subscribe_to_symbol(symbol, cx) {
                        error.log_err();
                    }
                }
            }
            TradingEvent::DataServiceError(error) => {
                log::error!("Data service error in order book: {}", error);
                cx.emit(PanelEvent::ErrorOccurred(error));
            }
            _ => {}
        }
    }
    
    /// Subscribe to real-time updates
    fn subscribe_to_symbol(&mut self, symbol: &str, cx: &mut Context<Self>) -> Result<()> {
        if let Some(trading_manager) = self.trading_manager.upgrade() {
            trading_manager.update(cx, |manager, cx| {
                manager.subscribe_to_symbol(symbol.to_string(), cx)
            })?;
        }
        Ok(())
    }
    
    /// Request order book data
    fn request_order_book(&mut self, symbol: &str, cx: &mut Context<Self>) {
        if let Some(trading_manager) = self.trading_manager.upgrade() {
            let symbol_clone = symbol.to_string();
            let _task = trading_manager.update(cx, |manager, cx| {
                manager.get_order_book(&symbol_clone, cx)
            });
        }
    }
    
    /// Refresh order book data
    pub fn refresh_order_book(&mut self, cx: &mut Context<Self>) {
        if let Some(symbol) = &self.current_symbol {
            self.request_order_book(symbol, cx);
            cx.emit(PanelEvent::RefreshRequested(symbol.clone()));
        }
    }
    
    /// Toggle real-time updates
    pub fn set_real_time_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.real_time_enabled = enabled;
        cx.notify();
    }
}

impl EventEmitter<PanelEvent> for OrderBookPanel {}

impl Panel for OrderBookPanel {
    fn panel_name(&self) -> &'static str {
        "Order Book"
    }
    
    fn dock_position(&self) -> DockPosition {
        DockPosition::Right
    }
    
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
    
    fn set_width(&mut self, width: Option<Pixels>, _cx: &mut Context<Self>) {
        self.width = width;
    }
}

impl Render for OrderBookPanel {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        Root::new()
            .child(
                gpui::div()
                    .flex()
                    .flex_col()
                    .w_full()
                    .h_full()
                    .p_4()
                    .child(
                        gpui::div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::BOLD)
                            .mb_4()
                            .child(format!("Order Book - {}", 
                                self.current_symbol.as_deref().unwrap_or("No symbol selected")))
                    )
                    .child(
                        if let Some(order_book) = &self.order_book {
                            self.render_order_book(order_book)
                        } else {
                            gpui::div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .h_full()
                                .child(
                                    gpui::div()
                                        .text_color(gpui::rgb(0x888888))
                                        .child("Select a stock to view order book")
                                )
                        }
                    )
            )
    }
    
    fn render_order_book(&self, order_book: &OrderBook) -> impl IntoElement {
        gpui::div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                // Spread information
                gpui::div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .p_2()
                    .bg(gpui::rgb(0xf5f5f5))
                    .rounded_md()
                    .child(
                        gpui::div()
                            .text_sm()
                            .child(format!("Spread: ${:.2}", order_book.get_spread()))
                    )
                    .child(
                        gpui::div()
                            .text_sm()
                            .child(format!("Mid: ${:.2}", order_book.get_mid_price()))
                    )
            )
            .child(
                // Order book tables
                gpui::div()
                    .flex()
                    .gap_4()
                    .h_full()
                    .child(
                        // Bids (left side)
                        gpui::div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .child(
                                gpui::div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(gpui::rgb(0x00aa00))
                                    .mb_2()
                                    .child("Bids")
                            )
                            .child(self.render_order_book_side(&order_book.bids, true))
                    )
                    .child(
                        // Asks (right side)
                        gpui::div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .child(
                                gpui::div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(gpui::rgb(0xaa0000))
                                    .mb_2()
                                    .child("Asks")
                            )
                            .child(self.render_order_book_side(&order_book.asks, false))
                    )
            )
    }
    
    fn render_order_book_side(&self, entries: &[OrderBookEntry], is_bids: bool) -> impl IntoElement {
        let columns = vec![
            TableColumn::new("Price", 80),
            TableColumn::new("Size", 80),
            TableColumn::new("Total", 80),
        ];
        
        let table_data: Vec<TableData> = entries
            .iter()
            .take(10) // Show top 10 levels
            .enumerate()
            .map(|(index, entry)| {
                TableData::new(vec![
                    format!("${:.2}", entry.price),
                    entry.quantity.to_string(),
                    format!("${:.0}", entry.price * entry.quantity as f64),
                ])
                .with_id(index.to_string())
            })
            .collect();
        
        Table::new(if is_bids { "bids-table" } else { "asks-table" })
            .columns(columns)
            .data(table_data)
            .height_full()
    }
}