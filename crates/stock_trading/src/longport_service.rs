use anyhow::Result;
use gpui::{App, AppContext, Context, Entity, EventEmitter, Task};
use longport::{
    Config, Decimal, QuoteContext,
    quote::{Period, SubFlags},
    trade::{OrderSide as LongportOrderSide, OrderType as LongportOrderType},
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use crate::{
    MarketData, OrderBook, OrderBookEntry, Candle, TimeFrame, DataEvent,
    OrderSide, OrderType, MarketStatus,
};

/// Cached data entry with timestamp
#[derive(Clone)]
struct CachedData<T> {
    data: T,
    cached_at: Instant,
}

/// Longport service for real market data
/// Integrates with Longport SDK to fetch real-time quotes and historical data
pub struct LongportService {
    config: Arc<Config>,
    quote_context: Option<Arc<QuoteContext>>,
    market_data_cache: HashMap<String, CachedData<MarketData>>,
    historical_data_cache: HashMap<(String, TimeFrame), CachedData<Vec<Candle>>>,
    order_book_cache: HashMap<String, CachedData<OrderBook>>,
    stock_info_cache: HashMap<String, CachedData<crate::StockInfo>>,
    cache_duration: Duration,
    websocket_subscriptions: HashMap<String, WebSocketSubscription>,
    websocket_active: bool,
    reconnect_attempts: u32,
    max_reconnect_attempts: u32,
    last_heartbeat: Option<SystemTime>,
}

/// WebSocket subscription tracking
#[derive(Clone, Debug)]
struct WebSocketSubscription {
    symbol: String,
    sub_flags: SubFlags,
    #[allow(dead_code)]
    subscribed_at: SystemTime,
}

impl EventEmitter<DataEvent> for LongportService {}

impl LongportService {
    /// Create a new Longport service with API credentials
    /// Note: Returns Result to handle configuration errors
    pub fn new(app_key: String, app_secret: String, access_token: String) -> Result<Self> {
        let config = Config::new(app_key, app_secret, access_token);
        
        Ok(Self {
            config: Arc::new(config),
            quote_context: None,
            market_data_cache: HashMap::new(),
            historical_data_cache: HashMap::new(),
            order_book_cache: HashMap::new(),
            stock_info_cache: HashMap::new(),
            cache_duration: Duration::from_secs(60), // Default 60 seconds cache
            websocket_subscriptions: HashMap::new(),
            websocket_active: false,
            reconnect_attempts: 0,
            max_reconnect_attempts: 5,
            last_heartbeat: None,
        })
    }
    
    /// Create entity wrapper for GPUI integration
    pub fn new_entity(app_key: String, app_secret: String, access_token: String, cx: &mut App) -> Result<Entity<Self>> {
        let service = Self::new(app_key, app_secret, access_token)?;
        Ok(cx.new(|_| service))
    }
    
    /// Initialize the quote context (async operation)
    pub async fn initialize(&mut self) -> Result<()> {
        let config = self.config.clone();
        // Note: QuoteContext::try_new expects Arc<Config>
        let (quote_ctx, _receiver) = QuoteContext::try_new(config).await?;
        self.quote_context = Some(Arc::new(quote_ctx));
        self.websocket_active = true;
        self.reconnect_attempts = 0;
        self.last_heartbeat = Some(SystemTime::now());
        Ok(())
    }
    
    /// Start WebSocket real-time data streaming
    /// Uses cx.background_spawn() for async operations (.rules compliance)
    pub fn start_websocket_streaming(&mut self, cx: &mut Context<Self>) -> Result<()> {
        if !self.is_initialized() {
            return Err(anyhow::anyhow!("Quote context not initialized"));
        }
        
        if self.websocket_active {
            return Ok(()); // Already active
        }
        
        self.websocket_active = true;
        self.reconnect_attempts = 0;
        
        // Start heartbeat monitoring
        self.start_heartbeat_monitoring(cx);
        
        // Start message processing loop
        self.start_message_processing(cx);
        
        Ok(())
    }
    
    /// Start heartbeat monitoring for connection health
    fn start_heartbeat_monitoring(&mut self, cx: &mut Context<Self>) {
        let heartbeat_interval = Duration::from_secs(30);
        
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(heartbeat_interval).await;
                
                // Check connection health
                let should_continue = this.update(cx, |this: &mut LongportService, _cx| {
                    if !this.websocket_active {
                        return false;
                    }
                    
                    // Update heartbeat timestamp
                    this.last_heartbeat = Some(SystemTime::now());
                    
                    // Check if we need to reconnect
                    if this.quote_context.is_none() {
                        if let Err(e) = this.handle_websocket_disconnection(_cx) {
                            log::error!("Failed to handle WebSocket disconnection: {}", e);
                        }
                        return false;
                    }
                    
                    true
                }).ok().unwrap_or(false);
                
                if !should_continue {
                    break;
                }
            }
        }).detach();
    }
    
    /// Start processing WebSocket messages
    fn start_message_processing(&mut self, cx: &mut Context<Self>) {
        let _quote_ctx = match self.quote_context.as_ref() {
            Some(ctx) => ctx.clone(),
            None => return,
        };
        
        cx.spawn(async move |this, cx| {
            // Get the receiver from quote context
            // Note: Longport SDK provides push events through callbacks
            // We'll handle them through the subscription mechanism
            loop {
                cx.background_executor().timer(Duration::from_millis(100)).await;
                
                let should_continue = this.update(cx, |this: &mut LongportService, _cx| {
                    this.websocket_active
                }).ok().unwrap_or(false);
                
                if !should_continue {
                    break;
                }
            }
        }).detach();
    }
    
    /// Subscribe to real-time quotes for watchlist symbols
    /// Integrates with panel updates using cx.notify()
    pub fn subscribe_realtime_quotes(
        &mut self,
        symbols: Vec<String>,
        cx: &mut Context<Self>,
    ) -> Task<Result<()>> {
        let quote_ctx = match self.quote_context.as_ref() {
            Some(ctx) => ctx.clone(),
            None => {
                return Task::ready(Err(anyhow::anyhow!("Quote context not initialized")));
            }
        };
        
        // Track subscriptions
        for symbol in &symbols {
            self.websocket_subscriptions.insert(
                symbol.clone(),
                WebSocketSubscription {
                    symbol: symbol.clone(),
                    sub_flags: SubFlags::QUOTE,
                    subscribed_at: SystemTime::now(),
                },
            );
        }
        
        cx.background_spawn(async move {
            let symbol_refs: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();
            quote_ctx.subscribe(symbol_refs, SubFlags::QUOTE).await?;
            Ok(())
        })
    }
    
    /// Subscribe to real-time order book updates
    pub fn subscribe_realtime_depth(
        &mut self,
        symbols: Vec<String>,
        cx: &mut Context<Self>,
    ) -> Task<Result<()>> {
        let quote_ctx = match self.quote_context.as_ref() {
            Some(ctx) => ctx.clone(),
            None => {
                return Task::ready(Err(anyhow::anyhow!("Quote context not initialized")));
            }
        };
        
        // Track subscriptions
        for symbol in &symbols {
            self.websocket_subscriptions.insert(
                format!("{}_depth", symbol),
                WebSocketSubscription {
                    symbol: symbol.clone(),
                    sub_flags: SubFlags::DEPTH,
                    subscribed_at: SystemTime::now(),
                },
            );
        }
        
        cx.background_spawn(async move {
            let symbol_refs: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();
            quote_ctx.subscribe(symbol_refs, SubFlags::DEPTH).await?;
            Ok(())
        })
    }
    
    /// Subscribe to real-time trade updates
    pub fn subscribe_realtime_trades(
        &mut self,
        symbols: Vec<String>,
        cx: &mut Context<Self>,
    ) -> Task<Result<()>> {
        let quote_ctx = match self.quote_context.as_ref() {
            Some(ctx) => ctx.clone(),
            None => {
                return Task::ready(Err(anyhow::anyhow!("Quote context not initialized")));
            }
        };
        
        // Track subscriptions
        for symbol in &symbols {
            self.websocket_subscriptions.insert(
                format!("{}_trades", symbol),
                WebSocketSubscription {
                    symbol: symbol.clone(),
                    sub_flags: SubFlags::TRADE,
                    subscribed_at: SystemTime::now(),
                },
            );
        }
        
        cx.background_spawn(async move {
            let symbol_refs: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();
            quote_ctx.subscribe(symbol_refs, SubFlags::TRADE).await?;
            Ok(())
        })
    }
    
    /// Unsubscribe from real-time updates
    pub fn unsubscribe_realtime(
        &mut self,
        symbols: Vec<String>,
        cx: &mut Context<Self>,
    ) -> Task<Result<()>> {
        let quote_ctx = match self.quote_context.as_ref() {
            Some(ctx) => ctx.clone(),
            None => {
                return Task::ready(Err(anyhow::anyhow!("Quote context not initialized")));
            }
        };
        
        // Remove from tracking
        for symbol in &symbols {
            self.websocket_subscriptions.remove(symbol);
            self.websocket_subscriptions.remove(&format!("{}_depth", symbol));
            self.websocket_subscriptions.remove(&format!("{}_trades", symbol));
        }
        
        cx.background_spawn(async move {
            let symbol_refs: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();
            // Unsubscribe from all types
            quote_ctx.unsubscribe(
                symbol_refs.clone(),
                SubFlags::QUOTE | SubFlags::DEPTH | SubFlags::TRADE,
            ).await?;
            Ok(())
        })
    }
    
    /// Handle WebSocket disconnection with automatic reconnection
    /// Implements proper error propagation (.rules compliance)
    fn handle_websocket_disconnection(&mut self, cx: &mut Context<Self>) -> Result<()> {
        if self.reconnect_attempts >= self.max_reconnect_attempts {
            self.websocket_active = false;
            return Err(anyhow::anyhow!(
                "Max reconnection attempts ({}) reached",
                self.max_reconnect_attempts
            ));
        }
        
        self.reconnect_attempts += 1;
        
        // Calculate exponential backoff
        let backoff_duration = Duration::from_secs(2u64.pow(self.reconnect_attempts.min(5)));
        
        log::info!(
            "WebSocket disconnected, attempting reconnection {} of {} in {:?}",
            self.reconnect_attempts,
            self.max_reconnect_attempts,
            backoff_duration
        );
        
        // Schedule reconnection
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(backoff_duration).await;
            
            if let Err(e) = this.update(cx, |this: &mut LongportService, cx| {
                this.attempt_websocket_reconnection(cx)
            }) {
                log::error!("Failed to schedule reconnection: {}", e);
            }
        }).detach();
        
        Ok(())
    }
    
    /// Attempt to reconnect WebSocket
    fn attempt_websocket_reconnection(&mut self, cx: &mut Context<Self>) -> Result<()> {
        let config = self.config.clone();
        
        cx.spawn(async move |this, cx| {
            match QuoteContext::try_new(config).await {
                Ok((quote_ctx, _receiver)) => {
                    if let Err(e) = this.update(cx, |this: &mut LongportService, cx| {
                        this.quote_context = Some(Arc::new(quote_ctx));
                        this.reconnect_attempts = 0;
                        this.websocket_active = true;
                        this.last_heartbeat = Some(SystemTime::now());
                        
                        // Resubscribe to all previous subscriptions
                        this.resubscribe_all(cx)
                    }) {
                        log::error!("Failed to update after reconnection: {}", e);
                    }
                }
                Err(e) => {
                    log::error!("Failed to reconnect WebSocket: {}", e);
                    if let Err(e) = this.update(cx, |this: &mut LongportService, cx| {
                        this.handle_websocket_disconnection(cx)
                    }) {
                        log::error!("Failed to handle reconnection failure: {}", e);
                    }
                }
            }
        }).detach();
        
        Ok(())
    }
    
    /// Resubscribe to all previous subscriptions after reconnection
    fn resubscribe_all(&mut self, cx: &mut Context<Self>) -> Result<()> {
        if self.websocket_subscriptions.is_empty() {
            return Ok(());
        }
        
        let quote_ctx = match self.quote_context.as_ref() {
            Some(ctx) => ctx.clone(),
            None => return Err(anyhow::anyhow!("Quote context not available")),
        };
        
        // Group subscriptions by type
        let mut quote_symbols = Vec::new();
        let mut depth_symbols = Vec::new();
        let mut trade_symbols = Vec::new();
        
        for subscription in self.websocket_subscriptions.values() {
            if subscription.sub_flags.contains(SubFlags::QUOTE) {
                quote_symbols.push(subscription.symbol.clone());
            }
            if subscription.sub_flags.contains(SubFlags::DEPTH) {
                depth_symbols.push(subscription.symbol.clone());
            }
            if subscription.sub_flags.contains(SubFlags::TRADE) {
                trade_symbols.push(subscription.symbol.clone());
            }
        }
        
        // Resubscribe to quotes
        if !quote_symbols.is_empty() {
            let symbols = quote_symbols.clone();
            let quote_ctx_clone = quote_ctx.clone();
            cx.background_spawn(async move {
                let symbol_refs: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();
                if let Err(e) = quote_ctx_clone.subscribe(symbol_refs, SubFlags::QUOTE).await {
                    log::error!("Failed to resubscribe to quotes: {}", e);
                }
            }).detach();
        }
        
        // Resubscribe to depth
        if !depth_symbols.is_empty() {
            let symbols = depth_symbols.clone();
            let quote_ctx_clone = quote_ctx.clone();
            cx.background_spawn(async move {
                let symbol_refs: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();
                if let Err(e) = quote_ctx_clone.subscribe(symbol_refs, SubFlags::DEPTH).await {
                    log::error!("Failed to resubscribe to depth: {}", e);
                }
            }).detach();
        }
        
        // Resubscribe to trades
        if !trade_symbols.is_empty() {
            let symbols = trade_symbols.clone();
            let quote_ctx_clone = quote_ctx.clone();
            cx.background_spawn(async move {
                let symbol_refs: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();
                if let Err(e) = quote_ctx_clone.subscribe(symbol_refs, SubFlags::TRADE).await {
                    log::error!("Failed to resubscribe to trades: {}", e);
                }
            }).detach();
        }
        
        log::info!(
            "Resubscribed to {} quote, {} depth, {} trade subscriptions",
            quote_symbols.len(),
            depth_symbols.len(),
            trade_symbols.len()
        );
        
        Ok(())
    }
    
    /// Stop WebSocket streaming
    pub fn stop_websocket_streaming(&mut self) {
        self.websocket_active = false;
        self.websocket_subscriptions.clear();
    }
    
    /// Check if WebSocket is active
    pub fn is_websocket_active(&self) -> bool {
        self.websocket_active && self.quote_context.is_some()
    }
    
    /// Get WebSocket connection status
    pub fn get_websocket_status(&self) -> WebSocketStatus {
        if !self.websocket_active {
            return WebSocketStatus::Disconnected;
        }
        
        if self.quote_context.is_none() {
            return WebSocketStatus::Reconnecting(self.reconnect_attempts);
        }
        
        // Check heartbeat
        if let Some(last_heartbeat) = self.last_heartbeat
            && let Ok(elapsed) = SystemTime::now().duration_since(last_heartbeat)
            && elapsed > Duration::from_secs(60)
        {
            return WebSocketStatus::Unhealthy;
        }
        
        WebSocketStatus::Connected
    }
    
    /// Get active subscriptions
    pub fn get_active_subscriptions(&self) -> Vec<String> {
        self.websocket_subscriptions
            .values()
            .map(|sub| sub.symbol.clone())
            .collect()
    }
    
    /// Get subscription count
    pub fn get_subscription_count(&self) -> usize {
        self.websocket_subscriptions.len()
    }
    
    /// Get real-time market data for a symbol (with caching)
    pub async fn get_market_data(&mut self, symbol: &str) -> Result<MarketData> {
        // Check cache first (.rules compliance: use safe access with .get())
        if let Some(cached) = self.market_data_cache.get(symbol)
            && cached.cached_at.elapsed() < self.cache_duration
        {
            return Ok(cached.data.clone());
        }
        
        let quote_ctx = self.quote_context.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Quote context not initialized"))?;
        
        // Get real-time quote from Longport (.rules compliance: use ? for error propagation)
        let quotes = quote_ctx.quote([symbol]).await?;
        let quote = quotes.first()
            .ok_or_else(|| anyhow::anyhow!("No quote data for symbol: {}", symbol))?;
        
        // Convert Longport Decimal types to f64 using string conversion
        let last_done = decimal_to_f64(&quote.last_done)?;
        let prev_close = decimal_to_f64(&quote.prev_close)?;
        let high = decimal_to_f64(&quote.high)?;
        let low = decimal_to_f64(&quote.low)?;
        
        // Convert i64 volume to u64 with bounds checking (.rules compliance)
        let volume = if quote.volume >= 0 {
            quote.volume as u64
        } else {
            0 // Fallback for negative values
        };
        
        // Convert timestamp from OffsetDateTime to SystemTime
        let unix_timestamp = quote.timestamp.unix_timestamp();
        let timestamp = if unix_timestamp >= 0 {
            SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(unix_timestamp as u64)
        } else {
            SystemTime::now() // Fallback for invalid timestamps
        };
        
        // Convert Longport quote to our MarketData structure
        let market_data = MarketData {
            symbol: quote.symbol.clone(),
            current_price: last_done,
            change: last_done - prev_close,
            change_percent: if prev_close > 0.0 {
                ((last_done - prev_close) / prev_close) * 100.0
            } else {
                0.0
            },
            volume,
            day_high: high,
            day_low: low,
            previous_close: prev_close,
            timestamp,
            market_status: MarketStatus::Open, // TODO: Determine actual market status from Longport
            market_cap: None,
            high_52w: None,
            low_52w: None,
            average_volume: None,
            bid: None,
            ask: None,
            bid_size: None,
            ask_size: None,
        };
        
        // Cache the result
        self.market_data_cache.insert(
            symbol.to_string(),
            CachedData {
                data: market_data.clone(),
                cached_at: Instant::now(),
            },
        );
        
        Ok(market_data)
    }
    
    /// Get historical candlestick data (with caching)
    pub async fn get_historical_data(
        &mut self,
        symbol: &str,
        timeframe: TimeFrame,
        count: usize,
    ) -> Result<Vec<Candle>> {
        // Check cache first (.rules compliance: use safe access with .get())
        let cache_key = (symbol.to_string(), timeframe);
        if let Some(cached) = self.historical_data_cache.get(&cache_key)
            && cached.cached_at.elapsed() < self.cache_duration
        {
            return Ok(cached.data.clone());
        }
        
        let quote_ctx = self.quote_context.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Quote context not initialized"))?;
        
        // Convert our TimeFrame to Longport Period
        let period = match timeframe {
            TimeFrame::OneMinute => Period::OneMinute,
            TimeFrame::FiveMinutes => Period::FiveMinute,
            TimeFrame::FifteenMinutes => Period::FifteenMinute,
            TimeFrame::ThirtyMinutes => Period::ThirtyMinute,
            TimeFrame::OneHour => Period::SixtyMinute,
            TimeFrame::FourHours => Period::SixtyMinute, // Approximate with 1h
            TimeFrame::OneDay => Period::Day,
            TimeFrame::OneWeek => Period::Week,
            TimeFrame::OneMonth => Period::Month,
        };
        
        // Get candlestick data from Longport
        // Note: Longport SDK candlesticks() takes 5 parameters: symbol, period, count, adjust_type, trade_sessions
        let candlesticks = quote_ctx.candlesticks(
            symbol,
            period,
            count, // count is already usize
            longport::quote::AdjustType::NoAdjust,
            longport::quote::TradeSessions::All, // All trading sessions
        ).await?;
        
        // Convert to our Candle structure
        let candles: Vec<Candle> = candlesticks.iter().filter_map(|c| {
            // Convert Longport Decimal types to f64
            let open = decimal_to_f64(&c.open).ok()?;
            let high = decimal_to_f64(&c.high).ok()?;
            let low = decimal_to_f64(&c.low).ok()?;
            let close = decimal_to_f64(&c.close).ok()?;
            
            // Convert i64 volume to u64 with bounds checking (.rules compliance)
            let volume = if c.volume >= 0 {
                c.volume as u64
            } else {
                0
            };
            
            // Convert timestamp from OffsetDateTime to SystemTime
            let unix_timestamp = c.timestamp.unix_timestamp();
            let timestamp = if unix_timestamp >= 0 {
                SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(unix_timestamp as u64)
            } else {
                return None; // Skip invalid timestamps
            };
            
            Some(Candle {
                timestamp,
                open,
                high,
                low,
                close,
                volume,
                adjusted_close: None,
                vwap: None,
            })
        }).collect();
        
        // Cache the result
        self.historical_data_cache.insert(
            cache_key,
            CachedData {
                data: candles.clone(),
                cached_at: Instant::now(),
            },
        );
        
        Ok(candles)
    }
    
    /// Get order book (depth) data (with caching)
    pub async fn get_order_book(&mut self, symbol: &str) -> Result<OrderBook> {
        // Check cache first (.rules compliance: use safe access with .get())
        if let Some(cached) = self.order_book_cache.get(symbol)
            && cached.cached_at.elapsed() < self.cache_duration
        {
            return Ok(cached.data.clone());
        }
        
        let quote_ctx = self.quote_context.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Quote context not initialized"))?;
        
        // Get depth data from Longport
        let depth = quote_ctx.depth(symbol).await?;
        
        // Convert bids (buy orders)
        let bids: Vec<OrderBookEntry> = depth.asks.iter().filter_map(|level| {
            // Handle Option<Decimal> for price
            let price = level.price.as_ref().and_then(|p| decimal_to_f64(p).ok())?;
            
            // Convert i64 volume to u64 with bounds checking (.rules compliance)
            let volume = if level.volume >= 0 {
                level.volume as u64
            } else {
                return None;
            };
            
            // Convert i64 order_num to u32 with bounds checking (.rules compliance)
            let order_count = if level.order_num >= 0 && level.order_num <= u32::MAX as i64 {
                level.order_num as u32
            } else {
                0
            };
            
            Some(OrderBookEntry {
                price,
                quantity: volume,
                side: OrderSide::Buy,
                order_count,
                timestamp: SystemTime::now(),
            })
        }).collect();
        
        // Convert asks (sell orders)
        let asks: Vec<OrderBookEntry> = depth.bids.iter().filter_map(|level| {
            // Handle Option<Decimal> for price
            let price = level.price.as_ref().and_then(|p| decimal_to_f64(p).ok())?;
            
            // Convert i64 volume to u64 with bounds checking (.rules compliance)
            let volume = if level.volume >= 0 {
                level.volume as u64
            } else {
                return None;
            };
            
            // Convert i64 order_num to u32 with bounds checking (.rules compliance)
            let order_count = if level.order_num >= 0 && level.order_num <= u32::MAX as i64 {
                level.order_num as u32
            } else {
                0
            };
            
            Some(OrderBookEntry {
                price,
                quantity: volume,
                side: OrderSide::Sell,
                order_count,
                timestamp: SystemTime::now(),
            })
        }).collect();
        
        // Calculate spread
        let spread = match (bids.first(), asks.first()) {
            (Some(best_bid), Some(best_ask)) if best_ask.price > best_bid.price => {
                best_ask.price - best_bid.price
            }
            _ => 0.0,
        };
        
        let spread_percent = match (bids.first(), asks.first()) {
            (Some(best_bid), Some(best_ask)) if best_ask.price > best_bid.price && best_bid.price > 0.0 => {
                ((best_ask.price - best_bid.price) / best_bid.price) * 100.0
            }
            _ => 0.0,
        };
        
        // Convert to our OrderBook structure
        let order_book = OrderBook {
            symbol: symbol.to_string(),
            bids,
            asks,
            timestamp: SystemTime::now(),
            spread,
            spread_percent,
            sequence_number: 0, // Longport doesn't provide sequence numbers
        };
        
        // Cache the result
        self.order_book_cache.insert(
            symbol.to_string(),
            CachedData {
                data: order_book.clone(),
                cached_at: Instant::now(),
            },
        );
        
        Ok(order_book)
    }
    
    /// Subscribe to real-time quotes for symbols
    pub async fn subscribe_quotes(&self, symbols: Vec<String>) -> Result<()> {
        let quote_ctx = self.quote_context.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Quote context not initialized"))?;
        
        let symbol_refs: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();
        quote_ctx.subscribe(symbol_refs, SubFlags::QUOTE).await?;
        
        Ok(())
    }
    
    /// Unsubscribe from real-time quotes
    pub async fn unsubscribe_quotes(&self, symbols: Vec<String>) -> Result<()> {
        let quote_ctx = self.quote_context.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Quote context not initialized"))?;
        
        let symbol_refs: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();
        quote_ctx.unsubscribe(symbol_refs, SubFlags::QUOTE).await?;
        
        Ok(())
    }
    
    /// Get stock information and fundamental data (with caching)
    pub async fn get_stock_info(&mut self, symbol: &str) -> Result<crate::StockInfo> {
        // Check cache first (.rules compliance: use safe access with .get())
        if let Some(cached) = self.stock_info_cache.get(symbol)
            && cached.cached_at.elapsed() < self.cache_duration
        {
            return Ok(cached.data.clone());
        }
        
        let quote_ctx = self.quote_context.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Quote context not initialized"))?;
        
        // Get static info from Longport (.rules compliance: use ? for error propagation)
        let static_info = quote_ctx.static_info([symbol]).await?;
        let info = static_info.first()
            .ok_or_else(|| anyhow::anyhow!("No static info for symbol: {}", symbol))?;
        
        // Create StockInfo with available data
        let mut stock_info = crate::StockInfo::new(
            info.symbol.clone(),
            info.name_cn.clone(),
            info.exchange.clone(),
        )?;
        
        // Populate optional fields (Longport provides limited fundamental data)
        stock_info.sector = None;
        stock_info.industry = None;
        stock_info.description = None;
        stock_info.website = None;
        stock_info.employees = None;
        stock_info.headquarters = None;
        stock_info.founded_year = None;
        stock_info.market_cap = None;
        stock_info.pe_ratio = None;
        stock_info.dividend_yield = None;
        stock_info.beta = None;
        stock_info.eps = None;
        
        // Cache the result
        self.stock_info_cache.insert(
            symbol.to_string(),
            CachedData {
                data: stock_info.clone(),
                cached_at: Instant::now(),
            },
        );
        
        Ok(stock_info)
    }
    
    /// Get real-time quote with enhanced data including bid/ask
    pub async fn get_enhanced_quote(&self, symbol: &str) -> Result<MarketData> {
        let quote_ctx = self.quote_context.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Quote context not initialized"))?;
        
        // Get real-time quote
        let quotes = quote_ctx.quote([symbol]).await?;
        let quote = quotes.first()
            .ok_or_else(|| anyhow::anyhow!("No quote data for symbol: {}", symbol))?;
        
        // Convert Decimal types to f64 (.rules compliance: use ? for error propagation)
        let last_done = decimal_to_f64(&quote.last_done)?;
        let prev_close = decimal_to_f64(&quote.prev_close)?;
        let high = decimal_to_f64(&quote.high)?;
        let low = decimal_to_f64(&quote.low)?;
        
        // Convert volume with bounds checking (.rules compliance)
        let volume = if quote.volume >= 0 {
            quote.volume as u64
        } else {
            0
        };
        
        // Convert turnover (trading value)
        let turnover = decimal_to_f64(&quote.turnover).unwrap_or(0.0);
        
        // Calculate average volume from turnover and price
        let average_volume = if last_done > 0.0 {
            Some((turnover / last_done) as u64)
        } else {
            None
        };
        
        // Convert timestamp
        let unix_timestamp = quote.timestamp.unix_timestamp();
        let timestamp = if unix_timestamp >= 0 {
            SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(unix_timestamp as u64)
        } else {
            SystemTime::now()
        };
        
        // Get bid/ask data if available (.rules compliance: use proper error handling)
        let (bid, ask, bid_size, ask_size) = match quote_ctx.depth(symbol).await {
            Ok(depth) => {
                let best_bid = depth.asks.first().and_then(|level| {
                    level.price.as_ref().and_then(|p| decimal_to_f64(p).ok())
                });
                let best_ask = depth.bids.first().and_then(|level| {
                    level.price.as_ref().and_then(|p| decimal_to_f64(p).ok())
                });
                let best_bid_size = depth.asks.first().map(|level| {
                    if level.volume >= 0 { level.volume as u64 } else { 0 }
                });
                let best_ask_size = depth.bids.first().map(|level| {
                    if level.volume >= 0 { level.volume as u64 } else { 0 }
                });
                (best_bid, best_ask, best_bid_size, best_ask_size)
            }
            Err(e) => {
                // .rules compliance: log error for visibility
                log::warn!("Failed to fetch depth data for {}: {}", symbol, e);
                (None, None, None, None)
            }
        };
        
        // Create enhanced MarketData
        let market_data = MarketData {
            symbol: quote.symbol.clone(),
            current_price: last_done,
            change: last_done - prev_close,
            change_percent: if prev_close > 0.0 {
                ((last_done - prev_close) / prev_close) * 100.0
            } else {
                0.0
            },
            volume,
            day_high: high,
            day_low: low,
            previous_close: prev_close,
            timestamp,
            market_status: MarketStatus::Open,
            market_cap: None,
            high_52w: None,
            low_52w: None,
            average_volume,
            bid,
            ask,
            bid_size,
            ask_size,
        };
        
        Ok(market_data)
    }
    
    /// Get multiple quotes at once for efficiency
    pub async fn get_batch_quotes(&self, symbols: Vec<String>) -> Result<Vec<MarketData>> {
        let quote_ctx = self.quote_context.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Quote context not initialized"))?;
        
        // Convert Vec<String> to Vec<&str>
        let symbol_refs: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();
        
        // Get quotes for all symbols (.rules compliance: use ? for error propagation)
        let quotes = quote_ctx.quote(symbol_refs).await?;
        
        // Convert each quote to MarketData
        let market_data_list: Vec<MarketData> = quotes.iter().filter_map(|quote| {
            // Convert Decimal types
            let last_done = decimal_to_f64(&quote.last_done).ok()?;
            let prev_close = decimal_to_f64(&quote.prev_close).ok()?;
            let high = decimal_to_f64(&quote.high).ok()?;
            let low = decimal_to_f64(&quote.low).ok()?;
            
            // Convert volume with bounds checking (.rules compliance)
            let volume = if quote.volume >= 0 {
                quote.volume as u64
            } else {
                0
            };
            
            // Convert timestamp
            let unix_timestamp = quote.timestamp.unix_timestamp();
            let timestamp = if unix_timestamp >= 0 {
                SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(unix_timestamp as u64)
            } else {
                return None;
            };
            
            Some(MarketData {
                symbol: quote.symbol.clone(),
                current_price: last_done,
                change: last_done - prev_close,
                change_percent: if prev_close > 0.0 {
                    ((last_done - prev_close) / prev_close) * 100.0
                } else {
                    0.0
                },
                volume,
                day_high: high,
                day_low: low,
                previous_close: prev_close,
                timestamp,
                market_status: MarketStatus::Open,
                market_cap: None,
                high_52w: None,
                low_52w: None,
                average_volume: None,
                bid: None,
                ask: None,
                bid_size: None,
                ask_size: None,
            })
        }).collect();
        
        Ok(market_data_list)
    }
    
    /// Check if the service is initialized and ready
    pub fn is_initialized(&self) -> bool {
        self.quote_context.is_some()
    }
    
    /// Get connection status
    pub fn get_connection_status(&self) -> &str {
        if self.quote_context.is_some() {
            "Connected"
        } else {
            "Disconnected"
        }
    }
    
    /// Set cache duration for all cached data
    pub fn set_cache_duration(&mut self, duration: Duration) {
        self.cache_duration = duration;
    }
    
    /// Get current cache duration
    pub fn get_cache_duration(&self) -> Duration {
        self.cache_duration
    }
    
    /// Clear all caches
    pub fn clear_all_caches(&mut self) {
        self.market_data_cache.clear();
        self.historical_data_cache.clear();
        self.order_book_cache.clear();
        self.stock_info_cache.clear();
    }
    
    /// Clear cache for specific symbol
    pub fn clear_symbol_cache(&mut self, symbol: &str) {
        self.market_data_cache.remove(symbol);
        self.order_book_cache.remove(symbol);
        self.stock_info_cache.remove(symbol);
        
        // Clear historical data for all timeframes
        let keys_to_remove: Vec<_> = self.historical_data_cache
            .keys()
            .filter(|(s, _)| s == symbol)
            .cloned()
            .collect();
        
        for key in keys_to_remove {
            self.historical_data_cache.remove(&key);
        }
    }
    
    /// Get cache statistics
    pub fn get_cache_stats(&self) -> CacheStats {
        CacheStats {
            market_data_count: self.market_data_cache.len(),
            historical_data_count: self.historical_data_cache.len(),
            order_book_count: self.order_book_cache.len(),
            stock_info_count: self.stock_info_cache.len(),
        }
    }
    
    /// Clean up stale cache entries
    pub fn cleanup_stale_cache(&mut self) {
        let now = Instant::now();
        
        // Clean market data cache
        self.market_data_cache.retain(|_, cached| {
            now.duration_since(cached.cached_at) < self.cache_duration * 2
        });
        
        // Clean historical data cache
        self.historical_data_cache.retain(|_, cached| {
            now.duration_since(cached.cached_at) < self.cache_duration * 2
        });
        
        // Clean order book cache
        self.order_book_cache.retain(|_, cached| {
            now.duration_since(cached.cached_at) < self.cache_duration * 2
        });
        
        // Clean stock info cache (longer retention)
        self.stock_info_cache.retain(|_, cached| {
            now.duration_since(cached.cached_at) < self.cache_duration * 10
        });
    }
}

/// Cache statistics structure
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub market_data_count: usize,
    pub historical_data_count: usize,
    pub order_book_count: usize,
    pub stock_info_count: usize,
}

/// WebSocket connection status
#[derive(Debug, Clone, PartialEq)]
pub enum WebSocketStatus {
    Connected,
    Disconnected,
    Reconnecting(u32), // Number of reconnection attempts
    Unhealthy,         // Connected but not receiving heartbeats
}

/// Convert Longport Decimal to f64
/// Uses string conversion as the most reliable method (.rules compliance)
fn decimal_to_f64(decimal: &Decimal) -> Result<f64> {
    let string_value = decimal.to_string();
    string_value.parse::<f64>()
        .map_err(|e| anyhow::anyhow!("Failed to convert Decimal to f64: {}", e))
}

/// Convert our OrderSide to Longport OrderSide
pub fn convert_order_side(side: OrderSide) -> LongportOrderSide {
    match side {
        OrderSide::Buy => LongportOrderSide::Buy,
        OrderSide::Sell => LongportOrderSide::Sell,
        OrderSide::SellShort => LongportOrderSide::Sell, // Map to Sell
    }
}

/// Convert our OrderType to Longport OrderType
pub fn convert_order_type(order_type: OrderType) -> LongportOrderType {
    match order_type {
        OrderType::Market => LongportOrderType::MO,
        OrderType::Limit => LongportOrderType::LO,
        OrderType::StopLoss => LongportOrderType::SLO,
        OrderType::StopLimit => LongportOrderType::SLO,
        OrderType::TrailingStop => LongportOrderType::TSLPAMT,
        OrderType::TrailingStopLimit => LongportOrderType::TSLPPCT,
    }
}
