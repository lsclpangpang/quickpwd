# QuickPwd

Windows 上用快捷键唤出的文本速贴工具，交互类似 Alfred / Raycast：

1. 按 **Alt + Space** 唤出搜索框
2. 输入标题关键字，模糊过滤
3. `↑` `↓` 选择，**回车**复制内容，然后自己粘贴

每个条目只有两个字段：**标题**（用来搜索）和**内容**（回车复制的文本，可多行）。

## 功能

- 全局热键 Alt + Space；如果被占用，会依次尝试 Ctrl+Alt+Space、Ctrl+Shift+Space、Shift+Alt+Space、Win+Alt+Space
- 搜索框使用 Win11 Mica / Win10 Acrylic 毛玻璃背景，失焦自动隐藏
- 复制的内容不进 Win+V 剪贴板历史，也不参与云同步；30 秒后自动清空剪贴板（这期间你自己复制了别的内容就不清）
- 数据经 Windows DPAPI 加密保存在 `%USERPROFILE%\.quickpwd\store.dat`，只有当前 Windows 账户能解开
- 托盘常驻：左键打开管理窗口，右键菜单可快速搜索或退出
- 管理窗口可以新增、编辑、删除条目，内置随机密码生成，可开关开机自启
- 单实例运行：再次启动 exe 会唤出已运行实例的管理窗口

## 快捷键

| 位置 | 按键 | 作用 |
|---|---|---|
| 搜索框 | `↑` `↓` / `PageUp` `PageDown` | 选择 |
| 搜索框 | `Enter` | 复制选中条目的内容 |
| 搜索框 | `Esc` | 清空输入，再按一次关闭 |
| 搜索框 | 输入 `:m` 回车 | 打开管理窗口 |
| 搜索框 | 输入 `:q` 回车 | 退出程序 |
| 管理窗口 | `Ctrl+N` / `F2` / `Delete` | 新增 / 编辑 / 删除 |
| 管理窗口 | `Ctrl+F` | 筛选 |
| 编辑框 | `Ctrl+Enter` | 保存 |

## 从旧版（Python）迁移

首次启动时，如果检测到旧版的 `%USERPROFILE%\.quickpwd\store.json`，会自动导入：旧的关键字作为标题，密码作为内容（旧版的用户名和备注不会导入）。导入后加密保存为 `store.dat`，原文件改名为 `store.json.imported`。确认数据无误后，**请手动删除这个明文文件**。

如果之前用旧版设置过开机自启，请删除 `shell:startup` 目录里的 `quickpwd_autostart.vbs`，然后在新版管理窗口里打开「开机自启」。

## 下载

在 [Releases](../../releases) 下载：

- `QuickPwd_x.y.z_x64-setup.exe`：安装版
- `QuickPwd-portable.exe`：免安装，直接运行

运行需要 WebView2 运行时（Win11 自带，Win10 一般已随 Edge 安装）。

## 构建

依赖：Rust stable、Node.js 18+、Windows 上的 MSVC 构建工具。

```sh
npm install
npm run tauri dev     # 开发
npm run tauri build   # 打包，产物在 src-tauri/target/release/bundle/nsis
```

推送到 `main` 会触发 GitHub Actions 构建；推送 `v*` 标签会自动发布 Release。

## 技术栈

Tauri 2（Rust 后端 + 原生 HTML/CSS/JS 前端，无打包器），用到的插件：global-shortcut、single-instance、autostart，以及 window-vibrancy、arboard、windows-sys（DPAPI）。
