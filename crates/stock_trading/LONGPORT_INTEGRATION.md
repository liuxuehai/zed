# Longport API 集成指南

## 概述

本文档说明如何将长桥（Longport）SDK 集成到 stock_trading 系统中，替换现有的 mock 数据源。

## 依赖配置

已在 `Cargo.toml` 中添加：

```toml
longport = "3.0.17"
tokio = { version = "1.0", features = ["sync", "rt-multi-thread", "macros", "time"] }
```

## 实现状态

### 已完成
- ✅ 添加 longport 依赖
- ✅ 创建 `longport_service.rs` 基础框架
- ✅ 定义 LongportService 结构

### 待完成
- [ ] 修复 Longport SDK API 类型不匹配问题
- [ ] 实现正确的 Quote 数据转换
- [ ] 实现正确的 Candlestick 数据转换
- [ ] 实现正确的 Depth (OrderBook) 数据转换
- [ ] 添加实时订阅功能
- [ ] 集成到 TradingManager
- [ ] 添加配置管理（API密钥存储）
- [ ] 实现错误处理和重试逻辑

## API 类型映射问题

当前遇到的主要问题：

1. **Decimal 类型**: Longport 使用 `Decimal` 类型表示价格，需要转换为 `f64`
2. **Period 枚举**: Longport 的 Period 枚举名称与预期不同
3. **Timestamp 类型**: Longport 使用 `OffsetDateTime`，需要转换为 `SystemTime`
4. **Quote 结构**: 需要查看实际的 Quote 结构字段名称

## 建议的实现步骤

### 步骤 1: 查看 Longport SDK 文档

```bash
# 查看 longport crate 文档
cargo doc --open -p longport
```

### 步骤 2: 创建类型转换辅助函数

```rust
// 将 Longport Decimal 转换为 f64
fn decimal_to_f64(decimal: Decimal) -> f64 {
    // 实现转换逻辑
}

// 将 OffsetDateTime 转换为 SystemTime
fn offset_datetime_to_system_time(dt: OffsetDateTime) -> SystemTime {
    // 实现转换逻辑
}
```

### 步骤 3: 更新 LongportService 实现

根据实际的 Longport SDK API 更新所有方法实现。

### 步骤 4: 添加配置管理

在 `StockTradingSettings` 中添加 Longport API 配置：

```rust
pub struct LongportConfig {
    pub app_key: String,
    pub app_secret: String,
    pub access_token: String,
    pub use_longport: bool,  // 是否使用真实数据
}
```

### 步骤 5: 集成到 TradingManager

修改 `TradingManager` 以支持 Longport 数据源：

```rust
pub struct TradingManager {
    data_service: Entity<DataService>,
    longport_service: Option<Entity<LongportService>>,
    use_real_data: bool,
    // ... 其他字段
}
```

### 步骤 6: 实现数据源切换

添加运行时切换 mock 数据和真实数据的功能。

## 安全注意事项

1. **API 密钥存储**: 不要将 API 密钥硬编码在代码中
2. **使用环境变量或配置文件**: 
   ```
   LONGPORT_APP_KEY=your_app_key
   LONGPORT_APP_SECRET=your_app_secret
   LONGPORT_ACCESS_TOKEN=your_access_token
   ```
3. **加密存储**: 考虑使用系统密钥链存储敏感信息

## 测试策略

1. **单元测试**: 测试类型转换函数
2. **集成测试**: 使用测试账号测试 API 调用
3. **Mock 测试**: 保留 mock 数据用于开发和测试

## 下一步行动

1. 查看 Longport SDK 3.0.17 的实际 API 文档
2. 根据实际 API 修复类型不匹配问题
3. 实现完整的数据转换逻辑
4. 添加错误处理和日志记录
5. 集成到现有系统

## 参考资源

- Longport SDK GitHub: https://github.com/longportapp/openapi-sdk
- Longport API 文档: https://open.longportapp.com/docs
- Rust SDK 文档: https://docs.rs/longport/latest/longport/
