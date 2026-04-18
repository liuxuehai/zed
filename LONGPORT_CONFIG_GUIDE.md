# Zed Lite 长桥配置指南

## 配置文件位置

### Windows
```
C:\Users\<你的用户名>\AppData\Roaming\Zed\settings.json
```

或使用环境变量：
```
%APPDATA%\Zed\settings.json
```

### macOS
```
~/.config/zed/settings.json
```

### Linux
```
~/.config/zed/settings.json
```

## 快速配置步骤

### 1. 创建配置目录（如果不存在）

**Windows PowerShell:**
```powershell
# 创建目录
New-Item -ItemType Directory -Force -Path "$env:APPDATA\Zed"

# 打开配置文件
notepad "$env:APPDATA\Zed\settings.json"
```

**macOS/Linux:**
```bash
# 创建目录
mkdir -p ~/.config/zed

# 编辑配置文件
nano ~/.config/zed/settings.json
```

### 2. 配置文件内容

将以下内容粘贴到 `settings.json` 文件中：

```json
{
  "stock_trading": {
    "use_mock_data": false,
    "default_watchlist": ["AAPL", "TSLA", "BABA", "00700"],
    "default_timeframe": "1D",
    "auto_refresh_interval": 30,
    "longport": {
      "enabled": true,
      "app_key": "YOUR_LONGPORT_APP_KEY",
      "app_secret": "YOUR_LONGPORT_APP_SECRET",
      "access_token": "YOUR_LONGPORT_ACCESS_TOKEN",
      "use_for_realtime": true,
      "use_for_historical": true,
      "rate_limit_per_minute": 30,
      "auto_fallback_to_mock": true
    },
    "api": {
      "timeout_seconds": 30,
      "retry_attempts": 3,
      "retry_delay_ms": 1000
    },
    "panels": {
      "default_layout": "horizontal",
      "auto_save_layout": true,
      "show_demo_panel": true,
      "show_watchlist_panel": true,
      "show_chart_panel": true,
      "show_order_panel": true
    },
    "theme": {
      "positive_color": "#00ff00",
      "negative_color": "#ff0000",
      "neutral_color": "#888888",
      "chart_background": "#1e1e1e",
      "grid_color": "#333333"
    },
    "cache": {
      "enabled": true,
      "ttl_seconds": 60,
      "max_size_mb": 100
    },
    "websocket": {
      "enabled": true,
      "auto_reconnect": true,
      "reconnect_delay_ms": 5000,
      "heartbeat_interval_ms": 30000
    }
  }
}
```

### 3. 替换长桥 API 凭证

将配置文件中的以下占位符替换为你的实际凭证：

- `YOUR_LONGPORT_APP_KEY` → 你的长桥 App Key
- `YOUR_LONGPORT_APP_SECRET` → 你的长桥 App Secret
- `YOUR_LONGPORT_ACCESS_TOKEN` → 你的长桥 Access Token

### 4. 获取长桥 API 凭证

1. 访问 [长桥开放平台](https://open.longportapp.com/)
2. 注册并登录账号
3. 在开发者中心创建应用
4. 获取 App Key、App Secret 和 Access Token

## 验证配置

### 启动 zed_lite 并查看日志

**Windows PowerShell:**
```powershell
# 设置日志级别
$env:RUST_LOG="info"

# 运行 zed_lite
.\target\debug\zed_lite.exe
```

**macOS/Linux:**
```bash
# 设置日志级别
export RUST_LOG=info

# 运行 zed_lite
./target/debug/zed_lite
```

### 预期日志输出

**配置正确时：**
```
[INFO] LongportService initialized successfully
[INFO] Stock trading system initialized successfully
```

**配置错误时：**
```
[ERROR] Invalid Longport configuration: Longport app_key is required...
[WARN] Longport integration is disabled. Using mock data instead.
```

## 配置说明

### use_mock_data
- `false`: 使用长桥真实数据（需要配置 API 凭证）
- `true`: 使用模拟数据（用于开发和测试）

### longport.enabled
- `true`: 启用长桥集成
- `false`: 禁用长桥集成，即使 `use_mock_data` 为 `false` 也会使用模拟数据

### longport.auto_fallback_to_mock
- `true`: 当长桥 API 出错时自动回退到模拟数据
- `false`: 长桥 API 出错时直接返回错误

### rate_limit_per_minute
- 每分钟最大请求数
- 建议值：30（根据长桥 API 限制调整）

## 故障排除

### 问题 1: 配置文件不生效

**原因：** 配置文件位置错误

**解决方案：**
1. 确认配置文件在正确的位置：`%APPDATA%\Zed\settings.json`（Windows）
2. 不要把配置文件放在 `target/debug/` 目录下
3. 重启 zed_lite

### 问题 2: 长桥 API 认证失败

**原因：** API 凭证错误或过期

**解决方案：**
1. 检查 App Key、App Secret 和 Access Token 是否正确
2. 确认 Access Token 没有过期
3. 在长桥开放平台重新生成凭证

### 问题 3: 仍然显示模拟数据

**原因：** 配置未正确加载或长桥服务初始化失败

**解决方案：**
1. 启用调试日志查看详细信息：
   ```powershell
   $env:RUST_LOG="debug"
   .\target\debug\zed_lite.exe
   ```
2. 检查日志中的错误信息
3. 确认 JSON 格式正确（使用 JSON 验证工具）

### 问题 4: JSON 格式错误

**原因：** 配置文件 JSON 语法错误

**解决方案：**
1. 使用 JSON 验证工具检查语法
2. 确保所有字符串使用双引号
3. 确保最后一个属性后面没有逗号
4. 确保所有括号正确闭合

## 最小配置示例

如果只想启用长桥真实数据，最小配置如下：

```json
{
  "stock_trading": {
    "use_mock_data": false,
    "longport": {
      "enabled": true,
      "app_key": "YOUR_APP_KEY",
      "app_secret": "YOUR_APP_SECRET",
      "access_token": "YOUR_ACCESS_TOKEN"
    }
  }
}
```

## 注意事项

1. **API 凭证安全**：不要将包含真实 API 凭证的配置文件提交到版本控制系统
2. **配置文件权限**：确保配置文件只有你的用户账户可以读取
3. **API 限流**：注意长桥 API 的请求限制，避免超出配额
4. **数据延迟**：真实市场数据可能有延迟，具体取决于你的长桥账户类型

## 相关文档

- [长桥开放平台文档](https://open.longportapp.com/docs)
- [Zed Lite 技术规范](./zed-lite-technical-specification.md)
- [股票交易系统实现文档](./crates/stock_trading/TASK_12_IMPLEMENTATION.md)
