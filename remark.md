1.AI 写一个支持自定义ai的provider


## 数据目录隔离

本项目使用独立目录，与官方 Zed 完全隔离。

### 目录对比

| 数据类型 | 官方 Zed | 本项目 |
|---------|---------|--------|
| 配置 (settings/keymap/tasks) | `%APPDATA%\Zed\` | `%APPDATA%\ZedDev\config\` |
| 扩展 (extensions) | `%LOCALAPPDATA%\Zed\extensions\` | `%APPDATA%\ZedDev\extensions\` |
| 数据库 | `%LOCALAPPDATA%\Zed\db\` | `%APPDATA%\ZedDev\db\` |
| 语言服务器 | `%LOCALAPPDATA%\Zed\languages\` | `%APPDATA%\ZedDev\languages\` |
| 日志 | `%LOCALAPPDATA%\Zed\logs\` | `%APPDATA%\ZedDev\logs\` |
| **登录信息** | Windows Keychain | `%APPDATA%\ZedDev\config\development_credentials` |

### 说明

- Release channel 为 `dev`，登录信息存储在文件而非 Windows 凭据管理器，避免与官方 Zed 的 Keychain 条目冲突。
- 登录信息文件路径：`C:\Users\<用户名>\AppData\Roaming\ZedDev\config\development_credentials`
- 如需强制使用系统 Keychain，设置环境变量 `ZED_DEVELOPMENT_USE_KEYCHAIN=1`。
- 如需自定义目录，启动时传入参数 `--user-data-dir <路径>`。
