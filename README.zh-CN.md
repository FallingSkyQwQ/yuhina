# Yuhina

**一个 Material 3 Expressive 风格的 Minecraft: Java 版启动器。**

Yuhina 可以安装并启动 Minecraft，把每个世界放在独立实例中，管理账户与 Java
运行时，并在启动器内浏览 Modrinth。视觉采用 Material 3 Expressive，交互与
信息层级按 Apple HIG 的品质标准打磨。

> **v0.2.x —— Rust 重写。** 旧的 Electron/React 版本、其 git 历史、release
> 产物以及所有 AI / 聊天 / Agent 功能均已删除。Yuhina 现在是一个纯粹的启动器。

[English](README.md)

## 功能

- **实例** —— 原版、Fabric、Quilt、Forge、NeoForge（NeoForge 1.20.2+）。
  独立游戏目录、内存、JVM 参数、日志与游玩时长。
- **启动** —— 版本元数据合并、库与资源下载、SHA 校验、natives 解压、Java
  选择、快速加入服务器。
- **账户** —— Microsoft 设备代码登录、离线账户、LittleSkin / authlib 服务器。
  令牌只保存在系统密钥库；Microsoft 与 LittleSkin 皮肤头像会下载并裁剪。
- **Java** —— 自动检测已有运行时，缺失时下载 Eclipse Temurin。
- **服务器** —— 收藏服务器、支持 SRV 的状态 ping（MOTD / 玩家 / 延迟）、
  一键加入。
- **Modrinth** —— 搜索、依赖安装、资源包、光影、`.mrpack` 整合包与文件移除。
- **备份与文件** —— zip 备份（恢复前自动创建安全副本）、实例导出/导入、
  只读文件浏览器与文本预览。
- **设置** —— 主题、语言、动效、文本缩放（100–200%）、下载源、Java、内存、
  关于与更新检查。
- **Material 3 Expressive + HIG** —— 自适应导航轨/底栏、5 个主目的地、
  每个视图一个主操作、行/卡片/搜索/分段选择使用组件库、深浅主题、中英文。
- **品牌** —— 使用 `design/brand/yuhina-mark.svg` 的几何 Y 标记；
  可执行文件图标由 `yuhina-icon.svg` 生成。

## 构建

要求：

- Rust **1.99.1**（固定在 `rust-toolchain.toml`）
- Windows：Visual Studio 2022 + MSVC 与 Windows SDK
- Linux：GPUI 构建依赖（X11/Wayland、Vulkan 驱动、fontconfig）
- macOS：Xcode 命令行工具

```bash
cargo build
cargo test
cargo build --release
```

首次构建会拉取 `gpui-kit`、`gpui-kit-material` 及固定引擎补丁，耗时数分钟。

**焦点策略：** 只有设置 `YUHINA_FOCUS=1` 时应用才请求焦点。Agent 与测试不会
启动 GUI；请手动运行应用。

## 数据目录

| 平台 | 目录 |
| --- | --- |
| Windows | `%APPDATA%\Yuhina` |
| macOS | `~/Library/Application Support/Yuhina` |
| Linux | `~/.config/Yuhina` |

目录结构见 [`docs/architecture.md`](docs/architecture.md)。

## 文档

- [`docs/product-spec.md`](docs/product-spec.md)
- [`docs/design-system.md`](docs/design-system.md)
- [`docs/architecture.md`](docs/architecture.md)
- [`docs/git-workflow.md`](docs/git-workflow.md)
- [`docs/testing.md`](docs/testing.md)
- [`design/brand/README.md`](design/brand/README.md)

## 状态与已知缺口

原版、Fabric、Quilt、Forge、NeoForge 实例，Microsoft / 离线 / LittleSkin 账户，
Modrinth 模组与整合包，备份、服务器与设置已经可用。已知缺口：

- NeoForge 1.20.1 使用旧 Forge 制品，暂不提供；
- LittleSkin 服务器关闭 session 端点时回退为字母头像；
- 安装器输出已由 core 提供，但 Activity 尚未完整展示日志；
- 应用内更新安装（目前跳转 release 页面）；
- Windows 标题栏主题适配仍在进行；
- 拖拽导入、命令面板、托盘行为未实现；
- HIG 文本缩放/组件整改进行中，尚未完成所有页面的 200% 验证。

## 许可证

MIT。第三方许可证见依赖树与 `gpui-kit-material` 的声明。
