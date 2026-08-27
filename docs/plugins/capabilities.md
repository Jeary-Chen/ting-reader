# 插件能力声明

`capabilities` 是插件和 Ting Reader 之间的契约：它声明插件提供哪些入口、由谁调用、调用哪个函数、需要显示什么 UI，以及相关权限应该如何配套。运行时文件不决定插件类别，`capabilities[].kind` 才决定插件能做什么。

## 1. 能力声明写在哪里

插件在 `plugin.yml` 或 `plugin.yaml` 的 `capabilities` 数组中声明能力。每一项至少需要 `id` 和 `kind`；如果省略 `invoke`，后端会默认使用 capability 自己的 `id` 作为运行时方法名。

```yaml
capabilities:
  - id: assistant.panel
    kind: ui_extension
    invoke: openAssistant
    slots: [app.sidebar_page, global.floating_action]
    title:
      zh: 书单助手
      en: Booklist Assistant
    icon: message-circle
    priority: 20
    contexts: [global, book, reader]
    render:
      mode: web_container
      entry: ui/assistant.html
      bridge:
        capabilities: [assistant.tools]
        host_methods: [user_settings.get]

  - id: metadata.search
    kind: metadata_provider
    invoke: search
    auto_scrape: true
    search_fields:
      - key: title
        label: { zh: 书名, en: Title }
        required: true
        default_from: book.title
    result_fields:
      - key: title
        label: { zh: 书名, en: Title }
      - key: author
        label: { zh: 作者, en: Author }
      - key: cover_url
        label: { zh: 封面, en: Cover }

  - id: assistant.tools
    kind: tool_provider
    invoke: invokeTool
    tools:
      - name: books.recommend
        description: Recommend books from the current user's library

permissions:
  - type: books_read
  - type: progress_read
  - type: network_access
    value: api.example.com
```

| 字段 | 说明 |
| --- | --- |
| `id` | 插件内唯一能力 ID，例如 `metadata.search`、`assistant.panel`。 |
| `kind` | 能力类型，决定系统发现和调用这项能力的方式。 |
| `invoke` | 运行时函数名。JavaScript 对应 `globalThis[invoke]`，WASM/Native 对应导出的调用分发。 |
| `title` / `label` | 面向用户的入口名，支持字符串或 `{ zh, en }`。 |
| `icon` | 客户端入口图标，UI 插件可写 lucide 名称，也可以写 emoji 或图片对象。 |
| `priority` | 同一 slot 下的排序，数字越小越靠前。 |
| `contexts` | 入口适用上下文，例如 `global`、`book`、`reader`、`chapter`。 |

## 2. 常用 kind

| kind | 用途 | 常见字段 |
| --- | --- | --- |
| `metadata_provider` | 搜索和刮削书籍、有声书元数据 | `auto_scrape`、`search_fields`、`result_fields`、`result_field_labels` |
| `format_handler` | 格式识别、播放 URL、解密、元数据读写 | `extensions` 或 `matches.extensions` |
| `content_processor` | 文档探测、章节、片段、分页和渲染 | `operations`、`extensions` |
| `ui_extension` / `client_extension` | 客户端按钮、面板、设置表单或 Web UI | `slot`、`title`、`icon`、`render`、`contexts` |
| `http_route` | 插件 HTTP 路由，例如 RSS、回调、公开 feed | `route.method`、`route.path`、`route.auth` |
| `tool_provider` | 向 AI 助手、客户端或其他插件暴露工具 | `tools[].name`、`tools[].description` |
| `plugin_store` | 提供可配置插件源 | 通常返回 `{ plugins: [...] }` |
| `task_handler` | 接收插件自定义后台任务 | `task_types` |
| `event_handler` | 订阅系统事件 | `events`，可用 `*` 订阅全部事件 |

`capabilities` 只声明插件能被怎样调用；读取书籍、进度、媒体、缓存、文件或发起网络请求仍必须在 `permissions` 中声明对应权限。

## 3. UI 入口、图标和菜单

`ui_extension` 的 `slot` 或 `slots` 决定入口出现在哪里。Web 和 Flutter 客户端会读取 `title`、`icon`、`priority` 和 `render` 来生成入口；`app.sidebar_page` 位于侧边栏底部，`global.floating_action` 位于现有右下角插件工具菜单。用户可以在个性化设置中关闭右下角菜单，侧边栏页面不受该开关影响。

```yaml
capabilities:
  - id: assistant.panel
    kind: ui_extension
    slots: [app.sidebar_page, global.floating_action]
    title: { zh: 书单助手, en: Booklist Assistant }
    icon: message-circle
    render:
      mode: web_container
      entry: ui/assistant.html
      bridge:
        capabilities: [assistant.tools]
        host_methods: [user_settings.get]

  - id: quick.note
    kind: ui_extension
    slot: global.floating_action
    title: { zh: 快速笔记, en: Quick Note }
    icon: "📝"
    render: action
```

| slot | 显示位置 |
| --- | --- |
| `app.sidebar_page` | 桌面侧边栏底部的插件页面；侧边栏折叠后仅显示图标 |
| `global.floating_action` | 全局右下角插件入口菜单 |
| `global.panel` | 全局右下角插件面板入口 |
| `book.detail_action` | 书籍详情页动作入口 |

`reader.toolbar_action`、`reader.side_panel` 和 `reader.document_viewer` 已停止渲染。客户端仍能解析这些值以兼容旧 manifest，但不会为它们创建入口；迁移时请改为 `app.sidebar_page`、`global.floating_action` 或 `book.detail_action`。

| render.mode | 说明 |
| --- | --- |
| `web_container` | 加载 `ui/` 或 `assets/` 下的 HTML 入口，插件 UI 通过 `postMessage` 与宿主通信 |
| `schema` | 由客户端根据 schema 渲染简单表单 |
| `builtin` | 调用宿主内置组件，例如文档阅读器 |
| `action` | 点击入口后直接调用 capability |

`icon` 推荐使用 lucide 图标名，例如 `message-circle`、`messages-square`、`book-open`，也可以使用 emoji。Web 前端会优先按 lucide 名称解析，无法解析时按文本图标显示；Flutter 会优先匹配 Lucide 图标 catalog，找不到时再按常见名称回退到 Material 图标。

图片图标支持对象写法：

```yaml
icon: { type: lucide, name: settings }
icon: { type: emoji, value: "✨" }
icon: { type: image, src: "assets/plugin-icon.png" }
```

图片图标只允许插件包内安全的 `assets/...` 相对路径，Web 和 Flutter 都通过 `GET /api/v1/plugin-assets/:client_grant/:plugin_id/*path` 解析，单文件最大 64 MiB。`client_grant` 由 `GET /api/v1/plugin-capabilities` 为 `ui_extension` / `client_extension` 注册项返回，绑定当前用户、插件和 UI capability，并会过期；受信客户端必须把它作为不透明秘密保管。grant 是资产 URL 的路径段，插件 UI 如从自身资源地址观察到该值，也不得解析、记录、持久化或外传。远程 URL、`data:` URL、站点根路径、空路径、`.` 和 `..` 会被拒绝并回退到默认插件图标，避免第三方跟踪、内容替换和网关根路径错误。图片建议保持正方形透明底；能用 lucide 或 emoji 时优先使用它们。

## 4. Web 容器运行细节

`web_container` 的 `render.entry` 会通过插件资产接口加载。静态资源建议放在 `ui/` 或 `assets/` 下，并使用相对路径引用 CSS、JS 和图片。入口必须是插件包内的安全相对路径，不能包含绝对路径、反斜杠或 `..`。

Web 容器运行在不含 `allow-same-origin` 的 sandbox 中，并注入严格 CSP：禁止直接联网、嵌套 frame、worker、对象和顶层导航。`capability.invoke` 默认只能调用当前 UI capability；如页面确实需要调用同插件的工具 capability，必须在 `render.bridge.capabilities` 中逐项声明。`host.invoke` 默认全部拒绝，必须在 `render.bridge.host_methods` 中逐项声明。客户端和后端都会重新校验来源 UI 与白名单，后端还会继续检查插件权限和当前用户权限。

```yaml
render:
  mode: web_container
  entry: ui/assistant.html
  bridge:
    allow_capability_invoke: true
    capabilities:
      - assistant.tools
    host_methods:
      - user_settings.get
      - progress.recent
```

Web 端不允许插件 iframe 逃离 sandbox 或创建不受控弹窗。Web 与 Flutter 的每个插件文档都有独立 `bridgeToken` 和宿主预先创建的 `MessagePort`；请求必须通过 `window.__TING_PLUGIN_BRIDGE__.postMessage()` 发出并携带匹配的 `bridge_token`。`bridgeToken` 只用于当前文档握手，不是服务端签发的 `client_grant` / `ui_grant`。受信客户端会在 capability 与 HostGateway HTTP 转发中附加匹配 grant，后端再次校验用户、插件、来源 UI 和有效期。文档刷新或跳转会丢失原端口，只知道旧 token 不能接管 bridge。Web 与 Flutter 都只允许真实用户点击产生的 HTTP(S) 外链请求，并在宿主界面显示目标地址、等待用户明确确认后才打开；插件不应依赖 `window.open()` 返回值或把外链、弹窗作为核心流程。

## 5. 能力与权限要配套

| 插件要做什么 | 常见权限 |
| --- | --- |
| 读取书籍、存储库列表 | `books_read` |
| 读取章节 | `chapters_read` |
| 读取最近播放进度 | `progress_read` |
| 获取播放地址 | `media_read_url` 或 `media_read` |
| 保存插件缓存 | `cache_read`、`cache_write` |
| 创建或修改播放列表 | `playlists_read`、`playlists_write` |
| 访问外部 API | `network_access`，建议填写具体域名 |
| 写元数据或创建任务 | `metadata_write`、`task_create`，通常需要管理员上下文 |

权限缺失时，HostGateway 会拒绝调用并返回权限错误。开发时建议先最小化权限，只有实际需要时再增加。
