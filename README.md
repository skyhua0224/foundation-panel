# Foundation Panel

<div align="center">

<p><picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/readme/hero-dark.webp">
  <img src="assets/readme/hero.webp" width="800" alt="Foundation Panel：管好你的被控端。设备与组件、串流设置、日志与诊断。">
</picture></p>

<p align="center"><a href="https://github.com/AlkaidLab/foundation-sunshine"><picture><source media="(prefers-color-scheme: dark)" srcset="assets/readme/b1-dark.webp"><img src="assets/readme/b1.webp" alt="了解 Foundation Sunshine" height="56"></picture></a>&nbsp;&nbsp;<a href="#快速开始"><picture><source media="(prefers-color-scheme: dark)" srcset="assets/readme/b2-dark.webp"><img src="assets/readme/b2.webp" alt="快速开始" height="56"></picture></a>&nbsp;&nbsp;<a href="#功能"><picture><source media="(prefers-color-scheme: dark)" srcset="assets/readme/b3-dark.webp"><img src="assets/readme/b3.webp" alt="功能一览" height="56"></picture></a></p>

[![Latest release](https://img.shields.io/github/v/release/AlkaidLab/foundation-panel?include_prereleases&sort=date&label=Latest)](https://github.com/AlkaidLab/foundation-panel/releases) [![Downloads](https://img.shields.io/github/downloads/AlkaidLab/foundation-panel/total?label=Downloads)](https://github.com/AlkaidLab/foundation-panel/releases) [![GitHub Stars](https://img.shields.io/github/stars/AlkaidLab/foundation-panel?style=flat&label=Stars)](https://github.com/AlkaidLab/foundation-panel) [![License](https://img.shields.io/badge/License-MIT-blue)](LICENSE)

简体中文

[官网](https://www.alkaidlab.com/) · [国内官网](https://www.alkaidlab.cn/) · [问题反馈](https://github.com/AlkaidLab/foundation-panel/issues)

</div>

Foundation Panel 是基于 Tauri 2 和 Vue 3 的 Windows 桌面面板，用来管理 Foundation 被控端，目前配合 [Foundation Sunshine](https://github.com/AlkaidLab/foundation-sunshine) 使用，之后也会承载其他 Foundation 被控端。

概览、设置、设备组件和日志诊断集中在一个窗口，托盘菜单、启动画面和大屏界面随连接一起工作。

<div align="center">

<p align="center"><picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/readme/show-dark.webp">
  <img src="assets/readme/show.webp" alt="Foundation Panel 展示图：概览窗口和托盘菜单，周围是语言数、码率上限、缩放、CPU 与已配对设备小组件。" width="880">
</picture></p>

</div>

[功能](#功能) · [构建](#构建) · [帮助与参与](#帮助与参与) · [致谢](#致谢)

## 下载

面板随 [Foundation Sunshine](https://github.com/AlkaidLab/foundation-sunshine/releases/latest) 安装包一起提供，安装 Foundation Sunshine 即可使用。本仓库的[发行版](https://github.com/AlkaidLab/foundation-panel/releases)供 Foundation Sunshine 打包使用。

需要 Windows 10 22H2 或更新版本。

## 快速开始

1. 安装被控端（例如 Foundation Sunshine）。
2. 打开面板，第一次启动会进入引导：检查网络、UPnP、公网地址和可达性，然后配对第一台控制端设备。
3. 在概览中查看设备、会话和更新；在设置中调整串流、显示器和输入。

## 功能

- **概览：** 设备、虚拟显示器、USB、麦克风和手柄状态，CPU、GPU 与内存负载，公告和配对卡片，配对二维码。
- **设置：** 串流设置、面板选项、外观、关于与网络概览；界面语言跟随系统，支持 18 种语言。
- **设备与组件：** 虚拟显示器、虚拟麦克风与 VB-Cable、虚拟鼠标、ViGEmBus、USB 转发、DualSense 原生状态与调校、DLSS NR 与 RTX HDR 组件的导入与管理。
- **诊断：** 日志与诊断合为一页，包含体检、错误记录、诊断包导出，以及面板自己的日志；崩溃报告在你同意后才会上传。
- **更新：** 更新窗口显示真实下载进度，支持 Mirror酱 渠道，Beta 与预发布版本可选。
- **托盘与启动画面：** 三种托盘菜单、快速面板、游戏启动时的启动画面。
- **大屏界面：** 手柄与键盘可操作的桌面界面，管理应用、封面和启动助手。
- **工具：** 码率调节、DPI 缩放、RTSS 叠加、延迟时钟、串流计时器和浮动工具栏。
- **助手：** 可选的 AI 助手，粘贴的日志会先脱敏。

## 构建

三个平台的初始化步骤见 [CONTRIBUTING.md](CONTRIBUTING.md)，版本与系统依赖说明见
[构建环境设计](docs/development-environment.md)。共享组件和设置数据均锁定到确定版本，
无需准备相邻源码目录。

```sh
just setup
just doctor
just build
just test
```

Linux 的 build/test 使用与 CI 相同的 Dev Container。Windows 的安装包用 `just package`，参数原样传给
`scripts/build-gui-component-installer.ps1`，构建前同样使用锁定的设置数据。

## 帮助与参与

- [提交问题](https://github.com/AlkaidLab/foundation-panel/issues)
- [官网](https://www.alkaidlab.com/) · [国内官网](https://www.alkaidlab.cn/)
- 设计与页面状态记录在 [docs/redesign](docs/redesign)，托盘菜单与右键菜单窗口见 [docs](docs)。

## 致谢

本项目基于 [qiin2333/sunshine-control-panel](https://github.com/qiin2333/sunshine-control-panel)（MIT）继续开发，保留其提交历史和许可证。详见 [NOTICE.md](NOTICE.md)。

感谢 [LizardByte/Sunshine](https://github.com/LizardByte/Sunshine)、[Moonlight](https://moonlight-stream.org/)，以及所有参与开发、测试和翻译的贡献者。项目遵循 [MIT 许可证](LICENSE)。
