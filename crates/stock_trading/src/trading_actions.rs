use gpui::actions;
use serde::Deserialize;

// Define all trading system actions using Zed's action system
// Following .rules: use full words for action names, no abbreviations
actions!(
    stock_trading,
    [
        // Panel toggle actions - can be triggered from anywhere
        ToggleWatchlistPanel,
        ToggleChartPanel,
        ToggleStockInfoPanel,
        ToggleOrderPanel,
        ToggleOrderBookPanel,
        ToggleStockTradingDemoPanel,  // Demo panel toggle action
        
        // Data refresh actions
        RefreshMarketData,
        RefreshAllPanels,
        
        // Watchlist panel actions
        FocusAddStockInput,
        ClearWatchlist,
        ExportWatchlist,
        ImportWatchlist,
        
        // Chart panel actions
        ZoomIn,
        ZoomOut,
        ResetZoom,
        PanLeft,
        PanRight,
        ToggleVolume,
        ToggleIndicators,
        CycleTimeFrame,
        ToggleFullScreen,
        
        // Order panel actions
        SubmitOrder,
        ClearOrderForm,
        FocusQuantityInput,
        FocusPriceInput,
        ToggleOrderType,
        ToggleOrderSide,
        
        // Order book panel actions
        FocusOrderBook,
        
        // Settings and configuration actions
        OpenTradingSettings,
        ToggleMockData,
        ToggleRealTimeUpdates,
        
        // Navigation actions
        NextPanel,
        PreviousPanel,
        FocusWatchlist,
        FocusChart,
        FocusOrderEntry,
    ]
);

/// Action with parameters for adding stock to watchlist
#[derive(Clone, PartialEq, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddStockToWatchlist {
    pub symbol: String,
}

/// Action with parameters for removing stock from watchlist
#[derive(Clone, PartialEq, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveStockFromWatchlist {
    pub index: usize,
}

/// Action with parameters for selecting stock
#[derive(Clone, PartialEq, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectStock {
    pub index: usize,
}

/// Action with parameters for changing timeframe
#[derive(Clone, PartialEq, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeTimeFrame {
    pub timeframe: String, // "1m", "5m", "15m", "1h", "1d", "1w", "1m"
}

/// Action with parameters for placing order
#[derive(Clone, PartialEq, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceOrderAction {
    pub symbol: String,
    pub side: String,      // "buy" or "sell"
    pub order_type: String, // "market" or "limit"
    pub quantity: u64,
    pub price: Option<f64>,
}

/// Action with parameters for canceling order
#[derive(Clone, PartialEq, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelOrder {
    pub order_id: String,
}

/// Action with parameters for setting zoom level
#[derive(Clone, PartialEq, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetZoomLevel {
    pub level: f64,
}

/// Action with parameters for setting pan offset
#[derive(Clone, PartialEq, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetPanOffset {
    pub offset: f64,
}

/// Action with parameters for selecting order book price level
#[derive(Clone, PartialEq, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectOrderBookLevel {
    pub price: f64,
    pub side: String, // "bid" or "ask"
}

/// Action with parameters for updating settings
#[derive(Clone, PartialEq, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTradingSettings {
    pub setting_name: String,
    pub value: String,
}

/// Action with parameters for subscribing to symbol
#[derive(Clone, PartialEq, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeToSymbol {
    pub symbol: String,
}

/// Action with parameters for unsubscribing from symbol
#[derive(Clone, PartialEq, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsubscribeFromSymbol {
    pub symbol: String,
}

/// Action with parameters for setting chart style
#[derive(Clone, PartialEq, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetChartStyle {
    pub style: String, // "candlestick", "line", "area"
}

/// Action with parameters for toggling chart indicator
#[derive(Clone, PartialEq, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToggleChartIndicator {
    pub indicator: String, // "sma", "ema", "rsi", "macd", etc.
}

/// Action with parameters for exporting data
#[derive(Clone, PartialEq, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportData {
    pub data_type: String, // "watchlist", "orders", "history"
    pub format: String,    // "json", "csv"
}

/// Action with parameters for importing data
#[derive(Clone, PartialEq, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportData {
    pub data_type: String, // "watchlist", "settings"
    pub file_path: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_add_stock_action_deserialization() {
        let json = r#"{"symbol": "AAPL"}"#;
        let action: AddStockToWatchlist = serde_json::from_str(json).unwrap();
        assert_eq!(action.symbol, "AAPL");
    }
    
    #[test]
    fn test_place_order_action_deserialization() {
        let json = r#"{
            "symbol": "GOOGL",
            "side": "buy",
            "orderType": "limit",
            "quantity": 100,
            "price": 150.50
        }"#;
        let action: PlaceOrderAction = serde_json::from_str(json).unwrap();
        assert_eq!(action.symbol, "GOOGL");
        assert_eq!(action.side, "buy");
        assert_eq!(action.order_type, "limit");
        assert_eq!(action.quantity, 100);
        assert_eq!(action.price, Some(150.50));
    }
    
    #[test]
    fn test_change_timeframe_action_deserialization() {
        let json = r#"{"timeframe": "1h"}"#;
        let action: ChangeTimeFrame = serde_json::from_str(json).unwrap();
        assert_eq!(action.timeframe, "1h");
    }
}
