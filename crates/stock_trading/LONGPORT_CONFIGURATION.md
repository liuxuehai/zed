# Longport API Configuration Guide

This guide explains how to configure the Longport API integration for the Stock Trading System in Zed Lite.

## Overview

The Stock Trading System supports integration with Longport API for real market data. This integration provides:
- Real-time stock quotes
- Historical candlestick data (K-line charts)
- Order book depth data
- Stock fundamental information
- WebSocket streaming for live updates

## Configuration

### 1. Obtain Longport API Credentials

Before configuring the system, you need to obtain API credentials from Longport:

1. Visit [Longport Developer Portal](https://open.longportapp.com/)
2. Create an account or sign in
3. Navigate to API Management
4. Create a new application to get:
   - **App Key**: Your application identifier
   - **App Secret**: Your application secret key
   - **Access Token**: Your authentication token

### 2. Configure Settings in Zed Lite

Add the following configuration to your Zed Lite settings file (typically `~/.config/zed-lite/settings.json`):

```json
{
  "stock_trading": {
    "use_mock_data": false,
    "longport": {
      "enabled": true,
      "app_key": "YOUR_APP_KEY_HERE",
      "app_secret": "YOUR_APP_SECRET_HERE",
      "access_token": "YOUR_ACCESS_TOKEN_HERE",
      "use_for_realtime": true,
      "use_for_historical": true,
      "rate_limit_per_minute": 30,
      "auto_fallback_to_mock": true
    }
  }
}
```

### 3. Configuration Options

#### Required Fields (when `enabled: true`)

- **`app_key`** (string): Your Longport application key
- **`app_secret`** (string): Your Longport application secret
- **`access_token`** (string): Your Longport access token

#### Optional Fields

- **`enabled`** (boolean, default: `false`): Enable/disable Longport integration
- **`api_endpoint`** (string, optional): Custom API endpoint URL (uses Longport default if not specified)
- **`use_for_realtime`** (boolean, default: `true`): Use Longport for real-time data
- **`use_for_historical`** (boolean, default: `true`): Use Longport for historical data
- **`daily_quota_limit`** (number, optional): Daily API call quota limit for monitoring
- **`rate_limit_per_minute`** (number, default: `30`): Maximum API requests per minute
- **`auto_fallback_to_mock`** (boolean, default: `true`): Automatically fallback to mock data on API errors

## Usage Modes

### Development Mode (Mock Data)

For development and testing without API credentials:

```json
{
  "stock_trading": {
    "use_mock_data": true,
    "longport": {
      "enabled": false
    }
  }
}
```

### Production Mode (Real Data)

For production use with real market data:

```json
{
  "stock_trading": {
    "use_mock_data": false,
    "longport": {
      "enabled": true,
      "app_key": "YOUR_APP_KEY",
      "app_secret": "YOUR_APP_SECRET",
      "access_token": "YOUR_ACCESS_TOKEN",
      "daily_quota_limit": 10000,
      "rate_limit_per_minute": 30
    }
  }
}
```

### Hybrid Mode (Fallback Support)

Use real data with automatic fallback to mock data on errors:

```json
{
  "stock_trading": {
    "use_mock_data": false,
    "longport": {
      "enabled": true,
      "app_key": "YOUR_APP_KEY",
      "app_secret": "YOUR_APP_SECRET",
      "access_token": "YOUR_ACCESS_TOKEN",
      "auto_fallback_to_mock": true
    }
  }
}
```

## API Quota Management

The system includes built-in API quota monitoring to help you stay within Longport's usage limits:

### Setting Quota Limits

```json
{
  "stock_trading": {
    "longport": {
      "daily_quota_limit": 10000
    }
  }
}
```

### Quota Monitoring

The system automatically:
- Tracks API usage count
- Prevents requests when quota is exceeded
- Falls back to mock data if `auto_fallback_to_mock` is enabled
- Resets usage count daily (requires manual reset or application restart)

## Rate Limiting

To comply with Longport API rate limits:

```json
{
  "stock_trading": {
    "longport": {
      "rate_limit_per_minute": 30
    }
  }
}
```

The system will:
- Queue requests when rate limit is approached
- Implement exponential backoff on rate limit errors
- Distribute requests evenly across the time window

## Error Handling

### Validation Errors

The system validates configuration on startup and provides clear error messages:

- **Missing credentials**: "Longport app_key is required when Longport integration is enabled"
- **Invalid endpoint**: "Longport api_endpoint must start with http:// or https://"
- **Invalid rate limit**: "Longport rate_limit_per_minute must be greater than 0"

### Runtime Errors

When API errors occur:
1. Error is logged with details
2. If `auto_fallback_to_mock` is enabled, switches to mock data
3. User is notified through UI error messages
4. System continues operating with degraded functionality

## Security Best Practices

### 1. Protect Your Credentials

- **Never commit credentials to version control**
- Store credentials in user-specific settings files
- Use environment variables for sensitive data in production
- Rotate access tokens regularly

### 2. Use Environment Variables (Advanced)

For production deployments, consider using environment variables:

```bash
export LONGPORT_APP_KEY="your_app_key"
export LONGPORT_APP_SECRET="your_app_secret"
export LONGPORT_ACCESS_TOKEN="your_access_token"
```

Then reference them in your configuration (requires custom implementation).

### 3. Restrict API Permissions

When creating Longport API credentials:
- Request only necessary permissions (read-only for market data)
- Set IP restrictions if available
- Monitor API usage regularly

## Troubleshooting

### Issue: "Quote context not initialized"

**Solution**: Ensure Longport credentials are correctly configured and the service has been initialized.

### Issue: API quota exceeded

**Solution**: 
1. Check your `daily_quota_limit` setting
2. Reduce refresh frequency in settings
3. Enable `auto_fallback_to_mock` for graceful degradation

### Issue: Connection timeout

**Solution**:
1. Check your internet connection
2. Verify API endpoint is accessible
3. Increase `request_timeout` in API configuration
4. Check Longport service status

### Issue: Invalid credentials error

**Solution**:
1. Verify credentials are correct (no extra spaces)
2. Check if access token has expired
3. Regenerate credentials from Longport portal
4. Ensure credentials have necessary permissions

## Example: Complete Configuration

```json
{
  "stock_trading": {
    "default_watchlist": ["AAPL", "GOOGL", "MSFT", "TSLA"],
    "default_timeframe": "one_day",
    "auto_refresh_interval": 30,
    "use_mock_data": false,
    
    "api": {
      "request_timeout": 30,
      "max_retry_attempts": 3,
      "rate_limit_per_minute": 60
    },
    
    "longport": {
      "enabled": true,
      "app_key": "YOUR_APP_KEY_HERE",
      "app_secret": "YOUR_APP_SECRET_HERE",
      "access_token": "YOUR_ACCESS_TOKEN_HERE",
      "use_for_realtime": true,
      "use_for_historical": true,
      "daily_quota_limit": 10000,
      "rate_limit_per_minute": 30,
      "auto_fallback_to_mock": true
    },
    
    "websocket": {
      "enabled": true,
      "max_reconnect_attempts": 10,
      "reconnect_delay": 5,
      "heartbeat_interval": 30
    },
    
    "cache": {
      "market_data_cache_duration": 60,
      "historical_data_cache_duration": 300,
      "auto_cleanup_enabled": true
    }
  }
}
```

## Support

For issues related to:
- **Longport API**: Contact Longport support or visit their documentation
- **Stock Trading System**: Check the project documentation or file an issue
- **Zed Lite Integration**: Refer to Zed Lite documentation

## References

- [Longport API Documentation](https://open.longportapp.com/docs)
- [Longport SDK for Rust](https://github.com/longportapp/openapi-sdk)
- Stock Trading System Design Document: `.kiro/specs/stock-trading-system/design.md`
- Stock Trading System Requirements: `.kiro/specs/stock-trading-system/requirements.md`
