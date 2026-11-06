# Yuhina

**一个 Material 3 Expressive 风格的 Minecraft: Java 版启动器。**

Yuhina 可以安装并启动 Minecraft，把每个世界放在独立实例中，管理账户与 Java
运行时，并在启动器内浏览 Modrinth。界面使用 Rust、[GPUI](https://github.com/zed-industries/zed)
与 [`gpui-kit-material`](https://github.com/mizorewww/gpui-kit-material) 构建。

> **v0.2.0 —— 完全重写。** 旧的 Electron/React 版本、其 git 历史、release 产物
> 以及所有 AI / 聊天 / Agent 功能均已删除。Yuhina 现在是一个纯粹的启动器：
> 没有助手、没有模型供应商、没有隐藏的网络请求。

[English](README.md)

## 功能

- **实例** —— 支持原版、Fabric、Quilt、Forge 与 NeoForge（NeoForge 1.20.2+）。
  每个实例拥有独立游戏目录、内存、JVM 参数与日志。
- **启动** —— 版本元数据合并、库与资源下载、SHA-1 校验、natives 解压、Java
  选择、快速加入服务器。
- **账户** —— Microsoft 设备代码登录、离线账户、LittleSkin / authlib 服务器。
  令牌只保存在系统密钥库中；可用时自动获取并裁剪 Microsoft 皮肤头像。
- **Java** —— 自动检测已有运行时，并在缺失时下载 Eclipse Temurin。
- **服务器** —— 收藏服务器、支持 SRV 的状态 ping（MOTD / 玩家 / 延迟）、一键加入。
- **Modrinth** —— 按类型 / 加载器 / 版本搜索，自动解析依赖，安装资源包、光影与
  整合包，支持移除文件。
- **备份与文件** —— zip 备份（恢复前自动创建安全副本）、实例导出/导入、
  只读文件浏览器与文本预览。
- **Material 3 Expressive 界面** —— 自适应导航栏 / 导航轨 / 抽屉、色调表面、
  Expressive 字体、深色与浅色主题、中文与英文。

## 构建

要求：

- Rust **1.99.1**（已固定在 `rust-toolchain.toml`）
- Windows：Visual Studio 2022 + MSVC 工具链与 Windows SDK
- Linux：GPUI 常规构建依赖（X11/Wayland、Vulkan 驱动）
- macOS：Xcode 命令行工具

```bash
cargo run            # 开发构建
cargo test           # 单元测试
cargo build --release
```

首次构建会拉取 `gpui-kit`、`gpui-kit-material` 及其固定版本的引擎补丁，耗时
数分钟。

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

## 状态与已知缺口

这是一个早期重写版本。原版、Fabric、Quilt、Forge、NeoForge 实例，
Microsoft / 离线 / LittleSkin 账户，Modrinth 模组与整合包，备份、服务器与设置
已经可用。已知缺口：

- NeoForge 1.20.1 使用旧的 Forge 制品，暂不提供；
- Forge / NeoForge 整合包返回明确的“不支持”错误；
- 整合包导入只显示导入状态，没有逐文件 Activity 进度；
- LittleSkin / authlib 皮肤纹理尚未获取（回退为字母头像）；
- 安装器输出仅在失败时展示，未逐行写入 Activity；
- 应用内更新（检查更新只跳转到 release 页面）；
- 系统标题栏主题仍由操作系统决定。

## 许可证

MIT。第三方许可证见依赖树与 `gpui-kit-material` 的声明。
