# 插件管理

## 数据结构

### LocalizedText

```json
{
  "zh": "中文文本",
  "en": "English text"
}
```

### ScraperSearchField

```json
{
  "key": "title",
  "label": "书名",
  "label_i18n": {
    "zh": "书名",
    "en": "Title"
  },
  "required": true,
  "type": "text",
  "field_type": "text",
  "placeholder": "输入书名",
  "placeholder_i18n": {
    "zh": "输入书名",
    "en": "Enter title"
  },
  "default_from": "book.title"
}
```

### ScraperCapabilities

```json
{
  "auto_scrape": true,
  "search_fields": [
    {
      "key": "title",
      "label": "书名",
      "label_i18n": {
        "zh": "书名",
        "en": "Title"
      },
      "required": true,
      "type": "text",
      "placeholder": "输入书名",
      "placeholder_i18n": {
        "zh": "输入书名",
        "en": "Enter title"
      },
      "default_from": "book.title"
    }
  ],
  "result_fields": ["title", "author", "cover_url", "intro", "tags"],
  "result_field_labels": {
    "title": {
      "zh": "书名",
      "en": "Title"
    },
    "cover_url": {
      "zh": "封面",
      "en": "Cover"
    },
    "intro": {
      "zh": "简介",
      "en": "Description"
    },
    "description": {
      "zh": "简介",
      "en": "Description"
    }
  }
}
```

### PluginInfo

插件管理 API 中的展示/业务摘要字段由服务端从 `capabilities` 派生；插件能力以 manifest 中的 `capabilities` 为准。

```json
{
  "id": "string",
  "name": "string",
  "version": "string",
  "plugin_type": "scraper | format | utility",
  "runtime": "wasm | javascript | native | null",
  "author": "string | null",
  "description": "string | null",
  "description_i18n": {
    "zh": "中文描述",
    "en": "English description"
  },
  "is_enabled": true,
  "state": "loading | loaded | active | unloading | unloaded | failed",
  "error": "string | null",
  "stats": {
    "total_calls": 0,
    "successful_calls": 0,
    "failed_calls": 0,
    "avg_execution_time_ms": 0.0
  },
  "config_schema": {
    "type": "object",
    "properties": {
      "api_key": {
        "type": "string",
        "title": "API 密钥",
        "title_i18n": {
          "zh": "API 密钥",
          "en": "API Key"
        },
        "description": "用于访问 API 的密钥",
        "description_i18n": {
          "zh": "用于访问 API 的密钥",
          "en": "Key used to access the API"
        }
      }
    }
  },
  "permissions": ["network_access: example.com"],
  "license": "string | null",
  "repo": "string | null",
  "capabilities": [
    {
      "id": "metadata.search",
      "kind": "metadata_provider",
      "invoke": "search",
      "auto_scrape": true,
      "search_fields": [],
      "result_fields": []
    }
  ],
  "scraper": {
    "auto_scrape": true,
    "search_fields": [],
    "result_fields": [],
    "result_field_labels": {}
  }
}
```

### StorePlugin

```json
{
  "id": "ximalaya-scraper-wasm",
  "name": "ximalaya scraper",
  "description": "从喜马拉雅获取有声书元数据（WASM 实现）",
  "description_i18n": {
    "zh": "从喜马拉雅获取有声书元数据（WASM 实现）",
    "en": "Fetch audiobook metadata from Ximalaya (WASM implementation)"
  },
  "version": "1.0.2",
  "download_url": "/plugins/ximalaya-scraper-wasm.tr",
  "size": "347.63 KB",
  "date": "2026-06-27T14:20:49.000Z",
  "runtime": "wasm",
  "license": "MIT",
  "author": "Ting Reader Team",
  "repo": "dqsq2e2/example-plugin",
  "permissions": ["network_access: www.ximalaya.com"],
  "dependencies": ["ffmpeg-utils"],
  "min_core_version": "1.4.8",
  "config_schema": {},
  "capabilities": [
    {
      "id": "metadata.search",
      "kind": "metadata_provider",
      "invoke": "search",
      "auto_scrape": true,
      "search_fields": [],
      "result_fields": []
    }
  ],
  "downloads": [
    {
      "name": "Download Plugin",
      "url": "https://www.tingreader.cn/plugins/ximalaya-scraper-wasm.tr"
    }
  ]
}
```

`download_url` 可以是字符串，也可以是平台映射：

```json
{
  "download_url": {
    "linux-x86_64": "https://example.com/plugin-linux-x86_64.tr",
    "linux-aarch64": "https://example.com/plugin-linux-arm64.tr",
    "windows-x86_64": "https://example.com/plugin-windows-amd64.tr"
  }
}
```

## 已安装插件

### GET /api/v1/plugins

获取已安装插件列表。

响应：`200 OK`

```json
[
  {
    "id": "string",
    "name": "string",
    "version": "string",
    "plugin_type": "scraper",
    "runtime": "wasm",
    "author": "Ting Reader Team",
    "description": "从示例站点获取元数据",
    "description_i18n": {
      "zh": "从示例站点获取元数据",
      "en": "Fetch metadata from the example site"
    },
    "is_enabled": true,
    "state": "active",
    "error": null,
    "stats": {
      "total_calls": 0,
      "successful_calls": 0,
      "failed_calls": 0,
      "avg_execution_time_ms": 0.0
    },
    "config_schema": {},
    "permissions": ["network_access: example.com"],
    "license": "MIT",
    "repo": "owner/repo",
    "capabilities": [
      {
        "id": "metadata.search",
        "kind": "metadata_provider",
        "invoke": "search",
        "auto_scrape": true,
        "search_fields": [],
        "result_fields": []
      }
    ],
    "scraper": {
      "auto_scrape": true,
      "search_fields": [],
      "result_fields": [],
      "result_field_labels": {}
    }
  }
]
```

### GET /api/v1/plugins/:id

获取插件详情。

路径参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `id` | string | 插件 ID |

响应：`200 OK`

```json
{
  "id": "string",
  "name": "string",
  "version": "string",
  "plugin_type": "format",
  "runtime": "native",
  "author": "Ting Reader Team",
  "description": "通过 FFmpeg 提供原生音频格式支持",
  "description_i18n": {
    "zh": "通过 FFmpeg 提供原生音频格式支持",
    "en": "Native audio format support via FFmpeg"
  },
  "license": "MIT",
  "repo": "owner/repo",
  "is_enabled": true,
  "state": "active",
  "error": null,
  "entry_point": "native_audio_support.dll",
  "dependencies": [
    {
      "plugin_name": "ffmpeg-utils",
      "version_requirement": "*"
    }
  ],
  "permissions": ["FileRead(\"./data/audio\")"],
  "supported_extensions": ["m4a", "flac"],
  "capabilities": [
    {
      "id": "format.audio",
      "kind": "format_handler",
      "invoke": "get_stream_url",
      "extensions": ["m4a", "flac"]
    }
  ],
  "config_schema": {},
  "scraper": null,
  "stats": {
    "total_calls": 0,
    "successful_calls": 0,
    "failed_calls": 0,
    "avg_execution_time_ms": 0.0
  }
}
```

### POST /api/v1/plugins/install

上传安装插件（`multipart/form-data`，最大 50MB）。

请求：字段名 `file`，值为 `.tr` 插件包。

`.tr` 包由 `trpack build` 生成，并会在安装前完成有效性校验。

安装接口仅允许管理员调用。manifest 的 `id`、`name`、`version` 和 `entry_point` 会按安全规则校验；压缩包最多 10,000 个条目，单文件展开后最大 128 MiB，总展开大小最大 256 MiB。上传和商店下载包最大 50 MiB。JavaScript 依赖安装会禁用 npm 生命周期脚本。

响应：`201 Created`

```json
{
  "plugin_id": "string",
  "message": "Plugin xxx installed successfully"
}
```

如果包未签名或签名 key id 不在受信任列表中，后端返回 `428 Precondition Required`，客户端应显示安全提示。用户同意后，用同一个文件重新提交，并增加字段 `accept_unverified=true`。

```json
{
  "requires_confirmation": true,
  "verification_status": "unsigned",
  "plugin_id": "example-plugin",
  "plugin_name": "Example Plugin",
  "plugin_version": "1.0.0",
  "publisher": "未知发布者",
  "warning": "Example Plugin由未知发布者提供，未经Ting Reader验证。单击同意，即表示你同意全权负责因使用该插件而可能导致的任何设备损坏或数据丢失。"
}
```

### DELETE /api/v1/plugins/:id

卸载插件。

路径参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `id` | string | 插件 ID |

响应：`200 OK`

```json
{
  "message": "Plugin xxx uninstalled successfully"
}
```

### POST /api/v1/plugins/:id/reload

重新加载插件。

路径参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `id` | string | 插件 ID |

响应：`200 OK`

```json
{
  "message": "Plugin xxx reloaded successfully"
}
```

## 插件配置

### GET /api/v1/plugins/:id/config

获取插件配置。

路径参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `id` | string | 插件 ID |

响应：`200 OK`

```json
{
  "plugin_id": "string",
  "config": {}
}
```

### PUT /api/v1/plugins/:id/config

更新插件配置。

路径参数：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `id` | string | 插件 ID |

请求体：

```json
{
  "config": {
    "api_key": "string"
  }
}
```

响应：`200 OK`

```json
{
  "message": "Plugin xxx configuration updated successfully"
}
```

## 插件商店

### GET /api/v1/store/plugins

获取商店插件列表。服务端会查找已安装且启用的 `plugin_store` capability，调用该 capability 的 `invoke` 方法获取列表；如果未安装插件商店插件，返回空数组。

查询参数：

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `refresh` | boolean | 否 | 为 `true` 时跳过后端商店缓存，并向商店插件传入 `force_refresh: true`。 |

响应：`200 OK`

```json
[
  {
    "id": "ximalaya-scraper-wasm",
    "name": "ximalaya scraper",
    "description": "从喜马拉雅获取有声书元数据（WASM 实现）",
    "description_i18n": {
      "zh": "从喜马拉雅获取有声书元数据（WASM 实现）",
      "en": "Fetch audiobook metadata from Ximalaya (WASM implementation)"
    },
    "version": "1.0.2",
    "download_url": "https://www.tingreader.cn/plugins/ximalaya-scraper-wasm.tr",
    "runtime": "wasm",
    "author": "Ting Reader Team",
    "permissions": ["network_access: www.ximalaya.com"],
    "capabilities": [
      {
        "id": "metadata.search",
        "kind": "metadata_provider",
        "invoke": "search",
        "auto_scrape": true,
        "search_fields": [
          {
            "key": "title",
            "label": "书名",
            "label_i18n": {
              "zh": "书名",
              "en": "Title"
            },
            "required": true,
            "type": "text",
            "placeholder": "输入书名",
            "placeholder_i18n": {
              "zh": "输入书名",
              "en": "Enter title"
            },
            "default_from": "book.title"
          }
        ],
        "result_fields": ["title", "author", "cover_url", "intro"],
        "result_field_labels": {
          "title": {
            "zh": "书名",
            "en": "Title"
          },
          "intro": {
            "zh": "简介",
            "en": "Description"
          },
          "description": {
            "zh": "简介",
            "en": "Description"
          }
        }
      }
    ]
  }
]
```

### POST /api/v1/store/install

从商店安装插件。

仅管理员可调用。插件商店返回的安装包下载地址必须使用 HTTPS；HTTP 和其他协议会被拒绝。服务端使用宿主生成的随机临时文件名流式下载插件，不使用下载 URL 拼接本地路径；单次商店下载最大 50 MiB，随后仍执行与上传安装相同的 manifest、签名和压缩包安全校验。

请求体：

```json
{
  "plugin_id": "string"
}
```

响应：`201 Created`

```json
{
  "plugin_id": "string",
  "message": "Plugin xxx installed successfully from store"
}
```

### POST /api/v1/store/cache/clear

清除插件商店缓存。客户端执行“更新插件列表”时应先调用此接口，再以 `refresh=true` 拉取 `/api/v1/store/plugins`。

响应：`200 OK`

```json
{
  "message": "Plugin cache cleared successfully"
}
```

## 插件能力 API

### GET /api/v1/plugin-capabilities

列出已启用插件声明的 capability。可用 `kind` 过滤，例如 `ui_extension`、`client_extension`、`content_processor`、`tool_provider`、`task_handler`、`event_handler`、`http_route`。

查询参数：

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | :---: | --- |
| `kind` | string | 否 | capability kind 过滤。 |

响应：`200 OK`

```json
[
  {
    "plugin_id": "advanced-capabilities-example@0.1.0",
    "plugin_name": "Advanced Capabilities Example",
    "client_grant": "<opaque-client-grant>",
    "capability": {
      "id": "assistant.panel",
      "kind": "ui_extension",
      "invoke": "openAssistant",
      "slot": "global.panel",
      "render": {
        "mode": "web_container",
        "entry": "ui/assistant.html"
      }
    }
  }
]
```

`client_grant` 只会出现在 `ui_extension` / `client_extension` 注册项中。它由服务端签名，绑定当前用户、插件和来源 UI capability，并带有过期时间。客户端必须把它当作不透明的临时凭据和秘密处理：不要解析、持久化到可公开读取的位置、写入日志或发送给插件页面之外的第三方；过期后重新调用本接口获取。其他 capability 不返回该字段。

### POST /api/v1/plugins/:plugin_id/capabilities/:capability_id/invoke

调用指定插件 capability。后端会自动附加可信 `_context`，包含插件、capability 和当前认证用户上下文。

从插件 UI 发起调用时必须同时传 `ui_capability_id` 和对应注册项返回的 `client_grant`（请求字段名为 `ui_grant`）。后端会验证签名、有效期、当前用户、插件和来源 UI capability，再只允许调用当前 UI capability 或其 `render.bridge.capabilities` 中显式声明的 capability。`tool_provider` 不接受缺少 UI 来源的直接 HTTP 调用。

请求体：

```json
{
  "ui_capability_id": "assistant.panel",
  "ui_grant": "<opaque-client-grant>",
  "params": {
    "slot": "book.detail_action",
    "context": {
      "book_id": "book-id"
    },
    "values": {
      "note": "example"
    }
  }
}
```

`ui_capability_id` 和 `ui_grant` 只有核心文档读取流程调用 `content_processor` 时可以一起省略。其他通过此客户端 HTTP 接口触发的 capability（包括 tool、task、event、metadata 和 UI）都必须携带来源 UI 和匹配的服务端签名凭据，并通过已安装 manifest 的 bridge 白名单校验；宿主内部调度不经过此客户端接口。

响应：`200 OK`

```json
{
  "result": {
    "ok": true
  }
}
```

### GET /api/v1/plugin-capabilities/content-processors

按扩展名查询内容处理插件。

查询参数：

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | :---: | --- |
| `extension` | string | 是 | 文件扩展名，例如 `txt`、`pdf`。 |
| `operation` | string | 否 | 操作过滤：`probe`、`extract_metadata`、`list_sections`、`read_chunk`、`render_page`。 |

### GET /api/v1/plugin-capabilities/tools

查询 `tool_provider`。可用 `name` 过滤工具名。

### GET /api/v1/plugin-capabilities/task-handlers

查询 `task_handler`。可用 `task_type` 过滤任务类型。

### GET /api/v1/plugin-capabilities/event-handlers

查询 `event_handler`。可用 `event` 过滤事件名。

## 插件 UI 资产

### GET /api/v1/plugin-assets/:client_grant/:plugin_id/*path

读取处于 `active` / `executing` 状态插件的 UI 静态文件。`:client_grant` 使用 `GET /api/v1/plugin-capabilities` 中对应 UI 注册项返回的 `client_grant`；后端会验证它仍绑定当前用户、插件和 UI capability，并重新检查 capability 可见性及 `admin_only`。仅允许访问插件包内 `ui/` 和 `assets/` 目录，路径会经过规范化、canonical containment 和符号链接逃逸检查。

所有资源响应使用 `no-store`、`nosniff`、`DENY` framing、无 referrer 和禁止执行的 CSP；HTML、XHTML 与 XML 还会强制下载。资源以流式方式返回，单文件最大 64 MiB。客户端获取入口 HTML 后，会由受信宿主解析并在注入 CSP、`base` 与桥接启动脚本后放入无同源 sandbox。签名凭据只限制谁能取得当前 UI 资产，不会把客户端代码变成秘密存储；插件包仍不应包含 API key、令牌、私钥或其他秘密。

浏览器/WebView 内部的 bridge 使用每文档随机 `bridgeToken`，并绑定到宿主在插件代码执行前创建的首个 `MessagePort`。`bridgeToken` 不是 `client_grant` / `ui_grant`，也不能用于插件资产或 HTTP API；服务端签名凭据由受信客户端获取并附加，不会作为 `bridgeToken` 或 `ting-plugin:init` 字段传入。由于 grant 同时是资产 URL 的路径段，插件 UI 仍可能从自身资源地址观察到它，因此也必须视为秘密，不得记录或外传。端口绑定用于阻止 iframe 跳转后的页面复用 `WindowProxy` 接管能力。插件请求必须通过 `window.__TING_PLUGIN_BRIDGE__.postMessage()` 发出，详见插件 HostGateway 指南。

## HostGateway API

### POST /api/v1/plugin-host/invoke

由前端受控调用插件可访问的 HostGateway 方法。后端会同时校验：

- `ui_capability_id` 是否属于同一插件的 UI capability。
- `ui_grant` 的签名、有效期、用户、插件和来源 UI capability 是否匹配。
- 对应 UI capability 的 `render.bridge.host_methods` 是否声明目标方法。
- 插件 manifest 是否声明了对应权限。
- 当前用户是否有目标书籍/书库访问权限。
- 目标方法是否允许在当前认证上下文中调用。

请求体：

```json
{
  "plugin_id": "advanced-capabilities-example@0.1.0",
  "ui_capability_id": "assistant.panel",
  "ui_grant": "<opaque-client-grant>",
  "method": "progress.recent",
  "params": {
    "limit": 5
  }
}
```

`ui_capability_id` 和 `ui_grant` 均为必填字段。旧客户端在升级后必须从 capability 注册响应保存当前 UI 的不透明凭据，并在转发 bridge 请求时附加；插件页面只处理每文档 `bridgeToken`，不应接触或缓存 `ui_grant`。仅修改请求字段而未在 manifest 声明 `render.bridge.host_methods` 仍会被拒绝。

响应：`200 OK`

```json
{
  "result": []
}
```

当前常用方法：

| 方法 | 权限 |
| --- | --- |
| `books.list` / `books.get` | `books_read` |
| `libraries.list` / `libraries.get` | `books_read` |
| `chapters.list` / `chapters.get` | `chapters_read` |
| `progress.recent` | `progress_read` |
| `media.get_url` | `media_read_url` 或 `media_read` |
| `metadata.write` | `metadata_write` + admin |
| `library.file.list` / `library.file.stat` / `library.file.read` | `file_read` |
| `library.file.write` | `file_write` + admin |
| `database.get` / `database.list` | `database_read` |
| `database.update` | `database_write` + admin |
| `tasks.create` | `task_create` |
| `cache.get` / `cache.has` | `cache_read` 或 `cache_write` |
| `cache.set` / `cache.delete` | `cache_write` |

## 插件路由签名

### POST /api/v1/plugin-route-signatures

为公共插件路由生成签名 URL。默认绑定当前用户，签名中包含 `user`，公共请求校验后会恢复 signed-user 上下文。RSS 订阅等外部客户端可用该 URL 访问当前用户有权限的内容。

请求体：

```json
{
  "method": "GET",
  "path": "/rss/library-id.xml",
  "expires_in_seconds": 86400,
  "bind_current_user": true
}
```

响应：`200 OK`

```json
{
  "path": "/rss/library-id.xml",
  "expires": 1790000000,
  "signature": "hex",
  "user_id": "user-id",
  "signed_url": "/api/v1/public/plugin-routes/rss/library-id.xml?expires=1790000000&signature=hex&user=user-id"
}
```
