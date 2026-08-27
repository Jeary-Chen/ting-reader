# JavaScript 运行时开发指南

JavaScript 运行时适合快速接入 HTTP API、解析网页、提供插件商店源、编写工具能力或轻量 UI 后端逻辑。它不局限于刮削；只要 capability 声明了调用入口，就可以承载工具、UI 后端、任务处理、插件商店源等能力。运行时提供 `fetch`、`Ting.log`、`Ting.host.invoke` 和声明式 npm 依赖。

## 结构化日志

后台 JavaScript 插件应使用 `Ting.log.debug/info/warn/error(message, fields)`。第二个参数是可选 JSON 对象；建议至少提供稳定的 `op`，并使用 ID、数量、状态等可检索字段，不要写访问令牌、Cookie、密码、API key、完整请求体或用户隐私内容。

```javascript
Ting.log.info("RSS feed generated", {
  op: "rss.generate",
  book_id: bookId,
  item_count: items.length
});

Ting.log.error("RSS feed generation failed", {
  op: "rss.generate",
  book_id: bookId,
  error_code: "upstream_timeout"
});
```

旧的 `Ting.log.info("message")` 继续可用，`console.log/info/warn/error` 也会被宿主转为插件日志。宿主会绑定 `plugin_id`、`plugin_instance_id`、`plugin_version`、`runtime`、`source` 和 `event_id`，插件代码不能伪造这些身份字段。日志只在当前 Ting Reader 实例的管理员系统日志中显示，不会自动上传，也不会向外部插件作者开放。

## 项目结构

```text
my-js-plugin/
  plugin.yml
  plugin.js
  ui/
    index.html
```

## plugin.yml 示例

```yaml
id: example-metadata-js
name: Example Metadata JS
version: 1.0.0
min_core_version: 1.4.8
runtime: javascript
entry_point: plugin.js
npm_dependencies:
  cheerio: "^1.0.0"
capabilities:
  - id: metadata.search
    kind: metadata_provider
    invoke: search
    auto_scrape: true
    search_fields:
      - key: title
        label: { zh: 书名, en: Title }
        type: text
        required: true
        default_from: book.title
    result_fields:
      - key: title
        label: { zh: 书名, en: Title }
      - key: author
        label: { zh: 作者, en: Author }
      - key: cover_url
        label: { zh: 封面, en: Cover }
permissions:
  - type: network_access
    value: "*.example.com"
```

`metadata_provider` 中的 `search_fields` 决定前端搜索表单，`result_fields` 决定搜索结果可采用字段。需要进入存储库自动刮削配置时设置 `auto_scrape: true`，并提供必填书名字段。

内置 `fetch` 只接受 HTTP/HTTPS URL，不允许 URL 携带用户名或密码。HTTPS 使用系统信任链校验证书；每次重定向都会重新检查 `network_access` 域名权限、DNS 解析结果和实际连接地址，回环、私网、链路本地及保留地址始终拒绝，即使权限值为 `*`。单次请求最多跟随 5 次重定向，响应体最多 16 MiB；超过限制时请求失败。跨来源重定向不会继续携带 `Authorization`、`Cookie` 或代理认证头。

插件需要管理员填写 API 地址、密钥、开关或模型参数时，在 `plugin.yml` 顶层声明 `config_schema`。完整写法见 [插件配置 `config_schema`](./plugin-config.md)。

```yaml
config_schema:
  type: object
  properties:
    api_key:
      type: string
      format: secret
      x-encrypted: true
      title:
        zh: API 密钥
        en: API key
    source_url:
      type: string
      title:
        zh: 数据源地址
        en: Source URL
      default: https://example.com/api
```

JavaScript 运行时通过 `Ting.config` 读取解密后的配置：

```javascript
const apiKey = Ting.config?.api_key || "";
const sourceUrl = Ting.config?.source_url || "https://example.com/api";
```

## plugin.js 示例

```javascript
const cheerio = require('cheerio');

async function search(args) {
  const keyword = args.title || args.query;
  const html = await (await fetch(
    'https://www.example.com/search?q=' + encodeURIComponent(keyword)
  )).text();
  const $ = cheerio.load(html);

  const items = $('.book').map((_, item) => ({
    id: $(item).attr('data-id'),
    title: $(item).find('.title').text().trim(),
    author: $(item).find('.author').text().trim() || null,
    cover_url: normalizeCover($(item).find('img').attr('src')),
    intro: $(item).find('.intro').text().trim() || null,
  })).get();

  return {
    items,
    total: items.length,
    page: args.page || 1,
    page_size: items.length,
  };
}

function normalizeCover(url) {
  if (!url) return null;
  return url.replace(/^http:/, 'https:').split('!')[0];
}

globalThis.search = search;
```

## npm 依赖

在 `npm_dependencies` 中声明的包会在插件加载前安装。运行时提供 CommonJS 风格的 `require`，只允许加载：

- manifest 中声明过的 npm 包。
- 插件目录内的相对模块，例如 `require('./helper')`。

```yaml
npm_dependencies:
  cheerio: "1.0.0"
  dayjs: "1.11.0"
  "@types/node": "18.0.0"
```

```javascript
const dayjs = require('dayjs');
const helper = require('./helper');
```

如果依赖包是 ESM-only，建议在插件项目构建阶段打包成 CommonJS 文件，或选择支持 CommonJS 的版本。

依赖声明只接受 npm registry 包：包名必须是小写的标准名称（支持 `@scope/package`），版本必须是完整、精确的 SemVer，例如 `1.2.3` 或 `2.0.0-beta.1`。`^`、`~`、比较器、通配符、联合范围、`file:`、`link:`、`git:`/`git+`、`http:`/`https:`、`workspace:`、`npm:` alias、相对路径和绝对路径都会被拒绝；`latest` 等 dist-tag 也不接受。这样缓存键和实际安装版本保持一致。需要本地源码或 Git 依赖时，请在发布前将其打包进插件产物，而不是交给实例端安装。

宿主安装依赖时固定使用 npm 官方 registry，并强制 `--ignore-scripts=true`、`--package-lock=false`。插件包中的 `.npmrc`、`package-lock.json`、`npm-shrinkwrap.json` 会导致安装被拒绝，`package.json` 也会在执行 npm 前按宿主的最小结构重写。因此插件不能通过 registry 配置、lockfile 或 `preinstall`/`install`/`postinstall` 改写安装行为。每次安装都会让 npm 重建完整 `node_modules` 依赖图，避免只还原顶层包时遗漏被提升的传递依赖；下载层仍可使用 npm 自身缓存。

## HostGateway

服务端 JS 插件可以通过 `Ting.host.invoke(method, params)` 访问核心数据。调用会按 manifest 权限和当前用户上下文校验。

```yaml
permissions:
  - type: books_read
  - type: database_read
  - type: cache_write
  - type: file_read
    value: library
```

```javascript
async function recentBooks() {
  const context = Ting.host.getContext();
  const recent = await Ting.host.invoke('progress.recent', { limit: 20 });
  const book = context?.book_id
    ? await Ting.host.invoke('database.get', { entity: 'book', id: context.book_id })
    : null;
  return { recent, book };
}

globalThis.recentBooks = recentBooks;
```

常用方法、请求参数、返回格式和错误处理见 [HostGateway 能力调用详解](./hostgateway.md)。

## 打包

```bash
trpack validate .
trpack build . --output dist/example-metadata-js.tr
trpack verify dist/example-metadata-js.tr
```
