# GPUI Component 版本冲突说明

## 问题描述

`gpui-component` 是一个优秀的开源 GPUI UI 组件库（https://github.com/longbridge/gpui-component），提供了 60+ 跨平台桌面 UI 组件，包括：

- Button, Input, Table, Chart 等基础组件
- 虚拟化 Table 和 List（支持大数据集）
- Dock 布局系统
- Markdown 和 HTML 渲染
- 代码编辑器

然而，当前版本的 `gpui-component` (v0.5.1) 存在与 Zed 项目的依赖冲突：

```
gpui-component v0.5.1 requires: tree-sitter ^0.25.4
Zed workspace requires:        tree-sitter 0.26.2
```

## 冲突原因

`tree-sitter` 链接到原生 C 库，Cargo 不允许同一个依赖图中存在多个版本的原生库链接。这是 Cargo 的设计限制，用于确保最终二进制文件中只链接一份原生库。

错误信息：
```
error: failed to select a version for `tree-sitter`.
package `tree-sitter` links to the native library `tree-sitter`, but it conflicts with a previous package
Only one package in the dependency graph may specify the same links value.
```

## 尝试的解决方案

### 1. ✗ 使用 `[patch.crates-io]` 强制版本
```toml
[patch.crates-io]
tree-sitter = { version = "0.26", features = ["wasm"] }
```
**结果**: 失败 - patch 不能指向同一个源（crates.io）

### 2. ✗ 使用 git 依赖获取最新版本
```toml
gpui-component = { git = "https://github.com/longbridge/gpui-component", branch = "main" }
```
**结果**: 失败 - main 分支仍然依赖 tree-sitter 0.25，且引入了其他版本冲突（bitflags）

### 3. ✗ 禁用 default features
```toml
gpui-component = { version = "0.5.1", default-features = false }
```
**结果**: 失败 - tree-sitter 是核心依赖，无法通过 features 禁用

## 当前解决方案

**暂时禁用 `gpui-component` 依赖**，使用 Zed 内置的 `ui` crate 组件替代。

### Zed 内置 UI 组件

Zed 的 `ui` crate 提供了以下组件（位于 `crates/ui/src/components/`）：

- **基础组件**: Button, Label, Icon, Divider, Toggle, Radio
- **输入组件**: 需要自定义实现（参考 `gpui/examples/input.rs`）
- **布局组件**: h_flex, v_flex, Stack, Group
- **数据展示**: DataTable（支持虚拟化）
- **交互组件**: Modal, Popover, ContextMenu, DropdownMenu, Tooltip
- **其他**: Avatar, Banner, Callout, Indicator, Progress

### 迁移策略

`panels.rs` 中使用的 `gpui-component` 组件可以这样替换：

| gpui-component | Zed ui crate 替代方案 |
|----------------|----------------------|
| `Root` | `div()` with styling |
| `Button` | `ui::Button` |
| `Input` | 自定义实现（参考 gpui/examples/input.rs） |
| `Table` | `ui::DataTable` |
| `Chart` | 需要自定义实现或使用第三方库 |

## 未来解决方案

### 选项 1: 等待 gpui-component 更新

监控 `gpui-component` 仓库，等待其更新到支持 tree-sitter 0.26：
- GitHub: https://github.com/longbridge/gpui-component
- Crates.io: https://crates.io/crates/gpui-component

### 选项 2: 贡献 PR 到 gpui-component

向 `gpui-component` 项目提交 PR，升级其 tree-sitter 依赖到 0.26。

### 选项 3: Fork gpui-component

Fork `gpui-component` 并自行维护一个兼容 tree-sitter 0.26 的版本。

### 选项 4: 完全使用 Zed UI 组件

重写 `panels.rs`，完全使用 Zed 内置的 `ui` crate 组件，不依赖外部 UI 库。这是最稳定的长期方案。

## 当前状态

- ✅ `demo_panel.rs` - 使用 Zed 内置组件的简单演示面板（已实现）
- ❌ `panels.rs` - 完整的交易面板（因 gpui-component 冲突而禁用）
- 📋 **下一步**: 使用 Zed `ui` crate 重写 `panels.rs` 中的所有面板

## 参考资料

- [gpui-component GitHub](https://github.com/longbridge/gpui-component)
- [Cargo Links 文档](https://doc.rust-lang.org/cargo/reference/resolver.html#links)
- [Zed UI Components](../ui/src/components/)
- [GPUI Input Example](../gpui/examples/input.rs)
