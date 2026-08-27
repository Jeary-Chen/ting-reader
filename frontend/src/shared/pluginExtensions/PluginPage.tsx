import { AlertCircle, Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useParams } from "react-router";
import { useClientExtensions } from "../../core/hooks/useClientExtensions";
import PluginExtensionIcon from "./PluginExtensionIcon";
import { PluginExtensionContent } from "./PluginExtensionSlot";

const PluginPage = () => {
  const { t } = useTranslation();
  const { pluginId, capabilityId } = useParams<{
    pluginId: string;
    capabilityId: string;
  }>();
  const { registry, loading, error } = useClientExtensions();
  const extension = (registry.bySlot["app.sidebar_page"] || []).find(
    (candidate) =>
      candidate.pluginId === pluginId &&
      candidate.capability.id === capabilityId,
  );

  if (loading) {
    return (
      <div className="flex flex-1 items-center justify-center gap-2 text-sm text-slate-500 dark:text-slate-400">
        <Loader2 size={18} className="animate-spin" />
        {t("pluginExtensions.loading")}
      </div>
    );
  }

  if (!extension) {
    return (
      <div className="flex flex-1 items-center justify-center p-6">
        <div className="flex max-w-md items-start gap-3 rounded-lg border border-slate-200 bg-white p-4 text-slate-600 shadow-sm dark:border-slate-800 dark:bg-slate-900 dark:text-slate-300">
          <AlertCircle size={20} className="mt-0.5 shrink-0 text-amber-500" />
          <div>
            <h1 className="font-semibold text-slate-950 dark:text-white">
              {t("pluginExtensions.pageUnavailable")}
            </h1>
            <p className="mt-1 text-sm">
              {error || t("pluginExtensions.pageUnavailableDescription")}
            </p>
          </div>
        </div>
      </div>
    );
  }

  const title =
    extension.title || extension.pluginName || extension.capability.id;

  return (
    <div className="flex min-h-full flex-1 flex-col">
      <header className="flex h-16 shrink-0 items-center gap-3 border-b border-slate-200 bg-white px-4 sm:px-6 dark:border-slate-800 dark:bg-slate-900">
        <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md bg-primary-50 text-primary-700 dark:bg-primary-950/40 dark:text-primary-300">
          <PluginExtensionIcon extension={extension} size={19} />
        </div>
        <div className="min-w-0">
          <h1 className="truncate text-base font-semibold text-slate-950 dark:text-white">
            {title}
          </h1>
          <p className="truncate text-xs text-slate-500 dark:text-slate-400">
            {extension.pluginName}
          </p>
        </div>
      </header>
      <div className="flex min-h-[32rem] flex-1 flex-col bg-white dark:bg-slate-950">
        <PluginExtensionContent
          extension={extension}
          context={{ page: "sidebar" }}
        />
      </div>
    </div>
  );
};

export default PluginPage;
