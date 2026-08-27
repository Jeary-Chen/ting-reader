# 插件入口与日志迁移规范

本规范用于把旧插件迁移到新的侧边栏入口、受控 Web 桥接和结构化插件日志。迁移不改变插件商店的插件化设计，也不会新增面向外部插件作者的日志权限。

## 1. UI 入口迁移

旧播放器入口已经停止渲染：

| 旧 slot | 迁移目标 |
| --- | --- |
| `reader.toolbar_action` | 简短动作迁移到 `global.floating_action` 或 `book.detail_action` |
| `reader.side_panel` | 完整页面迁移到 `app.sidebar_page` |
| `reader.document_viewer` | 按业务迁移到 `app.sidebar_page`，文档能力保留在插件后台 capability |

推荐让主要插件页面同时声明侧边栏和右下角快捷入口：

```yaml
capabilities:
  - id: assistant.panel
    kind: ui_extension
    invoke: openAssistant
    slots:
      - app.sidebar_page
      - global.floating_action
    title: { zh: 书单助手, en: Booklist Assistant }
    icon: message-circle
    priority: 20
    render:
      mode: web_container
      entry: ui/assistant.html
      bridge:
        capabilities:
          - assistant.tools
        host_methods:
          - user_settings.get
```

`book.detail_action` 继续保留。右下角工具菜单仍由 `global.floating_action` / `global.panel` 声明，位置不变；用户可在个性化设置中关闭，默认开启。侧边栏页面不受该开关影响。

## 2. Web 桥接迁移

1. 删除未声明的跨 capability 调用。当前 UI capability 默认可调用；确需调用同插件其他 capability 时，逐项写入 `render.bridge.capabilities`。
2. 收集 UI 实际使用的 HostGateway 方法，逐项写入 `render.bridge.host_methods`。
3. 确保 manifest 的 `permissions` 同时包含这些方法要求的最小权限。
4. 不依赖 iframe 同源、顶层跳转或逃离 sandbox 的弹窗。
5. `render.entry` 使用插件包内相对路径，禁止绝对路径、反斜杠和 `..`。
6. 仅从 `ting-plugin:init.bridgeToken` 取得当前文档令牌，通过只读的 `window.__TING_PLUGIN_BRIDGE__.postMessage()` 发送请求；不要再调用 `window.parent.postMessage()`。在每个 `ting-plugin:request` 中回传 `bridge_token`，并在处理响应前核对相同字段。宿主消息会投递到当前 `window`，监听器应校验 `event.source === window`。
7. 客户端从 `GET /api/v1/plugin-capabilities` 获取 UI 注册项时保存其 `client_grant`，把它当作不透明秘密，并在 capability 与 HostGateway HTTP 调用中以 `ui_grant` 传回。该 grant 绑定用户、插件和 UI capability，且会过期；不要解析、记录或通过 bridge 初始化消息主动暴露，过期后重新获取。grant 也是资产 URL 的路径段，插件 UI 如观察到该值，同样不得持久化或外传。

客户端和后端会同时校验来源 UI capability、服务端签名 grant 及 bridge 白名单。白名单只说明这个 UI 可以请求哪个方法，不会替代管理员上下文、用户数据范围和 manifest 权限校验。`POST /api/v1/plugin-host/invoke` 以及 UI 发起的 capability 调用现在都要求 `ui_capability_id` 和 `ui_grant`，旧客户端需要同步升级。插件资产路由也改为 `GET /api/v1/plugin-assets/:client_grant/:plugin_id/*path`，单文件最大 64 MiB。

Web 与 Flutter 容器都移除了 `allow-same-origin` 和逃离 sandbox 的弹窗权限，并校验消息来源、文档代际令牌、随机 nonce、消息结构、大小和频率。bridge 能力绑定到宿主脚本在插件代码执行前创建的首个 `MessagePort`，页面后续跳转不能只凭旧 token 接管能力。Flutter 的受信外层页面会先用 `DOMParser` 解析入口 HTML，再通过 DOM 注入 CSP、`base` 和桥接启动脚本，避免伪造 `<head>` 或提前脚本绕过策略。容器 CSP 禁止插件 UI 直接联网，也不允许用远程图片或媒体请求外传数据；需要宿主数据时只能走已声明 bridge。Web 和 Flutter 的 HTTP(S) 外链都必须来自真实用户点击，并由宿主展示目标地址、等待用户明确确认后才能打开；程序化跳转会被拒绝。插件 HTML、脚本、样式、图片和图标请放在包内 `ui/` / `assets/`，不要嵌入秘密。

## 3. 日志迁移

第一阶段继续写入现有 `system.json`，不要求插件单独管理日志文件。管理员可在系统日志中按插件、来源、等级和关键字筛选。

JavaScript 推荐写法：

```javascript
Ting.log.info("Booklist saved", {
  op: "booklist.save",
  playlist_id: playlistId,
  item_count: items.length
});
```

宿主自动补充：

- `event_id`
- `plugin_id`
- `plugin_instance_id`
- `plugin_version`
- `runtime`
- `source`
- `op`（从插件 fields 中提升，同时保留原始 fields）

禁止记录密钥、令牌、Cookie、Authorization 头、密码、完整请求/响应体和不必要的用户隐私。错误日志优先记录稳定错误码和必要上下文，不直接序列化未知异常对象。

## 4. 权限与可见性

- 插件日志只对当前实例管理员可见。
- 普通用户不能访问系统日志接口和页面。
- 外部插件作者没有远程查看实例日志的能力。
- 系统不会因为插件声明作者、仓库地址或商店来源而授予日志权限。

需要插件作者协助排查时，由实例管理员主动导出并脱敏后提供；未来如增加诊断包，也必须由管理员显式生成。

## 5. 清单与插件包安全迁移

升级旧插件时同时检查 manifest 和安装包：

- `id` 现在必填，最长 64 个字符；只能使用小写 ASCII 字母、数字、`-`、`_`、`.`，必须以字母或数字开头和结尾，不能包含 `..` 或使用 Windows 保留设备名。
- `version` 必须是合法 SemVer，例如 `1.2.0` 或 `2.0.0-beta.1`。
- `name` 不能包含控制字符、路径分隔符、Windows 保留设备名或危险的结尾字符。
- `entry_point` 必须是插件包内的安全相对路径，最长 240 个字符；不能包含反斜杠、绝对路径、`.`、`..` 或危险路径片段。
- JavaScript 依赖只接受标准小写 npm 包名（含 `@scope/package`）和精确 SemVer 版本；`^`、`~`、比较器、通配符、联合范围、`file:`、`link:`、Git、HTTP(S)、`workspace:`、npm alias、本地路径和 dist-tag 会被拒绝。旧 manifest 中的范围需先解析并固定到已验证版本，再重新打包。
- JavaScript 依赖安装固定官方 registry，禁用 npm 生命周期脚本和 lockfile。插件包不能携带 `.npmrc`、`package-lock.json` 或 `npm-shrinkwrap.json`，也不能依赖 `preinstall`、`install`、`postinstall` 等脚本完成构建；应在发布 `.tr` 前生成所需产物。
- 安装时不再从旧的顶层包缓存拼装 `node_modules`，而是始终让 npm 重建完整依赖图，避免遗漏被提升的传递依赖。旧缓存目录不会迁移复用，可由管理员清理；npm 自身的下载缓存不受影响。
- `.tr` 最多包含 10,000 个条目，单文件展开后最大 128 MiB，总展开大小最大 256 MiB。超限包会在安装前被拒绝。
- 商店安装包下载地址只接受 HTTPS，HTTP 和其他协议会被拒绝。下载会使用宿主生成的临时文件名并以流式方式写入，最大 50 MiB；不会再使用下载 URL 的末段作为本地路径。下载客户端禁用自动重定向，逐跳校验 HTTPS、DNS 和实际远端地址，拒绝本机、内网、链路本地及保留地址，并对连接和总请求设置超时；日志不记录查询参数或凭据。

插件目录和卸载目标会经过路径 containment 与符号链接检查。不要依赖软链接访问插件目录外文件，也不要根据未经校验的输入拼接插件 ID、版本或入口路径。

## 6. 发布前检查

1. 删除所有 `reader.*` UI slot。
2. 验证侧边栏折叠后自定义图标仍可识别。
3. 验证右下角插件工具开关关闭后入口隐藏、重新开启后恢复。
4. 验证书籍详情入口不受影响。
5. 验证未声明的 capability 和 `host.invoke` 均被拒绝，已声明调用仍通过后端权限检查。
6. 验证缺失、过期、其他用户、其他插件或其他 UI capability 的 `ui_grant` 均被拒绝，合法 grant 可以加载资产并转发 bridge 请求。
7. 验证 Web 与 Flutter 都会在宿主界面展示外链目标，并且只有用户明确确认后才打开。
8. 在系统日志中确认插件 ID、实例 ID、运行时、来源、操作和事件 ID 完整。
9. 扫描日志内容，确认没有敏感信息，也没有 `client_grant`、`ui_grant` 或完整资产 URL。
