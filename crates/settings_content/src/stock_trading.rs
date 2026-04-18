use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings_macros::{MergeFrom, with_fallible_options};

/// Stock trading settings content for serialization
#[with_fallible_options]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct StockTradingSettingsContent {
    /// Default symbols to show in watchlist on startup
    #[serde(default)]
    pub default_watchlist: Vec<String>,
    
    /// Default timeframe for charts
    #[serde(default)]
    pub default_timeframe: TimeFrameContent,
    
    /// Auto-refresh interval for market data (in seconds)
    #[serde(default)]
    pub auto_refresh_interval: u64,
    
    /// Whether to use mock data for development
    #[serde(default)]
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

/// Timeframe content for serialization
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, MergeFrom)]
#[serde(rename_all = "UPPERCASE")]
pub enum TimeFrameContent {
    #[serde(rename = "1m")]
    OneMinute,
    #[serde(rename = "5m")]
    FiveMinutes,
    #[serde(rename = "15m")]
    FifteenMinutes,
    #[serde(rename = "30m")]
    ThirtyMinutes,
    #[serde(rename = "1h")]
    OneHour,
    #[serde(rename = "4h")]
    FourHours,
    #[serde(rename = "1D")]
    OneDay,
    #[serde(rename = "1W")]
    OneWeek,
    #[serde(rename = "1M")]
    OneMonth,
}

impl Default for TimeFrameContent {
    fn default() -> Self {
        Self::OneDay
    }
}

impl TimeFrameContent {
    pub fn to_string(&self) -> &'static str {
        match self {
            Self::OneMinute => "1m",
            Self::FiveMinutes => "5m",
            Self::FifteenMinutes => "15m",
            Self::ThirtyMinutes => "30m",
            Self::OneHour => "1h",
            Self::FourHours => "4h",
            Self::OneDay => "1D",
            Self::OneWeek => "1W",
            Self::OneMonth => "1M",
        }
    }
}

// Forward declaration - actual TimeFrame enum is in stock_trading crate
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TimeFrame {
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
    ThirtyMinutes,
    OneHour,
    FourHours,
    OneDay,
    OneWeek,
    OneMonth,
}

/// API configuration content
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct ApiConfigContent {
    #[serde(default)]
    pub market_data_url: Option<String>,
    #[serde(default)]
    pub trading_api_url: Option<String>,
    #[serde(default)]
    pub websocket_url: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub request_timeout: u64,
    #[serde(default)]
    pub max_retry_attempts: u32,
    #[serde(default)]
    pub rate_limit_per_minute: u32,
}

/// Panel configuration content
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct PanelConfigContent {
    #[serde(default)]
    pub default_layout: Option<String>,
    #[serde(default)]
    pub auto_save_layout: bool,
    #[serde(default)]
    pub restore_on_startup: bool,
}

/// Theme configuration content
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct ThemeConfigContent {
    #[serde(default)]
    pub positive_color: Option<String>,
    #[serde(default)]
    pub negative_color: Option<String>,
    #[serde(default)]
    pub neutral_color: Option<String>,
    #[serde(default)]
    pub chart_background: Option<String>,
    #[serde(default)]
    pub grid_color: Option<String>,
    #[serde(default)]
    pub use_theme_colors: bool,
}

/// Cache configuration content
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct CacheConfigContent {
    #[serde(default)]
    pub market_data_cache_duration: u64,
    #[serde(default)]
    pub historical_data_cache_duration: u64,
    #[serde(default)]
    pub order_book_cache_duration: u64,
    #[serde(default)]
    pub max_cache_size: usize,
    #[serde(default)]
    pub auto_cleanup_enabled: bool,
    #[serde(default)]
    pub cleanup_interval: u64,
}

/// WebSocket configuration content
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct WebSocketConfigContent {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub max_reconnect_attempts: u32,
    #[serde(default)]
    pub reconnect_delay: u64,
    #[serde(default)]
    pub heartbeat_interval: u64,
    #[serde(default)]
    pub deduplication_window: u64,
    #[serde(default)]
    pub auto_subscribe: bool,
}

/// Longport API configuration content
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema, MergeFrom)]
pub struct LongportConfigContent {
    #[serde(default)]
    pub enabled: bool,
    
    #[serde(default)]
    pub app_key: Option<String>,
    
    #[serde(default)]
    pub app_secret: Option<String>,
    
    #[serde(default)]
    pub access_token: Option<String>,
    
    #[serde(default)]
    pub api_endpoint: Option<String>,
    
    #[serde(default)]
    pub use_for_realtime: bool,
    
    #[serde(default)]
    pub use_for_historical: bool,
    
    #[serde(default)]
    pub daily_quota_limit: Option<u32>,
    
    #[serde(default)]
    pub rate_limit_per_minute: u32,
    
    #[serde(default)]
    pub auto_fallback_to_mock: bool,
}
