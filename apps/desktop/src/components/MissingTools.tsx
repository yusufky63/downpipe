import { useEffect, useState } from "react";

import { useI18n } from "../i18n";
import { api, errorMessage, onToolsProgress, type SiteTool, type ToolsProgress, type ToolsStatus } from "../lib/api";
import { downloadSize, progressText, toolList } from "../lib/tools";
import { Banner, Button } from "./primitives";

/**
 * A download that needs a tool Downpipe does not have: one button fetches
 * the missing ones (checked against their published SHA-256, as in
 * Settings, Media), with progress, and the dialog carries on. Nothing is
 * downloaded before the click. One line of text and one button, so the
 * confirmation window stays as compact as it was.
 */
export function MissingTools({
  status,
  tools,
  reason,
  onInstalled,
}: {
  status: ToolsStatus;
  /** Missing and downloadable, from `missingTools`. */
  tools: SiteTool[];
  /** What needs them: the video page, or the chosen quality. */
  reason: "video" | "quality";
  onInstalled: (status: ToolsStatus) => void;
}) {
  const { t } = useI18n();
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<ToolsProgress | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!busy) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void onToolsProgress((p) => setProgress(p)).then((fn) => (disposed ? fn() : (unlisten = fn)));
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [busy]);

  const install = async () => {
    setBusy(true);
    setError(null);
    setProgress(null);
    try {
      // Only the ones still missing when its turn comes: another window may
      // be fetching the same tools.
      onInstalled(await api.installSiteTools(tools, true));
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  };

  const fraction = progress?.total ? Math.min(1, progress.done / progress.total) : null;
  return (
    <Banner tone="warning">
      <div className="flex flex-col gap-2">
        <span>{t(reason === "video" ? "media.tools.missingForVideo" : "media.tools.missingForQuality", { tools: toolList(tools) })}</span>
        {busy ? (
          <div className="flex flex-col gap-1" role="status" aria-live="polite">
            <span className="num text-[12px]">{progress ? progressText(progress) : t("media.tools.starting")}</span>
            <div className="h-1 overflow-hidden rounded-full bg-canvas">
              <div
                className="h-full bg-primary transition-[width] duration-200"
                style={{ width: `${Math.round((fraction ?? 0) * 100)}%` }}
              />
            </div>
          </div>
        ) : (
          <div>
            <Button size="sm" variant="primary" onClick={() => void install()} title={t("media.tools.sourceNote")}>
              {t("media.tools.offer", { size: downloadSize(status, tools) })}
            </Button>
          </div>
        )}
        {error && <span className="text-[12px] text-danger-fg select-text">{error}</span>}
      </div>
    </Banner>
  );
}
