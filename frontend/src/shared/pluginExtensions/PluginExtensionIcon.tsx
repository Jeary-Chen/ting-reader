import { icons, PlugZap } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useState } from "react";
import type {
  ClientExtensionDescriptor,
  ClientExtensionIcon,
} from "../../core/pluginExtensions";
import { useAuthStore } from "../../core/stores/authStore";
import { getRuntimeUrl } from "../../core/utils/runtimeUrl";

const iconComponents = icons as Record<string, LucideIcon | undefined>;

const toPascalCase = (value: string) =>
  value
    .trim()
    .replace(/(^|[-_\s]+)([a-zA-Z0-9])/g, (_, __, letter: string) =>
      letter.toUpperCase(),
    )
    .replace(/[^a-zA-Z0-9]/g, "");

const iconText = (icon: ClientExtensionIcon | undefined) => {
  if (typeof icon === "string") return icon.trim();
  if (!icon || typeof icon !== "object") return undefined;
  return (icon.src || icon.name || icon.value || "").trim();
};

const pluginImageUrl = (
  extension: ClientExtensionDescriptor,
  value: string,
  activeUrl?: string,
) => {
  if (!value.startsWith("assets/")) return undefined;
  const segments = value.split("/");
  if (
    segments.some(
      (segment) => !segment || segment === "." || segment === "..",
    )
  ) {
    return undefined;
  }
  const entry = segments.map(encodeURIComponent).join("/");
  if (!extension.clientGrant) return undefined;
  return getRuntimeUrl(
    `/api/v1/plugin-assets/${encodeURIComponent(extension.clientGrant)}/${encodeURIComponent(extension.pluginId)}/${entry}`,
    activeUrl,
  );
};

const PluginImageIcon = ({
  src,
  size,
}: {
  src: string;
  size: number;
}) => {
  const [failed, setFailed] = useState(false);
  if (failed) return <PlugZap size={size} strokeWidth={2.2} />;
  return (
    <img
      src={src}
      alt=""
      className="h-full w-full rounded-[inherit] object-cover"
      draggable={false}
      referrerPolicy="no-referrer"
      onError={() => setFailed(true)}
    />
  );
};

const PluginExtensionIcon = ({
  extension,
  size = 18,
}: {
  extension: ClientExtensionDescriptor;
  size?: number;
}) => {
  const activeUrl = useAuthStore((state) => state.activeUrl);
  const raw = extension.icon;
  const value = iconText(raw);
  const imageUrl = value
    ? pluginImageUrl(extension, value, activeUrl)
    : undefined;
  const explicitType =
    raw && typeof raw === "object" && typeof raw.type === "string"
      ? raw.type
      : undefined;

  if (
    imageUrl &&
    (explicitType === "image" || value?.startsWith("assets/"))
  ) {
    return <PluginImageIcon src={imageUrl} size={size} />;
  }

  if (value && explicitType !== "emoji") {
    const Icon =
      iconComponents[value] ||
      iconComponents[toPascalCase(value)] ||
      iconComponents[toPascalCase(value.replace(/^lucide:/i, ""))];
    if (Icon) {
      return <Icon size={size} strokeWidth={2.2} />;
    }
  }

  if (value) {
    return (
      <span
        className="text-center text-[18px] leading-none"
        aria-hidden="true"
      >
        {value}
      </span>
    );
  }

  return <PlugZap size={size} strokeWidth={2.2} />;
};

export default PluginExtensionIcon;
