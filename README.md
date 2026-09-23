# wallflow

macOS 自动更换壁纸。没有界面，全部走 TOML 配置。登录后由 LaunchAgent 常驻，按间隔同时换桌面和锁屏。

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
# 守护进程是否输出运行日志，默认关闭
log_enabled = false
# 可选：日志文件路径，不填写到 stderr
# log_file = "~/Library/Logs/wallflow.log"

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

没有 `json_path` 时 `url` 当图片直链。有 `json_path` 时按点路径从 JSON 取地址，对象和数组都能拆。取到多张时按 `mode` 选一张。相对路径按接口地址补全。解析失败则跳过这次，当前壁纸不动。

每次更换会先设桌面，再把同一张图写入锁屏（Sonoma 及之后改 `Index.plist` 并重启 WallpaperAgent）。锁屏写入失败时桌面已经换完，只跳过锁屏。

改配置后保存即可，守护进程最多约 5 秒内重载并立即换一张，不用重启。

## 日志

默认不输出日志。要看日志，在 config 里打开开关并指定文件：

```toml
log_enabled = true
log_file = "~/Library/Logs/wallflow.log"
```

`log_file` 不填时日志写到 stderr；LaunchAgent 不会自动把 stderr 存成文件，所以装成守护进程后想留档就填 `log_file`。父目录会自动创建，文件以追加方式写入。改动保存后最多约 5 秒生效。

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
launchctl bootout gui/$(id -u)/com.wallflow
```

## 省电

- 屏幕关闭或睡眠时挂起等待系统亮屏通知，不轮询、不换壁纸；亮屏后若间隔已到则马上更换
- 电池供电或低电量时不更换壁纸，挂起等待接入电源
- 定时带 leeway，低优先级 IO，LaunchAgent 标成 Background
- 远程图只保留当前这一张：`~/Library/Application Support/wallflow/cache/`，换成功后删除旧缓存
