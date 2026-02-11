use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings::{Settings, SettingsContent};
use std::collections::HashMap;
use std::time::Duration;

use crate::TimeFrame;

/// Panel dock position (re-exported from workspace for convenience)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DockPosition {
    Left,
    Right,
    Bottom,
}

/// Stock trading system settings with proper error handling (.rules compliance)
#[derive(Clone, Debug)]
pub struct StockTradingSettings {
    /// Default symbols to show in watchlist on startup
    pub default_watchlist: Vec<String>,
    /// Default timeframe for charts
    pub default_timeframe: TimeFrame,
    /// Auto-refresh interval for market data (in seconds)
    pub auto_refresh_interval: Duration,
    /// Whether to use mock data for development
    pub use_mock_data: bool,
    /// API configuration
    pub api_config: ApiConfig,
    /// Panel configuration
    pub panel_config: PanelConfig,
    /// Theme configuration
    pub theme_config: ThemeConfig,
    /// Cache configuration
    pub cache_config: CacheConfig,
    /// WebSocket configuration
    pub websocket_config: WebSocketConfig,
}

/// API configuration with proper validation
#[derive(Clone, Debug)]
pub struct ApiConfig {
    /// Market data API endpoint URL
    pub market_data_url: String,
    /// Trading API endpoint URL (optional)
    pub trading_api_url: Option<String>,
    /// WebSocket endpoint URL for real-time data
    pub websocket_url: Option<String>,
    /// API key for authentication (optional, stored securely)
    pub api_key: Option<String>,
    /// Request timeout in seconds
    pub request_timeout: Duration,
    /// Maximum retry attempts for failed requests
    pub max_retry_attempts: u32,
    /// Rate limit: maximum requests per minute
    pub rate_limit_per_minute: u32,
}

/// Panel configuration for persistence
#[derive(Clone, Debug)]
pub struct PanelConfig {
    /// Default panel positions
    pub default_positions: HashMap<String, DockPosition>,
    /// Panel sizes (width/height in pixels)
    pub panel_sizes: HashMap<String, f32>,
    /// Whether panels are visible by default
    pub panel_visibility: HashMap<String, bool>,
    /// Whether to restore panel state on startup
    pub restore_on_startup: bool,
}

/// Theme configuration for trading UI
#[derive(Clone, Debug)]
pub struct ThemeConfig {
    /// Color for positive price changes (green)
    pub positive_color: String,
    /// Color for negative price changes (red)
    pub negative_color: String,
    /// Color for neutral/unchanged prices
    pub neutral_color: String,
    /// Chart background color (optional, uses theme default if None)
    pub chart_background: Option<String>,
    /// Grid line color for charts
    pub grid_color: String,
    /// Whether to use theme colors or custom colors
    pub use_theme_colors: bool,
}

/// Cache configuration for data management
#[derive(Clone, Debug)]
pub struct CacheConfig {
    /// Cache duration for market data (in seconds)
    pub market_data_cache_duration: Duration,
    /// Cache duration for historical data (in seconds)
    pub historical_data_cache_duration: Duration,
    /// Cache duration for order book data (in seconds)
    pub order_book_cache_duration: Duration,
    /// Maximum cache size (number of entries)
    pub max_cache_size: usize,
    /// Whether to enable automatic cache cleanup
    pub auto_cleanup_enabled: bool,
    /// Cleanup interval (in seconds)
    pub cleanup_interval: Duration,
}

/// WebSocket configuration for real-time data
#[derive(Clone, Debug)]
pub struct WebSocketConfig {
    /// Whether WebSocket is enabled
    pub enabled: bool,
    /// Reconnection attempts before giving up
    pub max_reconnect_attempts: u32,
    /// Initial reconnection delay (in seconds)
    pub reconnect_delay: Duration,
    /// Heartbeat interval (in seconds)
    pub heartbeat_interval: Duration,
    /// Message deduplication window (in seconds)
    pub deduplication_window: Duration,
    /// Whether to automatically subscribe to active symbols
    pub auto_subscribe: bool,
}

/// Settings content structure for JSON serialization
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct StockTradingSettingsContent {
    /// Default symbols to show in watchlist on startup
    #[serde(default = "default_watchlist")]
    pub default_watchlist: Vec<String>,
    
    /// Default timeframe for charts
    #[serde(default = "default_timeframe")]
    pub default_timeframe: TimeFrameContent,
    
    /// Auto-refresh interval for market data (in seconds)
    #[serde(default = "default_refresh_interval")]
    pub auto_refresh_interval: u64,
    
    /// Whether to use mock data for development
    #[serde(default = "default_use_mock_data")]
    pub use_mock_data: bool,
    
    /// API configuration
    #[serde(default)]
    pub api: ApiConfigContent,
    
    /// Panel configuration
    #[serde(default)]
    pub panels: PanelConfigContent,
    
    /// Theme configuration
    #[serde(default)]
    pub theme: ThemeConfigContent,
    
    /// Cache configuration
    #[serde(default)]
    pub cache: CacheConfigContent,
    
    /// WebSocket configuration
    #[serde(default)]
    pub websocket: WebSocketConfigContent,
}

/// TimeFrame content for JSON serialization
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TimeFrameContent {
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
    OneHour,
    OneDay,
    OneWeek,
    OneMonth,
}

/// API configuration content for JSON
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ApiConfigContent {
    #[serde(default = "default_market_data_url")]
    pub market_data_url: String,
    
    #[serde(default)]
    pub trading_api_url: Option<String>,
    
    #[serde(default)]
    pub websocket_url: Option<String>,
    
    #[serde(default)]
    pub api_key: Option<String>,
    
    #[serde(default = "default_request_timeout")]
    pub request_timeout: u64,
    
    #[serde(default = "default_max_retry_attempts")]
    pub max_retry_attempts: u32,
    
    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_minute: u32,
}

/// Panel configuration content for JSON
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct PanelConfigContent {
    #[serde(default)]
    pub default_positions: HashMap<String, String>,
    
    #[serde(default)]
    pub panel_sizes: HashMap<String, f32>,
    
    #[serde(default)]
    pub panel_visibility: HashMap<String, bool>,
    
    #[serde(default = "default_restore_on_startup")]
    pub restore_on_startup: bool,
}

/// Theme configuration content for JSON
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ThemeConfigContent {
    #[serde(default = "default_positive_color")]
    pub positive_color: String,
    
    #[serde(default = "default_negative_color")]
    pub negative_color: String,
    
    #[serde(default = "default_neutral_color")]
    pub neutral_color: String,
    
    #[serde(default)]
    pub chart_background: Option<String>,
    
    #[serde(default = "default_grid_color")]
    pub grid_color: String,
    
    #[serde(default = "default_use_theme_colors")]
    pub use_theme_colors: bool,
}

/// Cache configuration content for JSON
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct CacheConfigContent {
    #[serde(default = "default_market_data_cache_duration")]
    pub market_data_cache_duration: u64,
    
    #[serde(default = "default_historical_data_cache_duration")]
    pub historical_data_cache_duration: u64,
    
    #[serde(default = "default_order_book_cache_duration")]
    pub order_book_cache_duration: u64,
    
    #[serde(default = "default_max_cache_size")]
    pub max_cache_size: usize,
    
    #[serde(default = "default_auto_cleanup_enabled")]
    pub auto_cleanup_enabled: bool,
    
    #[serde(default = "default_cleanup_interval")]
    pub cleanup_interval: u64,
}

/// WebSocket configuration content for JSON
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct WebSocketConfigContent {
    #[serde(default = "default_websocket_enabled")]
    pub enabled: bool,
    
    #[serde(default = "default_max_reconnect_attempts")]
    pub max_reconnect_attempts: u32,
    
    #[serde(default = "default_reconnect_delay")]
    pub reconnect_delay: u64,
    
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval: u64,
    
    #[serde(default = "default_deduplication_window")]
    pub deduplication_window: u64,
    
    #[serde(default = "default_auto_subscribe")]
    pub auto_subscribe: bool,
}

// Default value functions
fn default_watchlist() -> Vec<String> {
    vec![
        "AAPL".to_string(),
        "GOOGL".to_string(),
        "MSFT".to_string(),
        "TSLA".to_string(),
    ]
}

fn default_timeframe() -> TimeFrameContent {
    TimeFrameContent::OneDay
}

fn default_refresh_interval() -> u64 {
    30 // 30 seconds
}

fn default_use_mock_data() -> bool {
    true // Default to mock data for development
}

fn default_market_data_url() -> String {
    "https://api.example.com/market-data".to_string()
}

fn default_request_timeout() -> u64 {
    30 // 30 seconds
}

fn default_max_retry_attempts() -> u32 {
    3
}

fn default_rate_limit() -> u32 {
    60 // 60 requests per minute
}

fn default_restore_on_startup() -> bool {
    true
}

fn default_positive_color() -> String {
    "#00ff00".to_string() // Green
}

fn default_negative_color() -> String {
    "#ff0000".to_string() // Red
}

fn default_neutral_color() -> String {
    "#808080".to_string() // Gray
}

fn default_grid_color() -> String {
    "#404040".to_string() // Dark gray
}

fn default_use_theme_colors() -> bool {
    true
}

fn default_market_data_cache_duration() -> u64 {
    60 // 1 minute
}

fn default_historical_data_cache_duration() -> u64 {
    300 // 5 minutes
}

fn default_order_book_cache_duration() -> u64 {
    5 // 5 seconds
}

fn default_max_cache_size() -> usize {
    1000
}

fn default_auto_cleanup_enabled() -> bool {
    true
}

fn default_cleanup_interval() -> u64 {
    300 // 5 minutes
}

fn default_websocket_enabled() -> bool {
    true
}

fn default_max_reconnect_attempts() -> u32 {
    10
}

fn default_reconnect_delay() -> u64 {
    5 // 5 seconds
}

fn default_heartbeat_interval() -> u64 {
    30 // 30 seconds
}

fn default_deduplication_window() -> u64 {
    5 // 5 seconds
}

fn default_auto_subscribe() -> bool {
    true
}

impl Default for ApiConfigContent {
    fn default() -> Self {
        Self {
            market_data_url: default_market_data_url(),
            trading_api_url: None,
            websocket_url: None,
            api_key: None,
            request_timeout: default_request_timeout(),
            max_retry_attempts: default_max_retry_attempts(),
            rate_limit_per_minute: default_rate_limit(),
        }
    }
}

impl Default for PanelConfigContent {
    fn default() -> Self {
        Self {
            default_positions: HashMap::new(),
            panel_sizes: HashMap::new(),
            panel_visibility: HashMap::new(),
            restore_on_startup: default_restore_on_startup(),
        }
    }
}

impl Default for ThemeConfigContent {
    fn default() -> Self {
        Self {
            positive_color: default_positive_color(),
            negative_color: default_negative_color(),
            neutral_color: default_neutral_color(),
            chart_background: None,
            grid_color: default_grid_color(),
            use_theme_colors: default_use_theme_colors(),
        }
    }
}

impl Default for CacheConfigContent {
    fn default() -> Self {
        Self {
            market_data_cache_duration: default_market_data_cache_duration(),
            historical_data_cache_duration: default_historical_data_cache_duration(),
            order_book_cache_duration: default_order_book_cache_duration(),
            max_cache_size: default_max_cache_size(),
            auto_cleanup_enabled: default_auto_cleanup_enabled(),
            cleanup_interval: default_cleanup_interval(),
        }
    }
}

impl Default for WebSocketConfigContent {
    fn default() -> Self {
        Self {
            enabled: default_websocket_enabled(),
            max_reconnect_attempts: default_max_reconnect_attempts(),
            reconnect_delay: default_reconnect_delay(),
            heartbeat_interval: default_heartbeat_interval(),
            deduplication_window: default_deduplication_window(),
            auto_subscribe: default_auto_subscribe(),
        }
    }
}

impl Settings for StockTradingSettings {
    fn from_settings(content: &SettingsContent) -> Self {
        // For now, use default settings since we need to add stock_trading field to SettingsContent
        // This will be properly integrated when SettingsContent is extended
        let _ = content; // Acknowledge unused parameter
        
        Self {
            default_watchlist: default_watchlist(),
            default_timeframe: TimeFrame::OneDay,
            auto_refresh_interval: Duration::from_secs(default_refresh_interval()),
            use_mock_data: default_use_mock_data(),
            api_config: ApiConfig {
                market_data_url: default_market_data_url(),
                trading_api_url: None,
                websocket_url: None,
                api_key: None,
                request_timeout: Duration::from_secs(default_request_timeout()),
                max_retry_attempts: default_max_retry_attempts(),
                rate_limit_per_minute: default_rate_limit(),
            },
            panel_config: PanelConfig {
                default_positions: HashMap::new(),
                panel_sizes: HashMap::new(),
                panel_visibility: HashMap::new(),
                restore_on_startup: default_restore_on_startup(),
            },
            theme_config: ThemeConfig {
                positive_color: default_positive_color(),
                negative_color: default_negative_color(),
                neutral_color: default_neutral_color(),
                chart_background: None,
                grid_color: default_grid_color(),
                use_theme_colors: default_use_theme_colors(),
            },
            cache_config: CacheConfig {
                market_data_cache_duration: Duration::from_secs(default_market_data_cache_duration()),
                historical_data_cache_duration: Duration::from_secs(default_historical_data_cache_duration()),
                order_book_cache_duration: Duration::from_secs(default_order_book_cache_duration()),
                max_cache_size: default_max_cache_size(),
                auto_cleanup_enabled: default_auto_cleanup_enabled(),
                cleanup_interval: Duration::from_secs(default_cleanup_interval()),
            },
            websocket_config: WebSocketConfig {
                enabled: default_websocket_enabled(),
                max_reconnect_attempts: default_max_reconnect_attempts(),
                reconnect_delay: Duration::from_secs(default_reconnect_delay()),
                heartbeat_interval: Duration::from_secs(default_heartbeat_interval()),
                deduplication_window: Duration::from_secs(default_deduplication_window()),
                auto_subscribe: default_auto_subscribe(),
            },
        }
    }
}

/// Parse dock position string with proper error handling (.rules compliance)
fn parse_dock_position(position_str: &str) -> Option<DockPosition> {
    match position_str.to_lowercase().as_str() {
        "left" => Some(DockPosition::Left),
        "right" => Some(DockPosition::Right),
        "bottom" => Some(DockPosition::Bottom),
        _ => None,
    }
}

/// Validate settings with proper error handling (.rules compliance)
pub fn validate_settings(settings: &StockTradingSettings) -> Result<()> {
    // Validate API configuration
    if settings.api_config.market_data_url.is_empty() {
        return Err(anyhow::anyhow!("Market data URL cannot be empty"));
    }
    
    if settings.api_config.request_timeout.as_secs() == 0 {
        return Err(anyhow::anyhow!("Request timeout must be greater than 0"));
    }
    
    if settings.api_config.max_retry_attempts == 0 {
        return Err(anyhow::anyhow!("Max retry attempts must be greater than 0"));
    }
    
    // Validate cache configuration
    if settings.cache_config.max_cache_size == 0 {
        return Err(anyhow::anyhow!("Max cache size must be greater than 0"));
    }
    
    // Validate WebSocket configuration
    if settings.websocket_config.enabled && settings.websocket_config.max_reconnect_attempts == 0 {
        return Err(anyhow::anyhow!("WebSocket max reconnect attempts must be greater than 0 when enabled"));
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_settings() {
        let content = StockTradingSettingsContent {
            default_watchlist: default_watchlist(),
            default_timeframe: default_timeframe(),
            auto_refresh_interval: default_refresh_interval(),
            use_mock_data: default_use_mock_data(),
            api: ApiConfigContent::default(),
            panels: PanelConfigContent::default(),
            theme: ThemeConfigContent::default(),
            cache: CacheConfigContent::default(),
            websocket: WebSocketConfigContent::default(),
        };
        
        assert_eq!(content.default_watchlist.len(), 4);
        assert!(content.use_mock_data);
        assert_eq!(content.auto_refresh_interval, 30);
    }
    
    #[test]
    fn test_parse_dock_position() {
        assert_eq!(parse_dock_position("left"), Some(DockPosition::Left));
        assert_eq!(parse_dock_position("RIGHT"), Some(DockPosition::Right));
        assert_eq!(parse_dock_position("Bottom"), Some(DockPosition::Bottom));
        assert_eq!(parse_dock_position("invalid"), None);
    }
    
    #[test]
    fn test_validate_settings() {
        let mut settings = StockTradingSettings {
            default_watchlist: vec!["AAPL".to_string()],
            default_timeframe: TimeFrame::OneDay,
            auto_refresh_interval: Duration::from_secs(30),
            use_mock_data: true,
            api_config: ApiConfig {
                market_data_url: "https://api.example.com".to_string(),
                trading_api_url: None,
                websocket_url: None,
                api_key: None,
                request_timeout: Duration::from_secs(30),
                max_retry_attempts: 3,
                rate_limit_per_minute: 60,
            },
            panel_config: PanelConfig {
                default_positions: HashMap::new(),
                panel_sizes: HashMap::new(),
                panel_visibility: HashMap::new(),
                restore_on_startup: true,
            },
            theme_config: ThemeConfig {
                positive_color: "#00ff00".to_string(),
                negative_color: "#ff0000".to_string(),
                neutral_color: "#808080".to_string(),
                chart_background: None,
                grid_color: "#404040".to_string(),
                use_theme_colors: true,
            },
            cache_config: CacheConfig {
                market_data_cache_duration: Duration::from_secs(60),
                historical_data_cache_duration: Duration::from_secs(300),
                order_book_cache_duration: Duration::from_secs(5),
                max_cache_size: 1000,
                auto_cleanup_enabled: true,
                cleanup_interval: Duration::from_secs(300),
            },
            websocket_config: WebSocketConfig {
                enabled: true,
                max_reconnect_attempts: 10,
                reconnect_delay: Duration::from_secs(5),
                heartbeat_interval: Duration::from_secs(30),
                deduplication_window: Duration::from_secs(5),
                auto_subscribe: true,
            },
        };
        
        // Valid settings should pass
        assert!(validate_settings(&settings).is_ok());
        
        // Invalid market data URL
        settings.api_config.market_data_url = String::new();
        assert!(validate_settings(&settings).is_err());
        settings.api_config.market_data_url = "https://api.example.com".to_string();
        
        // Invalid cache size
        settings.cache_config.max_cache_size = 0;
        assert!(validate_settings(&settings).is_err());
    }
}
