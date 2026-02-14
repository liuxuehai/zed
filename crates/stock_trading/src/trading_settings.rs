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
    /// Longport API configuration
    pub longport_config: LongportConfig,
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

/// Longport API configuration for real market data
#[derive(Clone, Debug)]
pub struct LongportConfig {
    /// Whether Longport integration is enabled
    pub enabled: bool,
    /// Longport API app key (required for authentication)
    pub app_key: Option<String>,
    /// Longport API app secret (required for authentication)
    pub app_secret: Option<String>,
    /// Longport API access token (required for authentication)
    pub access_token: Option<String>,
    /// Longport API endpoint URL (optional, uses default if None)
    pub api_endpoint: Option<String>,
    /// Whether to use Longport for real-time data (vs mock data)
    pub use_for_realtime: bool,
    /// Whether to use Longport for historical data
    pub use_for_historical: bool,
    /// API quota limit per day (for monitoring)
    pub daily_quota_limit: Option<u32>,
    /// Current API usage count (for monitoring)
    pub current_usage_count: u32,
    /// Rate limit: maximum requests per minute
    pub rate_limit_per_minute: u32,
    /// Whether to automatically fallback to mock data on API errors
    pub auto_fallback_to_mock: bool,
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
    
    /// Longport API configuration
    #[serde(default)]
    pub longport: LongportConfigContent,
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

/// Longport API configuration content for JSON
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct LongportConfigContent {
    #[serde(default = "default_longport_enabled")]
    pub enabled: bool,
    
    #[serde(default)]
    pub app_key: Option<String>,
    
    #[serde(default)]
    pub app_secret: Option<String>,
    
    #[serde(default)]
    pub access_token: Option<String>,
    
    #[serde(default)]
    pub api_endpoint: Option<String>,
    
    #[serde(default = "default_use_for_realtime")]
    pub use_for_realtime: bool,
    
    #[serde(default = "default_use_for_historical")]
    pub use_for_historical: bool,
    
    #[serde(default)]
    pub daily_quota_limit: Option<u32>,
    
    #[serde(default = "default_longport_rate_limit")]
    pub rate_limit_per_minute: u32,
    
    #[serde(default = "default_auto_fallback_to_mock")]
    pub auto_fallback_to_mock: bool,
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

fn default_longport_enabled() -> bool {
    false // Default to disabled, user must configure
}

fn default_use_for_realtime() -> bool {
    true
}

fn default_use_for_historical() -> bool {
    true
}

fn default_longport_rate_limit() -> u32 {
    30 // Conservative default for Longport API
}

fn default_auto_fallback_to_mock() -> bool {
    true // Automatically fallback to mock data on errors
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

impl Default for LongportConfigContent {
    fn default() -> Self {
        Self {
            enabled: default_longport_enabled(),
            app_key: None,
            app_secret: None,
            access_token: None,
            api_endpoint: None,
            use_for_realtime: default_use_for_realtime(),
            use_for_historical: default_use_for_historical(),
            daily_quota_limit: None,
            rate_limit_per_minute: default_longport_rate_limit(),
            auto_fallback_to_mock: default_auto_fallback_to_mock(),
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
            longport_config: LongportConfig {
                enabled: default_longport_enabled(),
                app_key: None,
                app_secret: None,
                access_token: None,
                api_endpoint: None,
                use_for_realtime: default_use_for_realtime(),
                use_for_historical: default_use_for_historical(),
                daily_quota_limit: None,
                current_usage_count: 0,
                rate_limit_per_minute: default_longport_rate_limit(),
                auto_fallback_to_mock: default_auto_fallback_to_mock(),
            },
        }
    }
}

/// Parse dock position string with proper error handling (.rules compliance)
#[allow(dead_code)]
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
    
    // Validate Longport configuration
    validate_longport_config(&settings.longport_config)?;
    
    Ok(())
}

/// Validate Longport configuration with proper error messages (.rules compliance)
pub fn validate_longport_config(config: &LongportConfig) -> Result<()> {
    if !config.enabled {
        return Ok(()); // Skip validation if Longport is disabled
    }
    
    // Validate required credentials
    if config.app_key.is_none() || config.app_key.as_ref().map(|k| k.is_empty()).unwrap_or(true) {
        return Err(anyhow::anyhow!(
            "Longport app_key is required when Longport integration is enabled. \
            Please configure your Longport API credentials in settings."
        ));
    }
    
    if config.app_secret.is_none() || config.app_secret.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
        return Err(anyhow::anyhow!(
            "Longport app_secret is required when Longport integration is enabled. \
            Please configure your Longport API credentials in settings."
        ));
    }
    
    if config.access_token.is_none() || config.access_token.as_ref().map(|t| t.is_empty()).unwrap_or(true) {
        return Err(anyhow::anyhow!(
            "Longport access_token is required when Longport integration is enabled. \
            Please configure your Longport API credentials in settings."
        ));
    }
    
    // Validate rate limit
    if config.rate_limit_per_minute == 0 {
        return Err(anyhow::anyhow!(
            "Longport rate_limit_per_minute must be greater than 0. \
            Recommended value: 30 requests per minute."
        ));
    }
    
    // Validate API endpoint if provided
    if let Some(endpoint) = &config.api_endpoint {
        if endpoint.is_empty() {
            return Err(anyhow::anyhow!(
                "Longport api_endpoint cannot be empty if specified. \
                Leave as None to use default endpoint."
            ));
        }
        
        // Basic URL validation
        if !endpoint.starts_with("http://") && !endpoint.starts_with("https://") {
            return Err(anyhow::anyhow!(
                "Longport api_endpoint must start with http:// or https://. \
                Provided: {}", endpoint
            ));
        }
    }
    
    // Validate daily quota if provided
    if let Some(quota) = config.daily_quota_limit
        && quota == 0
    {
        return Err(anyhow::anyhow!(
            "Longport daily_quota_limit must be greater than 0 if specified. \
            Leave as None for unlimited quota."
        ));
    }
    
    // Warn if both realtime and historical are disabled
    if !config.use_for_realtime && !config.use_for_historical {
        log::warn!(
            "Longport is enabled but both use_for_realtime and use_for_historical are disabled. \
            Longport will not be used for any data fetching."
        );
    }
    
    Ok(())
}

/// Check if Longport credentials are configured
pub fn has_longport_credentials(config: &LongportConfig) -> bool {
    config.app_key.is_some() 
        && config.app_secret.is_some() 
        && config.access_token.is_some()
}

/// Check if Longport API quota is exceeded
pub fn is_longport_quota_exceeded(config: &LongportConfig) -> bool {
    if let Some(limit) = config.daily_quota_limit {
        config.current_usage_count >= limit
    } else {
        false // No limit set
    }
}

/// Get remaining Longport API quota
pub fn get_remaining_longport_quota(config: &LongportConfig) -> Option<u32> {
    config.daily_quota_limit.map(|limit| {
        limit.saturating_sub(config.current_usage_count)
    })
}

/// Increment Longport API usage count
pub fn increment_longport_usage(config: &mut LongportConfig) {
    config.current_usage_count = config.current_usage_count.saturating_add(1);
}

/// Reset Longport API usage count (typically called daily)
pub fn reset_longport_usage(config: &mut LongportConfig) {
    config.current_usage_count = 0;
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
            longport: LongportConfigContent::default(),
        };
        
        assert_eq!(content.default_watchlist.len(), 4);
        assert!(content.use_mock_data);
        assert_eq!(content.auto_refresh_interval, 30);
        assert!(!content.longport.enabled); // Longport disabled by default
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
            longport_config: LongportConfig {
                enabled: false,
                app_key: None,
                app_secret: None,
                access_token: None,
                api_endpoint: None,
                use_for_realtime: true,
                use_for_historical: true,
                daily_quota_limit: None,
                current_usage_count: 0,
                rate_limit_per_minute: 30,
                auto_fallback_to_mock: true,
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
    
    #[test]
    fn test_validate_longport_config_disabled() {
        let config = LongportConfig {
            enabled: false,
            app_key: None,
            app_secret: None,
            access_token: None,
            api_endpoint: None,
            use_for_realtime: true,
            use_for_historical: true,
            daily_quota_limit: None,
            current_usage_count: 0,
            rate_limit_per_minute: 30,
            auto_fallback_to_mock: true,
        };
        
        // Disabled config should pass validation even without credentials
        assert!(validate_longport_config(&config).is_ok());
    }
    
    #[test]
    fn test_validate_longport_config_missing_credentials() {
        let mut config = LongportConfig {
            enabled: true,
            app_key: None,
            app_secret: None,
            access_token: None,
            api_endpoint: None,
            use_for_realtime: true,
            use_for_historical: true,
            daily_quota_limit: None,
            current_usage_count: 0,
            rate_limit_per_minute: 30,
            auto_fallback_to_mock: true,
        };
        
        // Missing app_key
        assert!(validate_longport_config(&config).is_err());
        
        // Missing app_secret
        config.app_key = Some("test_key".to_string());
        assert!(validate_longport_config(&config).is_err());
        
        // Missing access_token
        config.app_secret = Some("test_secret".to_string());
        assert!(validate_longport_config(&config).is_err());
        
        // All credentials provided
        config.access_token = Some("test_token".to_string());
        assert!(validate_longport_config(&config).is_ok());
    }
    
    #[test]
    fn test_validate_longport_config_invalid_endpoint() {
        let mut config = LongportConfig {
            enabled: true,
            app_key: Some("test_key".to_string()),
            app_secret: Some("test_secret".to_string()),
            access_token: Some("test_token".to_string()),
            api_endpoint: Some("invalid_url".to_string()),
            use_for_realtime: true,
            use_for_historical: true,
            daily_quota_limit: None,
            current_usage_count: 0,
            rate_limit_per_minute: 30,
            auto_fallback_to_mock: true,
        };
        
        // Invalid endpoint URL
        assert!(validate_longport_config(&config).is_err());
        
        // Valid HTTPS endpoint
        config.api_endpoint = Some("https://api.longport.com".to_string());
        assert!(validate_longport_config(&config).is_ok());
        
        // Valid HTTP endpoint
        config.api_endpoint = Some("http://localhost:8080".to_string());
        assert!(validate_longport_config(&config).is_ok());
    }
    
    #[test]
    fn test_validate_longport_config_rate_limit() {
        let mut config = LongportConfig {
            enabled: true,
            app_key: Some("test_key".to_string()),
            app_secret: Some("test_secret".to_string()),
            access_token: Some("test_token".to_string()),
            api_endpoint: None,
            use_for_realtime: true,
            use_for_historical: true,
            daily_quota_limit: None,
            current_usage_count: 0,
            rate_limit_per_minute: 0,
            auto_fallback_to_mock: true,
        };
        
        // Zero rate limit should fail
        assert!(validate_longport_config(&config).is_err());
        
        // Valid rate limit
        config.rate_limit_per_minute = 30;
        assert!(validate_longport_config(&config).is_ok());
    }
    
    #[test]
    fn test_has_longport_credentials() {
        let mut config = LongportConfig {
            enabled: true,
            app_key: None,
            app_secret: None,
            access_token: None,
            api_endpoint: None,
            use_for_realtime: true,
            use_for_historical: true,
            daily_quota_limit: None,
            current_usage_count: 0,
            rate_limit_per_minute: 30,
            auto_fallback_to_mock: true,
        };
        
        assert!(!has_longport_credentials(&config));
        
        config.app_key = Some("key".to_string());
        assert!(!has_longport_credentials(&config));
        
        config.app_secret = Some("secret".to_string());
        assert!(!has_longport_credentials(&config));
        
        config.access_token = Some("token".to_string());
        assert!(has_longport_credentials(&config));
    }
    
    #[test]
    fn test_longport_quota_management() {
        let mut config = LongportConfig {
            enabled: true,
            app_key: Some("key".to_string()),
            app_secret: Some("secret".to_string()),
            access_token: Some("token".to_string()),
            api_endpoint: None,
            use_for_realtime: true,
            use_for_historical: true,
            daily_quota_limit: Some(100),
            current_usage_count: 0,
            rate_limit_per_minute: 30,
            auto_fallback_to_mock: true,
        };
        
        // Initial state
        assert!(!is_longport_quota_exceeded(&config));
        assert_eq!(get_remaining_longport_quota(&config), Some(100));
        
        // Increment usage
        for _ in 0..50 {
            increment_longport_usage(&mut config);
        }
        assert_eq!(config.current_usage_count, 50);
        assert!(!is_longport_quota_exceeded(&config));
        assert_eq!(get_remaining_longport_quota(&config), Some(50));
        
        // Reach limit
        for _ in 0..50 {
            increment_longport_usage(&mut config);
        }
        assert_eq!(config.current_usage_count, 100);
        assert!(is_longport_quota_exceeded(&config));
        assert_eq!(get_remaining_longport_quota(&config), Some(0));
        
        // Reset usage
        reset_longport_usage(&mut config);
        assert_eq!(config.current_usage_count, 0);
        assert!(!is_longport_quota_exceeded(&config));
        assert_eq!(get_remaining_longport_quota(&config), Some(100));
    }
    
    #[test]
    fn test_longport_quota_no_limit() {
        let config = LongportConfig {
            enabled: true,
            app_key: Some("key".to_string()),
            app_secret: Some("secret".to_string()),
            access_token: Some("token".to_string()),
            api_endpoint: None,
            use_for_realtime: true,
            use_for_historical: true,
            daily_quota_limit: None,
            current_usage_count: 1000,
            rate_limit_per_minute: 30,
            auto_fallback_to_mock: true,
        };
        
        // No limit set, should never be exceeded
        assert!(!is_longport_quota_exceeded(&config));
        assert_eq!(get_remaining_longport_quota(&config), None);
    }
}
