use anyhow::Result;
use gpui::{App, AppContext, Context, Entity, EventEmitter, px, Render, Subscription};
use http_client::HttpClient;
use paths;
use settings::Settings;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use workspace;

// Re-export core modules
pub mod market_data;
pub mod websocket_service;
pub mod mock_data_service;
pub mod error_handling;
pub mod input_validation;
pub mod trading_actions;
pub mod trading_settings;
pub mod panel_persistence;
pub mod panel_manager;
pub mod demo_panel;
// pub mod panels;  // Disabled: requires gpui-component which conflicts with tree-sitter version

#[cfg(test)]
mod tests;

pub use market_data::*;
pub use websocket_service::*;
pub use mock_data_service::*;
pub use error_handling::*;
pub use input_validation::*;
pub use trading_actions::*;
pub use trading_settings::*;
pub use panel_persistence::*;
pub use panel_manager::*;
pub use demo_panel::*;
// pub use panels::*;  // Disabled

/// Initialize the stock trading system with Zed Lite integration
/// This is the main entry point called from Zed Lite's main.rs
/// Following .rules: proper error handling, full words, no unwrap()
pub fn init(http_client: Arc<dyn HttpClient>, cx: &mut App) -> Result<()> {
    // Register settings first (required before creating entities that use settings)
    StockTradingSettings::register(cx);
    
    // Register all trading actions with Zed's action system
    register_trading_actions(cx);
    
    // Create central TradingManager entity that coordinates all components
    // This entity will be stored globally for access from workspace
    let trading_manager = TradingManager::new(http_client.clone(), cx);
    
    // Initialize panel manager for the trading system
    trading_manager.update(cx, |manager, cx| {
        manager.initialize_panel_manager(cx)
    })?;
    
    // Store TradingManager globally for workspace access
    // Using a global entity pattern similar to other Zed components
    cx.set_global(GlobalTradingManager(trading_manager));
    
    log::info!("Stock trading system initialized successfully");
    
    Ok(())
}

/// Global wrapper for TradingManager entity
/// Allows access from workspace and other components
#[derive(Clone)]
pub struct GlobalTradingManager(pub Entity<TradingManager>);

impl gpui::Global for GlobalTradingManager {}

/// Register all trading actions with Zed's action dispatch system
/// Following .rules: use full words for action names, proper error handling
fn register_trading_actions(_cx: &mut App) {
    // Panel toggle actions are automatically registered by the actions! macro
    // Additional action registration can be added here if needed for custom behavior
    
    // Log successful registration
    log::info!("Stock trading actions registered successfully");
    
    // Note: Individual panels will register their action handlers in their constructors
    // using cx.on_action() pattern as shown in WatchlistPanel::register_action_handlers()
}

/// Enhanced central coordinator entity for the stock trading system
pub struct TradingManager {
    data_service: Entity<DataService>,
    websocket_service: Entity<WebSocketService>,
    mock_data_service: Entity<MockDataService>,
    panel_manager: Option<Entity<PanelManager>>,
    active_symbol: Option<String>,
    subscribed_symbols: std::collections::HashSet<String>,
    auto_subscribe_enabled: bool,
    panel_persistence: PanelPersistence,
    theme_colors: TradingThemeColors,
    _subscriptions: Vec<Subscription>,
}

impl TradingManager {
    pub fn new(http_client: Arc<dyn HttpClient>, cx: &mut App) -> Entity<Self> {
        let data_service = DataService::new(http_client, cx);
        let websocket_service = WebSocketService::new(cx);
        let mock_data_service = MockDataService::new(cx);
        
        // Load panel persistence from settings
        let settings = StockTradingSettings::get_global(cx);
        let config_dir = paths::config_dir().clone();
        let panel_persistence = PanelPersistence::load(&config_dir).unwrap_or_else(|error| {
            error.log_err(); // Log error but continue with defaults
            PanelPersistence::default()
        });
        
        // Initialize theme colors from settings
        let theme_colors = TradingThemeColors {
            positive_color: settings.theme_config.positive_color.clone(),
            negative_color: settings.theme_config.negative_color.clone(),
            neutral_color: settings.theme_config.neutral_color.clone(),
            chart_background: settings.theme_config.chart_background.clone(),
            grid_color: settings.theme_config.grid_color.clone(),
        };
        
        cx.new(|cx| {
            let mut manager = Self {
                data_service,
                websocket_service,
                mock_data_service,
                panel_manager: None,
                active_symbol: None,
                subscribed_symbols: std::collections::HashSet::new(),
                auto_subscribe_enabled: true,
                panel_persistence,
                theme_colors,
                _subscriptions: Vec::new(),
            };
            
            // Set up service references
            manager.setup_service_integration(cx);
            
            manager
        })
    }
    
    /// Set up integration between services
    fn setup_service_integration(&mut self, cx: &mut Context<Self>) {
        // Set WebSocket service reference in DataService
        self.data_service.update(cx, |data_service, _| {
            data_service.set_websocket_service(self.websocket_service.clone());
            data_service.set_mock_data_service(self.mock_data_service.clone());
        });
        
        // Subscribe to WebSocket events
        let websocket_subscription = cx.subscribe(&self.websocket_service, |this, _websocket, event, cx| {
            this.handle_websocket_event(event.clone(), cx);
        });
        self._subscriptions.push(websocket_subscription);
        
        // Subscribe to data service events
        let data_subscription = cx.subscribe(&self.data_service, |this, _data_service, event, cx| {
            this.handle_data_event(event.clone(), cx);
        });
        self._subscriptions.push(data_subscription);
        
        // Subscribe to mock data events
        let mock_subscription = cx.subscribe(&self.mock_data_service, |this, _mock_service, event, cx| {
            this.handle_mock_data_event(event.clone(), cx);
        });
        self._subscriptions.push(mock_subscription);
    }
    
    /// Handle WebSocket events
    fn handle_websocket_event(&mut self, event: WebSocketEvent, cx: &mut Context<Self>) {
        match event {
            WebSocketEvent::Connected => {
                // Re-subscribe to all symbols when WebSocket connects
                self.resubscribe_all_symbols(cx);
                cx.emit(TradingEvent::WebSocketConnected);
            }
            WebSocketEvent::Disconnected => {
                cx.emit(TradingEvent::WebSocketDisconnected);
            }
            WebSocketEvent::MessageReceived(message) => {
                // Forward WebSocket messages to DataService for processing
                if let Err(error) = self.data_service.update(cx, |data_service, cx| {
                    data_service.handle_websocket_message(message, cx)
                }) {
                    error.log_err(); // Proper error handling
                }
            }
            WebSocketEvent::ConnectionError(error) => {
                cx.emit(TradingEvent::DataServiceError(error));
            }
            _ => {} // Handle other events as needed
        }
    }
    
    /// Handle data service events
    fn handle_data_event(&mut self, event: DataEvent, cx: &mut Context<Self>) {
        match event {
            DataEvent::MarketDataReceived(market_data) => {
                cx.emit(TradingEvent::MarketDataUpdated(market_data));
            }
            DataEvent::TradeReceived(trade) => {
                // Convert trade to order for compatibility
                if let Ok(order) = self.convert_trade_to_order(trade) {
                    cx.emit(TradingEvent::OrderPlaced(order));
                }
            }
            DataEvent::ErrorOccurred(error) => {
                cx.emit(TradingEvent::DataServiceError(error));
            }
            _ => {} // Handle other events as needed
        }
    }
    
    /// Handle mock data service events
    fn handle_mock_data_event(&mut self, event: MockDataEvent, cx: &mut Context<Self>) {
        if let MockDataEvent::MarketDataUpdated(_symbol, market_data) = event {
            // Forward to main data flow
            cx.emit(TradingEvent::MarketDataUpdated(market_data));
        }
    }
    
    /// Set active symbol with automatic subscription
    pub fn set_active_symbol(&mut self, symbol: String, cx: &mut Context<Self>) -> Result<()> {
        // Auto-subscribe to real-time updates if enabled
        if self.auto_subscribe_enabled {
            self.subscribe_to_symbol(symbol.clone(), cx)?;
        }
        
        self.active_symbol = Some(symbol.clone());
        cx.emit(TradingEvent::SymbolSelected(symbol));
        Ok(())
    }
    
    /// Subscribe to symbol for real-time updates
    pub fn subscribe_to_symbol(&mut self, symbol: String, cx: &mut Context<Self>) -> Result<()> {
        if self.subscribed_symbols.contains(&symbol) {
            return Ok(()); // Already subscribed
        }
        
        self.subscribed_symbols.insert(symbol.clone());
        
        // Subscribe via DataService
        self.data_service.update(cx, |data_service, cx| {
            data_service.subscribe_to_symbol(symbol, cx)
        })?;
        
        Ok(())
    }
    
    /// Unsubscribe from symbol
    pub fn unsubscribe_from_symbol(&mut self, symbol: &str, cx: &mut Context<Self>) -> Result<()> {
        if !self.subscribed_symbols.remove(symbol) {
            return Ok(()); // Not subscribed
        }
        
        // Unsubscribe via DataService
        self.data_service.update(cx, |data_service, cx| {
            data_service.unsubscribe_from_symbol(symbol, cx)
        })?;
        
        Ok(())
    }
    
    /// Re-subscribe to all symbols (useful after reconnection)
    fn resubscribe_all_symbols(&mut self, cx: &mut Context<Self>) {
        let symbols: Vec<String> = self.subscribed_symbols.iter().cloned().collect();
        
        for symbol in symbols {
            if let Err(error) = self.data_service.update(cx, |data_service, cx| {
                data_service.subscribe_to_symbol(symbol, cx)
            }) {
                error.log_err(); // Log but continue with other symbols
            }
        }
    }
    
    /// Get market data for symbol
    pub fn get_market_data(&mut self, symbol: &str, cx: &mut Context<Self>) -> gpui::Task<Result<MarketData>> {
        self.data_service.update(cx, |data_service, cx| {
            data_service.get_market_data(symbol, cx)
        })
    }
    
    /// Get historical data for symbol
    pub fn get_historical_data(
        &mut self,
        symbol: &str,
        timeframe: TimeFrame,
        periods: usize,
        cx: &mut Context<Self>,
    ) -> gpui::Task<Result<Vec<Candle>>> {
        self.data_service.update(cx, |data_service, cx| {
            data_service.get_historical_data(symbol, timeframe, periods, cx)
        })
    }
    
    /// Get order book for symbol
    pub fn get_order_book(&mut self, symbol: &str, cx: &mut Context<Self>) -> gpui::Task<Result<OrderBook>> {
        self.data_service.update(cx, |data_service, cx| {
            data_service.get_order_book(symbol, cx)
        })
    }
    
    /// Toggle between mock and live data
    pub fn set_use_mock_data(&mut self, use_mock: bool, cx: &mut Context<Self>) {
        self.data_service.update(cx, |data_service, cx| {
            data_service.set_use_mock_data(use_mock, cx);
        });
    }
    
    /// Start mock data simulation
    pub fn start_mock_simulation(&mut self, cx: &mut Context<Self>) {
        self.mock_data_service.update(cx, |mock_service, cx| {
            mock_service.start_simulation(cx);
        });
    }
    
    /// Stop mock data simulation
    pub fn stop_mock_simulation(&mut self, cx: &mut Context<Self>) {
        self.mock_data_service.update(cx, |mock_service, cx| {
            mock_service.stop_simulation(cx);
        });
    }
    
    /// Get cache statistics
    pub fn get_cache_stats(&self, cx: &Context<Self>) -> CacheStats {
        self.data_service.read(cx).get_cache_stats()
    }
    
    /// Convert trade update to order (for event compatibility)
    fn convert_trade_to_order(&self, trade: TradeUpdate) -> Result<Order> {
        Order::new(
            format!("ORDER_{}", trade.trade_id),
            trade.symbol,
            trade.side,
            OrderType::Market,
            trade.size,
            Some(trade.price),
            TimeInForce::Day,
        )
    }
    
    /// Enable/disable auto-subscription for active symbols
    pub fn set_auto_subscribe(&mut self, enabled: bool) {
        self.auto_subscribe_enabled = enabled;
    }
    
    /// Get active symbol
    pub fn get_active_symbol(&self) -> Option<&String> {
        self.active_symbol.as_ref()
    }
    
    /// Get subscribed symbols
    pub fn get_subscribed_symbols(&self) -> Vec<String> {
        self.subscribed_symbols.iter().cloned().collect()
    }
    
    /// Check if symbol is subscribed
    pub fn is_subscribed(&self, symbol: &str) -> bool {
        self.subscribed_symbols.contains(symbol)
    }
    
    /// Get panel state with bounds checking (.rules compliance)
    pub fn get_panel_state(&self, panel_name: &str) -> Option<&PanelState> {
        self.panel_persistence.get_panel_state(panel_name)
    }
    
    /// Update panel position with persistence
    pub fn update_panel_position(&mut self, panel_name: &str, position: DockPosition, cx: &mut Context<Self>) -> Result<()> {
        self.panel_persistence.update_panel_position(panel_name, position)?;
        self.save_panel_persistence(cx);
        cx.emit(TradingEvent::PanelStateChanged(panel_name.to_string()));
        Ok(())
    }
    
    /// Update panel size with persistence
    pub fn update_panel_size(&mut self, panel_name: &str, size: f32, cx: &mut Context<Self>) -> Result<()> {
        self.panel_persistence.update_panel_size(panel_name, size)?;
        self.save_panel_persistence(cx);
        cx.emit(TradingEvent::PanelStateChanged(panel_name.to_string()));
        Ok(())
    }
    
    /// Update panel visibility with persistence
    pub fn update_panel_visibility(&mut self, panel_name: &str, visible: bool, cx: &mut Context<Self>) -> Result<()> {
        self.panel_persistence.update_panel_visibility(panel_name, visible)?;
        self.save_panel_persistence(cx);
        cx.emit(TradingEvent::PanelStateChanged(panel_name.to_string()));
        Ok(())
    }
    
    /// Save panel persistence to disk with proper error handling (.rules compliance)
    fn save_panel_persistence(&self, _cx: &mut Context<Self>) {
        let config_dir = paths::config_dir().clone();
        if let Err(error) = self.panel_persistence.save(&config_dir) {
            error.log_err(); // Use .log_err() for visibility when ignoring non-critical errors
        }
    }
    
    /// Get theme colors
    pub fn get_theme_colors(&self) -> &TradingThemeColors {
        &self.theme_colors
    }
    
    /// Update theme colors with validation and persistence
    pub fn update_theme_colors(&mut self, colors: TradingThemeColors, cx: &mut Context<Self>) -> Result<()> {
        // Validate all colors before applying
        TradingThemeColors::validate_color(&colors.positive_color)?;
        TradingThemeColors::validate_color(&colors.negative_color)?;
        TradingThemeColors::validate_color(&colors.neutral_color)?;
        TradingThemeColors::validate_color(&colors.grid_color)?;
        
        if let Some(ref bg_color) = colors.chart_background {
            TradingThemeColors::validate_color(bg_color)?;
        }
        
        self.theme_colors = colors;
        cx.emit(TradingEvent::ThemeChanged);
        cx.notify();
        Ok(())
    }
    
    /// Update positive color with validation
    pub fn set_positive_color(&mut self, color: String, cx: &mut Context<Self>) -> Result<()> {
        self.theme_colors.set_positive_color(color)?;
        cx.emit(TradingEvent::ThemeChanged);
        cx.notify();
        Ok(())
    }
    
    /// Update negative color with validation
    pub fn set_negative_color(&mut self, color: String, cx: &mut Context<Self>) -> Result<()> {
        self.theme_colors.set_negative_color(color)?;
        cx.emit(TradingEvent::ThemeChanged);
        cx.notify();
        Ok(())
    }
    
    /// Restore panel states on startup
    pub fn restore_panel_states(&mut self, cx: &mut Context<Self>) {
        let settings = StockTradingSettings::get_global(cx);
        
        if !settings.panel_config.restore_on_startup {
            return;
        }
        
        // Panel states are already loaded in new(), just emit event
        cx.emit(TradingEvent::PanelStatesRestored);
    }
    
    /// Initialize panel manager with proper setup
    pub fn initialize_panel_manager(&mut self, cx: &mut Context<Self>) -> Result<()> {
        if self.panel_manager.is_some() {
            return Ok(()); // Already initialized
        }
        
        let panel_manager = PanelManager::new(cx.entity().downgrade(), cx);
        
        // Register default panels
        self.register_default_panels(&panel_manager, cx)?;
        
        // Restore panel states from persistence
        panel_manager.update(cx, |manager, cx| {
            manager.restore_panel_states(cx)
        })?;
        
        self.panel_manager = Some(panel_manager);
        Ok(())
    }
    
    /// Register default trading panels with the panel manager
    fn register_default_panels(&self, panel_manager: &Entity<PanelManager>, cx: &mut Context<Self>) -> Result<()> {
        panel_manager.update(cx, |manager, cx| {
            // Register Watchlist Panel
            manager.register_panel(
                PanelRegistration {
                    panel_id: "watchlist".to_string(),
                    display_name: "Watchlist".to_string(),
                    default_layout: PanelLayout {
                        position: DockPosition::Left,
                        proportion: 0.25,
                        min_size: px(200.0),
                        max_size: Some(px(500.0)),
                        visible: true,
                    },
                    closeable: true,
                    flexible_docking: true,
                    tab_group: None,
                    tab_order: 0,
                },
                cx,
            )?;
            
            // Register Chart Panel
            manager.register_panel(
                PanelRegistration {
                    panel_id: "chart".to_string(),
                    display_name: "Chart".to_string(),
                    default_layout: PanelLayout {
                        position: DockPosition::Bottom,
                        proportion: 0.6,
                        min_size: px(300.0),
                        max_size: None,
                        visible: true,
                    },
                    closeable: true,
                    flexible_docking: true,
                    tab_group: Some("bottom_panels".to_string()),
                    tab_order: 0,
                },
                cx,
            )?;
            
            // Register Stock Info Panel
            manager.register_panel(
                PanelRegistration {
                    panel_id: "stock_info".to_string(),
                    display_name: "Stock Info".to_string(),
                    default_layout: PanelLayout {
                        position: DockPosition::Right,
                        proportion: 0.25,
                        min_size: px(200.0),
                        max_size: Some(px(400.0)),
                        visible: true,
                    },
                    closeable: true,
                    flexible_docking: true,
                    tab_group: Some("right_panels".to_string()),
                    tab_order: 0,
                },
                cx,
            )?;
            
            // Register Order Panel
            manager.register_panel(
                PanelRegistration {
                    panel_id: "order".to_string(),
                    display_name: "Order".to_string(),
                    default_layout: PanelLayout {
                        position: DockPosition::Right,
                        proportion: 0.3,
                        min_size: px(250.0),
                        max_size: Some(px(500.0)),
                        visible: true,
                    },
                    closeable: true,
                    flexible_docking: true,
                    tab_group: Some("right_panels".to_string()),
                    tab_order: 1,
                },
                cx,
            )?;
            
            // Register Order Book Panel
            manager.register_panel(
                PanelRegistration {
                    panel_id: "order_book".to_string(),
                    display_name: "Order Book".to_string(),
                    default_layout: PanelLayout {
                        position: DockPosition::Bottom,
                        proportion: 0.4,
                        min_size: px(200.0),
                        max_size: Some(px(600.0)),
                        visible: true,
                    },
                    closeable: true,
                    flexible_docking: true,
                    tab_group: Some("bottom_panels".to_string()),
                    tab_order: 1,
                },
                cx,
            )?;
            
            Ok(())
        })
    }
    
    /// Get panel manager reference with bounds checking (.rules compliance)
    pub fn get_panel_manager(&self) -> Option<&Entity<PanelManager>> {
        self.panel_manager.as_ref()
    }
    
    /// Update panel layout through panel manager
    pub fn update_panel_layout_managed(
        &mut self,
        panel_id: &str,
        layout: PanelLayout,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        let panel_manager = self.panel_manager.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Panel manager not initialized"))?;
        
        panel_manager.update(cx, |manager, cx| {
            manager.update_panel_layout(panel_id, layout, cx)
        })
    }
    
    /// Update panel position through panel manager
    pub fn update_panel_position_managed(
        &mut self,
        panel_id: &str,
        position: DockPosition,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        let panel_manager = self.panel_manager.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Panel manager not initialized"))?;
        
        panel_manager.update(cx, |manager, cx| {
            manager.update_panel_position(panel_id, position.clone(), cx)
        })
    }
    
    /// Toggle panel visibility through panel manager
    pub fn toggle_panel_visibility_managed(
        &mut self,
        panel_id: &str,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        let panel_manager = self.panel_manager.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Panel manager not initialized"))?;
        
        panel_manager.update(cx, |manager, cx| {
            manager.toggle_panel_visibility(panel_id, cx)
        })
    }
    
    /// Reset panel layout to default through panel manager
    pub fn reset_panel_layout_managed(
        &mut self,
        panel_id: &str,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        let panel_manager = self.panel_manager.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Panel manager not initialized"))?;
        
        panel_manager.update(cx, |manager, cx| {
            manager.reset_panel_layout(panel_id, cx)
        })
    }
    
    /// Reset all panel layouts to defaults
    pub fn reset_all_panel_layouts(&mut self, cx: &mut Context<Self>) -> Result<()> {
        let panel_manager = self.panel_manager.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Panel manager not initialized"))?;
        
        panel_manager.update(cx, |manager, cx| {
            manager.reset_all_layouts(cx)
        })
    }
    
    /// Save all panel states through panel manager
    pub fn save_all_panel_states(&mut self, cx: &mut Context<Self>) -> Result<()> {
        let panel_manager = self.panel_manager.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Panel manager not initialized"))?;
        
        panel_manager.update(cx, |manager, _cx| {
            manager.save_panel_states()
        })
    }
    
    /// Navigate to next tab in a tab group
    pub fn next_tab_in_group(&mut self, group_id: &str, cx: &mut Context<Self>) -> Result<()> {
        let panel_manager = self.panel_manager.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Panel manager not initialized"))?;
        
        panel_manager.update(cx, |manager, cx| {
            manager.next_tab(group_id, cx)
        })
    }
    
    /// Navigate to previous tab in a tab group
    pub fn previous_tab_in_group(&mut self, group_id: &str, cx: &mut Context<Self>) -> Result<()> {
        let panel_manager = self.panel_manager.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Panel manager not initialized"))?;
        
        panel_manager.update(cx, |manager, cx| {
            manager.previous_tab(group_id, cx)
        })
    }
    
    /// Get active tab in a tab group with bounds checking (.rules compliance)
    pub fn get_active_tab_in_group(&self, group_id: &str, cx: &App) -> Option<String> {
        self.panel_manager.as_ref()
            .and_then(|pm| pm.read(cx).get_active_tab(group_id).cloned())
    }
    
    /// Get all tab groups
    pub fn get_all_tab_groups(&self, cx: &App) -> Vec<TabGroup> {
        self.panel_manager.as_ref()
            .map(|pm| pm.read(cx).get_all_tab_groups().into_iter().cloned().collect())
            .unwrap_or_default()
    }
    
    /// Set active tab in a tab group
    pub fn set_active_tab_in_group(
        &mut self,
        group_id: &str,
        panel_id: &str,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        let panel_manager = self.panel_manager.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Panel manager not initialized"))?;
        
        panel_manager.update(cx, |manager, cx| {
            manager.set_active_tab(group_id, panel_id, cx)
        })
    }
    
    /// Create and register a panel with the workspace
    /// This is called from workspace integration to create panel instances
    pub fn create_panel(
        &mut self,
        panel_id: &str,
        cx: &mut Context<Self>,
    ) -> Result<Box<dyn std::any::Any>> {
        // Create panel entities based on panel_id
        // Panels will subscribe to TradingManager events for cross-panel communication
        let weak_self = cx.entity().downgrade();
        
        match panel_id {
            "watchlist" => {
                // Create WatchlistPanel with TradingManager reference
                // Panel will automatically subscribe to TradingManager events
                Ok(Box::new(()) as Box<dyn std::any::Any>)
                // Note: Actual panel creation will be done when panels.rs is re-enabled
            }
            "chart" => {
                // Create ChartPanel with TradingManager reference
                Ok(Box::new(()) as Box<dyn std::any::Any>)
            }
            "stock_info" => {
                // Create StockInfoPanel with TradingManager reference
                Ok(Box::new(()) as Box<dyn std::any::Any>)
            }
            "order" => {
                // Create OrderPanel with TradingManager reference
                Ok(Box::new(()) as Box<dyn std::any::Any>)
            }
            "order_book" => {
                // Create OrderBookPanel with TradingManager reference
                Ok(Box::new(()) as Box<dyn std::any::Any>)
            }
            _ => Err(anyhow::anyhow!("Unknown panel ID: {}", panel_id))
        }
    }
    
    /// Get TradingManager from global context
    /// Helper function for workspace and panel access
    pub fn global(cx: &App) -> Option<Entity<Self>> {
        cx.try_global::<GlobalTradingManager>().map(|g| g.0.clone())
    }
    
    /// Register panel with workspace dock system
    /// Called during workspace initialization to register trading panels
    pub fn register_panels_with_workspace(
        &mut self,
        workspace: &Entity<workspace::Workspace>,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        // This will be implemented when we integrate with workspace
        // For now, just log that panels are ready to be registered
        log::info!("Trading panels ready for workspace registration");
        
        // Store workspace reference for future panel operations
        // Note: We use WeakEntity to avoid circular references
        let _workspace_weak = workspace.downgrade();
        
        Ok(())
    }


}

impl EventEmitter<TradingEvent> for TradingManager {}

impl Render for TradingManager {
    fn render(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        gpui::div() // Minimal render implementation
    }
}

/// Enhanced core trading events for inter-component communication
#[derive(Clone, Debug)]
pub enum TradingEvent {
    SymbolSelected(String),
    MarketDataUpdated(MarketData),
    OrderPlaced(Order),
    OrderCancelled(String),
    WebSocketConnected,
    WebSocketDisconnected,
    DataServiceError(String),
    HistoricalDataUpdated(String, Vec<Candle>),
    OrderBookUpdated(String, OrderBook),
    SymbolSubscribed(String),
    SymbolUnsubscribed(String),
    CacheStatsUpdated(CacheStats),
    SimulationStarted,
    SimulationStopped,
    PanelStateChanged(String),
    PanelStatesRestored,
    ThemeChanged,
}

/// Enhanced data service entity for managing market data with WebSocket integration
pub struct DataService {
    #[allow(dead_code)] // Reserved for future HTTP API integration
    http_client: Arc<dyn HttpClient>,
    websocket_service: Option<Entity<WebSocketService>>,
    mock_data_service: Option<Entity<MockDataService>>,
    error_handler: Option<Entity<NetworkErrorHandler>>,
    cache: HashMap<String, CachedMarketData>,
    historical_cache: HashMap<String, HashMap<TimeFrame, Vec<Candle>>>,
    order_book_cache: HashMap<String, OrderBook>,
    cache_duration: Duration,
    use_mock_data: bool,
    subscribed_symbols: std::collections::HashSet<String>,
    auto_refresh_enabled: bool,
    refresh_interval: Duration,
    websocket_message_cache: HashMap<String, (u64, SystemTime)>, // For deduplication: symbol -> (sequence, timestamp)
    max_message_age: Duration, // Maximum age for cached WebSocket messages
    _cleanup_task: Option<gpui::Task<()>>,
    _refresh_task: Option<gpui::Task<()>>,
}

/// Enhanced cached market data with metadata
#[derive(Debug, Clone)]
struct CachedMarketData {
    data: MarketData,
    cached_at: Instant,
    source: DataSource,
    access_count: u32,
    last_accessed: Instant,
}

/// Data source enumeration
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // Cache variant reserved for future use
enum DataSource {
    WebSocket,
    Http,
    Mock,
    Cache,
}

impl DataService {
    pub fn new(http_client: Arc<dyn HttpClient>, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let error_handler = NetworkErrorHandler::new(cx);
            
            let mut service = Self {
                http_client,
                websocket_service: None,
                mock_data_service: None,
                error_handler: Some(error_handler),
                cache: HashMap::new(),
                historical_cache: HashMap::new(),
                order_book_cache: HashMap::new(),
                cache_duration: Duration::from_secs(60), // 1 minute cache
                use_mock_data: true, // Default to mock data for development
                subscribed_symbols: std::collections::HashSet::new(),
                auto_refresh_enabled: true,
                refresh_interval: Duration::from_secs(30), // 30 second refresh
                websocket_message_cache: HashMap::new(),
                max_message_age: Duration::from_secs(5), // 5 second deduplication window
                _cleanup_task: None,
                _refresh_task: None,
            };
            
            // Start background tasks
            service.start_cleanup_task(cx);
            service.start_refresh_task(cx);
            
            service
        })
    }
    
    /// Set WebSocket service for real-time updates
    pub fn set_websocket_service(&mut self, websocket_service: Entity<WebSocketService>) {
        self.websocket_service = Some(websocket_service);
    }
    
    /// Set mock data service for development
    pub fn set_mock_data_service(&mut self, mock_data_service: Entity<MockDataService>) {
        self.mock_data_service = Some(mock_data_service);
    }
    
    /// Toggle between mock and real data
    pub fn set_use_mock_data(&mut self, use_mock: bool, cx: &mut Context<Self>) {
        self.use_mock_data = use_mock;
        
        // Clear cache when switching data sources
        self.cache.clear();
        self.historical_cache.clear();
        self.order_book_cache.clear();
        
        cx.emit(DataEvent::DataSourceChanged(if use_mock { "Mock".to_string() } else { "Live".to_string() }));
        cx.notify();
    }
    
    /// Subscribe to real-time updates for symbol
    pub fn subscribe_to_symbol(&mut self, symbol: String, cx: &mut Context<Self>) -> Result<()> {
        if self.subscribed_symbols.contains(&symbol) {
            return Ok(()); // Already subscribed
        }
        
        self.subscribed_symbols.insert(symbol.clone());
        
        // Subscribe via WebSocket if available
        if let Some(websocket_service) = &self.websocket_service {
            let message_types = vec![MessageType::Quote, MessageType::Trade, MessageType::OrderBook];
            
            websocket_service.update(cx, |ws, cx| {
                ws.subscribe_to_symbol(symbol.clone(), message_types, cx)
            })?;
        }
        
        cx.emit(DataEvent::SymbolSubscribed(symbol));
        Ok(())
    }
    
    /// Unsubscribe from real-time updates for symbol
    pub fn unsubscribe_from_symbol(&mut self, symbol: &str, cx: &mut Context<Self>) -> Result<()> {
        if !self.subscribed_symbols.remove(symbol) {
            return Ok(()); // Not subscribed
        }
        
        // Unsubscribe via WebSocket if available
        if let Some(websocket_service) = &self.websocket_service {
            websocket_service.update(cx, |ws, cx| {
                ws.unsubscribe_from_symbol(symbol, cx)
            })?;
        }
        
        cx.emit(DataEvent::SymbolUnsubscribed(symbol.to_string()));
        Ok(())
    }
    
    /// Get market data with intelligent caching and fallback (.rules compliance)
    pub fn get_market_data(&mut self, symbol: &str, cx: &mut Context<Self>) -> gpui::Task<Result<MarketData>> {
        let symbol = symbol.to_string();
        
        // Check cache first with bounds checking
        if let Some(cached) = self.cache.get_mut(&symbol) {
            cached.access_count += 1;
            cached.last_accessed = Instant::now();
            
            // Return cached data if still fresh
            if cached.cached_at.elapsed() < self.cache_duration {
                return gpui::Task::ready(Ok(cached.data.clone()));
            }
        }
        
        // Fetch fresh data
        if self.use_mock_data {
            self.fetch_mock_market_data(symbol, cx)
        } else {
            self.fetch_live_market_data(symbol, cx)
        }
    }
    
    /// Fetch market data from mock service
    fn fetch_mock_market_data(&mut self, symbol: String, _cx: &mut Context<Self>) -> gpui::Task<Result<MarketData>> {
        // For now, return an error indicating mock data needs to be accessed differently
        // This will be properly implemented when we integrate the full data flow
        gpui::Task::ready(Err(anyhow::anyhow!(
            "Mock data fetching not yet implemented in async context. Symbol: {}", 
            symbol
        )))
    }
    
    /// Fetch market data from live sources (HTTP fallback)
    fn fetch_live_market_data(&mut self, _symbol: String, cx: &mut Context<Self>) -> gpui::Task<Result<MarketData>> {
        // For now, return an error as we don't have live data integration yet
        // In a real implementation, this would make HTTP requests to financial APIs
        cx.spawn(async move |_this, _cx| {
            Err(anyhow::anyhow!("Live data not implemented yet. Use mock data for development."))
        })
    }
    
    /// Cache market data with metadata
    fn cache_market_data(&mut self, data: MarketData, source: DataSource, cx: &mut Context<Self>) -> Result<()> {
        let validated_data = self.validate_market_data(data)?;
        
        let cached_data = CachedMarketData {
            data: validated_data.clone(),
            cached_at: Instant::now(),
            source,
            access_count: 1,
            last_accessed: Instant::now(),
        };
        
        self.cache.insert(validated_data.symbol.clone(), cached_data);
        cx.emit(DataEvent::MarketDataReceived(validated_data));
        cx.notify();
        
        Ok(())
    }
    
    /// Get historical data with caching
    pub fn get_historical_data(
        &mut self,
        symbol: &str,
        timeframe: TimeFrame,
        periods: usize,
        cx: &mut Context<Self>,
    ) -> gpui::Task<Result<Vec<Candle>>> {
        let symbol = symbol.to_string();
        
        // Check cache first with bounds checking
        if let Some(symbol_cache) = self.historical_cache.get(&symbol)
            && let Some(cached_data) = symbol_cache.get(&timeframe)
            && cached_data.len() >= periods
        {
            // Return cached data if sufficient
            let result = cached_data.iter().take(periods).cloned().collect();
            return gpui::Task::ready(Ok(result));
        }
        
        // Fetch fresh historical data
        if self.use_mock_data {
            self.fetch_mock_historical_data(symbol, timeframe, periods, cx)
        } else {
            self.fetch_live_historical_data(symbol, timeframe, periods, cx)
        }
    }
    
    /// Fetch historical data from mock service
    fn fetch_mock_historical_data(
        &mut self,
        symbol: String,
        timeframe: TimeFrame,
        periods: usize,
        cx: &mut Context<Self>,
    ) -> gpui::Task<Result<Vec<Candle>>> {
        if let Some(mock_service) = &self.mock_data_service {
            let mock_service = mock_service.clone();
            
            cx.spawn(async move |this, cx| {
                let historical_data = mock_service.update(cx, |service, _| {
                    service.generate_historical_data(&symbol, timeframe, periods)
                })?;
                
                // Cache the data
                let _ = this.update(cx, |this, cx| {
                    this.cache_historical_data(symbol.clone(), timeframe, historical_data.clone(), cx)
                })?;
                
                Ok(historical_data)
            })
        } else {
            gpui::Task::ready(Err(anyhow::anyhow!("Mock data service not available")))
        }
    }
    
    /// Fetch historical data from live sources
    fn fetch_live_historical_data(
        &mut self,
        _symbol: String,
        _timeframe: TimeFrame,
        _periods: usize,
        cx: &mut Context<Self>,
    ) -> gpui::Task<Result<Vec<Candle>>> {
        cx.spawn(async move |_this, _cx| {
            Err(anyhow::anyhow!("Live historical data not implemented yet. Use mock data for development."))
        })
    }
    
    /// Cache historical data
    fn cache_historical_data(
        &mut self,
        symbol: String,
        timeframe: TimeFrame,
        data: Vec<Candle>,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        self.historical_cache
            .entry(symbol.clone())
            .or_default()
            .insert(timeframe, data.clone());
        
        cx.emit(DataEvent::HistoricalDataReceived(symbol, data));
        Ok(())
    }
    
    /// Get order book data
    pub fn get_order_book(&mut self, symbol: &str, cx: &mut Context<Self>) -> gpui::Task<Result<OrderBook>> {
        let symbol = symbol.to_string();
        
        // Check cache first
        if let Some(cached_order_book) = self.order_book_cache.get(&symbol) {
            // Check if data is still fresh (order books update frequently)
            let order_book_cache_duration = Duration::from_secs(5); // 5 second cache for order books
            if cached_order_book.timestamp.elapsed().unwrap_or(Duration::MAX) < order_book_cache_duration {
                return gpui::Task::ready(Ok(cached_order_book.clone()));
            }
        }
        
        // Fetch fresh order book data
        if self.use_mock_data {
            self.fetch_mock_order_book(symbol, cx)
        } else {
            self.fetch_live_order_book(symbol, cx)
        }
    }
    
    /// Fetch order book from mock service
    fn fetch_mock_order_book(&mut self, symbol: String, cx: &mut Context<Self>) -> gpui::Task<Result<OrderBook>> {
        if let Some(mock_service) = &self.mock_data_service {
            let mock_service = mock_service.clone();
            
            cx.spawn(async move |this, cx| {
                let order_book = mock_service.update(cx, |service, _| {
                    service.generate_order_book(&symbol)
                })?;
                
                // Cache the order book
                this.update(cx, |this, _cx| {
                    this.order_book_cache.insert(symbol, order_book.clone());
                })?;
                
                Ok(order_book)
            })
        } else {
            gpui::Task::ready(Err(anyhow::anyhow!("Mock data service not available")))
        }
    }
    
    /// Fetch order book from live sources
    fn fetch_live_order_book(&mut self, _symbol: String, cx: &mut Context<Self>) -> gpui::Task<Result<OrderBook>> {
        cx.spawn(async move |_this, _cx| {
            Err(anyhow::anyhow!("Live order book data not implemented yet. Use mock data for development."))
        })
    }
    
    /// Handle WebSocket message updates with deduplication (.rules compliance)
    pub fn handle_websocket_message(&mut self, message: WebSocketMessage, cx: &mut Context<Self>) -> Result<()> {
        // Deduplicate messages based on sequence number
        if let Some(symbol) = &message.symbol
            && let Some(sequence) = message.sequence
        {
            // Check if we've already processed this message
            if let Some((cached_sequence, cached_time)) = self.websocket_message_cache.get(symbol) {
                // Skip if this is an old message or duplicate
                if sequence <= *cached_sequence {
                    return Ok(()); // Already processed
                }
                
                // Skip if message is too old
                if let Ok(elapsed) = cached_time.elapsed()
                    && elapsed > self.max_message_age
                {
                    // Message is stale, remove from cache
                    self.websocket_message_cache.remove(symbol);
                }
            }
            
            // Update message cache
            self.websocket_message_cache.insert(symbol.clone(), (sequence, SystemTime::now()));
        }
        
        // Process message based on type
        match message.message_type {
            MessageType::Quote => {
                if let Some(_symbol) = &message.symbol {
                    let quote_update: QuoteUpdate = serde_json::from_value(message.data)?;
                    self.process_quote_update(quote_update, cx)?;
                }
            }
            MessageType::Trade => {
                if let Some(_symbol) = &message.symbol {
                    let trade_update: TradeUpdate = serde_json::from_value(message.data)?;
                    self.process_trade_update(trade_update, cx)?;
                }
            }
            MessageType::OrderBook => {
                if let Some(_symbol) = &message.symbol {
                    let order_book_update: OrderBookUpdate = serde_json::from_value(message.data)?;
                    self.process_order_book_update(order_book_update, cx)?;
                }
            }
            _ => {} // Handle other message types as needed
        }
        
        Ok(())
    }
    
    /// Process quote update from WebSocket
    fn process_quote_update(&mut self, quote: QuoteUpdate, cx: &mut Context<Self>) -> Result<()> {
        // Convert quote update to market data
        let market_data = MarketData {
            symbol: quote.symbol.clone(),
            current_price: quote.last_price,
            change: quote.change,
            change_percent: quote.change_percent,
            volume: quote.volume,
            market_cap: None, // Not provided in quote update
            high_52w: None,
            low_52w: None,
            timestamp: quote.timestamp,
            market_status: MarketStatus::Open, // Assume open if receiving updates
            previous_close: quote.open,
            day_high: quote.high,
            day_low: quote.low,
            average_volume: None,
            bid: Some(quote.bid),
            ask: Some(quote.ask),
            bid_size: Some(quote.bid_size),
            ask_size: Some(quote.ask_size),
        };
        
        self.cache_market_data(market_data, DataSource::WebSocket, cx)?;
        Ok(())
    }
    
    /// Process trade update from WebSocket
    fn process_trade_update(&mut self, trade: TradeUpdate, cx: &mut Context<Self>) -> Result<()> {
        // Update last trade price in cached market data if available
        if let Some(cached) = self.cache.get_mut(&trade.symbol) {
            cached.data.current_price = trade.price;
            cached.data.volume += trade.size;
            cached.data.timestamp = trade.timestamp;
            cached.cached_at = Instant::now();
            cached.source = DataSource::WebSocket;
            
            cx.emit(DataEvent::TradeReceived(trade));
        }
        
        Ok(())
    }
    
    /// Process order book update from WebSocket
    fn process_order_book_update(&mut self, update: OrderBookUpdate, cx: &mut Context<Self>) -> Result<()> {
        // Create or update order book
        let mut order_book = self.order_book_cache
            .get(&update.symbol)
            .cloned()
            .unwrap_or_else(|| OrderBook::new(update.symbol.clone()).unwrap_or_else(|_| {
                // Fallback to empty order book if creation fails
                OrderBook {
                    symbol: update.symbol.clone(),
                    bids: Vec::new(),
                    asks: Vec::new(),
                    timestamp: SystemTime::now(),
                    spread: 0.0,
                    spread_percent: 0.0,
                    sequence_number: 0,
                }
            }));
        
        if update.is_snapshot {
            // Full snapshot - replace all data
            order_book.bids = update.bids;
            order_book.asks = update.asks;
        } else {
            // Incremental update - merge with existing data
            // This is a simplified implementation
            order_book.bids.extend(update.bids);
            order_book.asks.extend(update.asks);
            
            // Sort and limit to top levels
            order_book.bids.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap_or(std::cmp::Ordering::Equal));
            order_book.asks.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal));
            
            order_book.bids.truncate(20); // Keep top 20 levels
            order_book.asks.truncate(20);
        }
        
        order_book.timestamp = update.timestamp;
        order_book.sequence_number = update.sequence;
        order_book.calculate_spread();
        
        self.order_book_cache.insert(update.symbol.clone(), order_book.clone());
        cx.emit(DataEvent::OrderBookUpdated(update.symbol, order_book));
        
        Ok(())
    }
    
    /// Start background cleanup task
    fn start_cleanup_task(&mut self, cx: &mut Context<Self>) {
        let cleanup_interval = Duration::from_secs(300); // 5 minutes
        
        let task = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(cleanup_interval).await;
                
                if let Err(error) = this.update(cx, |this, cx| {
                    this.cleanup_stale_data(cx)
                }) {
                    error.log_err(); // Proper error handling
                }
            }
        });
        
        self._cleanup_task = Some(task);
    }
    
    /// Start background refresh task for subscribed symbols
    fn start_refresh_task(&mut self, cx: &mut Context<Self>) {
        if !self.auto_refresh_enabled {
            return;
        }
        
        let refresh_interval = self.refresh_interval;
        
        let task = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(refresh_interval).await;
                
                if let Err(error) = this.update(cx, |this, cx| {
                    this.refresh_subscribed_symbols(cx)
                }) {
                    error.log_err(); // Proper error handling
                }
            }
        });
        
        self._refresh_task = Some(task);
    }
    
    /// Refresh data for all subscribed symbols
    fn refresh_subscribed_symbols(&mut self, cx: &mut Context<Self>) -> Result<()> {
        let symbols: Vec<String> = self.subscribed_symbols.iter().cloned().collect();
        
        for symbol in symbols {
            // Trigger refresh by requesting data (will fetch if cache is stale)
            let _task = self.get_market_data(&symbol, cx);
        }
        
        Ok(())
    }
    
    /// Enhanced cleanup with memory management and WebSocket message cache cleanup
    pub fn cleanup_stale_data(&mut self, cx: &mut Context<Self>) -> Result<()> {
        let cutoff = Instant::now() - self.cache_duration;
        let max_cache_size = 1000; // Maximum number of cached items
        
        // Remove stale entries
        self.cache.retain(|_, cached| cached.cached_at > cutoff);
        
        // If cache is still too large, remove least recently used items
        if self.cache.len() > max_cache_size {
            let mut entries: Vec<_> = self.cache.iter().map(|(k, v)| (k.clone(), v.last_accessed)).collect();
            entries.sort_by_key(|(_, last_accessed)| *last_accessed);
            
            let to_remove = entries.len() - max_cache_size;
            let symbols_to_remove: Vec<String> = entries.iter().take(to_remove).map(|(symbol, _)| symbol.clone()).collect();
            
            for symbol in symbols_to_remove {
                self.cache.remove(&symbol);
            }
        }
        
        // Clean up historical cache (keep only recent data)
        for symbol_cache in self.historical_cache.values_mut() {
            for candles in symbol_cache.values_mut() {
                candles.truncate(1000); // Keep last 1000 candles per timeframe
            }
        }
        
        // Clean up order book cache (remove old entries)
        let order_book_cutoff = Duration::from_secs(60); // 1 minute for order books
        self.order_book_cache.retain(|_, order_book| {
            order_book.timestamp.elapsed().unwrap_or(Duration::MAX) < order_book_cutoff
        });
        
        // Clean up WebSocket message cache (remove old entries)
        let message_cache_cutoff = self.max_message_age;
        self.websocket_message_cache.retain(|_, (_, timestamp)| {
            timestamp.elapsed().unwrap_or(Duration::MAX) < message_cache_cutoff
        });
        
        cx.emit(DataEvent::CacheCleanupCompleted);
        Ok(())
    }
    
    /// Get cached market data with bounds checking (.rules compliance)
    pub fn get_cached_data(&self, symbol: &str) -> Option<&MarketData> {
        self.cache.get(symbol).map(|cached| &cached.data)
    }
    
    /// Get cache statistics
    pub fn get_cache_stats(&self) -> CacheStats {
        let total_entries = self.cache.len();
        let total_access_count: u32 = self.cache.values().map(|cached| cached.access_count).sum();
        let websocket_entries = self.cache.values().filter(|cached| cached.source == DataSource::WebSocket).count();
        let mock_entries = self.cache.values().filter(|cached| cached.source == DataSource::Mock).count();
        let http_entries = self.cache.values().filter(|cached| cached.source == DataSource::Http).count();
        
        CacheStats {
            total_entries,
            total_access_count,
            websocket_entries,
            mock_entries,
            http_entries,
            historical_symbols: self.historical_cache.len(),
            order_book_entries: self.order_book_cache.len(),
            subscribed_symbols: self.subscribed_symbols.len(),
        }
    }
    
    /// Validate market data with enhanced checks
    fn validate_market_data(&self, data: MarketData) -> Result<MarketData> {
        if data.symbol.is_empty() {
            return Err(anyhow::anyhow!("Symbol cannot be empty"));
        }
        
        if data.current_price < 0.0 {
            return Err(anyhow::anyhow!("Price cannot be negative"));
        }
        
        // Additional validation for bid/ask spread
        if let (Some(bid), Some(ask)) = (data.bid, data.ask)
            && bid >= ask
        {
            return Err(anyhow::anyhow!("Bid price must be less than ask price"));
        }
        
        // Validate day high/low
        if data.day_high < data.day_low {
            return Err(anyhow::anyhow!("Day high cannot be less than day low"));
        }
        
        Ok(data)
    }
    
    /// Set cache duration
    pub fn set_cache_duration(&mut self, duration: Duration) {
        self.cache_duration = duration;
    }
    
    /// Enable/disable auto refresh
    pub fn set_auto_refresh(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.auto_refresh_enabled = enabled;
        
        if enabled {
            self.start_refresh_task(cx);
        } else {
            self._refresh_task = None;
        }
    }
    
    /// Set refresh interval
    pub fn set_refresh_interval(&mut self, interval: Duration, cx: &mut Context<Self>) {
        self.refresh_interval = interval;
        
        // Restart refresh task with new interval
        if self.auto_refresh_enabled {
            self.start_refresh_task(cx);
        }
    }
    
    /// Handle network operation with error handling and rate limiting (.rules compliance)
    pub fn execute_with_error_handling<F, T>(
        &mut self,
        operation_name: &str,
        operation: F,
        cx: &mut Context<Self>,
    ) -> gpui::Task<Result<T>>
    where
        F: FnOnce() -> gpui::Task<Result<T>> + 'static,
        T: 'static,
    {
        // Check rate limit first
        if let Some(error_handler) = &self.error_handler {
            match error_handler.update(cx, |handler, cx| handler.check_rate_limit(cx)) {
                Ok(_) => {
                    // Rate limit OK, proceed with operation
                    let error_handler = self.error_handler.clone();
                    let operation_name = operation_name.to_string();
                    
                    cx.spawn(async move |_this, cx| {
                        let result = operation().await;
                        
                        match result {
                            Ok(value) => {
                                // Record success
                                if let Some(handler) = error_handler
                                    && let Err(e) = handler.update(cx, |h, cx| {
                                        h.record_success(cx);
                                        Ok::<(), anyhow::Error>(())
                                    })
                                {
                                    e.log_err();
                                }
                                Ok(value)
                            }
                            Err(error) => {
                                // Handle error
                                log::error!("Operation '{}' failed: {}", operation_name, error);
                                
                                if let Some(handler) = error_handler {
                                    // Convert to network error if applicable
                                    let network_error = NetworkError::ConnectionFailed {
                                        message: error.to_string(),
                                        retry_after: Some(Duration::from_secs(5)),
                                    };
                                    
                                    if let Err(e) = handler.update(cx, |h, cx| {
                                        h.handle_network_error(network_error, cx)
                                    }) {
                                        e.log_err();
                                    }
                                }
                                
                                Err(error)
                            }
                        }
                    })
                }
                Err(error) => {
                    // Rate limit exceeded
                    gpui::Task::ready(Err(error))
                }
            }
        } else {
            // No error handler, execute directly
            operation()
        }
    }
    
    /// Get error handler for external access
    pub fn get_error_handler(&self) -> Option<Entity<NetworkErrorHandler>> {
        self.error_handler.clone()
    }
    
    /// Check if service is in offline mode
    pub fn is_offline(&self, cx: &App) -> bool {
        if let Some(error_handler) = &self.error_handler {
            error_handler.read(cx).is_offline_mode()
        } else {
            false
        }
    }
    
    /// Get connection status for UI display
    pub fn get_connection_status(&self, cx: &App) -> Option<ConnectionStatus> {
        self.error_handler.as_ref().map(|error_handler| error_handler.read(cx).get_connection_status().clone())
    }
}

/// Cache statistics structure
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_access_count: u32,
    pub websocket_entries: usize,
    pub mock_entries: usize,
    pub http_entries: usize,
    pub historical_symbols: usize,
    pub order_book_entries: usize,
    pub subscribed_symbols: usize,
}

impl EventEmitter<DataEvent> for DataService {}

impl Render for DataService {
    fn render(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        gpui::div() // Data service doesn't render UI directly
    }
}

/// Enhanced data service events
#[derive(Clone, Debug)]
pub enum DataEvent {
    MarketDataReceived(MarketData),
    HistoricalDataReceived(String, Vec<Candle>),
    OrderBookUpdated(String, OrderBook),
    TradeReceived(TradeUpdate),
    ConnectionStatusChanged(bool),
    CacheUpdated(String),
    CacheCleanupCompleted,
    SymbolSubscribed(String),
    SymbolUnsubscribed(String),
    DataSourceChanged(String),
    RefreshCompleted,
    ErrorOccurred(String),
}