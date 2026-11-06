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

- **实例** —— 目前支持原版、Fabric 与 Quilt；Forge / NeoForge 已识别但暂时禁用。
  每个实例拥有独立游戏目录、内存、JVM 参数与日志。
- **启动** —— 版本元数据合并、库与资源下载、SHA-1 校验、natives 解压、Java
  选择、快速加入服务器。
- **账户** —— Microsoft 设备代码登录、离线账户、LittleSkin / authlib 服务器。
  令牌只保存在系统密钥库中。
- **Java** —— 自动检测已有运行时，并在缺失时下载 Eclipse Temurin。
- **服务器** —— 收藏服务器、状态 ping（MOTD / 玩家 / 延迟）、一键加入。
- **Modrinth** —— 按类型 / 加载器 / 版本搜索，自动解析依赖，安装资源包与光影，
  支持移除文件。
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

这是一个早期重写版本。原版、Fabric、Quilt 实例，Microsoft / 离线 / LittleSkin
账户，Modrinth 模组、服务器与设置已经可用。已知缺口：

- Forge / NeoForge 安装流程
- `.mrpack` 整合包导入
- 服务器地址 SRV 查询
- 实例文件浏览器、备份恢复、导入导出
- 皮肤头像渲染（账户目前使用字母头像）
- 应用内更新（检查更新只跳转到 release 页面）

## 许可证

MIT。第三方许可证见依赖树与 `gpui-kit-material` 的声明。
