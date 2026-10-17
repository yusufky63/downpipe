import { FolderOpen } from "lucide-react";
import { useEffect, useId, useState } from "react";

import { api, errorMessage, type BridgeRegistration, type Settings } from "../lib/api";
import { BROWSER_NAMES, bridgeStateInfo, canRegister } from "../lib/bridge";
import { bytesToRate, rateToBytes, type RateUnit } from "../lib/format";
import { Badge, Banner, Button, Dialog, Field, Switch, inputClass } from "./primitives";

export function SettingsDialog({
  settings,
  onClose,
  onSaved,
}: {
  settings: Settings;
  onClose: () => void;
  onSaved: (s: Settings) => void;
}) {
  const initialRate = bytesToRate(settings.speed_limit);
  const [dir, setDir] = useState(settings.download_dir);
  const [concurrent, setConcurrent] = useState(String(settings.max_concurrent_downloads));
  const [connections, setConnections] = useState(String(settings.max_connections_per_download));
  const [limit, setLimit] = useState(initialRate.value > 0 ? String(initialRate.value) : "");
  const [unit, setUnit] = useState<RateUnit>(initialRate.unit);
  const [registerHost, setRegisterHost] = useState(settings.register_browser_host);
  const [error, setError] = useState<string | null>(null);

  const save = async () => {
    try {
      const saved = await api.updateSettings({
        download_dir: dir.trim(),
        max_concurrent_downloads: Number.parseInt(concurrent, 10) || settings.max_concurrent_downloads,
        max_connections_per_download: Number.parseInt(connections, 10) || settings.max_connections_per_download,
        speed_limit: rateToBytes(Number.parseFloat(limit), unit),
        register_browser_host: registerHost,
      });
      onSaved(saved);
      onClose();
    } catch (err) {
      setError(errorMessage(err));
    }
  };

  return (
    <Dialog
      eyebrow="Settings"
      title="Preferences"
      onClose={onClose}
      footer={
        <>
          {error && <p className="mr-auto text-[13px] text-danger-fg">{error}</p>}
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" onClick={() => void save()}>
            Save
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-5">
        <Field label="Default folder">
          {(id) => (
            <div className="flex gap-2">
              <input id={id} value={dir} onChange={(e) => setDir(e.target.value)} spellCheck={false} className={`${inputClass} h-9 font-mono text-[13px]`} />
              <Button
                onClick={async () => {
                  const picked = await api.pickFolder(dir || null).catch(() => null);
                  if (picked) setDir(picked);
                }}
              >
                <FolderOpen size={16} strokeWidth={1.75} />
                Browse
              </Button>
            </div>
          )}
        </Field>

        <div className="grid grid-cols-1 gap-5 sm:grid-cols-2">
          <Field label="Downloads at once" hint="Others wait in the queue. 1 to 16.">
            {(id) => (
              <input
                id={id}
                type="number"
                min={1}
                max={16}
                value={concurrent}
                onChange={(e) => setConcurrent(e.target.value)}
                className={`${inputClass} num h-9 font-mono`}
              />
            )}
          </Field>
          <Field label="Connections per download" hint="Upper limit. Downpipe starts with 2 and adds more only while they help. 1 to 32.">
            {(id) => (
              <input
                id={id}
                type="number"
                min={1}
                max={32}
                value={connections}
                onChange={(e) => setConnections(e.target.value)}
                className={`${inputClass} num h-9 font-mono`}
              />
            )}
          </Field>
        </div>

        <Field label="Speed limit" hint="Shared by all downloads. Leave empty for no limit. Applies immediately.">
          {(id) => (
            <div className="flex gap-2">
              <input
                id={id}
                type="number"
                min={0}
                step="any"
                value={limit}
                placeholder="No limit"
                onChange={(e) => setLimit(e.target.value)}
                className={`${inputClass} num h-9 font-mono`}
              />
              <select
                aria-label="Speed limit unit"
                value={unit}
                onChange={(e) => setUnit(e.target.value as RateUnit)}
                className={`${inputClass} h-9 w-28 font-mono`}
              >
                <option value="KiB">KiB/s</option>
                <option value="MiB">MiB/s</option>
              </select>
            </div>
          )}
        </Field>

        <BrowserBridge registerHost={registerHost} onRegisterHostChange={setRegisterHost} />
      </div>
    </Dialog>
  );
}

function BrowserBridge({
  registerHost,
  onRegisterHostChange,
}: {
  registerHost: boolean;
  onRegisterHostChange: (on: boolean) => void;
}) {
  const headingId = useId();
  const [status, setStatus] = useState<BridgeRegistration | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let live = true;
    api
      .bridgeStatus()
      .then((s) => live && setStatus(s))
      .catch((err: unknown) => live && setError(errorMessage(err)));
    return () => {
      live = false;
    };
  }, []);

  const register = async () => {
    setBusy(true);
    try {
      setStatus(await api.registerBridge());
      setError(null);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section aria-labelledby={headingId} className="flex flex-col gap-4 border-t border-line pt-5">
      <h3 id={headingId} className="label">
        Browser bridge
      </h3>
      <Switch
        label="Register with browsers at start-up"
        hint="Lets the Downpipe extension in Chrome, Edge, Chromium and Firefox reach this copy of the app. Changes your user account only. Turn it off to manage the registration with downpipe-native-host yourself."
        checked={registerHost}
        onChange={onRegisterHostChange}
      />

      {error && <Banner tone="danger">{error}</Banner>}
      {!status && !error && <p className="text-[13px] text-ink-muted">Checking the registration…</p>}
      {status && !status.host_found && (
        <Banner tone="warning">
          This copy of Downpipe has no browser bridge at <span className="font-mono break-all">{status.host}</span>. Reinstall Downpipe; in a
          development build, run <span className="font-mono">pnpm sidecar</span>.
        </Banner>
      )}
      {status && (
        <ul aria-label="Registration per browser" className="divide-y divide-line rounded-md border border-line">
          {status.browsers.map((b) => {
            const info = bridgeStateInfo(b.state);
            return (
              <li key={b.browser} className="flex flex-col gap-1 px-3 py-2">
                <div className="flex items-center justify-between gap-3">
                  <span className="text-[13px] text-ink">{BROWSER_NAMES[b.browser]}</span>
                  <Badge tone={info.tone}>{info.label}</Badge>
                </div>
                {b.detail && <p className="font-mono text-[12px] break-all text-ink-muted">{b.detail}</p>}
              </li>
            );
          })}
        </ul>
      )}
      {status && canRegister(status) && (
        <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
          <Button size="sm" onClick={() => void register()} disabled={busy}>
            {busy ? "Registering…" : "Register now"}
          </Button>
          <span className="text-[12px] text-ink-muted">Open browsers pick it up the next time the extension checks.</span>
        </div>
      )}
    </section>
  );
}
