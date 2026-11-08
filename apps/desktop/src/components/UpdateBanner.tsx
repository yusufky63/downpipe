import { useEffect, useState } from "react";

import { useI18n } from "../i18n";
import { api, errorMessage, inTauri, onUpdate, type AppUpdate } from "../lib/api";
import { Banner, Button } from "./primitives";

/**
 * A newer Downpipe, above the list: what is new, and on Windows "Install
 * now", which downloads and checks the installer and quits for it. "Later"
 * hides it until the app starts again.
 */
export function UpdateBanner() {
  const { t } = useI18n();
  const [update, setUpdate] = useState<AppUpdate | null>(null);
  const [hidden, setHidden] = useState<string | null>(null);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!inTauri()) return;
    let live = true;
    api
      .updateStatus()
      .then((found) => live && setUpdate(found))
      .catch(() => {});
    const unlisten = onUpdate((found) => live && setUpdate(found));
    return () => {
      live = false;
      void unlisten.then((stop) => stop());
    };
  }, []);

  if (!update || hidden === update.version) return null;

  const install = () => {
    setInstalling(true);
    setError(null);
    api.installUpdate().catch((err: unknown) => {
      setInstalling(false);
      setError(errorMessage(err));
    });
  };

  return (
    <div className="px-4 pt-3">
      <Banner tone="neutral">
        <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
          <span className="min-w-0 flex-1 text-ink">
            {installing ? t("app.update.installing") : t("app.update.available", { version: update.version })}
          </span>
          <Button size="sm" variant="ghost" onClick={() => void api.openUpdatePage()}>
            {t("app.update.whatsNew")}
          </Button>
          {update.installer ? (
            <Button size="sm" variant="primary" onClick={install} disabled={installing}>
              {t("app.update.install")}
            </Button>
          ) : (
            <Button size="sm" variant="primary" onClick={() => void api.openUpdatePage()}>
              {t("app.update.download")}
            </Button>
          )}
          <Button size="sm" variant="ghost" onClick={() => setHidden(update.version)} disabled={installing}>
            {t("app.update.later")}
          </Button>
        </div>
        {error && <p className="mt-2 text-warning-fg">{t("app.update.failed", { error })}</p>}
      </Banner>
    </div>
  );
}
