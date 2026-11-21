# 马卡龙计时器 · 开发计划 v0.2

> 2026-11-21 ｜ 状态：开发中（M0–M3 已完成，M4 进行中）
> 相较 v0.1 的变化：确认仅支持 Win10+；不做多计时器并行，改为"我的计时器"预设库；配色改为马卡龙浅色系；前端由 Svelte 改为零依赖原生 ES Module。

## 1. 产品定义

一个没有传统 Windows 窗口的悬浮计时器：无边框、圆角、浅色马卡龙渐变、常驻托盘。
一个时刻只跑一个计时器，但可以提前把常用时长存成"我的计时器"，点一下就开始。

**明确不做**：账号/云同步、手机端、任务管理、并行多计时器、自由拉伸窗口、视频/白噪音。

## 2. 关键技术决策（ADR）

| # | 决策 | 理由 | 代价 |
|---|---|---|---|
| 1 | Tauri 2 + Rust | 安装包个位数 MB，窗口能力（无边框/透明/置顶/托盘/快捷键）齐全 | 依赖 WebView2（Win10 1809+） |
| 2 | 前端零框架（原生 ESM + Vite） | 产物 ~17 KB，无构建链风险；计时 UI 用直接 DOM 更新足够 | 状态管理手写，页面多了会累 |
| 3 | 计时真值放 Rust | 窗口隐藏/WebView 节流都不影响计时与响铃 | UI 只能投影，需事件同步 |
| 4 | 单调时钟 + 墙钟双轨 | 不受改系统时间影响；休眠唤醒后能补回漏掉的时长 | 每 tick 多两次时钟读取 |
| 5 | 音频独立线程（rodio） | cpal 的 Stream 非 Send，不能放 Tauri State；播放与解码不阻塞 UI | 需要 channel 通信 |
| 6 | 导入即校验，不做预转码 | rodio 流式解码首帧很快（<50 ms），省掉 PCM 缓存与内存 | 极大文件首次解码稍慢 |
| 7 | 不做窗口自由缩放，固定三档 | 省掉 8 向 resize 与 DPI 边界 bug，符合"简洁" | 用户不能自定义尺寸 |
| 8 | Win10/11 不启用实时毛玻璃 | Acrylic 在拖动/缩放时掉帧（window-vibrancy 官方说明） | 用浅色渐变 + 细阴影替代 |

## 3. 架构

```
WebView2 (HTML/CSS/JS)
   │ invoke 命令            ▲ 事件 timer://update / state / finished
   ▼                        │
Rust 主进程 ── Mutex<Engine> ── 50ms tick 线程
   ├─ audio 线程（rodio + symphonia）
   ├─ store（settings / presets / sounds JSON，tmp+rename）
   ├─ tray（显示隐藏 / 开始暂停 / 静音 / 退出）
   └─ global shortcut（Ctrl+Alt+Space / Ctrl+Alt+T）
```

**状态机**：`Idle → Running ⇄ Paused → Finished → Idle`；番茄钟在 `Finished` 后按配置自动 `advance` 到下一阶段。
**精度**：`elapsed = accumulated + (now - started_at)`，每 tick 现算；检测到墙钟与单调钟增量差 > 2 s 时判定休眠并补偿。
**tick 策略**：仅 Running 时每 50 ms 发一次 `timer://update`；空闲完全停发（CPU≈0）。

## 4. 数据与接口

存储目录 `%APPDATA%\com.macaron.timer\`：

| 文件 | 内容 |
|---|---|
| settings.json | 音量、渐强、静音、选中铃声、置顶、自启、主题色、番茄参数、窗口模式 |
| presets.json | 我的计时器：`{id, name, ms, color}` |
| sounds.json | `{id, name, file, builtin}`，音频本体在 sounds\ |
| sounds\*.wav | 首次运行合成的两个内置铃声 + 用户导入的文件 |

主要命令：`bootstrap`、`start/pause/reset/lap`、`switch_kind`、`set_countdown`、`add_time`、`skip_phase`、
`start_preset`、`add_preset/remove_preset`、`update_settings`、`set_mode`、`import_sound/remove_sound/preview_sound`、`hide_window`。
事件：`timer://update`、`timer://state`、`timer://finished`、`sound://imported`、`settings://changed`、`toast`。

## 5. 里程碑与进度

| 阶段 | 内容 | 状态 |
|---|---|---|
| M0 骨架 | 无边框透明窗口、图标、体积基线 | ✅ |
| M1 内核 | 状态机、秒表/倒计时/番茄、休眠补偿 | ✅ 已真机验证（倒计时/番茄/完成态） |
| M2 界面 | 三页 UI、环形进度、迷你模式、马卡龙配色 | ✅ |
| M3 铃声/设置 | 导入校验、试听、音量渐强、托盘、单实例、自启 | ✅ 托盘已验证；导入路径代码完成 |
| M4 打磨 | 通知、快捷键、DPI、边界用例、内存 | ✅ 关闭到托盘/单实例/全局快捷键/125% DPI 已验证 |
| M5 发布 | Release 编译、体积测量、NSIS 安装包 | ✅ exe 5.1 MB；⏳ NSIS 安装包待打包 |

## 6. 验收清单

- [x] 无边框圆角窗口，无黑边；关闭收进托盘
- [x] 倒计时到点：响铃、环形填满、显示"时间到"、按钮变"再来一次"
- [x] 预设计时器可创建、持久化、一键启动
- [x] 迷你模式 300×96 可拖动、可播放/展开
- [x] 单实例：再次启动唤起已有窗口
- [x] 全局 `Ctrl+Alt+Space` 可开始/暂停，窗口隐藏时优先呼出
- [x] 置顶开关即时生效并持久化
- [ ] 8 小时长跑误差与睡眠唤醒（需长时间实测）
- [ ] 导入 mp3/wav/ogg/flac 全格式实测（代码路径已就绪）
- [ ] 150%/200% DPI 与双屏拖动
- [ ] NSIS 安装包与自动更新

## 7. 后续（v1.1+）

- 铃声循环次数 / 多铃声轮换
- 计时历史（最近 50 条）与简单统计
- 中英双语
- 自动更新（tauri-plugin-updater + GitHub Releases）
- 可选：WebView2 缺失时的引导安装页
