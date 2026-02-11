# Stock Trading System Keyboard Shortcuts

This document describes the keyboard shortcuts available in the Stock Trading System.

## Global Actions

These actions can be triggered from anywhere in the application:

- **Toggle Watchlist Panel**: Opens or closes the watchlist panel
  - Action: `stock_trading_panels::ToggleWatchlistPanel`
  
- **Toggle Chart Panel**: Opens or closes the chart panel
  - Action: `stock_trading_panels::ToggleChartPanel`
  
- **Toggle Stock Info Panel**: Opens or closes the stock information panel
  - Action: `stock_trading_panels::ToggleStockInfoPanel`
  
- **Toggle Order Panel**: Opens or closes the order entry panel
  - Action: `stock_trading_panels::ToggleOrderPanel`
  
- **Toggle Order Book Panel**: Opens or closes the order book panel
  - Action: `stock_trading_panels::ToggleOrderBookPanel`

## Watchlist Panel Actions

These actions are available when the watchlist panel is focused:

- **Refresh Market Data**: Refreshes market data for all stocks in the watchlist
  - Action: `stock_trading_panels::RefreshMarketData`
  
- **Focus Add Stock Input**: Focuses the stock symbol input field
  - Action: `stock_trading_panels::FocusAddStockInput`
  
- **Add Stock to Watchlist**: Adds a stock to the watchlist
  - Action: `stock_trading_panels::AddStockToWatchlist`
  - Parameters: `{ symbol: "AAPL" }`
  
- **Remove Stock from Watchlist**: Removes a stock from the watchlist
  - Action: `stock_trading_panels::RemoveStockFromWatchlist`
  - Parameters: `{ index: 0 }`
  
- **Select Stock**: Selects a stock from the watchlist
  - Action: `stock_trading_panels::SelectStock`
  - Parameters: `{ index: 0 }`

## Order Panel Actions

These actions are available when the order panel is focused:

- **Submit Order**: Places the order with current form values
  - Action: `stock_trading_panels::SubmitOrder`
  
- **Clear Order Form**: Clears all input fields in the order form
  - Action: `stock_trading_panels::ClearOrderForm`
  
- **Focus Quantity Input**: Focuses the quantity input field
  - Action: `stock_trading_panels::FocusQuantityInput`
  
- **Focus Price Input**: Focuses the price input field (limit orders only)
  - Action: `stock_trading_panels::FocusPriceInput`

## Configuring Keyboard Shortcuts

To configure keyboard shortcuts for these actions, add them to your Zed keymap configuration file (`.config/zed/keymap.json`):

```json
[
  {
    "context": "Workspace",
    "bindings": {
      "ctrl-shift-w": "stock_trading_panels::ToggleWatchlistPanel",
      "ctrl-shift-c": "stock_trading_panels::ToggleChartPanel",
      "ctrl-shift-i": "stock_trading_panels::ToggleStockInfoPanel",
      "ctrl-shift-o": "stock_trading_panels::ToggleOrderPanel",
      "ctrl-shift-b": "stock_trading_panels::ToggleOrderBookPanel"
    }
  },
  {
    "context": "WatchlistPanel",
    "bindings": {
      "f5": "stock_trading_panels::RefreshMarketData",
      "ctrl-n": "stock_trading_panels::FocusAddStockInput"
    }
  },
  {
    "context": "OrderPanel",
    "bindings": {
      "ctrl-enter": "stock_trading_panels::SubmitOrder",
      "escape": "stock_trading_panels::ClearOrderForm"
    }
  }
]
```

## UI Interactions

### Watchlist Panel

- **Click on stock row**: Selects the stock and loads its data in other panels
- **Click "Remove" button**: Removes the stock from the watchlist
- **Type in symbol input**: Enter a stock symbol (automatically converted to uppercase)
- **Click "Add" button**: Adds the stock to the watchlist
- **Click "Refresh" button**: Refreshes market data for all stocks

### Chart Panel

- **Click timeframe buttons**: Changes the chart timeframe (1D, 1W, 1M)
- **Chart interactions**: Zoom and pan functionality (provided by gpui-component Chart)

### Order Panel

- **Click "Buy" or "Sell"**: Selects the order side
- **Click "Market" or "Limit"**: Selects the order type
- **Type in quantity field**: Enter the number of shares (numeric only)
- **Type in price field**: Enter the limit price (numeric with decimal, shown for limit orders only)
- **Click "Place Order"**: Submits the order
- **Click "Clear"**: Clears the order form

### Order Book Panel

- **View bid/ask levels**: See the top 10 price levels for bids and asks
- **Spread information**: View the current spread and mid price

## Input Validation

All input fields include validation with helpful error messages:

### Stock Symbol Validation
- Cannot be empty
- Must contain only alphanumeric characters
- Must be 1-5 characters long
- Cannot be a duplicate of existing watchlist items

### Order Quantity Validation
- Cannot be empty
- Must be a valid positive integer
- Must be greater than zero
- Must be less than 1,000,000 shares

### Order Price Validation (Limit Orders)
- Cannot be empty
- Must be a valid positive number
- Must be greater than zero
- Must be less than $1,000,000

All validation errors are displayed with clear, actionable messages to help users correct their input.
