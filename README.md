# wallflow

macOS 自动更换壁纸。没有界面，全部走 TOML 配置。登录后由 LaunchAgent 常驻，按间隔换桌面。

## 配置

默认路径：

```text
~/Library/Application Support/wallflow/config.toml
```

完整带注释的示例见 `config.example.toml`。字段说明：

```toml
# 更换间隔，分钟，1–1440
interval_minutes = 30
# sequential 按顺序，random 随机
mode = "sequential"
# all 所有显示器，main 仅主屏
apply_to = "all"
# 息屏/合盖时挂起，等亮屏
pause_when_display_off = true
# 电池或低电量时不更换，等接入电源
pause_on_battery = true
# 远程图下载超时，秒，1–120
download_timeout_secs = 15

[[sources]]
url = "https://picsum.photos/id/1015/2560/1440"

# JSON 接口：每次更换现场请求，按 json_path 取图片地址
# data 可以是对象或数组，例如 data.url、data.0.url
[[sources]]
url = "https://wp.upx8.com/api.php?resolution=2560x1440&count=1&format=json"
json_path = "data.url"

[[sources]]
path = "~/Pictures/Wallpapers"
```

没有 `json_path` 时 `url` 当图片直链。有 `json_path` 时按点路径从 JSON 取地址，对象和数组都能拆。取到多张时按 `mode` 选一张。相对路径按接口地址补全。解析失败则跳过这次，桌面不动。

改配置后，下次间隔生效。

## 在 Mac 上编译和安装

```bash
cargo build --release
./target/release/wallflow --once
./target/release/wallflow --install
```

`--install` 会复制二进制、写出默认配置、安装 LaunchAgent。

卸载：

```bash
./target/release/wallflow --uninstall
```

或已安装后：

```bash
~/Library/Application\ Support/wallflow/wallflow --uninstall
```

`--uninstall` 会停止 LaunchAgent，并删除 `~/Library/LaunchAgents/com.wallflow.plist` 和 `~/Library/Application Support/wallflow/`。当前桌面壁纸保持不变。

手动常驻：

```bash
./target/release/wallflow --config ~/Library/Application\ Support/wallflow/config.toml
```

仅停止、不删除文件：

```bash
launchctl unload ~/Library/LaunchAgents/com.wallflow.plist
```

## 省电

- 屏幕关闭或睡眠时挂起等待系统亮屏通知，不轮询、不换壁纸；亮屏后若间隔已到则马上更换
- 电池供电或低电量时不更换壁纸，挂起等待接入电源
- 定时带 leeway，低优先级 IO，LaunchAgent 标成 Background
- 远程图只保留当前这一张：`~/Library/Application Support/wallflow/cache/`，换成功后删除旧缓存
