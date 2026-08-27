import type {
  CapabilityRegistrationLike,
  ClientExtensionDescriptor,
  ClientExtensionRegistrySnapshot,
  ClientExtensionRenderMode,
  ClientExtensionSlot,
  UiExtensionCapabilityExtra,
  UiExtensionRenderConfig,
} from "./types";

const defaultSlots: ClientExtensionSlot[] = ["global.panel"];

const isClientExtensionSlot = (value: unknown): value is ClientExtensionSlot =>
  typeof value === "string" &&
  [
    "app.sidebar_page",
    "global.floating_action",
    "global.panel",
    "book.detail_action",
  ].includes(value);

const isRenderMode = (value: unknown): value is ClientExtensionRenderMode =>
  typeof value === "string" &&
  ["schema", "builtin", "web_container", "action"].includes(value);

const capabilityExtra = (
  registration: CapabilityRegistrationLike,
): UiExtensionCapabilityExtra =>
  registration.capability as UiExtensionCapabilityExtra;

const renderConfig = (
  extra: UiExtensionCapabilityExtra,
): UiExtensionRenderConfig | undefined =>
  typeof extra.render === "object" && extra.render !== null
    ? extra.render
    : undefined;

const normalizeSlots = (extra: UiExtensionCapabilityExtra) => {
  const declaredSlots = [
    ...(Array.isArray(extra.slots) ? extra.slots : []),
    extra.slot,
  ].filter((slot) => typeof slot === "string");
  const slots = declaredSlots.filter(isClientExtensionSlot);
  if (slots.length > 0) return slots;

  // Explicit legacy or unknown declarations must not silently become global panels.
  if (declaredSlots.length > 0) {
    return [];
  }

  return defaultSlots;
};

const normalizeContexts = (extra: UiExtensionCapabilityExtra) =>
  Array.isArray(extra.contexts || extra.context)
    ? (extra.contexts || extra.context || []).filter(
        (context): context is string => typeof context === "string",
      )
    : [];

const localizedText = (
  value: unknown,
  locale?: string,
): string | undefined => {
  if (typeof value === "string") {
    const trimmed = value.trim();
    return trimmed || undefined;
  }
  if (!value || typeof value !== "object") return undefined;

  const record = value as Record<string, unknown>;
  const normalizedLocale = locale?.replace("_", "-");
  const language = normalizedLocale?.split("-")[0];
  const candidates = [
    normalizedLocale ? record[normalizedLocale] : undefined,
    language ? record[language] : undefined,
    record["zh-CN"],
    record.zh,
    record["en-US"],
    record.en,
    ...Object.values(record),
  ];
  for (const candidate of candidates) {
    if (typeof candidate === "string" && candidate.trim()) {
      return candidate.trim();
    }
  }
  return undefined;
};

export const createClientExtensionDescriptor = (
  registration: CapabilityRegistrationLike,
  slot: ClientExtensionSlot,
  locale?: string,
): ClientExtensionDescriptor => {
  const extra = capabilityExtra(registration);
  const render = renderConfig(extra);
  const renderMode = isRenderMode(extra.render_mode)
    ? extra.render_mode
    : isRenderMode(extra.render)
      ? extra.render
      : isRenderMode(render?.mode)
        ? render.mode
        : "action";

  return {
    id: `${registration.plugin_id}:${registration.capability.id}:${slot}`,
    pluginId: registration.plugin_id,
    pluginName: registration.plugin_name,
    clientGrant: registration.client_grant,
    slot,
    renderMode,
    render,
    title:
      localizedText(extra.title, locale) || localizedText(extra.label, locale),
    icon: extra.icon,
    capability: registration.capability,
    priority: typeof extra.priority === "number" ? extra.priority : 100,
    contexts: normalizeContexts(extra),
  };
};

export const buildClientExtensionRegistry = (
  registrations: CapabilityRegistrationLike[],
  locale?: string,
): ClientExtensionRegistrySnapshot => {
  const extensions = registrations
    .filter(
      (registration) =>
        registration.capability.kind === "ui_extension" ||
        registration.capability.kind === "client_extension",
    )
    .flatMap((registration) =>
      normalizeSlots(capabilityExtra(registration)).map((slot) =>
        createClientExtensionDescriptor(registration, slot, locale),
      ),
    )
    .sort(
      (left, right) =>
        left.priority - right.priority || left.id.localeCompare(right.id),
    );

  const bySlot: ClientExtensionRegistrySnapshot["bySlot"] = {};
  for (const extension of extensions) {
    bySlot[extension.slot] = [...(bySlot[extension.slot] || []), extension];
  }

  return { extensions, bySlot };
};
