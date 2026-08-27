import { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import {
  buildClientExtensionRegistry,
  type ClientExtensionRegistrySnapshot,
} from "../pluginExtensions";
import { useAuthStore } from "../stores/authStore";
import { usePluginExtensionsStore } from "../stores/pluginExtensionsStore";

type ClientExtensionState = {
  loading: boolean;
  error?: string;
  registry: ClientExtensionRegistrySnapshot;
  refresh: () => Promise<void>;
};

const emptyRegistry: ClientExtensionRegistrySnapshot = {
  extensions: [],
  bySlot: {},
};

export const useClientExtensions = (): ClientExtensionState => {
  const { i18n } = useTranslation();
  const token = useAuthStore((state) => state.token);
  const registrations = usePluginExtensionsStore(
    (state) => state.registrations,
  );
  const loading = usePluginExtensionsStore((state) => state.loading);
  const loaded = usePluginExtensionsStore((state) => state.loaded);
  const loadedForToken = usePluginExtensionsStore(
    (state) => state.loadedForToken,
  );
  const error = usePluginExtensionsStore((state) => state.error);
  const ensureLoaded = usePluginExtensionsStore((state) => state.ensureLoaded);
  const refresh = usePluginExtensionsStore((state) => state.refresh);
  const reset = usePluginExtensionsStore((state) => state.reset);

  useEffect(() => {
    if (!token) {
      reset();
      return;
    }
    void ensureLoaded();
  }, [ensureLoaded, reset, token]);

  const locale = i18n.resolvedLanguage || i18n.language;

  const registry = useMemo(
    () => {
      if (
        !token ||
        loadedForToken !== token ||
        registrations.length === 0
      ) {
        return emptyRegistry;
      }
      return buildClientExtensionRegistry(registrations, locale);
    },
    [loadedForToken, locale, registrations, token],
  );

  return {
    loading:
      loading || (!!token && (!loaded || loadedForToken !== token)),
    error,
    registry,
    refresh,
  };
};
