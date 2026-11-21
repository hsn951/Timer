# 开发与发版说明

## 环境

- Windows 10 1809+ / 11
- Rust stable（MSVC 工具链）+ Visual Studio Build Tools（C++ 工作负载）+ Windows SDK
- Node.js 20+、pnpm

## 本地开发

```powershell
pnpm install
pnpm dev            # 浏览器预览（mock 数据）
# 或
.\scripts\build.ps1 # 构建前端 + Rust Release
.\scripts\run.ps1   # 运行
```

## 目录约定

| 路径 | 说明 |
|---|---|
| `src/` | 前端（原生 ES Module，无框架） |
| `src-tauri/src/` | Rust：engine / commands / audio / tray / store |
| `docs/screenshots/` | 界面截图（文档用） |
| `scripts/` | 构建、图标生成、版本号脚本 |
| `dist/`、`src-tauri/target/`、`release/` | 构建产物，不提交 |

## 版本规则

采用语义化版本 `MAJOR.MINOR.PATCH`，三处版本号必须一致：

- `package.json` → `version`
- `src-tauri/Cargo.toml` → `version`
- `src-tauri/tauri.conf.json` → `version`

用脚本一次改完：

```powershell
.\scripts\bump-version.ps1 0.2.0
```

同时更新 `CHANGELOG.md`，把 `[Unreleased]` 改为新版本并补一条链接。

## 发布流程

1. 改版本号：`.\scripts\bump-version.ps1 0.2.0`
2. 更新 `CHANGELOG.md`
3. 提交：`git commit -am "chore(release): v0.2.0"`
4. 打标签并推送：`git tag v0.2.0; git push origin main --tags`
5. GitHub Actions（`.github/workflows/release.yml`）会自动在 `windows-latest` 上构建 NSIS 安装包并创建 **Draft Release**，检查后手动 Publish 即可

也可以在本地出安装包：

```powershell
pnpm add -D @tauri-apps/cli@^2   # 只需一次
pnpm tauri build                # 产物在 src-tauri/target/release/bundle/
```

## 代码约定

- Rust：`cargo fmt`、`cargo clippy`
- 前端：保持零框架、零大依赖；新增依赖前先看体积
- 提交信息：Conventional Commits（`feat:` / `fix:` / `chore:` / `docs:`）
