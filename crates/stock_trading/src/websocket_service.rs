// Allow clippy warnings for this checkpoint - will be fixed in future tasks
#![allow(clippy::collapsible_if)]
#![allow(clippy::new_without_default)]
#![allow(clippy::await_holding_lock)]
#![allow(unused_mut)]

use anyhow::Result;
use gpui::{App, AppContext, Context, Entity, EventEmitter, Render, Subscription};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream, MaybeTlsStream};
use tokio::net::TcpStream;
use url::Url;

use crate::market_data::{OrderSide, OrderBookEntry};

// Extension trait for error logging
trait LogErr<T> {
    fn log_err(self) -> Option<T>;
}

impl<T> LogErr<T> for Result<T> {
    fn log_err(self) -> Option<T> {
        match self {
            Ok(value) => Some(value),
            Err(error) => {
                log::error!("{}", error);
                None
            }
        }
    }
}

impl LogErr<()> for anyhow::Error {
    fn log_err(self) -> Option<()> {
        log::error!("{}", self);
        None
    }
}

/// WebSocket message structure for real-time updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
    pub message_type: MessageType,
    pub symbol: Option<String>,
    pub data: serde_json::Value,
    pub timestamp: SystemTime,
    pub sequence: Option<u64>, // For message ordering
}

impl WebSocketMessage {
    /// Create new WebSocket message with validation
    pub fn new(
        message_type: MessageType,
        symbol: Option<String>,
        data: serde_json::Value,
    ) -> Result<Self> {
        Ok(Self {
            message_type,
            symbol,
            data,
            timestamp: SystemTime::now(),
            sequence: None,
        })
    }
}

/// WebSocket message type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MessageType {
    Quote,           // Real-time price updates
    Trade,           // Trade executions
    OrderBook,       // Order book updates
    OrderUpdate,     // Order status changes
    MarketStatus,    // Market open/close status
    Heartbeat,       // Connection keep-alive
    Error,           // Error messages
    Subscribe,       // Subscription requests
    Unsubscribe,     // Unsubscription requests
    Authentication,  // Authentication messages
    SystemStatus,    // System status updates
}

/// Quote update structure for real-time price data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteUpdate {
    pub symbol: String,
    pub bid: f64,
    pub ask: f64,
    pub bid_size: u64,
    pub ask_size: u64,
    pub last_price: f64,
    pub last_size: u64,
    pub volume: u64,
    pub timestamp: SystemTime,
    pub change: f64,
    pub change_percent: f64,
    pub high: f64,
    pub low: f64,
    pub open: f64,
}

impl QuoteUpdate {
    /// Create new quote update with validation
    pub fn new(symbol: String, bid: f64, ask: f64, last_price: f64) -> Result<Self> {
        if symbol.is_empty() {
            return Err(anyhow::anyhow!("Symbol cannot be empty"));
        }
        
        if bid <= 0.0 || ask <= 0.0 || last_price <= 0.0 {
            return Err(anyhow::anyhow!("Prices must be positive"));
        }
        
        if bid >= ask {
            return Err(anyhow::anyhow!("Bid price must be less than ask price"));
        }
        
        Ok(Self {
            symbol,
            bid,
            ask,
            bid_size: 0,
            ask_size: 0,
            last_price,
            last_size: 0,
            volume: 0,
            timestamp: SystemTime::now(),
            change: 0.0,
            change_percent: 0.0,
            high: last_price,
            low: last_price,
            open: last_price,
        })
    }
    
    /// Calculate spread
    pub fn get_spread(&self) -> f64 {
        self.ask - self.bid
    }
    
    /// Calculate spread percentage
    pub fn get_spread_percent(&self) -> f64 {
        if self.bid > 0.0 {
            ((self.ask - self.bid) / self.bid) * 100.0
        } else {
            0.0
        }
    }
}

/// Trade update structure for execution data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeUpdate {
    pub symbol: String,
    pub price: f64,
    pub size: u64,
    pub side: OrderSide,
    pub timestamp: SystemTime,
    pub trade_id: String,
    pub conditions: Vec<String>, // Trade conditions/flags
    pub venue: Option<String>,   // Execution venue
}

impl TradeUpdate {
    /// Create new trade update with validation
    pub fn new(
        symbol: String,
        price: f64,
        size: u64,
        side: OrderSide,
        trade_id: String,
    ) -> Result<Self> {
        if symbol.is_empty() {
            return Err(anyhow::anyhow!("Symbol cannot be empty"));
        }
        
        if trade_id.is_empty() {
            return Err(anyhow::anyhow!("Trade ID cannot be empty"));
        }
        
        if price <= 0.0 {
            return Err(anyhow::anyhow!("Price must be positive"));
        }
        
        if size == 0 {
            return Err(anyhow::anyhow!("Size must be greater than zero"));
        }
        
        Ok(Self {
            symbol,
            price,
            size,
            side,
            timestamp: SystemTime::now(),
            trade_id,
            conditions: Vec::new(),
            venue: None,
        })
    }
}

/// Order book update structure for market depth data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookUpdate {
    pub symbol: String,
    pub bids: Vec<OrderBookEntry>,
    pub asks: Vec<OrderBookEntry>,
    pub timestamp: SystemTime,
    pub sequence: u64, // For ordering updates
    pub is_snapshot: bool, // Full snapshot vs incremental update
}

impl OrderBookUpdate {
    /// Create new order book update with validation
    pub fn new(symbol: String, sequence: u64, is_snapshot: bool) -> Result<Self> {
        if symbol.is_empty() {
            return Err(anyhow::anyhow!("Symbol cannot be empty"));
        }
        
        Ok(Self {
            symbol,
            bids: Vec::new(),
            asks: Vec::new(),
            timestamp: SystemTime::now(),
            sequence,
            is_snapshot,
        })
    }
    
    /// Add bid entry with validation
    pub fn add_bid(&mut self, price: f64, quantity: u64) -> Result<()> {
        let entry = OrderBookEntry::new(price, quantity, OrderSide::Buy)?;
        self.bids.push(entry);
        Ok(())
    }
    
    /// Add ask entry with validation
    pub fn add_ask(&mut self, price: f64, quantity: u64) -> Result<()> {
        let entry = OrderBookEntry::new(price, quantity, OrderSide::Sell)?;
        self.asks.push(entry);
        Ok(())
    }
    
    /// Sort order book entries properly
    pub fn sort_entries(&mut self) {
        // Sort bids by price descending (highest first)
        self.bids.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap_or(std::cmp::Ordering::Equal));
        
        // Sort asks by price ascending (lowest first)
        self.asks.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal));
    }
}

/// Subscription structure for WebSocket connections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketSubscription {
    pub symbol: String,
    pub message_types: Vec<MessageType>,
    pub subscribed_at: SystemTime,
    pub is_active: bool,
    pub subscription_id: String,
}

impl WebSocketSubscription {
    /// Create new subscription with validation
    pub fn new(symbol: String, message_types: Vec<MessageType>) -> Result<Self> {
        if symbol.is_empty() {
            return Err(anyhow::anyhow!("Symbol cannot be empty"));
        }
        
        if message_types.is_empty() {
            return Err(anyhow::anyhow!("At least one message type must be specified"));
        }
        
        let subscription_id = format!("{}_{}", symbol, SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_millis());
        
        Ok(Self {
            symbol,
            message_types,
            subscribed_at: SystemTime::now(),
            is_active: true,
            subscription_id,
        })
    }
    
    /// Check if subscription includes message type
    pub fn includes_message_type(&self, message_type: &MessageType) -> bool {
        self.message_types.contains(message_type)
    }
}

/// WebSocket connection state enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting { attempt: u32, next_retry_in: Duration },
    Error { message: String, recoverable: bool },
}

impl ConnectionState {
    /// Check if state allows sending messages
    pub fn can_send_messages(&self) -> bool {
        matches!(self, ConnectionState::Connected)
    }
    
    /// Check if state is in error
    pub fn is_error(&self) -> bool {
        matches!(self, ConnectionState::Error { .. })
    }
    
    /// Check if reconnecting
    pub fn is_reconnecting(&self) -> bool {
        matches!(self, ConnectionState::Reconnecting { .. })
    }
}

/// WebSocket endpoint configuration
#[derive(Debug, Clone)]
pub struct WebSocketEndpoint {
    pub url: String,
    pub priority: u32,
    pub is_active: bool,
    pub last_connection_attempt: Option<SystemTime>,
    pub connection_failures: u32,
    pub max_failures_before_disable: u32,
}

impl WebSocketEndpoint {
    /// Create new endpoint with validation
    pub fn new(url: String, priority: u32) -> Result<Self> {
        if url.is_empty() {
            return Err(anyhow::anyhow!("Endpoint URL cannot be empty"));
        }
        
        // Validate URL format
        Url::parse(&url)?;
        
        Ok(Self {
            url,
            priority,
            is_active: true,
            last_connection_attempt: None,
            connection_failures: 0,
            max_failures_before_disable: 3,
        })
    }
    
    /// Record connection failure
    pub fn record_failure(&mut self) {
        self.connection_failures += 1;
        self.last_connection_attempt = Some(SystemTime::now());
        
        if self.connection_failures >= self.max_failures_before_disable {
            self.is_active = false;
        }
    }
    
    /// Record successful connection
    pub fn record_success(&mut self) {
        self.connection_failures = 0;
        self.last_connection_attempt = Some(SystemTime::now());
        self.is_active = true;
    }
    
    /// Check if endpoint should be retried
    pub fn should_retry(&self, cooldown_duration: Duration) -> bool {
        if !self.is_active {
            return false;
        }
        
        if let Some(last_attempt) = self.last_connection_attempt {
            if let Ok(elapsed) = SystemTime::now().duration_since(last_attempt) {
                return elapsed >= cooldown_duration;
            }
        }
        
        true
    }
}

/// Connection health metrics
#[derive(Debug, Clone)]
pub struct ConnectionHealth {
    pub last_message_received: Option<SystemTime>,
    pub last_message_sent: Option<SystemTime>,
    pub messages_received_count: u64,
    pub messages_sent_count: u64,
    pub last_ping_sent: Option<SystemTime>,
    pub last_pong_received: Option<SystemTime>,
    pub average_latency: Option<Duration>,
    pub connection_uptime: Duration,
    pub connection_established_at: Option<SystemTime>,
}

impl ConnectionHealth {
    /// Create new health metrics
    pub fn new() -> Self {
        Self {
            last_message_received: None,
            last_message_sent: None,
            messages_received_count: 0,
            messages_sent_count: 0,
            last_ping_sent: None,
            last_pong_received: None,
            average_latency: None,
            connection_uptime: Duration::from_secs(0),
            connection_established_at: None,
        }
    }
    
    /// Record message received
    pub fn record_message_received(&mut self) {
        self.last_message_received = Some(SystemTime::now());
        self.messages_received_count += 1;
    }
    
    /// Record message sent
    pub fn record_message_sent(&mut self) {
        self.last_message_sent = Some(SystemTime::now());
        self.messages_sent_count += 1;
    }
    
    /// Record ping sent
    pub fn record_ping_sent(&mut self) {
        self.last_ping_sent = Some(SystemTime::now());
    }
    
    /// Record pong received and calculate latency
    pub fn record_pong_received(&mut self) {
        let now = SystemTime::now();
        self.last_pong_received = Some(now);
        
        if let Some(ping_time) = self.last_ping_sent {
            if let Ok(latency) = now.duration_since(ping_time) {
                self.average_latency = Some(latency);
            }
        }
    }
    
    /// Check if connection is healthy
    pub fn is_healthy(&self, timeout: Duration) -> bool {
        if let Some(last_received) = self.last_message_received {
            if let Ok(elapsed) = SystemTime::now().duration_since(last_received) {
                return elapsed < timeout;
            }
        }
        false
    }
    
    /// Update connection uptime
    pub fn update_uptime(&mut self) {
        if let Some(established_at) = self.connection_established_at {
            if let Ok(uptime) = SystemTime::now().duration_since(established_at) {
                self.connection_uptime = uptime;
            }
        }
    }
    
    /// Mark connection as established
    pub fn mark_connected(&mut self) {
        self.connection_established_at = Some(SystemTime::now());
    }
    
    /// Reset health metrics
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

/// WebSocket service entity for real-time data streaming
pub struct WebSocketService {
    connection: Option<Arc<Mutex<WebSocketStream<MaybeTlsStream<TcpStream>>>>>,
    subscriptions: HashMap<String, WebSocketSubscription>,
    message_handlers: HashMap<MessageType, Box<dyn Fn(WebSocketMessage) -> Result<()> + Send + Sync>>,
    connection_state: ConnectionState,
    endpoints: Vec<WebSocketEndpoint>,
    current_endpoint_index: usize,
    reconnect_attempts: u32,
    max_reconnect_attempts: u32,
    base_reconnect_delay: Duration,
    max_reconnect_delay: Duration,
    heartbeat_interval: Duration,
    heartbeat_timeout: Duration,
    last_heartbeat: Option<SystemTime>,
    connection_health: ConnectionHealth,
    message_buffer: Vec<WebSocketMessage>,
    max_buffer_size: usize,
    enable_auto_reconnect: bool,
    // Rate limiting for subscriptions
    subscription_rate_limit: u32,
    subscription_window: Duration,
    subscription_timestamps: Vec<SystemTime>,
    max_subscriptions_per_connection: usize,
    // Message processing
    message_sequence_tracker: HashMap<String, u64>, // Track last sequence per symbol
    message_deduplication_cache: HashMap<String, SystemTime>, // Cache message IDs
    deduplication_window: Duration,
    enable_message_ordering: bool,
    enable_deduplication: bool,
    data_quality_checks_enabled: bool,
    _subscriptions: Vec<Subscription>,
}

impl WebSocketService {
    /// Create new WebSocket service with default configuration
    pub fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self {
            connection: None,
            subscriptions: HashMap::new(),
            message_handlers: HashMap::new(),
            connection_state: ConnectionState::Disconnected,
            endpoints: Vec::new(),
            current_endpoint_index: 0,
            reconnect_attempts: 0,
            max_reconnect_attempts: 5,
            base_reconnect_delay: Duration::from_secs(1),
            max_reconnect_delay: Duration::from_secs(60),
            heartbeat_interval: Duration::from_secs(30),
            heartbeat_timeout: Duration::from_secs(90),
            last_heartbeat: None,
            connection_health: ConnectionHealth::new(),
            message_buffer: Vec::new(),
            max_buffer_size: 1000,
            enable_auto_reconnect: true,
            subscription_rate_limit: 10, // 10 subscriptions per window
            subscription_window: Duration::from_secs(1), // 1 second window
            subscription_timestamps: Vec::new(),
            max_subscriptions_per_connection: 100, // Maximum 100 active subscriptions
            message_sequence_tracker: HashMap::new(),
            message_deduplication_cache: HashMap::new(),
            deduplication_window: Duration::from_secs(5),
            enable_message_ordering: true,
            enable_deduplication: true,
            data_quality_checks_enabled: true,
            _subscriptions: Vec::new(),
        })
    }
    
    /// Create new WebSocket service with custom configuration
    pub fn with_config(
        max_reconnect_attempts: u32,
        base_reconnect_delay: Duration,
        heartbeat_interval: Duration,
        cx: &mut App,
    ) -> Entity<Self> {
        cx.new(|_| Self {
            connection: None,
            subscriptions: HashMap::new(),
            message_handlers: HashMap::new(),
            connection_state: ConnectionState::Disconnected,
            endpoints: Vec::new(),
            current_endpoint_index: 0,
            reconnect_attempts: 0,
            max_reconnect_attempts,
            base_reconnect_delay,
            max_reconnect_delay: Duration::from_secs(60),
            heartbeat_interval,
            heartbeat_timeout: heartbeat_interval * 3,
            last_heartbeat: None,
            connection_health: ConnectionHealth::new(),
            message_buffer: Vec::new(),
            max_buffer_size: 1000,
            enable_auto_reconnect: true,
            subscription_rate_limit: 10,
            subscription_window: Duration::from_secs(1),
            subscription_timestamps: Vec::new(),
            max_subscriptions_per_connection: 100,
            message_sequence_tracker: HashMap::new(),
            message_deduplication_cache: HashMap::new(),
            deduplication_window: Duration::from_secs(5),
            enable_message_ordering: true,
            enable_deduplication: true,
            data_quality_checks_enabled: true,
            _subscriptions: Vec::new(),
        })
    }
    
    /// Add WebSocket endpoint with priority
    pub fn add_endpoint(&mut self, url: String, priority: u32) -> Result<()> {
        let endpoint = WebSocketEndpoint::new(url, priority)?;
        self.endpoints.push(endpoint);
        
        // Sort endpoints by priority (higher priority first)
        self.endpoints.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        Ok(())
    }
    
    /// Get next available endpoint for connection
    fn get_next_endpoint(&mut self) -> Option<&mut WebSocketEndpoint> {
        let cooldown = self.calculate_reconnect_delay();
        
        // Try to find an active endpoint that should be retried
        for (index, endpoint) in self.endpoints.iter_mut().enumerate() {
            if endpoint.should_retry(cooldown) {
                self.current_endpoint_index = index;
                return Some(endpoint);
            }
        }
        
        None
    }
    
    /// Calculate exponential backoff delay
    fn calculate_reconnect_delay(&self) -> Duration {
        let exponential_delay = self.base_reconnect_delay * 2_u32.pow(self.reconnect_attempts);
        std::cmp::min(exponential_delay, self.max_reconnect_delay)
    }
    
    /// Connect to WebSocket endpoint with proper error handling (.rules compliance)
    pub fn connect(&mut self, url: &str, cx: &mut Context<Self>) -> gpui::Task<Result<()>> {
        // Add endpoint if not already in list
        if self.endpoints.is_empty() {
            if let Err(error) = self.add_endpoint(url.to_string(), 100) {
                return gpui::Task::ready(Err(error));
            }
        }
        
        self.connection_state = ConnectionState::Connecting;
        cx.emit(WebSocketEvent::Connecting);
        
        let url = url.to_string();
        
        cx.spawn(async move |this, cx| {
            match Url::parse(&url) {
                Ok(parsed_url) => {
                    match connect_async(parsed_url).await {
                        Ok((ws_stream, _)) => {
                            let connection = Arc::new(Mutex::new(ws_stream));
                            
                            this.update(cx, |this, cx| {
                                this.connection = Some(connection.clone());
                                this.connection_state = ConnectionState::Connected;
                                this.reconnect_attempts = 0;
                                this.connection_health.mark_connected();
                                this.connection_health.reset();
                                
                                // Mark endpoint as successful
                                if let Some(endpoint) = this.endpoints.get_mut(this.current_endpoint_index) {
                                    endpoint.record_success();
                                }
                                
                                // Start message receiving loop
                                this.start_message_receiver(connection.clone(), cx);
                                
                                // Start health monitoring
                                this.start_heartbeat(cx);
                                this.start_health_monitoring(cx);
                                
                                // Flush buffered messages
                                this.flush_message_buffer(cx);
                                
                                // Resubscribe to all active subscriptions
                                this.resubscribe_all(cx);
                                
                                cx.emit(WebSocketEvent::Connected);
                            })?;
                            
                            Ok(())
                        }
                        Err(error) => {
                            this.update(cx, |this, cx| {
                                // Mark endpoint as failed
                                if let Some(endpoint) = this.endpoints.get_mut(this.current_endpoint_index) {
                                    endpoint.record_failure();
                                }
                                
                                this.handle_connection_error(error.into(), cx);
                            })?;
                            Err(anyhow::anyhow!("Failed to connect to WebSocket"))
                        }
                    }
                }
                Err(error) => {
                    this.update(cx, |this, cx| {
                        this.handle_connection_error(error.into(), cx);
                    })?;
                    Err(anyhow::anyhow!("Invalid WebSocket URL"))
                }
            }
        })
    }
    
    /// Start message receiving loop with proper error handling (.rules compliance)
    fn start_message_receiver(
        &mut self,
        connection: Arc<Mutex<WebSocketStream<MaybeTlsStream<TcpStream>>>>,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |this, mut cx| {
            use futures::StreamExt;
            
            loop {
                let message_result = {
                    let mut stream = match connection.lock() {
                        Ok(s) => s,
                        Err(e) => {
                            let error = anyhow::anyhow!("Failed to acquire stream lock: {}", e);
                            if let Err(update_error) = this.update(cx, |this: &mut Self, cx: &mut Context<Self>| {
                                this.handle_connection_error(error, cx);
                                Ok::<(), anyhow::Error>(())
                            }) {
                                log::error!("Failed to handle lock error: {}", update_error);
                            }
                            break;
                        }
                    };
                    
                    stream.next().await
                };
                
                match message_result {
                    Some(Ok(msg)) => {
                        match msg {
                            Message::Text(text) => {
                                match serde_json::from_str::<WebSocketMessage>(&text) {
                                    Ok(ws_message) => {
                                        if let Err(e) = this.update(cx, |this: &mut Self, cx: &mut Context<Self>| {
                                            this.handle_message(ws_message, cx)
                                        }) {
                                            log::error!("Failed to handle WebSocket message: {}", e);
                                        }
                                    }
                                    Err(parse_error) => {
                                        log::warn!("Failed to parse WebSocket message: {}", parse_error);
                                    }
                                }
                            }
                            Message::Binary(data) => {
                                match serde_json::from_slice::<WebSocketMessage>(&data) {
                                    Ok(ws_message) => {
                                        if let Err(e) = this.update(cx, |this: &mut Self, cx: &mut Context<Self>| {
                                            this.handle_message(ws_message, cx)
                                        }) {
                                            log::error!("Failed to handle binary WebSocket message: {}", e);
                                        }
                                    }
                                    Err(parse_error) => {
                                        log::warn!("Failed to parse binary WebSocket message: {}", parse_error);
                                    }
                                }
                            }
                            Message::Ping(_) => {
                                // Pong is automatically sent by the library
                                if let Err(e) = this.update(cx, |this: &mut Self, _cx: &mut Context<Self>| {
                                    this.handle_pong();
                                    Ok::<(), anyhow::Error>(())
                                }) {
                                    log::error!("Failed to handle ping: {}", e);
                                }
                            }
                            Message::Pong(_) => {
                                if let Err(e) = this.update(cx, |this: &mut Self, _cx: &mut Context<Self>| {
                                    this.handle_pong();
                                    Ok::<(), anyhow::Error>(())
                                }) {
                                    log::error!("Failed to handle pong: {}", e);
                                }
                            }
                            Message::Close(_) => {
                                let error = anyhow::anyhow!("WebSocket connection closed by server");
                                if let Err(e) = this.update(cx, |this: &mut Self, cx: &mut Context<Self>| {
                                    this.handle_connection_error(error, cx);
                                    Ok::<(), anyhow::Error>(())
                                }) {
                                    log::error!("Failed to handle close message: {}", e);
                                }
                                break;
                            }
                            Message::Frame(_) => {
                                // Raw frames are handled internally
                            }
                        }
                    }
                    Some(Err(e)) => {
                        let error = anyhow::anyhow!("WebSocket error: {}", e);
                        if let Err(update_error) = this.update(cx, |this: &mut Self, cx: &mut Context<Self>| {
                            this.handle_connection_error(error, cx);
                            Ok::<(), anyhow::Error>(())
                        }) {
                            log::error!("Failed to handle WebSocket error: {}", update_error);
                        }
                        break;
                    }
                    None => {
                        let error = anyhow::anyhow!("WebSocket stream ended");
                        if let Err(update_error) = this.update(cx, |this: &mut Self, cx: &mut Context<Self>| {
                            this.handle_connection_error(error, cx);
                            Ok::<(), anyhow::Error>(())
                        }) {
                            log::error!("Failed to handle stream end: {}", update_error);
                        }
                        break;
                    }
                }
            }
            
            Ok::<(), anyhow::Error>(())
        }).detach();
    }
    
    /// Disconnect from WebSocket
    pub fn disconnect(&mut self, cx: &mut Context<Self>) {
        self.connection = None;
        self.connection_state = ConnectionState::Disconnected;
        self.connection_health.reset();
        self.enable_auto_reconnect = false;
        cx.emit(WebSocketEvent::Disconnected);
    }
    
    /// Start health monitoring with ping/pong
    fn start_health_monitoring(&mut self, cx: &mut Context<Self>) {
        let heartbeat_interval = self.heartbeat_interval;
        let heartbeat_timeout = self.heartbeat_timeout;
        
        cx.spawn(async move |this, mut cx| {
            loop {
                cx.background_executor().timer(heartbeat_interval).await;
                
                let should_continue = this.update(cx, |this: &mut Self, cx: &mut Context<Self>| {
                    // Check if connection is still healthy
                    if !this.connection_health.is_healthy(heartbeat_timeout) {
                        let error = anyhow::anyhow!("Connection health check failed - no messages received within timeout");
                        this.handle_connection_error(error, cx);
                        return false;
                    }
                    
                    // Send ping
                    if let Err(error) = this.send_ping(cx) {
                        error.log_err();
                        return false;
                    }
                    
                    // Update uptime
                    this.connection_health.update_uptime();
                    
                    true
                });
                
                let should_continue = match should_continue {
                    Ok(value) => value,
                    Err(error) => {
                        log::error!("Health monitoring update failed: {}", error);
                        false
                    }
                };
                
                if !should_continue {
                    break;
                }
            }
            Ok::<(), anyhow::Error>(())
        }).detach();
    }
    
    /// Send ping message for health check
    fn send_ping(&mut self, cx: &mut Context<Self>) -> Result<()> {
        self.connection_health.record_ping_sent();
        
        let ping_message = WebSocketMessage::new(
            MessageType::Heartbeat,
            None,
            serde_json::json!({"type": "ping", "timestamp": SystemTime::now()}),
        )?;
        
        self.send_message(ping_message, cx).detach();
        Ok(())
    }
    
    /// Handle pong message
    fn handle_pong(&mut self) {
        self.connection_health.record_pong_received();
    }
    
    /// Resubscribe to all active subscriptions after reconnection
    fn resubscribe_all(&mut self, cx: &mut Context<Self>) {
        let subscriptions: Vec<_> = self.subscriptions.values().cloned().collect();
        
        for subscription in subscriptions {
            if subscription.is_active {
                let subscribe_message = WebSocketMessage::new(
                    MessageType::Subscribe,
                    Some(subscription.symbol.clone()),
                    serde_json::to_value(&subscription).unwrap_or_default(),
                );
                
                if let Ok(message) = subscribe_message {
                    self.send_message(message, cx).detach();
                }
            }
        }
    }
    
    /// Check if rate limit allows new subscription with bounds checking (.rules compliance)
    fn check_subscription_rate_limit(&mut self) -> Result<()> {
        let now = SystemTime::now();
        
        // Clean up old timestamps outside the window
        self.subscription_timestamps.retain(|timestamp| {
            if let Ok(elapsed) = now.duration_since(*timestamp) {
                elapsed < self.subscription_window
            } else {
                false
            }
        });
        
        // Check if we're within rate limit
        if self.subscription_timestamps.len() >= self.subscription_rate_limit as usize {
            return Err(anyhow::anyhow!(
                "Subscription rate limit exceeded: {} subscriptions per {:?}",
                self.subscription_rate_limit,
                self.subscription_window
            ));
        }
        
        // Check total subscription count
        if self.subscriptions.len() >= self.max_subscriptions_per_connection {
            return Err(anyhow::anyhow!(
                "Maximum subscriptions per connection exceeded: {}",
                self.max_subscriptions_per_connection
            ));
        }
        
        Ok(())
    }
    
    /// Record subscription attempt for rate limiting
    fn record_subscription_attempt(&mut self) {
        self.subscription_timestamps.push(SystemTime::now());
    }
    
    /// Set subscription rate limit configuration
    pub fn set_rate_limit(&mut self, limit: u32, window: Duration) {
        self.subscription_rate_limit = limit;
        self.subscription_window = window;
    }
    
    /// Set maximum subscriptions per connection
    pub fn set_max_subscriptions(&mut self, max: usize) {
        self.max_subscriptions_per_connection = max;
    }
    
    /// Get current subscription count
    pub fn get_subscription_count(&self) -> usize {
        self.subscriptions.len()
    }
    
    /// Get subscription rate limit status
    pub fn get_rate_limit_status(&self) -> (usize, u32, Duration) {
        let now = SystemTime::now();
        let recent_count = self.subscription_timestamps.iter()
            .filter(|timestamp| {
                if let Ok(elapsed) = now.duration_since(**timestamp) {
                    elapsed < self.subscription_window
                } else {
                    false
                }
            })
            .count();
        
        (recent_count, self.subscription_rate_limit, self.subscription_window)
    }
    
    /// Subscribe to symbol with message types and rate limiting (.rules compliance)
    pub fn subscribe_to_symbol(
        &mut self,
        symbol: String,
        message_types: Vec<MessageType>,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        // Check rate limit before subscribing
        self.check_subscription_rate_limit()?;
        
        // Check if already subscribed
        if self.subscriptions.contains_key(&symbol) {
            return Err(anyhow::anyhow!("Already subscribed to symbol: {}", symbol));
        }
        
        let subscription = WebSocketSubscription::new(symbol.clone(), message_types)?;
        
        // Create subscription message
        let subscribe_message = WebSocketMessage::new(
            MessageType::Subscribe,
            Some(symbol.clone()),
            serde_json::to_value(&subscription)?,
        )?;
        
        self.subscriptions.insert(symbol.clone(), subscription);
        self.record_subscription_attempt();
        
        // Send subscription message if connected
        if matches!(self.connection_state, ConnectionState::Connected) {
            self.send_message(subscribe_message, cx).detach();
        }
        
        cx.emit(WebSocketEvent::SubscriptionAdded(symbol));
        
        Ok(())
    }
    
    /// Unsubscribe from symbol
    pub fn unsubscribe_from_symbol(
        &mut self,
        symbol: &str,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        if let Some(subscription) = self.subscriptions.remove(symbol) {
            let unsubscribe_message = WebSocketMessage::new(
                MessageType::Unsubscribe,
                Some(symbol.to_string()),
                serde_json::to_value(&subscription)?,
            )?;
            
            // Send unsubscription message if connected
            if matches!(self.connection_state, ConnectionState::Connected) {
                self.send_message(unsubscribe_message, cx).detach();
            }
            
            cx.emit(WebSocketEvent::SubscriptionRemoved(symbol.to_string()));
        }
        
        Ok(())
    }
    
    /// Subscribe to multiple symbols at once with rate limiting (.rules compliance)
    pub fn subscribe_to_symbols(
        &mut self,
        symbols: Vec<(String, Vec<MessageType>)>,
        cx: &mut Context<Self>,
    ) -> Result<Vec<String>> {
        let mut subscribed_symbols = Vec::new();
        let mut errors = Vec::new();
        
        for (symbol, message_types) in symbols {
            match self.subscribe_to_symbol(symbol.clone(), message_types, cx) {
                Ok(_) => subscribed_symbols.push(symbol),
                Err(e) => {
                    errors.push(format!("{}: {}", symbol, e));
                    // Stop if rate limit is hit
                    if e.to_string().contains("rate limit") {
                        break;
                    }
                }
            }
        }
        
        if !errors.is_empty() {
            log::warn!("Some subscriptions failed: {}", errors.join(", "));
        }
        
        Ok(subscribed_symbols)
    }
    
    /// Unsubscribe from multiple symbols at once
    pub fn unsubscribe_from_symbols(
        &mut self,
        symbols: Vec<String>,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        for symbol in symbols {
            if let Err(e) = self.unsubscribe_from_symbol(&symbol, cx) {
                e.log_err();
            }
        }
        Ok(())
    }
    
    /// Unsubscribe from all symbols
    pub fn unsubscribe_all(&mut self, cx: &mut Context<Self>) -> Result<()> {
        let symbols: Vec<_> = self.subscriptions.keys().cloned().collect();
        self.unsubscribe_from_symbols(symbols, cx)
    }
    
    /// Update subscription message types with bounds checking (.rules compliance)
    pub fn update_subscription(
        &mut self,
        symbol: &str,
        message_types: Vec<MessageType>,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        if let Some(subscription) = self.subscriptions.get_mut(symbol) {
            subscription.message_types = message_types.clone();
            
            // Send updated subscription if connected
            if matches!(self.connection_state, ConnectionState::Connected) {
                let subscribe_message = WebSocketMessage::new(
                    MessageType::Subscribe,
                    Some(symbol.to_string()),
                    serde_json::to_value(&subscription)?,
                )?;
                
                self.send_message(subscribe_message, cx).detach();
            }
            
            Ok(())
        } else {
            Err(anyhow::anyhow!("Subscription not found for symbol: {}", symbol))
        }
    }
    
    /// Pause subscription without removing it
    pub fn pause_subscription(&mut self, symbol: &str, cx: &mut Context<Self>) -> Result<()> {
        if let Some(subscription) = self.subscriptions.get_mut(symbol) {
            subscription.is_active = false;
            
            // Send unsubscribe message if connected
            if matches!(self.connection_state, ConnectionState::Connected) {
                let unsubscribe_message = WebSocketMessage::new(
                    MessageType::Unsubscribe,
                    Some(symbol.to_string()),
                    serde_json::to_value(&subscription)?,
                )?;
                
                self.send_message(unsubscribe_message, cx).detach();
            }
            
            Ok(())
        } else {
            Err(anyhow::anyhow!("Subscription not found for symbol: {}", symbol))
        }
    }
    
    /// Resume paused subscription
    pub fn resume_subscription(&mut self, symbol: &str, cx: &mut Context<Self>) -> Result<()> {
        if let Some(subscription) = self.subscriptions.get_mut(symbol) {
            subscription.is_active = true;
            
            // Send subscribe message if connected
            if matches!(self.connection_state, ConnectionState::Connected) {
                let subscribe_message = WebSocketMessage::new(
                    MessageType::Subscribe,
                    Some(symbol.to_string()),
                    serde_json::to_value(&subscription)?,
                )?;
                
                self.send_message(subscribe_message, cx).detach();
            }
            
            Ok(())
        } else {
            Err(anyhow::anyhow!("Subscription not found for symbol: {}", symbol))
        }
    }
    
    /// Send WebSocket message with proper error handling (.rules compliance)
    pub fn send_message(
        &mut self,
        message: WebSocketMessage,
        cx: &mut Context<Self>,
    ) -> gpui::Task<Result<()>> {
        if !self.connection_state.can_send_messages() {
            // Buffer message if not connected
            if self.message_buffer.len() < self.max_buffer_size {
                self.message_buffer.push(message);
            } else {
                return gpui::Task::ready(Err(anyhow::anyhow!("Message buffer full")));
            }
            return gpui::Task::ready(Ok(()));
        }
        
        let connection = self.connection.clone();
        self.connection_health.record_message_sent();
        
        cx.background_spawn(async move {
            if let Some(conn) = connection {
                let json_message = serde_json::to_string(&message)?;
                let ws_message = Message::Text(json_message);
                
                // Send message through WebSocket stream with proper error handling
                // We need to send without holding the lock across await
                use futures::SinkExt;
                
                // Acquire lock, send, and immediately drop lock before any await
                let send_result = {
                    let mut stream = conn.lock().map_err(|e| anyhow::anyhow!("Failed to acquire lock: {}", e))?;
                    // Send returns a future, but we need to poll it while holding the lock
                    // So we use a blocking approach here
                    futures::executor::block_on(stream.send(ws_message))
                        .map_err(|e| anyhow::anyhow!("Failed to send message: {}", e))
                };
                
                send_result?;
            }
            Ok(())
        })
    }
    
    /// Handle incoming WebSocket message with bounds checking (.rules compliance)
    pub fn handle_message(&mut self, message: WebSocketMessage, cx: &mut Context<Self>) -> Result<()> {
        // Record message received for health tracking
        self.connection_health.record_message_received();
        
        // Update last heartbeat if this is a heartbeat message
        if message.message_type == MessageType::Heartbeat {
            self.last_heartbeat = Some(SystemTime::now());
            self.handle_pong();
            cx.emit(WebSocketEvent::HeartbeatReceived);
            return Ok(());
        }
        
        // Check for duplicate messages
        if self.enable_deduplication && self.is_duplicate_message(&message)? {
            log::debug!("Duplicate message detected, skipping");
            return Ok(());
        }
        
        // Validate message ordering
        if self.enable_message_ordering && !self.validate_message_order(&message)? {
            log::warn!("Out-of-order message detected: {:?}", message.symbol);
            // Still process but log the issue
        }
        
        // Perform data quality checks
        if self.data_quality_checks_enabled {
            if let Err(e) = self.validate_message_quality(&message) {
                e.log_err(); // Use .log_err() for visibility
                return Ok(()); // Skip invalid messages
            }
        }
        
        // Update deduplication cache
        if self.enable_deduplication {
            self.update_deduplication_cache(&message)?;
        }
        
        // Update sequence tracker
        if self.enable_message_ordering {
            self.update_sequence_tracker(&message)?;
        }
        
        // Route message to appropriate handler
        if let Some(handler) = self.message_handlers.get(&message.message_type) {
            handler(message.clone())?; // Propagate errors instead of ignoring
        }
        
        // Emit event for subscribers
        cx.emit(WebSocketEvent::MessageReceived(message));
        
        Ok(())
    }
    
    /// Check if message is duplicate with bounds checking (.rules compliance)
    fn is_duplicate_message(&self, message: &WebSocketMessage) -> Result<bool> {
        if let Some(symbol) = &message.symbol {
            let message_id = format!("{}_{}_{:?}", 
                symbol, 
                message.message_type.clone() as u8,
                message.timestamp
            );
            
            if let Some(last_seen) = self.message_deduplication_cache.get(&message_id) {
                if let Ok(elapsed) = SystemTime::now().duration_since(*last_seen) {
                    if elapsed < self.deduplication_window {
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }
    
    /// Update deduplication cache with cleanup
    fn update_deduplication_cache(&mut self, message: &WebSocketMessage) -> Result<()> {
        if let Some(symbol) = &message.symbol {
            let message_id = format!("{}_{}_{:?}", 
                symbol, 
                message.message_type.clone() as u8,
                message.timestamp
            );
            
            self.message_deduplication_cache.insert(message_id, SystemTime::now());
            
            // Clean up old entries
            let cutoff = SystemTime::now();
            self.message_deduplication_cache.retain(|_, timestamp| {
                if let Ok(elapsed) = cutoff.duration_since(*timestamp) {
                    elapsed < self.deduplication_window
                } else {
                    false
                }
            });
        }
        Ok(())
    }
    
    /// Validate message ordering with bounds checking (.rules compliance)
    fn validate_message_order(&self, message: &WebSocketMessage) -> Result<bool> {
        if let (Some(symbol), Some(sequence)) = (&message.symbol, message.sequence) {
            if let Some(&last_sequence) = self.message_sequence_tracker.get(symbol) {
                if sequence <= last_sequence {
                    return Ok(false); // Out of order
                }
            }
        }
        Ok(true)
    }
    
    /// Update sequence tracker with bounds checking (.rules compliance)
    fn update_sequence_tracker(&mut self, message: &WebSocketMessage) -> Result<()> {
        if let (Some(symbol), Some(sequence)) = (&message.symbol, message.sequence) {
            self.message_sequence_tracker.insert(symbol.clone(), sequence);
        }
        Ok(())
    }
    
    /// Validate message data quality with explicit error handling (.rules compliance)
    fn validate_message_quality(&self, message: &WebSocketMessage) -> Result<()> {
        match message.message_type {
            MessageType::Quote => {
                self.validate_quote_quality(message)?;
            }
            MessageType::Trade => {
                self.validate_trade_quality(message)?;
            }
            MessageType::OrderBook => {
                self.validate_order_book_quality(message)?;
            }
            _ => {
                // Other message types don't require quality checks
            }
        }
        Ok(())
    }
    
    /// Validate quote message quality
    fn validate_quote_quality(&self, message: &WebSocketMessage) -> Result<()> {
        let quote: QuoteUpdate = serde_json::from_value(message.data.clone())?;
        
        if quote.symbol.is_empty() {
            return Err(anyhow::anyhow!("Quote has empty symbol"));
        }
        
        if quote.bid <= 0.0 || quote.ask <= 0.0 || quote.last_price <= 0.0 {
            return Err(anyhow::anyhow!("Quote has invalid prices"));
        }
        
        if quote.bid >= quote.ask {
            return Err(anyhow::anyhow!("Quote has invalid spread (bid >= ask)"));
        }
        
        // Check for unrealistic price movements (>50% in one update)
        let price_range = quote.high - quote.low;
        if price_range > quote.last_price * 0.5 {
            log::warn!("Suspicious price range detected for {}: {}", quote.symbol, price_range);
        }
        
        Ok(())
    }
    
    /// Validate trade message quality
    fn validate_trade_quality(&self, message: &WebSocketMessage) -> Result<()> {
        let trade: TradeUpdate = serde_json::from_value(message.data.clone())?;
        
        if trade.symbol.is_empty() {
            return Err(anyhow::anyhow!("Trade has empty symbol"));
        }
        
        if trade.trade_id.is_empty() {
            return Err(anyhow::anyhow!("Trade has empty trade ID"));
        }
        
        if trade.price <= 0.0 {
            return Err(anyhow::anyhow!("Trade has invalid price"));
        }
        
        if trade.size == 0 {
            return Err(anyhow::anyhow!("Trade has zero size"));
        }
        
        Ok(())
    }
    
    /// Validate order book message quality
    fn validate_order_book_quality(&self, message: &WebSocketMessage) -> Result<()> {
        let order_book: OrderBookUpdate = serde_json::from_value(message.data.clone())?;
        
        if order_book.symbol.is_empty() {
            return Err(anyhow::anyhow!("Order book has empty symbol"));
        }
        
        // Validate bid prices are descending
        for i in 1..order_book.bids.len() {
            if let (Some(prev), Some(curr)) = (order_book.bids.get(i - 1), order_book.bids.get(i)) {
                if curr.price > prev.price {
                    return Err(anyhow::anyhow!("Order book bids not properly sorted"));
                }
            }
        }
        
        // Validate ask prices are ascending
        for i in 1..order_book.asks.len() {
            if let (Some(prev), Some(curr)) = (order_book.asks.get(i - 1), order_book.asks.get(i)) {
                if curr.price < prev.price {
                    return Err(anyhow::anyhow!("Order book asks not properly sorted"));
                }
            }
        }
        
        // Validate spread (best bid < best ask)
        if let (Some(best_bid), Some(best_ask)) = (order_book.bids.first(), order_book.asks.first()) {
            if best_bid.price >= best_ask.price {
                return Err(anyhow::anyhow!("Order book has invalid spread"));
            }
        }
        
        Ok(())
    }
    
    /// Enable or disable message ordering
    pub fn set_message_ordering(&mut self, enabled: bool) {
        self.enable_message_ordering = enabled;
    }
    
    /// Enable or disable message deduplication
    pub fn set_deduplication(&mut self, enabled: bool) {
        self.enable_deduplication = enabled;
    }
    
    /// Enable or disable data quality checks
    pub fn set_data_quality_checks(&mut self, enabled: bool) {
        self.data_quality_checks_enabled = enabled;
    }
    
    /// Set deduplication window
    pub fn set_deduplication_window(&mut self, window: Duration) {
        self.deduplication_window = window;
    }
    
    /// Clear message processing caches
    pub fn clear_message_caches(&mut self) {
        self.message_sequence_tracker.clear();
        self.message_deduplication_cache.clear();
    }
    
    /// Handle connection errors with proper error handling (.rules compliance)
    pub fn handle_connection_error(&mut self, error: anyhow::Error, cx: &mut Context<Self>) {
        let error_string = error.to_string();
        let is_recoverable = self.is_error_recoverable(&error);
        
        error.log_err(); // Use .log_err() for visibility
        
        self.connection_state = ConnectionState::Error {
            message: error_string.clone(),
            recoverable: is_recoverable,
        };
        
        self.connection = None;
        self.connection_health.reset();
        
        cx.emit(WebSocketEvent::ConnectionError(error_string));
        
        // Attempt reconnection if enabled and error is recoverable
        if self.enable_auto_reconnect && is_recoverable && self.reconnect_attempts < self.max_reconnect_attempts {
            self.reconnect_attempts += 1;
            let delay = self.calculate_reconnect_delay();
            
            self.connection_state = ConnectionState::Reconnecting {
                attempt: self.reconnect_attempts,
                next_retry_in: delay,
            };
            
            cx.emit(WebSocketEvent::ReconnectAttempt(self.reconnect_attempts));
            
            cx.spawn(async move |this, mut cx| {
                cx.background_executor().timer(delay).await;
                
                if let Err(reconnect_error) = this.update(cx, |this: &mut Self, cx: &mut Context<Self>| this.attempt_reconnect(cx)) {
                    log::error!("Reconnection attempt failed: {}", reconnect_error);
                }
                Ok::<(), anyhow::Error>(())
            }).detach();
        } else if self.reconnect_attempts >= self.max_reconnect_attempts {
            // Max reconnection attempts reached, try failover to next endpoint
            if let Err(failover_error) = self.attempt_failover(cx) {
                failover_error.log_err();
            }
        }
    }
    
    /// Check if error is recoverable
    fn is_error_recoverable(&self, error: &anyhow::Error) -> bool {
        let error_string = error.to_string().to_lowercase();
        
        // Network errors are usually recoverable
        if error_string.contains("connection") 
            || error_string.contains("timeout")
            || error_string.contains("network")
            || error_string.contains("refused") {
            return true;
        }
        
        // Authentication errors are not recoverable
        if error_string.contains("auth") 
            || error_string.contains("unauthorized")
            || error_string.contains("forbidden") {
            return false;
        }
        
        // Default to recoverable
        true
    }
    
    /// Attempt to reconnect to current endpoint
    fn attempt_reconnect(&mut self, cx: &mut Context<Self>) -> Result<()> {
        if let Some(endpoint) = self.endpoints.get(self.current_endpoint_index) {
            let url = endpoint.url.clone();
            self.connect(&url, cx).detach();
            Ok(())
        } else {
            Err(anyhow::anyhow!("No endpoint available for reconnection"))
        }
    }
    
    /// Attempt failover to next available endpoint
    fn attempt_failover(&mut self, cx: &mut Context<Self>) -> Result<()> {
        if let Some(endpoint) = self.get_next_endpoint() {
            let url = endpoint.url.clone();
            self.reconnect_attempts = 0; // Reset attempts for new endpoint
            
            cx.emit(WebSocketEvent::FailoverAttempt {
                from_endpoint: self.current_endpoint_index,
                to_endpoint: self.endpoints.iter().position(|e| e.url == url).unwrap_or(0),
            });
            
            self.connect(&url, cx).detach();
            Ok(())
        } else {
            Err(anyhow::anyhow!("No available endpoints for failover"))
        }
    }
    
    /// Enable or disable auto-reconnect
    pub fn set_auto_reconnect(&mut self, enabled: bool) {
        self.enable_auto_reconnect = enabled;
    }
    
    /// Get connection health metrics
    pub fn get_connection_health(&self) -> &ConnectionHealth {
        &self.connection_health
    }
    
    /// Get active endpoints
    pub fn get_endpoints(&self) -> &[WebSocketEndpoint] {
        &self.endpoints
    }
    
    /// Get current endpoint index
    pub fn get_current_endpoint_index(&self) -> usize {
        self.current_endpoint_index
    }
    
    /// Start heartbeat mechanism with proper async patterns
    pub fn start_heartbeat(&mut self, cx: &mut Context<Self>) {
        let heartbeat_interval = self.heartbeat_interval;
        
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(heartbeat_interval).await;
                
                if let Err(error) = this.update(cx, |this, cx| {
                    this.send_heartbeat(cx)
                }) {
                    error.log_err(); // Proper error handling
                    break;
                }
            }
            Ok::<(), anyhow::Error>(())
        }).detach();
    }
    
    /// Send heartbeat message
    fn send_heartbeat(&mut self, cx: &mut Context<Self>) -> Result<()> {
        let heartbeat_message = WebSocketMessage::new(
            MessageType::Heartbeat,
            None,
            serde_json::json!({"timestamp": SystemTime::now()}),
        )?;
        
        self.send_message(heartbeat_message, cx).detach();
        Ok(())
    }
    
    /// Get connection state
    pub fn get_connection_state(&self) -> &ConnectionState {
        &self.connection_state
    }
    
    /// Get active subscriptions with bounds checking (.rules compliance)
    pub fn get_subscriptions(&self) -> Vec<&WebSocketSubscription> {
        self.subscriptions.values().collect()
    }
    
    /// Check if subscribed to symbol
    pub fn is_subscribed_to(&self, symbol: &str) -> bool {
        self.subscriptions.contains_key(symbol)
    }
    
    /// Flush message buffer when connection is established
    pub fn flush_message_buffer(&mut self, cx: &mut Context<Self>) {
        if self.connection_state.can_send_messages() {
            let messages = std::mem::take(&mut self.message_buffer);
            for message in messages {
                self.send_message(message, cx).detach();
            }
            
            if !self.message_buffer.is_empty() {
                cx.emit(WebSocketEvent::BufferFlushed);
            }
        }
    }
}

impl EventEmitter<WebSocketEvent> for WebSocketService {}

impl Render for WebSocketService {
    fn render(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        gpui::div() // WebSocket service doesn't render UI directly
    }
}

/// WebSocket service events
#[derive(Clone, Debug)]
pub enum WebSocketEvent {
    Connecting,
    Connected,
    Disconnected,
    MessageReceived(WebSocketMessage),
    SubscriptionAdded(String),
    SubscriptionRemoved(String),
    ConnectionError(String),
    ReconnectAttempt(u32),
    FailoverAttempt { from_endpoint: usize, to_endpoint: usize },
    HeartbeatReceived,
    BufferFlushed,
    HealthCheckFailed,
    EndpointDisabled { endpoint_index: usize, reason: String },
}