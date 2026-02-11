// Comprehensive error handling module following Zed patterns (.rules compliance)

use anyhow::{anyhow, Result};
use gpui::{AppContext, Context, Entity, EventEmitter, Render};
use std::collections::VecDeque;
use std::time::{Duration, Instant, SystemTime};

/// Extension trait for error logging (.rules compliance)
pub trait LogErr<T> {
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

/// Network error types with proper categorization
#[derive(Debug, Clone, PartialEq)]
pub enum NetworkError {
    ConnectionFailed { message: String, retry_after: Option<Duration> },
    Timeout { operation: String, duration: Duration },
    RateLimitExceeded { retry_after: Duration, limit: u32 },
    ServiceUnavailable { service: String, estimated_recovery: Option<SystemTime> },
    InvalidResponse { details: String },
    AuthenticationFailed { reason: String },
    Offline,
}

impl std::fmt::Display for NetworkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkError::ConnectionFailed { message, retry_after } => {
                if let Some(duration) = retry_after {
                    write!(f, "Connection failed: {}. Retry in {:?}", message, duration)
                } else {
                    write!(f, "Connection failed: {}", message)
                }
            }
            NetworkError::Timeout { operation, duration } => {
                write!(f, "Operation '{}' timed out after {:?}", operation, duration)
            }
            NetworkError::RateLimitExceeded { retry_after, limit } => {
                write!(f, "Rate limit exceeded ({} requests). Retry in {:?}", limit, retry_after)
            }
            NetworkError::ServiceUnavailable { service, estimated_recovery } => {
                if let Some(recovery_time) = estimated_recovery {
                    write!(f, "Service '{}' unavailable. Estimated recovery: {:?}", service, recovery_time)
                } else {
                    write!(f, "Service '{}' unavailable", service)
                }
            }
            NetworkError::InvalidResponse { details } => {
                write!(f, "Invalid response: {}", details)
            }
            NetworkError::AuthenticationFailed { reason } => {
                write!(f, "Authentication failed: {}", reason)
            }
            NetworkError::Offline => {
                write!(f, "Network is offline. Using cached data.")
            }
        }
    }
}

impl std::error::Error for NetworkError {}

/// Connection status for UI display
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Online,
    Offline,
    Degraded { reason: String },
    RateLimited { retry_after: Duration },
    Reconnecting { attempt: u32, max_attempts: u32 },
}

impl ConnectionStatus {
    /// Check if operations should be allowed
    pub fn can_perform_operations(&self) -> bool {
        matches!(self, ConnectionStatus::Online | ConnectionStatus::Degraded { .. })
    }
    
    /// Get user-friendly status message
    pub fn get_display_message(&self) -> String {
        match self {
            ConnectionStatus::Online => "Connected".to_string(),
            ConnectionStatus::Offline => "Offline - Using cached data".to_string(),
            ConnectionStatus::Degraded { reason } => format!("Degraded: {}", reason),
            ConnectionStatus::RateLimited { retry_after } => {
                format!("Rate limited - Retry in {:?}", retry_after)
            }
            ConnectionStatus::Reconnecting { attempt, max_attempts } => {
                format!("Reconnecting... (Attempt {}/{})", attempt, max_attempts)
            }
        }
    }
}

/// Rate limiter with exponential backoff
pub struct RateLimiter {
    max_requests_per_window: u32,
    window_duration: Duration,
    request_timestamps: VecDeque<Instant>,
    backoff_multiplier: f64,
    current_backoff: Duration,
    max_backoff: Duration,
    consecutive_failures: u32,
}

impl RateLimiter {
    /// Create new rate limiter with configuration
    pub fn new(max_requests_per_window: u32, window_duration: Duration) -> Self {
        Self {
            max_requests_per_window,
            window_duration,
            request_timestamps: VecDeque::new(),
            backoff_multiplier: 2.0,
            current_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(300), // 5 minutes max
            consecutive_failures: 0,
        }
    }
    
    /// Check if request is allowed with bounds checking (.rules compliance)
    pub fn check_rate_limit(&mut self) -> Result<(), NetworkError> {
        let now = Instant::now();
        
        // Remove timestamps outside the current window
        while let Some(timestamp) = self.request_timestamps.front() {
            if now.duration_since(*timestamp) > self.window_duration {
                self.request_timestamps.pop_front();
            } else {
                break;
            }
        }
        
        // Check if we're within the rate limit
        if self.request_timestamps.len() >= self.max_requests_per_window as usize {
            let retry_after = if let Some(oldest) = self.request_timestamps.front() {
                self.window_duration.saturating_sub(now.duration_since(*oldest))
            } else {
                self.window_duration
            };
            
            return Err(NetworkError::RateLimitExceeded {
                retry_after,
                limit: self.max_requests_per_window,
            });
        }
        
        // Record this request
        self.request_timestamps.push_back(now);
        Ok(())
    }
    
    /// Record successful request (reset backoff)
    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
        self.current_backoff = Duration::from_secs(1);
    }
    
    /// Record failed request (increase backoff)
    pub fn record_failure(&mut self) {
        self.consecutive_failures += 1;
        
        // Calculate exponential backoff
        let backoff_seconds = (self.current_backoff.as_secs_f64() * self.backoff_multiplier) as u64;
        self.current_backoff = Duration::from_secs(backoff_seconds).min(self.max_backoff);
    }
    
    /// Get current backoff duration
    pub fn get_backoff_duration(&self) -> Duration {
        self.current_backoff
    }
    
    /// Get consecutive failure count
    pub fn get_failure_count(&self) -> u32 {
        self.consecutive_failures
    }
    
    /// Reset rate limiter state
    pub fn reset(&mut self) {
        self.request_timestamps.clear();
        self.consecutive_failures = 0;
        self.current_backoff = Duration::from_secs(1);
    }
}

/// Network error handler entity with proper GPUI integration
pub struct NetworkErrorHandler {
    connection_status: ConnectionStatus,
    rate_limiter: RateLimiter,
    error_history: VecDeque<(NetworkError, Instant)>,
    max_error_history: usize,
    offline_mode_enabled: bool,
    auto_retry_enabled: bool,
    max_retry_attempts: u32,
    fallback_to_cache_enabled: bool,
}

impl NetworkErrorHandler {
    /// Create new error handler with default configuration
    pub fn new(cx: &mut impl AppContext) -> Entity<Self> {
        cx.new(|_| Self {
            connection_status: ConnectionStatus::Online,
            rate_limiter: RateLimiter::new(100, Duration::from_secs(60)), // 100 requests per minute
            error_history: VecDeque::new(),
            max_error_history: 100,
            offline_mode_enabled: false,
            auto_retry_enabled: true,
            max_retry_attempts: 3,
            fallback_to_cache_enabled: true,
        })
    }
    
    /// Handle network error with proper error propagation (.rules compliance)
    pub fn handle_network_error(
        &mut self,
        error: NetworkError,
        cx: &mut Context<Self>,
    ) -> Result<ErrorHandlingStrategy> {
        // Record error in history with bounds checking
        self.error_history.push_back((error.clone(), Instant::now()));
        if self.error_history.len() > self.max_error_history {
            self.error_history.pop_front();
        }
        
        // Update connection status based on error type
        let strategy = match &error {
            NetworkError::ConnectionFailed { retry_after, .. } => {
                self.connection_status = ConnectionStatus::Offline;
                self.rate_limiter.record_failure();
                
                if self.auto_retry_enabled {
                    ErrorHandlingStrategy::RetryWithBackoff {
                        delay: retry_after.unwrap_or_else(|| self.rate_limiter.get_backoff_duration()),
                        max_attempts: self.max_retry_attempts,
                    }
                } else if self.fallback_to_cache_enabled {
                    ErrorHandlingStrategy::FallbackToCache
                } else {
                    ErrorHandlingStrategy::ShowError { user_message: error.to_string() }
                }
            }
            NetworkError::Timeout { .. } => {
                self.rate_limiter.record_failure();
                
                if self.auto_retry_enabled && self.rate_limiter.get_failure_count() < self.max_retry_attempts {
                    ErrorHandlingStrategy::RetryWithBackoff {
                        delay: self.rate_limiter.get_backoff_duration(),
                        max_attempts: self.max_retry_attempts,
                    }
                } else {
                    ErrorHandlingStrategy::FallbackToCache
                }
            }
            NetworkError::RateLimitExceeded { retry_after, .. } => {
                self.connection_status = ConnectionStatus::RateLimited { retry_after: *retry_after };
                
                ErrorHandlingStrategy::QueueRequest {
                    retry_after: *retry_after,
                }
            }
            NetworkError::ServiceUnavailable { .. } => {
                self.connection_status = ConnectionStatus::Degraded {
                    reason: "Service temporarily unavailable".to_string(),
                };
                
                if self.fallback_to_cache_enabled {
                    ErrorHandlingStrategy::FallbackToCache
                } else {
                    ErrorHandlingStrategy::ShowError { user_message: error.to_string() }
                }
            }
            NetworkError::InvalidResponse { .. } => {
                // Invalid response might indicate a temporary issue
                if self.fallback_to_cache_enabled {
                    ErrorHandlingStrategy::FallbackToCache
                } else {
                    ErrorHandlingStrategy::ShowError { user_message: error.to_string() }
                }
            }
            NetworkError::AuthenticationFailed { .. } => {
                // Authentication errors should not retry automatically
                ErrorHandlingStrategy::ShowError {
                    user_message: "Authentication failed. Please check your credentials.".to_string(),
                }
            }
            NetworkError::Offline => {
                self.connection_status = ConnectionStatus::Offline;
                self.offline_mode_enabled = true;
                
                ErrorHandlingStrategy::FallbackToCache
            }
        };
        
        // Emit event for UI updates
        cx.emit(ErrorHandlerEvent::ErrorOccurred {
            error: error.clone(),
            strategy: strategy.clone(),
        });
        cx.emit(ErrorHandlerEvent::ConnectionStatusChanged(self.connection_status.clone()));
        cx.notify();
        
        Ok(strategy)
    }
    
    /// Check rate limit before making request (.rules compliance)
    pub fn check_rate_limit(&mut self, cx: &mut Context<Self>) -> Result<()> {
        match self.rate_limiter.check_rate_limit() {
            Ok(()) => Ok(()),
            Err(error) => {
                // Handle rate limit error
                let strategy = self.handle_network_error(error.clone(), cx)?;
                
                match strategy {
                    ErrorHandlingStrategy::QueueRequest { retry_after } => {
                        Err(anyhow!("Rate limit exceeded. Retry after {:?}", retry_after))
                    }
                    _ => Err(anyhow!("Rate limit exceeded")),
                }
            }
        }
    }
    
    /// Record successful operation
    pub fn record_success(&mut self, cx: &mut Context<Self>) {
        self.rate_limiter.record_success();
        
        // Update connection status if we were offline
        if matches!(self.connection_status, ConnectionStatus::Offline | ConnectionStatus::Degraded { .. }) {
            self.connection_status = ConnectionStatus::Online;
            self.offline_mode_enabled = false;
            cx.emit(ErrorHandlerEvent::ConnectionStatusChanged(self.connection_status.clone()));
            cx.notify();
        }
    }
    
    /// Get current connection status
    pub fn get_connection_status(&self) -> &ConnectionStatus {
        &self.connection_status
    }
    
    /// Check if offline mode is enabled
    pub fn is_offline_mode(&self) -> bool {
        self.offline_mode_enabled
    }
    
    /// Get error history with bounds checking (.rules compliance)
    pub fn get_recent_errors(&self, count: usize) -> Vec<(NetworkError, Instant)> {
        self.error_history
            .iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }
    
    /// Clear error history
    pub fn clear_error_history(&mut self) {
        self.error_history.clear();
    }
    
    /// Set auto-retry configuration
    pub fn set_auto_retry(&mut self, enabled: bool, max_attempts: u32) {
        self.auto_retry_enabled = enabled;
        self.max_retry_attempts = max_attempts;
    }
    
    /// Set fallback to cache configuration
    pub fn set_fallback_to_cache(&mut self, enabled: bool) {
        self.fallback_to_cache_enabled = enabled;
    }
    
    /// Configure rate limiter
    pub fn configure_rate_limiter(&mut self, max_requests: u32, window: Duration) {
        self.rate_limiter = RateLimiter::new(max_requests, window);
    }
    
    /// Force offline mode
    pub fn set_offline_mode(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.offline_mode_enabled = enabled;
        self.connection_status = if enabled {
            ConnectionStatus::Offline
        } else {
            ConnectionStatus::Online
        };
        cx.emit(ErrorHandlerEvent::ConnectionStatusChanged(self.connection_status.clone()));
        cx.notify();
    }
}

/// Error handling strategy enumeration
#[derive(Debug, Clone)]
pub enum ErrorHandlingStrategy {
    RetryWithBackoff { delay: Duration, max_attempts: u32 },
    QueueRequest { retry_after: Duration },
    FallbackToCache,
    ShowError { user_message: String },
    Ignore,
}

/// Error handler events
#[derive(Clone, Debug)]
pub enum ErrorHandlerEvent {
    ErrorOccurred { error: NetworkError, strategy: ErrorHandlingStrategy },
    ConnectionStatusChanged(ConnectionStatus),
    RateLimitWarning { current_usage: u32, limit: u32 },
    OfflineModeEnabled,
    OfflineModeDisabled,
}

impl EventEmitter<ErrorHandlerEvent> for NetworkErrorHandler {}

impl Render for NetworkErrorHandler {
    fn render(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        gpui::div() // Error handler doesn't render UI directly
    }
}

/// Helper function to create network error from HTTP status code
pub fn network_error_from_status(status_code: u16, message: String) -> NetworkError {
    match status_code {
        429 => NetworkError::RateLimitExceeded {
            retry_after: Duration::from_secs(60),
            limit: 100,
        },
        503 => NetworkError::ServiceUnavailable {
            service: "API".to_string(),
            estimated_recovery: None,
        },
        401 | 403 => NetworkError::AuthenticationFailed {
            reason: message,
        },
        _ => NetworkError::ConnectionFailed {
            message,
            retry_after: Some(Duration::from_secs(5)),
        },
    }
}

/// Helper function to determine if error is retryable
pub fn is_retryable_error(error: &NetworkError) -> bool {
    matches!(
        error,
        NetworkError::ConnectionFailed { .. }
            | NetworkError::Timeout { .. }
            | NetworkError::ServiceUnavailable { .. }
    )
}

/// Helper function to get retry delay for error
pub fn get_retry_delay(error: &NetworkError, attempt: u32) -> Duration {
    match error {
        NetworkError::ConnectionFailed { retry_after, .. } => {
            retry_after.unwrap_or_else(|| Duration::from_secs(2_u64.pow(attempt).min(60)))
        }
        NetworkError::Timeout { .. } => Duration::from_secs(2_u64.pow(attempt).min(30)),
        NetworkError::RateLimitExceeded { retry_after, .. } => *retry_after,
        NetworkError::ServiceUnavailable { .. } => Duration::from_secs(10 * (attempt as u64).min(6)),
        _ => Duration::from_secs(5),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rate_limiter_basic() {
        let mut limiter = RateLimiter::new(3, Duration::from_secs(1));
        
        // First 3 requests should succeed
        assert!(limiter.check_rate_limit().is_ok());
        assert!(limiter.check_rate_limit().is_ok());
        assert!(limiter.check_rate_limit().is_ok());
        
        // 4th request should fail
        assert!(limiter.check_rate_limit().is_err());
    }
    
    #[test]
    fn test_rate_limiter_backoff() {
        let mut limiter = RateLimiter::new(10, Duration::from_secs(60));
        
        // Record failures and check backoff increases
        let initial_backoff = limiter.get_backoff_duration();
        limiter.record_failure();
        let after_one_failure = limiter.get_backoff_duration();
        limiter.record_failure();
        let after_two_failures = limiter.get_backoff_duration();
        
        assert!(after_one_failure > initial_backoff);
        assert!(after_two_failures > after_one_failure);
    }
    
    #[test]
    fn test_connection_status_display() {
        let status = ConnectionStatus::Online;
        assert_eq!(status.get_display_message(), "Connected");
        
        let status = ConnectionStatus::Offline;
        assert!(status.get_display_message().contains("Offline"));
        
        let status = ConnectionStatus::RateLimited {
            retry_after: Duration::from_secs(30),
        };
        assert!(status.get_display_message().contains("Rate limited"));
    }
}
