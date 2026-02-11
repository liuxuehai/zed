# Stock Trading System - Action System Integration

## Overview

The Stock Trading System integrates with Zed's action dispatch system to provide keyboard shortcuts and programmatic control over trading functionality. This document describes the action system architecture, available actions, and integration patterns.

## Architecture

### Action Definition

All actions are defined in `src/trading_actions.rs` using Zed's `actions!` macro. This follows Zed's established patterns for action registration and dispatch.

```rust
actions!(
    stock_trading,
    [
        ToggleWatchlistPanel,
        ToggleChartPanel,
        // ... more actions
    ]
);
```

### Action Registration

Actions are automatically registered when the stock trading system is initialized:

```rust
pub fn init(http_client: Arc<dyn HttpClient>, cx: &mut App) -> Result<()> {
    register_trading_actions(cx);
    // ... initialize services
}
```

### Action Handlers

Each panel registers its action handlers in its constructor using the `cx.on_action()` pattern:

```rust
impl WatchlistPanel {
    fn register_action_handlers(&mut self, cx: &mut Context<Self>) {
        cx.on_action(|this: &mut Self, _action: &RefreshMarketData, cx| {
            this.refresh_all_market_data(cx);
        });
        // ... more handlers
    }
}
```

## Available Actions

### Panel Toggle Actions

These actions can be triggered from anywhere in the application to show/hide trading panels:

- `stock_trading::ToggleWatchlistPanel` - Toggle watchlist panel visibility
- `stock_trading::ToggleChartPanel` - Toggle chart panel visibility
- `stock_trading::ToggleStockInfoPanel` - Toggle stock info panel visibility
- `stock_trading::ToggleOrderPanel` - Toggle order panel visibility
- `stock_trading::ToggleOrderBookPanel` - Toggle order book panel visibility

**Default Shortcuts:**
- `Ctrl+Shift+W` - Toggle Watchlist
- `Ctrl+Shift+C` - Toggle Chart
- `Ctrl+Shift+I` - Toggle Stock Info
- `Ctrl+Shift+O` - Toggle Order Panel
- `Ctrl+Shift+B` - Toggle Order Book

### Data Refresh Actions

- `stock_trading::RefreshMarketData` - Refresh market data for current context
- `stock_trading::RefreshAllPanels` - Refresh all panels simultaneously

**Default Shortcuts:**
- `Ctrl+Shift+R` - Refresh Market Data
- `F5` - Refresh (context-specific)

### Watchlist Actions

- `stock_trading::FocusAddStockInput` - Focus the stock symbol input field
- `stock_trading::AddStockToWatchlist` - Add stock to watchlist (with parameter)
- `stock_trading::RemoveStockFromWatchlist` - Remove stock from watchlist (with parameter)
- `stock_trading::SelectStock` - Select stock from watchlist (with parameter)
- `stock_trading::ClearWatchlist` - Clear all stocks from watchlist
- `stock_trading::ExportWatchlist` - Export watchlist to file
- `stock_trading::ImportWatchlist` - Import watchlist from file

**Default Shortcuts (Watchlist Panel):**
- `Ctrl+N` - Focus Add Stock Input
- `Ctrl+Shift+E` - Export Watchlist
- `Ctrl+Shift+I` - Import Watchlist
- `Ctrl+Shift+X` - Clear Watchlist
- `Delete` - Remove Selected Stock
- `Enter` - Select Stock

### Chart Actions

- `stock_trading::ZoomIn` - Zoom in on chart
- `stock_trading::ZoomOut` - Zoom out on chart
- `stock_trading::ResetZoom` - Reset chart zoom to default
- `stock_trading::PanLeft` - Pan chart left
- `stock_trading::PanRight` - Pan chart right
- `stock_trading::ToggleVolume` - Toggle volume display
- `stock_trading::ToggleIndicators` - Toggle technical indicators
- `stock_trading::CycleTimeFrame` - Cycle through timeframes
- `stock_trading::ToggleFullScreen` - Toggle chart fullscreen mode
- `stock_trading::ChangeTimeFrame` - Change to specific timeframe (with parameter)
- `stock_trading::SetChartStyle` - Set chart style (with parameter)
- `stock_trading::ToggleChartIndicator` - Toggle specific indicator (with parameter)

**Default Shortcuts (Chart Panel):**
- `Ctrl++` - Zoom In
- `Ctrl+-` - Zoom Out
- `Ctrl+0` - Reset Zoom
- `Left Arrow` - Pan Left
- `Right Arrow` - Pan Right
- `V` - Toggle Volume
- `I` - Toggle Indicators
- `T` - Cycle Timeframe
- `F11` - Toggle Fullscreen

### Order Actions

- `stock_trading::SubmitOrder` - Submit order with current form values
- `stock_trading::ClearOrderForm` - Clear order form
- `stock_trading::FocusQuantityInput` - Focus quantity input field
- `stock_trading::FocusPriceInput` - Focus price input field
- `stock_trading::ToggleOrderType` - Toggle between market and limit orders
- `stock_trading::ToggleOrderSide` - Toggle between buy and sell
- `stock_trading::PlaceOrderAction` - Place order with parameters
- `stock_trading::CancelOrder` - Cancel order (with parameter)

**Default Shortcuts (Order Panel):**
- `Ctrl+Enter` - Submit Order
- `Escape` - Clear Order Form
- `Ctrl+Q` - Focus Quantity Input
- `Ctrl+P` - Focus Price Input
- `Ctrl+T` - Toggle Order Type
- `Ctrl+S` - Toggle Order Side

### Settings Actions

- `stock_trading::OpenTradingSettings` - Open trading settings panel
- `stock_trading::ToggleMockData` - Toggle between mock and live data
- `stock_trading::ToggleRealTimeUpdates` - Toggle real-time updates
- `stock_trading::UpdateTradingSettings` - Update specific setting (with parameter)

**Default Shortcuts:**
- `Ctrl+Shift+S` - Open Trading Settings
- `Ctrl+Shift+M` - Toggle Mock Data
- `Ctrl+Shift+T` - Toggle Real-Time Updates

### Navigation Actions

- `stock_trading::NextPanel` - Navigate to next panel
- `stock_trading::PreviousPanel` - Navigate to previous panel
- `stock_trading::FocusWatchlist` - Focus watchlist panel
- `stock_trading::FocusChart` - Focus chart panel
- `stock_trading::FocusOrderEntry` - Focus order entry panel
- `stock_trading::FocusOrderBook` - Focus order book panel

## Parameterized Actions

Some actions accept parameters for more specific control:

### AddStockToWatchlist

```rust
#[derive(Clone, PartialEq, Debug, Deserialize)]
pub struct AddStockToWatchlist {
    pub symbol: String,
}
```

**Usage:**
```json
["stock_trading::AddStockToWatchlist", { "symbol": "AAPL" }]
```

### PlaceOrderAction

```rust
#[derive(Clone, PartialEq, Debug, Deserialize)]
pub struct PlaceOrderAction {
    pub symbol: String,
    pub side: String,      // "buy" or "sell"
    pub order_type: String, // "market" or "limit"
    pub quantity: u64,
    pub price: Option<f64>,
}
```

**Usage:**
```json
["stock_trading::PlaceOrderAction", {
    "symbol": "GOOGL",
    "side": "buy",
    "orderType": "limit",
    "quantity": 100,
    "price": 150.50
}]
```

### ChangeTimeFrame

```rust
#[derive(Clone, PartialEq, Debug, Deserialize)]
pub struct ChangeTimeFrame {
    pub timeframe: String, // "1m", "5m", "15m", "1h", "1d", "1w", "1m"
}
```

**Usage:**
```json
["stock_trading::ChangeTimeFrame", { "timeframe": "1h" }]
```

## Keymap Configuration

### User-Level Configuration

Users can customize keyboard shortcuts by editing their keymap configuration file at:
- Windows: `%APPDATA%\Zed\keymap.json`
- macOS: `~/.config/zed/keymap.json`
- Linux: `~/.config/zed/keymap.json`

### Example Configuration

```json
[
  {
    "context": "Workspace",
    "bindings": {
      "ctrl-shift-w": "stock_trading::ToggleWatchlistPanel",
      "ctrl-shift-c": "stock_trading::ToggleChartPanel"
    }
  },
  {
    "context": "WatchlistPanel",
    "bindings": {
      "f5": "stock_trading::RefreshMarketData",
      "ctrl-n": "stock_trading::FocusAddStockInput"
    }
  }
]
```

### Context-Specific Bindings

Actions can be bound to specific contexts to avoid conflicts:

- `Workspace` - Global actions available everywhere
- `WatchlistPanel` - Actions available when watchlist is focused
- `ChartPanel` - Actions available when chart is focused
- `OrderPanel` - Actions available when order panel is focused
- `OrderBookPanel` - Actions available when order book is focused

## Integration with Zed's Action System

### Action Dispatch Flow

1. User triggers keyboard shortcut or programmatic action
2. Zed's action system looks up the action by name
3. Action is dispatched to the focused entity
4. Entity's action handler is invoked
5. Handler executes the action logic

### Error Handling

All action handlers follow .rules compliance for error handling:

```rust
cx.on_action(|this: &mut Self, action: &AddStockToWatchlist, cx| {
    if let Err(error) = this.add_stock(action.symbol.clone(), cx) {
        error.log_err(); // Proper error handling - never unwrap()
    }
});
```

### Async Actions

For actions that require async operations:

```rust
cx.on_action(|this: &mut Self, _action: &RefreshMarketData, cx| {
    let task = cx.spawn(|this, mut cx| async move {
        // Async operation
        if let Err(error) = this.update(&mut cx, |this, cx| {
            this.fetch_data(cx)
        }) {
            error.log_err(); // Proper error propagation
        }
    });
    task.detach(); // Or store for cancellation
});
```

## Testing Actions

### Unit Tests

Action deserialization is tested in `trading_actions.rs`:

```rust
#[test]
fn test_add_stock_action_deserialization() {
    let json = r#"{"symbol": "AAPL"}"#;
    let action: AddStockToWatchlist = serde_json::from_str(json).unwrap();
    assert_eq!(action.symbol, "AAPL");
}
```

### Integration Tests

Action dispatch can be tested using GPUI's test framework:

```rust
#[gpui::test]
async fn test_action_dispatch(cx: &mut TestAppContext) {
    let panel = WatchlistPanel::new(trading_manager, cx);
    
    // Dispatch action
    cx.dispatch_action(AddStockToWatchlist {
        symbol: "AAPL".to_string(),
    });
    
    // Verify result
    assert!(panel.read(cx).has_symbol("AAPL"));
}
```

## Best Practices

### Action Naming

- Use full words, no abbreviations (e.g., `ToggleWatchlistPanel` not `ToggleWL`)
- Use PascalCase for action names
- Use descriptive names that clearly indicate the action's purpose

### Action Handlers

- Always use proper error handling with `?` or `.log_err()`
- Never use `unwrap()` or panic in action handlers
- Keep handlers focused on a single responsibility
- Use `cx.notify()` to trigger UI updates when state changes

### Keyboard Shortcuts

- Avoid conflicts with Zed's built-in shortcuts
- Use `Ctrl+Shift` prefix for global trading actions
- Use context-specific shortcuts for panel actions
- Document all shortcuts in user-facing documentation

### Parameter Validation

- Validate all action parameters before use
- Provide helpful error messages for invalid parameters
- Use the input validation system for consistency

## Future Enhancements

### Planned Actions

- `stock_trading::CreateAlert` - Create price alert
- `stock_trading::ManageAlerts` - Open alerts management
- `stock_trading::ExportHistory` - Export trading history
- `stock_trading::ImportSettings` - Import settings from file
- `stock_trading::ToggleTheme` - Toggle between light/dark themes
- `stock_trading::ShowHelp` - Show keyboard shortcuts help

### Planned Features

- Action recording and playback for automation
- Custom action sequences (macros)
- Action history and undo/redo support
- Voice command integration
- Gesture-based action triggers

## Troubleshooting

### Action Not Triggering

1. Check that the action is registered in `trading_actions.rs`
2. Verify the action handler is registered in the panel constructor
3. Ensure the correct context is focused
4. Check for keyboard shortcut conflicts

### Parameter Errors

1. Verify parameter names match the struct definition (use camelCase in JSON)
2. Check parameter types are correct
3. Ensure required parameters are provided
4. Validate parameter values are within acceptable ranges

### Performance Issues

1. Avoid heavy computation in action handlers
2. Use `cx.spawn()` for async operations
3. Debounce rapid action triggers
4. Cache frequently accessed data

## References

- [Zed Action System Documentation](https://zed.dev/docs/actions)
- [GPUI Event System](https://zed.dev/docs/gpui/events)
- [Keyboard Shortcuts Guide](./KEYBOARD_SHORTCUTS.md)
- [Trading Actions Source](./src/trading_actions.rs)

---

*This document follows Zed's technical specifications and coding guidelines (.rules compliance)*
