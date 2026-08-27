import { ExternalLink, Loader2, X } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  invokePluginCapability,
  invokePluginHost,
} from "../../core/api/pluginCapabilities";
import apiClient from "../../core/api/client";
import type { ClientExtensionDescriptor } from "../../core/pluginExtensions";
import { useAuthStore } from "../../core/stores/authStore";
import { getRuntimeUrl } from "../../core/utils/runtimeUrl";

type PluginBridgeRequest = {
  type: "ting-plugin:request";
  bridge_token: string;
  id: string;
  method: "capability.invoke" | "host.invoke";
  params?: unknown;
};

type PluginBridgeResponse = {
  type: "ting-plugin:response";
  bridge_token: string;
  id: string;
  ok: boolean;
  result?: unknown;
  error?: string;
};

type PluginExternalUrlRequest = {
  type: "ting-plugin:external-url";
  bridge_token: string;
  url: string;
};

type PluginDocument = {
  html: string;
  token: string;
};

type PluginDocumentGeneration = {
  token: string;
  frame?: Window;
  port?: MessagePort;
  loadCount: number;
  initialized: boolean;
  invalidated: boolean;
  requestTimestamps: number[];
};

type PluginWebContainerProps = {
  extension: ClientExtensionDescriptor;
  context?: Record<string, unknown>;
};

type PluginTheme = {
  colorScheme: "light" | "dark";
  brightness: "light" | "dark";
  cssVariables: Record<string, string>;
};

const MAX_BRIDGE_MESSAGE_BYTES = 256 * 1024;
const MAX_BRIDGE_REQUESTS_PER_WINDOW = 100;
const BRIDGE_RATE_WINDOW_MS = 10_000;
const MAX_BRIDGE_JSON_NODES = 20_000;

const pluginThemeFromDocument = (): PluginTheme => {
  const colorScheme = document.documentElement.classList.contains("dark")
    ? "dark"
    : "light";
  const cssVariables =
    colorScheme === "dark"
      ? {
          "--bg": "#020617",
          "--panel": "#0f172a",
          "--text": "#f8fafc",
          "--muted": "#cbd5e1",
          "--line": "#1e293b",
          "--accent": "#7dd3fc",
          "--soft": "#082f49",
          "--danger": "#fca5a5",
        }
      : {
          "--bg": "#f8fafc",
          "--panel": "#ffffff",
          "--text": "#0f172a",
          "--muted": "#475569",
          "--line": "#e2e8f0",
          "--accent": "#0284c7",
          "--soft": "#f0f9ff",
          "--danger": "#dc2626",
        };
  return { colorScheme, brightness: colorScheme, cssVariables };
};

const createBridgeToken = () => {
  const bytes = new Uint8Array(24);
  crypto.getRandomValues(bytes);
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join(
    "",
  );
};

const isPlainBridgeValue = (
  value: unknown,
  depth: number,
  state: { nodes: number; seen: WeakSet<object> },
): boolean => {
  if (value === null) return true;
  if (typeof value === "string") {
    return value.length <= MAX_BRIDGE_MESSAGE_BYTES;
  }
  if (typeof value === "boolean") return true;
  if (typeof value === "number") return Number.isFinite(value);
  if (typeof value !== "object" || depth > 64) return false;

  state.nodes += 1;
  if (state.nodes > MAX_BRIDGE_JSON_NODES || state.seen.has(value)) {
    return false;
  }
  state.seen.add(value);

  let valid: boolean;
  if (Array.isArray(value)) {
    valid = value.every((entry) =>
      isPlainBridgeValue(entry, depth + 1, state),
    );
  } else {
    const prototype = Object.getPrototypeOf(value);
    valid =
      (prototype === Object.prototype || prototype === null) &&
      Object.entries(value as Record<string, unknown>).every(
        ([key, entry]) =>
          key.length <= MAX_BRIDGE_MESSAGE_BYTES &&
          isPlainBridgeValue(entry, depth + 1, state),
      );
  }

  state.seen.delete(value);
  return valid;
};

const isBridgeMessageWithinLimit = (value: unknown) => {
  try {
    if (
      !isPlainBridgeValue(value, 0, {
        nodes: 0,
        seen: new WeakSet<object>(),
      })
    ) {
      return false;
    }
    const serialized = JSON.stringify(value);
    if (typeof serialized !== "string") return false;
    const byteLength =
      typeof TextEncoder === "undefined"
        ? new Blob([serialized]).size
        : new TextEncoder().encode(serialized).byteLength;
    return byteLength <= MAX_BRIDGE_MESSAGE_BYTES;
  } catch {
    return false;
  }
};

const isBridgeRequest = (
  value: unknown,
  expectedToken: string,
): value is PluginBridgeRequest => {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Partial<PluginBridgeRequest>;
  return (
    candidate.type === "ting-plugin:request" &&
    candidate.bridge_token === expectedToken &&
    typeof candidate.id === "string" &&
    candidate.id.length > 0 &&
    candidate.id.length <= 128 &&
    (candidate.method === "capability.invoke" ||
      candidate.method === "host.invoke")
  );
};

const isDocumentReady = (value: unknown, expectedToken: string) => {
  if (!value || typeof value !== "object") return false;
  const candidate = value as {
    type?: unknown;
    bridge_token?: unknown;
  };
  return (
    candidate.type === "ting-plugin:document-ready" &&
    candidate.bridge_token === expectedToken
  );
};

const isDocumentUnloading = (value: unknown, expectedToken: string) => {
  if (!value || typeof value !== "object") return false;
  const candidate = value as {
    type?: unknown;
    bridge_token?: unknown;
  };
  return (
    candidate.type === "ting-plugin:document-unloading" &&
    candidate.bridge_token === expectedToken
  );
};

const externalUrlRequest = (
  value: unknown,
  expectedToken: string,
): PluginExternalUrlRequest | undefined => {
  if (!value || typeof value !== "object") return undefined;
  const candidate = value as Partial<PluginExternalUrlRequest>;
  if (
    candidate.type !== "ting-plugin:external-url" ||
    candidate.bridge_token !== expectedToken ||
    typeof candidate.url !== "string" ||
    candidate.url.length === 0 ||
    candidate.url.length > 2048
  ) {
    return undefined;
  }
  try {
    const parsed = new URL(candidate.url);
    if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
      return undefined;
    }
    return {
      type: "ting-plugin:external-url",
      bridge_token: expectedToken,
      url: parsed.href,
    };
  } catch {
    return undefined;
  }
};

const sanitizeStringArray = (value: unknown, maxLength: number) => {
  if (!Array.isArray(value)) return [];
  return value
    .filter((item): item is string => typeof item === "string")
    .map((item) => item.trim())
    .filter((item) => item.length > 0 && item.length <= maxLength);
};

const pluginAssetPath = (extension: ClientExtensionDescriptor) => {
  const entry = extension.render?.entry?.replace(/^\/+/, "");
  if (!entry) return undefined;
  if (entry.includes("\\") || entry.includes("\0")) return undefined;
  const segments = entry.split("/");
  if (
    segments.some(
      (segment) => !segment || segment === "." || segment === "..",
    )
  ) {
    return undefined;
  }
  const encodedEntry = segments
    .map((segment) => encodeURIComponent(segment))
    .join("/");
  if (!extension.clientGrant) return undefined;
  return `/api/v1/plugin-assets/${encodeURIComponent(extension.clientGrant)}/${encodeURIComponent(extension.pluginId)}/${encodedEntry}`;
};

const absoluteAssetUrl = (path: string, activeUrl?: string) => {
  try {
    const runtimeUrl = getRuntimeUrl(path, activeUrl);
    if (/^https?:\/\//i.test(runtimeUrl)) return runtimeUrl;
    if (typeof window === "undefined") return undefined;
    return new URL(runtimeUrl, window.location.href).toString();
  } catch {
    return undefined;
  }
};

const withPluginDocumentPolicy = (
  html: string,
  href: string,
  token: string,
  theme: PluginTheme,
) => {
  const document = new DOMParser().parseFromString(html, "text/html");
  const origin = new URL(href).origin;
  const policy = [
    "default-src 'none'",
    `script-src 'unsafe-inline' ${origin}`,
    `style-src 'unsafe-inline' ${origin}`,
    `font-src data: ${origin}`,
    `img-src data: blob: ${origin}`,
    `media-src data: blob: ${origin}`,
    "connect-src 'none'",
    "object-src 'none'",
    "frame-src 'none'",
    "worker-src 'none'",
    "form-action 'none'",
    `base-uri ${origin}`,
  ].join("; ");

  document.querySelectorAll("base").forEach((base) => base.remove());
  const base = document.createElement("base");
  base.href = href;
  const csp = document.createElement("meta");
  csp.httpEquiv = "Content-Security-Policy";
  csp.content = policy;
  const lifecycle = document.createElement("script");
  lifecycle.textContent = `(() => {
    const bridgeToken = ${JSON.stringify(token)};
    const initialTheme = ${JSON.stringify(theme)};
    const channel = new MessageChannel();
    const bridgePort = channel.port1;
    const sendPort = bridgePort.postMessage.bind(bridgePort);
    const scheduleTask = window.setTimeout.bind(window);
    const lifecycleScript = document.currentScript;
    if (lifecycleScript) lifecycleScript.remove();
    let pluginListenersReady = document.readyState !== "loading";
    let externalNavigationPermit = false;
    let permitGeneration = 0;
    const queuedHostMessages = [];
    const deliverHostMessage = (message) => window.postMessage(message, "*");
    const applyTheme = (theme) => {
      const rawScheme = String(
        theme && (theme.colorScheme || theme.brightness) || "light"
      ).toLowerCase();
      const colorScheme = rawScheme.includes("dark") ? "dark" : "light";
      const root = document.documentElement;
      const variables = theme && theme.cssVariables || {};
      root.style.setProperty("color-scheme", colorScheme, "important");
      root.dataset.tingTheme = colorScheme;
      root.dataset.theme = colorScheme;
      root.classList.toggle("dark", colorScheme === "dark");
      root.classList.toggle("light", colorScheme === "light");
      Object.entries(variables).forEach(([name, value]) => {
        root.style.setProperty(name, String(value), "important");
      });
      if (document.body) {
        if (variables["--bg"]) {
          document.body.style.setProperty(
            "background-color",
            String(variables["--bg"]),
            "important",
          );
        }
        if (variables["--text"]) {
          document.body.style.setProperty(
            "color",
            String(variables["--text"]),
            "important",
          );
        }
      }
      window.__tingPluginTheme = theme;
    };
    Object.defineProperty(window, "__tingPluginApplyTheme", {
      value: applyTheme,
      configurable: false,
      enumerable: false,
      writable: false,
    });
    applyTheme(initialTheme);
    const markPluginListenersReady = () => {
      if (pluginListenersReady) return;
      pluginListenersReady = true;
      queuedHostMessages.splice(0).forEach(deliverHostMessage);
    };
    if (!pluginListenersReady) {
      document.addEventListener("DOMContentLoaded", () => {
        scheduleTask(markPluginListenersReady, 0);
      }, { capture: true, once: true });
    }
    const postBridgeMessage = (message) => {
      sendPort(Object.assign({}, message, { bridge_token: bridgeToken }));
    };
    Object.defineProperty(window, "__TING_PLUGIN_BRIDGE__", {
      value: Object.freeze({
        postMessage(message) {
          if (
            !message ||
            typeof message !== "object" ||
            message.type !== "ting-plugin:request"
          ) return;
          postBridgeMessage(message);
        },
      }),
      configurable: false,
      enumerable: false,
      writable: false,
    });
    bridgePort.addEventListener("message", (event) => {
      const message = event.data;
      if (!message || message.bridge_token !== bridgeToken) return;
      if (message.type === "ting-plugin:theme") applyTheme(message.theme);
      if (pluginListenersReady) {
        deliverHostMessage(message);
      } else if (queuedHostMessages.length < 128) {
        queuedHostMessages.push(message);
      }
    });
    bridgePort.start();
    window.addEventListener("beforeunload", () => {
      postBridgeMessage({ type: "ting-plugin:document-unloading" });
    }, { capture: true });
    window.addEventListener("pagehide", () => bridgePort.close(), {
      capture: true,
      once: true,
    });
    const pluginAssetUrl = ${JSON.stringify(href)};
    const absolutePluginUrl = (url) => {
      try { return new URL(url, pluginAssetUrl).href; } catch (_error) { return ""; }
    };
    const isExternalUrl = (url) => {
      const absoluteUrl = absolutePluginUrl(url);
      if (!absoluteUrl) return false;
      try {
        const parsed = new URL(absoluteUrl);
        if (parsed.protocol !== "http:" && parsed.protocol !== "https:") return false;
        return absoluteUrl !== pluginAssetUrl &&
          !absoluteUrl.startsWith(pluginAssetUrl + "#") &&
          !absoluteUrl.startsWith(pluginAssetUrl + "?");
      } catch (_error) {
        return false;
      }
    };
    const grantExternalNavigation = () => {
      const generation = ++permitGeneration;
      externalNavigationPermit = true;
      scheduleTask(() => {
        if (permitGeneration === generation) externalNavigationPermit = false;
      }, 0);
    };
    const requestExternalNavigation = (url) => {
      if (!externalNavigationPermit) return;
      externalNavigationPermit = false;
      const absoluteUrl = absolutePluginUrl(url);
      if (!isExternalUrl(absoluteUrl)) return;
      postBridgeMessage({ type: "ting-plugin:external-url", url: absoluteUrl });
    };
    Object.defineProperty(window, "open", {
      value(url) {
        if (url && isExternalUrl(url)) requestExternalNavigation(url);
        return null;
      },
      configurable: false,
      writable: false,
    });
    window.addEventListener("click", (event) => {
      if (event.isTrusted) grantExternalNavigation();
      const target = event.target;
      const anchor = target && target.closest ? target.closest("a[href]") : null;
      if (!anchor || !isExternalUrl(anchor.href)) return;
      event.preventDefault();
      event.stopImmediatePropagation();
      requestExternalNavigation(anchor.href);
    }, { capture: true });
    window.parent.postMessage({
      type: "ting-plugin:document-ready",
      bridge_token: bridgeToken,
    }, "*", [channel.port2]);
  })();`;
  document.head.prepend(lifecycle);
  document.head.prepend(base);
  document.head.prepend(csp);
  return `<!doctype html>\n${document.documentElement.outerHTML}`;
};

const responseFor = (
  request: PluginBridgeRequest,
  payload: Omit<PluginBridgeResponse, "type" | "bridge_token" | "id">,
): PluginBridgeResponse => ({
  type: "ting-plugin:response",
  bridge_token: request.bridge_token,
  id: request.id,
  ...payload,
});

const PluginWebContainer = ({
  extension,
  context,
}: PluginWebContainerProps) => {
  const { t } = useTranslation();
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const generationRef = useRef<PluginDocumentGeneration | undefined>(
    undefined,
  );
  const activeUrl = useAuthStore((state) => state.activeUrl);
  const [loadError, setLoadError] = useState<string>();
  const [pluginDocument, setPluginDocument] = useState<PluginDocument>();
  const [pendingExternalUrl, setPendingExternalUrl] = useState<string>();
  const src = useMemo(() => pluginAssetPath(extension), [extension]);
  const srcBaseUrl = useMemo(
    () => (src ? absoluteAssetUrl(src, activeUrl) : undefined),
    [activeUrl, src],
  );
  const allowedHostMethods = useMemo(
    () =>
      new Set(
        sanitizeStringArray(extension.render?.bridge?.host_methods, 128),
      ),
    [extension.render?.bridge?.host_methods],
  );
  const allowedCapabilityIds = useMemo(() => {
    const configured = sanitizeStringArray(
      extension.render?.bridge?.capabilities,
      128,
    );
    const currentCapabilityId =
      typeof extension.capability.id === "string"
        ? extension.capability.id.trim()
        : "";
    return new Set(
      currentCapabilityId && currentCapabilityId.length <= 128
        ? [currentCapabilityId, ...configured]
        : configured,
    );
  }, [extension.capability.id, extension.render?.bridge?.capabilities]);
  const bridgeGenerationKey = JSON.stringify({
    pluginId: extension.pluginId,
    pluginName: extension.pluginName,
    capabilityId: extension.capability.id,
    slot: extension.slot,
    contexts: extension.contexts,
    bridge: extension.render?.bridge || {},
    context: context || {},
  });

  const invalidateGeneration = (generation?: PluginDocumentGeneration) => {
    if (!generation) return;
    generation.invalidated = true;
    generation.port?.close();
    generation.port = undefined;
    generation.frame = undefined;
    generation.requestTimestamps = [];
  };

  const blockPluginNavigation = (generation: PluginDocumentGeneration) => {
    if (generationRef.current !== generation || generation.invalidated) return;
    invalidateGeneration(generation);
    setPendingExternalUrl(undefined);
    setPluginDocument(undefined);
    setLoadError(t("pluginExtensions.navigationBlocked"));
  };

  const postToGeneration = (
    generation: PluginDocumentGeneration,
    message: unknown,
  ) => {
    if (
      generationRef.current !== generation ||
      generation.invalidated ||
      !generation.initialized ||
      !generation.port
    ) {
      return;
    }
    generation.port.postMessage(message);
  };

  const handleLoad = () => {
    const generation = generationRef.current;
    const frame = iframeRef.current?.contentWindow;
    if (!generation || !frame || generation.invalidated) return;
    generation.loadCount += 1;
    if (generation.loadCount > 1) {
      blockPluginNavigation(generation);
    }
  };

  useEffect(() => {
    let cancelled = false;
    const timer = window.setTimeout(() => {
      invalidateGeneration(generationRef.current);
      generationRef.current = undefined;
      setPendingExternalUrl(undefined);
      if (!src || !srcBaseUrl) {
        setPluginDocument(undefined);
        return;
      }

      setLoadError(undefined);
      setPluginDocument(undefined);
      void apiClient
        .get<string>(src, { responseType: "text" })
        .then((response) => {
          if (cancelled) return;
          const html =
            typeof response.data === "string"
              ? response.data
              : String(response.data ?? "");
          const token = createBridgeToken();
          const generation: PluginDocumentGeneration = {
            token,
            loadCount: 0,
            initialized: false,
            invalidated: false,
            requestTimestamps: [],
          };
          generationRef.current = generation;
          setPluginDocument({
            html: withPluginDocumentPolicy(
              html,
              srcBaseUrl,
              token,
              pluginThemeFromDocument(),
            ),
            token,
          });
        })
        .catch((error) => {
          if (cancelled) return;
          setLoadError(
            error instanceof Error
              ? error.message
              : "Plugin UI failed to load.",
          );
        });
    }, 0);

    return () => {
      cancelled = true;
      window.clearTimeout(timer);
      invalidateGeneration(generationRef.current);
    };
  }, [bridgeGenerationKey, src, srcBaseUrl]);

  const handleBridgeMessage = async (
    generation: PluginDocumentGeneration,
    data: unknown,
  ) => {
    if (
      generationRef.current !== generation ||
      generation.invalidated ||
      !generation.initialized ||
      !isBridgeMessageWithinLimit(data)
    ) {
      return;
    }
    if (isDocumentUnloading(data, generation.token)) {
      blockPluginNavigation(generation);
      return;
    }

    const externalRequest = externalUrlRequest(data, generation.token);
    if (externalRequest) {
      setPendingExternalUrl(externalRequest.url);
      return;
    }
    if (!isBridgeRequest(data, generation.token)) return;

    const now = Date.now();
    generation.requestTimestamps = generation.requestTimestamps.filter(
      (timestamp) => now - timestamp < BRIDGE_RATE_WINDOW_MS,
    );
    if (
      generation.requestTimestamps.length >= MAX_BRIDGE_REQUESTS_PER_WINDOW
    ) {
      postToGeneration(
        generation,
        responseFor(data, {
          ok: false,
          error: "Plugin bridge rate limit exceeded",
        }),
      );
      return;
    }
    generation.requestTimestamps.push(now);

    const request = data;
    try {
      if (request.method === "capability.invoke") {
        if (extension.render?.bridge?.allow_capability_invoke === false) {
          throw new Error("Capability invocation is disabled for this view");
        }
        const params =
          request.params && typeof request.params === "object"
            ? (request.params as {
                capabilityId?: string;
                params?: unknown;
              })
            : {};
        const requestedCapabilityId =
          typeof params.capabilityId === "string" &&
          params.capabilityId.trim()
            ? params.capabilityId.trim()
            : extension.capability.id;
        if (
          requestedCapabilityId.length > 128 ||
          !allowedCapabilityIds.has(requestedCapabilityId)
        ) {
          throw new Error("Capability is not allowed for this view");
        }
        const result = await invokePluginCapability(
          extension.pluginId,
          requestedCapabilityId,
          params.params ?? {},
          extension.capability.id,
          extension.clientGrant,
        );
        postToGeneration(
          generation,
          responseFor(request, { ok: true, result }),
        );
        return;
      }

      const params =
        request.params && typeof request.params === "object"
          ? (request.params as { method?: string; params?: unknown })
          : {};
      const hostMethod =
        typeof params.method === "string" ? params.method.trim() : "";
      if (!hostMethod) {
        throw new Error("Missing host method");
      }
      if (hostMethod.length > 128 || !allowedHostMethods.has(hostMethod)) {
        throw new Error("Host method is not allowed for this view");
      }

      const result = await invokePluginHost({
        plugin_id: extension.pluginId,
        ui_capability_id: extension.capability.id,
        ui_grant: extension.clientGrant || "",
        method: hostMethod,
        params: params.params ?? {},
      });
      postToGeneration(
        generation,
        responseFor(request, { ok: true, result }),
      );
    } catch (err) {
      postToGeneration(
        generation,
        responseFor(request, {
          ok: false,
          error: err instanceof Error ? err.message : String(err),
        }),
      );
    }
  };

  const handleMessage = (event: MessageEvent) => {
    const generation = generationRef.current;
    const frame = iframeRef.current?.contentWindow;
    if (
      !generation ||
      generation.invalidated ||
      generation.initialized ||
      !frame ||
      event.source !== frame ||
      event.ports.length !== 1 ||
      !isBridgeMessageWithinLimit(event.data) ||
      !isDocumentReady(event.data, generation.token)
    ) {
      return;
    }

    const port = event.ports[0];
    generation.initialized = true;
    generation.frame = frame;
    generation.port = port;
    port.onmessage = (portEvent) => {
      void handleBridgeMessage(generation, portEvent.data);
    };
    port.onmessageerror = () => invalidateGeneration(generation);
    port.start();
    setLoadError(undefined);
    postToGeneration(generation, {
      type: "ting-plugin:init",
      bridge_token: generation.token,
      bridgeToken: generation.token,
      pluginId: extension.pluginId,
      pluginName: extension.pluginName,
      capabilityId: extension.capability.id,
      slot: extension.slot,
      contexts: extension.contexts,
      context: context || {},
      theme: pluginThemeFromDocument(),
    });
  };

  useEffect(() => {
    window.addEventListener("message", handleMessage);
    return () => window.removeEventListener("message", handleMessage);
  });

  useEffect(() => {
    const root = document.documentElement;
    const observer = new MutationObserver(() => {
      const generation = generationRef.current;
      if (
        !generation ||
        generation.invalidated ||
        !generation.initialized ||
        !generation.port
      ) {
        return;
      }
      generation.port.postMessage({
        type: "ting-plugin:theme",
        bridge_token: generation.token,
        theme: pluginThemeFromDocument(),
      });
    });
    observer.observe(root, { attributes: true, attributeFilter: ["class"] });
    return () => observer.disconnect();
  }, []);

  if (!src) {
    return (
      <div className="flex h-full items-center justify-center px-6 text-center text-sm text-slate-500 dark:text-slate-400">
        {t("pluginExtensions.missingEntry")}
      </div>
    );
  }

  return (
    <div className="relative h-full w-full">
      {loadError ? (
        <div className="absolute inset-x-4 top-4 z-10 rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700 dark:border-red-900/40 dark:bg-red-950/40 dark:text-red-300">
          {loadError}
        </div>
      ) : null}
      {pendingExternalUrl ? (
        <div className="absolute inset-x-4 bottom-4 z-20 flex min-w-0 items-center gap-2 rounded-md border border-slate-200 bg-white px-3 py-2 shadow-lg dark:border-slate-700 dark:bg-slate-900">
          <ExternalLink size={16} className="shrink-0 text-slate-500" />
          <span className="min-w-0 flex-1 truncate text-sm text-slate-700 dark:text-slate-200" title={pendingExternalUrl}>
            {pendingExternalUrl}
          </span>
          <a
            href={pendingExternalUrl}
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex size-8 shrink-0 items-center justify-center rounded-md text-slate-600 hover:bg-slate-100 dark:text-slate-300 dark:hover:bg-slate-800"
            title={t("pluginExtensions.openExternalLink")}
            aria-label={t("pluginExtensions.openExternalLink")}
            onClick={() => setPendingExternalUrl(undefined)}
          >
            <ExternalLink size={17} />
          </a>
          <button
            type="button"
            className="inline-flex size-8 shrink-0 items-center justify-center rounded-md text-slate-600 hover:bg-slate-100 dark:text-slate-300 dark:hover:bg-slate-800"
            title={t("common.cancel")}
            aria-label={t("common.cancel")}
            onClick={() => setPendingExternalUrl(undefined)}
          >
            <X size={17} />
          </button>
        </div>
      ) : null}
      {!pluginDocument && !loadError ? (
        <div className="flex h-full items-center justify-center gap-2 text-sm text-slate-500 dark:text-slate-400">
          <Loader2 size={16} className="animate-spin" />
          <span>{t("pluginExtensions.loadingUi")}</span>
        </div>
      ) : null}
      {pluginDocument ? (
        <iframe
          key={pluginDocument.token}
          ref={iframeRef}
          srcDoc={pluginDocument.html}
          title={extension.title || extension.capability.id}
          sandbox="allow-scripts allow-forms"
          referrerPolicy="no-referrer"
          className="h-full w-full border-0 bg-white dark:bg-slate-950"
          onLoad={handleLoad}
        />
      ) : null}
    </div>
  );
};

export default PluginWebContainer;
