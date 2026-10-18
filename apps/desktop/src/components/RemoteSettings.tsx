import { Copy, KeyRound, QrCode as QrIcon, RefreshCw } from "lucide-react";
import { useEffect, useId, useState } from "react";

import { useI18n } from "../i18n";
import { api, errorMessage, type RemoteStatus, type RemoteUpdate } from "../lib/api";
import { groupToken, linkWithToken } from "../lib/remote";
import { Badge, Banner, Button, Field, Switch, inputClass } from "./primitives";
import { QrCode } from "./QrCode";

const LOCAL = "127.0.0.1";
const EVERY_INTERFACE = "0.0.0.0";

/**
 * Settings, Remote. Unlike the other tabs it acts at once: the server starts
 * and stops with the switch, and a new token exists only in this panel's
 * state, so it is gone once the dialog closes.
 */
export function RemoteSettingsPanel() {
  const { t } = useI18n();
  const portId = useId();
  const [status, setStatus] = useState<RemoteStatus | null>(null);
  const [token, setToken] = useState<string | null>(null);
  const [port, setPort] = useState("");
  const [showQr, setShowQr] = useState(false);
  const [copied, setCopied] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    api
      .remoteStatus()
      .then((s) => {
        if (!live) return;
        setStatus(s);
        setPort(String(s.port));
      })
      .catch((err: unknown) => live && setError(errorMessage(err)));
    return () => {
      live = false;
    };
  }, []);

  if (!status) return error ? <Banner tone="danger">{error}</Banner> : null;

  const run = async (change: () => Promise<RemoteUpdate>) => {
    setBusy(true);
    setError(null);
    try {
      const update = await change();
      setStatus(update.status);
      setPort(String(update.status.port));
      if (update.token) {
        setToken(update.token);
        setShowQr(true);
      }
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  };

  const portValue = Number.parseInt(port, 10);
  const portValid = Number.isInteger(portValue) && portValue >= 1024 && portValue <= 65535;
  const apply = (enabled: boolean, bind: string, nextPort = status.port) => void run(() => api.setRemote(enabled, bind, nextPort));
  const copy = (key: string, text: string) => {
    void navigator.clipboard.writeText(text).then(() => setCopied(key));
  };

  const network = status.bind !== LOCAL && status.bind !== "::1";
  const binds = [LOCAL, EVERY_INTERFACE];
  if (status.lan_address && !binds.includes(status.lan_address)) binds.push(status.lan_address);
  if (!binds.includes(status.bind)) binds.push(status.bind);
  const bindLabel = (bind: string) =>
    bind === LOCAL ? t("remote.settings.bindLocal") : bind === EVERY_INTERFACE ? t("remote.settings.bindNetwork") : t("remote.settings.bindLan", { address: bind });
  const qrText = status.url && token ? linkWithToken(status.url, token) : status.url;

  return (
    <>
      <p className="text-[13px] text-ink-secondary">{t("remote.settings.intro")}</p>
      <Switch
        label={t("remote.settings.enable")}
        hint={t("remote.settings.enableHint")}
        checked={status.enabled}
        onChange={(enabled) => apply(enabled, status.bind)}
      />

      <div className="grid grid-cols-1 gap-5 sm:grid-cols-[minmax(0,1fr)_200px]">
        <Field label={t("remote.settings.listenOn")} hint={t("remote.settings.bindHint")}>
          {(id) => (
            <select id={id} value={status.bind} disabled={busy} onChange={(e) => apply(status.enabled, e.target.value)} className={`${inputClass} h-9`}>
              {binds.map((bind) => (
                <option key={bind} value={bind}>
                  {bindLabel(bind)}
                </option>
              ))}
            </select>
          )}
        </Field>
        <div className="flex flex-col gap-1.5">
          <label htmlFor={portId} className="label">
            {t("remote.settings.port")}
          </label>
          <div className="flex gap-2">
            <input
              id={portId}
              type="number"
              min={1024}
              max={65535}
              value={port}
              onChange={(e) => setPort(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && portValid) apply(status.enabled, status.bind, portValue);
              }}
              className={`${inputClass} num h-9 font-mono`}
            />
            <Button disabled={busy || !portValid || portValue === status.port} onClick={() => apply(status.enabled, status.bind, portValue)}>
              {t("remote.settings.apply")}
            </Button>
          </div>
          <p className="text-[12px] text-ink-muted">{t("remote.settings.portHint")}</p>
        </div>
      </div>

      {network && <Banner tone="warning">{t("remote.settings.networkWarning")}</Banner>}
      {error && <Banner tone="danger">{error}</Banner>}
      {status.error === "port_in_use" && <Banner tone="danger">{t("remote.settings.portInUse", { port: status.port })}</Banner>}
      {status.error === "bind_failed" && <Banner tone="danger">{t("remote.settings.bindFailed", { address: status.bind })}</Banner>}

      <div className="flex min-w-0 flex-wrap items-center gap-2">
        <Badge tone={status.running ? "positive" : "neutral"}>{status.running ? t("remote.settings.running") : t("remote.settings.off")}</Badge>
        {status.running && status.url && (
          <>
            <span className="truncate font-mono text-[13px] text-ink select-text">{status.url}</span>
            <Button size="sm" variant="ghost" onClick={() => copy("address", status.url ?? "")}>
              <Copy size={14} strokeWidth={1.75} />
              {copied === "address" ? t("remote.settings.copied") : t("remote.settings.copyAddress")}
            </Button>
          </>
        )}
      </div>

      {/* Switching it on the first time makes the token; until then there is nothing to show. */}
      {(token || status.has_token) && (
        <section aria-labelledby={`${portId}-token`} className="flex flex-col gap-3 rounded-md border border-line p-4">
          <h3 id={`${portId}-token`} className="flex items-center gap-2 text-[14px] font-medium text-ink">
            <KeyRound size={16} strokeWidth={1.75} className="text-ink-muted" />
            {t("remote.settings.token")}
          </h3>
          {token ? (
            <>
              <Banner tone="neutral">{t("remote.settings.tokenShown")}</Banner>
              <div className="flex min-w-0 items-center gap-2">
                <code className="min-w-0 flex-1 truncate rounded-sm bg-surface-muted px-2 py-1.5 font-mono text-[13px] text-ink select-all">
                  {groupToken(token)}
                </code>
                <Button size="sm" onClick={() => copy("token", token)}>
                  <Copy size={14} strokeWidth={1.75} />
                  {copied === "token" ? t("remote.settings.copied") : t("remote.settings.copy")}
                </Button>
              </div>
              {status.url && (
                <div className="flex min-w-0 flex-col gap-1.5">
                  <span className="label">{t("remote.settings.linkWithToken")}</span>
                  <div className="flex min-w-0 items-center gap-2">
                    <code className="min-w-0 flex-1 truncate font-mono text-[12px] text-ink-secondary select-all">{linkWithToken(status.url, token)}</code>
                    <Button size="sm" variant="ghost" onClick={() => copy("link", linkWithToken(status.url ?? "", token))}>
                      <Copy size={14} strokeWidth={1.75} />
                      {copied === "link" ? t("remote.settings.copied") : t("remote.settings.copy")}
                    </Button>
                  </div>
                </div>
              )}
            </>
          ) : (
            status.has_token && <p className="text-[13px] text-ink-secondary">{t("remote.settings.tokenSet")}</p>
          )}
          <div className="flex flex-wrap items-center gap-2">
            <Button size="sm" disabled={busy} onClick={() => void run(() => api.newRemoteToken())}>
              <RefreshCw size={14} strokeWidth={1.75} />
              {t("remote.settings.newToken")}
            </Button>
            {token && (
              <Button
                size="sm"
                variant="ghost"
                onClick={() => {
                  setToken(null);
                  setShowQr(false);
                }}
              >
                {t("remote.settings.done")}
              </Button>
            )}
            {status.running && qrText && (
              <Button size="sm" variant="ghost" onClick={() => setShowQr(!showQr)}>
                <QrIcon size={14} strokeWidth={1.75} />
                {showQr ? t("remote.settings.hideQr") : t("remote.settings.showQr")}
              </Button>
            )}
          </div>
          <p className="text-[12px] text-ink-muted">{t("remote.settings.newTokenHint")}</p>
          {showQr && status.running && qrText && status.url && (
            <div className="flex flex-col items-start gap-2 sm:flex-row sm:items-center sm:gap-4">
              <QrCode text={qrText} label={t("remote.settings.qrLabel", { url: status.url })} />
              <p className="text-[12px] text-ink-muted">{token ? t("remote.settings.qrWithToken") : t("remote.settings.qrHint")}</p>
            </div>
          )}
        </section>
      )}
    </>
  );
}
