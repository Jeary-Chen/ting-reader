# Ting Reader

Ting Reader 是一个轻量级的自托管有声书平台，专为有声书爱好者打造。它支持自动刮削元数据、多端播放进度同步、极致的视觉体验以及多架构 Docker 部署。后端现已完全重写为 **Rust**，带来更强的性能和更低的资源占用。

> **🌐 官方网站：[https://www.tingreader.cn](https://www.tingreader.cn)**
>
> 访问官网查看最新文档、下载客户端及在线演示。

![License](https://img.shields.io/github/license/dqsq2e2/ting-reader?cache=none)
![Docker Pulls](https://img.shields.io/docker/pulls/dqsq2e2/ting-reader)
![GitHub Release](https://img.shields.io/github/v/release/dqsq2e2/ting-reader)
[![Official Website](https://img.shields.io/badge/Official-Website-blue?logo=internetexplorer)](https://www.tingreader.cn)

## 📸 界面展示

<details>
<summary><b>📱 移动端与响应式界面 (点击展开)</b></summary>

#### 🔹 登录认证
| 用户登录 | 离线模式 |
| :---: | :---: |
| <img src="image/login.jpg" width="200"> | <img src="image/login_offline.jpg" width="200"> |

#### 🔹 主菜单
| 首页 | 书架 | 搜索 | 收藏 |
| :---: | :---: | :---: | :---: |
| <img src="image/home.jpg" width="200"> | <img src="image/bookshelf.jpg" width="200"> | <img src="image/search.jpg" width="200"> | <img src="image/favorites.jpg" width="200"> |

#### 🔹 沉浸式播放
| 播放界面 | 章节列表 | 播放设置 | 小窗播放 |
| :---: | :---: | :---: | :---: |
| <img src="image/player.jpg" width="200"> | <img src="image/player_chapters.jpg" width="200"> | <img src="image/player_settings.jpg" width="200"> | <img src="image/mini_player.jpg" width="200"> |

#### 🔹 书籍详情与管理
| 详情概览 | 章节目录 | 元数据编辑 |
| :---: | :---: | :---: |
| <img src="image/book_detail.jpg" width="200"> | <img src="image/book_chapters.jpg" width="200"> | <img src="image/book_edit_metadata.jpg" width="200"> |

#### 🔹 后台管理
| 存储库管理 | 用户管理 | 任务日志 |
| :---: | :---: | :---: |
| <img src="image/admin_repos.jpg" width="200"> | <img src="image/admin_users.jpg" width="200"> | <img src="image/admin_logs.jpg" width="200"> |

</details>

<details>
<summary><b>🖥️ 桌面端功能展示 (点击展开)</b></summary>

#### 🔹 基础与搜索
- **首页**
<img src="image/daznf0.png" width="900">
- **书架**
<img src="image/dbav84.png" width="900">
- **搜索与收藏**
<img src="image/dbhmg1.png" width="900">
<img src="image/dbkpra.png" width="900">

#### 🔹 播放与设置
- **书籍详情与播放小窗**
<img src="image/dduqb0.png" width="900">
- **个性化设置**
<img src="image/dckldc.png" width="900">

#### 🔹 管理后台
- **库管理与任务日志**
<img src="image/dbwxfj.png" width="900">
<img src="image/dbzir4.png" width="900">
- **用户管理**
<img src="image/dc1rpr.png" width="900">

</details>

## ✨ 功能特性

- ⚡ **Rust 核心**：后端采用 Rust 重写，资源占用极低，响应速度极快，稳定性大幅提升。
- 📚 **自动刮削**：集成喜马拉雅元数据刮削，自动获取书名、作者、演播者、简介及标签。
- 🔌 **插件系统**：支持 JS、WASM 和 Native 插件，轻松扩展刮削源和格式支持。
- 🎨 **自适应主题**：根据书籍封面**自动提取主色调**并实时调整书籍详情页背景与按钮颜色，视觉体验极致沉浸。
- ☁️ **多源支持**：支持本地目录挂载，未来将支持 WebDAV（如 Alist、PikPak）远程存储。
- 🎵 **格式兼容**：支持多种音频格式，包括 **MP3, M4A, M4B, WAV, FLAC, OGG, OPUS, AAC, WMA** 以及喜马拉雅加密格式 **XM**。
- 🎧 **沉浸播放**：支持跳过片头/片尾，支持播放速度调节及进度记忆。
- 🏷️ **智能标签**：支持标签筛选，标签云横向滚动展示，交互体验佳。
- 🧩 **[外挂组件](https://github.com/dqsq2e2/ting-reader/wiki/WIDGET_GUIDE)**：支持将播放器以 Widget 形式嵌入博客、Notion 或个人网站，支持吸底、悬浮等多种布局及自定义 CSS。
- 🌓 **深色模式**：完美的深色模式适配，夜间听书更护眼。
- 🐳 **Docker 部署**：支持 amd64 和 arm64 多架构构建，一键启动。
- 🔐 **权限管理**：完善的登录系统与管理员后台。

## 🚀 快速开始

### 使用 Docker Compose (推荐)

创建 `docker-compose.yml` 文件：

```yaml
services:
  ting-reader:
    image: dqsq2e2/ting-reader:latest
    container_name: ting-reader
    ports:
      - "3000:3000"
    volumes:
      - ./data:/app/data        # 数据库和配置
      - ./storage:/app/storage  # 有声书文件目录
      - ./plugins:/app/plugins  # 插件目录
      - ./temp:/app/temp        # 临时缓存目录
    restart: unless-stopped
    environment:
      - RUST_LOG=info
      - TING_HOST=0.0.0.0
      - TING_PORT=3000
```

启动容器：

```bash
docker-compose up -d
```

### 飞牛 fnOS 部署 (FPK)

如果您使用的是飞牛 fnOS 系统，可以通过官方应用中心的“手动导入”功能快速一键部署：

1.  **下载安装包**：前往 [GitHub Releases](https://github.com/dqsq2e2/ting-reader/releases) 下载最新版本的 `ting-reader-[版本号].fpk` 文件。
2.  **手动安装**：
    - 进入飞牛 fnOS 的 **应用中心**。
    - 点击右上角的 **手动安装** 按钮。
    - 选择并上传下载好的 `.fpk` 文件。
3.  **完成向导**：按照图形化引导界面配置访问端口以及有声书存储路径，点击“完成”后应用将自动创建容器并添加桌面启动图标。

访问 `http://localhost:3000` (或您自定义的端口) 即可开始使用。

> ⚠️ **注意**：首次登录请使用管理员账号：`admin`，密码：`admin123`。登录后请务必及时在设置页面修改密码以保证安全。

## 🛠️ 开发指南

### 环境要求
- Node.js 20+
- Rust 1.75+
- SQLite3

### 项目结构
```
ting-reader/
├── backend/    # Rust 后端源代码
├── frontend/   # React 前端源代码
├── plugins/    # 官方插件源码
└── .github/    # GitHub 工作流与 FPK 配置
```

### 本地开发

1. **克隆仓库**：
   ```bash
   git clone https://github.com/dqsq2e2/ting-reader.git
   cd ting-reader
   ```

2. **启动后端**：
   ```bash
   cd backend
   # 确保 config.toml 配置正确
   cargo run
   ```

3. **启动前端**：
   ```bash
   cd ../frontend
   npm install
   npm run dev
   ```

## 💬 交流与支持

如果您在安装或使用过程中遇到任何问题，或者有功能建议，欢迎加入我们的社群：

- **QQ 交流群**：[**1082770534**](https://qm.qq.com/q/gGrl1fzeiQ)

点击链接即可快速加入群聊，获得最新动态、技术支持及使用技巧分享。

## 📜 更新日志

关于项目的详细版本变更记录，请参考 [CHANGELOG.md](CHANGELOG.md)。

## 📄 开源协议

本项目采用 [MIT License](LICENSE) 协议。

## 🤝 贡献指南

欢迎提交 Issue 或 Pull Request！请参考 [CONTRIBUTING.md](CONTRIBUTING.md) 了解更多细节。
