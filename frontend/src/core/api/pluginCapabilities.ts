import type {
  PluginCapability,
  PluginCapabilityRegistration,
  ToolProviderRegistration,
} from "../types";
import apiClient from "./client";

type JsonRecord = Record<string, unknown>;

const isJsonRecord = (value: unknown): value is JsonRecord =>
  value !== null && typeof value === "object" && !Array.isArray(value);

const normalizeCapability = (value: unknown): PluginCapability | undefined => {
  if (!isJsonRecord(value)) return undefined;
  if (typeof value.id !== "string" || typeof value.kind !== "string") {
    return undefined;
  }
  return value as PluginCapability;
};

const normalizeRegistration = (
  value: unknown,
): PluginCapabilityRegistration | undefined => {
  if (!isJsonRecord(value)) return undefined;

  // The current API wraps the capability in `capability`. Keep accepting the
  // legacy flattened form so a gateway serving an older backend cannot crash
  // the authenticated shell.
  const capability =
    normalizeCapability(value.capability) || normalizeCapability(value);
  if (!capability) return undefined;

  const pluginId =
    typeof value.plugin_id === "string"
      ? value.plugin_id
      : typeof value.pluginId === "string"
        ? value.pluginId
        : undefined;
  const pluginName =
    typeof value.plugin_name === "string"
      ? value.plugin_name
      : typeof value.pluginName === "string"
        ? value.pluginName
        : undefined;
  if (!pluginId || !pluginName) return undefined;

  return {
    plugin_id: pluginId,
    plugin_name: pluginName,
    admin_only: value.admin_only === true,
    client_grant:
      typeof value.client_grant === "string" ? value.client_grant : undefined,
    capability,
  };
};

const normalizeRegistrationList = (
  payload: unknown,
): PluginCapabilityRegistration[] => {
  let values: unknown[] = [];
  if (Array.isArray(payload)) {
    values = payload;
  } else if (isJsonRecord(payload)) {
    const wrapped = payload.capabilities ?? payload.items ?? payload.data;
    if (Array.isArray(wrapped)) values = wrapped;
  }

  const registrations = values.flatMap((value) => {
    const registration = normalizeRegistration(value);
    return registration ? [registration] : [];
  });

  if (registrations.length !== values.length) {
    console.warn(
      `Ignored ${values.length - registrations.length} invalid plugin capability registration(s)`,
    );
  }

  return registrations;
};

export type PluginCapabilityInvokeResult<T = unknown> = {
  result: T;
};

export type SignPluginRouteRequest = {
  method: string;
  path: string;
  expires_in_seconds?: number;
  bind_current_user?: boolean;
};

export type SignPluginRouteResponse = {
  path: string;
  expires: number;
  signature: string;
  user_id?: string | null;
  signed_url: string;
};

export type InvokePluginHostRequest = {
  plugin_id: string;
  ui_capability_id: string;
  ui_grant: string;
  method: string;
  params?: unknown;
};

export type InvokePluginHostResponse<T = unknown> = {
  result: T;
};

export const listPluginCapabilities = async (kind?: string) => {
  const response = await apiClient.get<unknown>(
    "/api/v1/plugin-capabilities",
    {
      params: kind ? { kind } : undefined,
    },
  );
  return normalizeRegistrationList(response.data);
};

export const findContentProcessors = async (
  extension: string,
  operation?: string,
) => {
  const response = await apiClient.get<PluginCapabilityRegistration[]>(
    "/api/v1/plugin-capabilities/content-processors",
    {
      params: { extension, operation },
    },
  );
  return response.data;
};

export const findToolProviders = async (name?: string) => {
  const response = await apiClient.get<ToolProviderRegistration[]>(
    "/api/v1/plugin-capabilities/tools",
    {
      params: name ? { name } : undefined,
    },
  );
  return response.data;
};

export const findTaskHandlers = async (taskType?: string) => {
  const response = await apiClient.get<PluginCapabilityRegistration[]>(
    "/api/v1/plugin-capabilities/task-handlers",
    {
      params: taskType ? { task_type: taskType } : undefined,
    },
  );
  return response.data;
};

export const findEventHandlers = async (event?: string) => {
  const response = await apiClient.get<PluginCapabilityRegistration[]>(
    "/api/v1/plugin-capabilities/event-handlers",
    {
      params: event ? { event } : undefined,
    },
  );
  return response.data;
};

export const invokePluginCapability = async <T = unknown>(
  pluginId: string,
  capabilityId: string,
  params: unknown = {},
  uiCapabilityId?: string,
  uiGrant?: string,
) => {
  const response = await apiClient.post<PluginCapabilityInvokeResult<T>>(
    `/api/v1/plugins/${encodeURIComponent(pluginId)}/capabilities/${encodeURIComponent(capabilityId)}/invoke`,
    { params, ui_capability_id: uiCapabilityId, ui_grant: uiGrant },
  );
  return response.data.result;
};

export const signPluginRoute = async (request: SignPluginRouteRequest) => {
  const response = await apiClient.post<SignPluginRouteResponse>(
    "/api/v1/plugin-route-signatures",
    request,
  );
  return response.data;
};

export const invokePluginHost = async <T = unknown>(
  request: InvokePluginHostRequest,
) => {
  const response = await apiClient.post<InvokePluginHostResponse<T>>(
    "/api/v1/plugin-host/invoke",
    request,
  );
  return response.data.result;
};
