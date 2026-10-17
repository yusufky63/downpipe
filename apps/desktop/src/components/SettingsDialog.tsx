import { FolderOpen, Plus, Trash2 } from "lucide-react";
import { useEffect, useId, useState, type ReactNode } from "react";

import { MAIN_QUEUE, api, errorMessage, type BridgeRegistration, type Category, type Queue, type Settings } from "../lib/api";
import { BROWSER_NAMES, bridgeStateInfo, canRegister } from "../lib/bridge";
import { bytesToRate, rateToBytes, type RateUnit } from "../lib/format";
import { Badge, Banner, Button, Dialog, Field, IconButton, Switch, inputClass } from "./primitives";

export type SettingsTab = "downloads" | "queues" | "categories" | "network" | "media" | "schedule" | "app";

const TABS: { id: SettingsTab; label: string }[] = [
  { id: "downloads", label: "Downloads" },
  { id: "queues", label: "Queues" },
  { id: "categories", label: "Categories" },
  { id: "network", label: "Network" },
  { id: "media", label: "Media" },
  { id: "schedule", label: "Schedule" },
  { id: "app", label: "App" },
];

const DAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/** Categories as edited: extensions stay a free-form string until saved. */
interface CategoryDraft {
  key: number;
  name: string;
  extensions: string;
  folder: string;
}

/** Queues as edited: the limit stays a string until saved. */
interface QueueDraft {
  key: number;
  id: string;
  name: string;
  max: string;
}

let draftKey = 0;
const toDraft = (c: Category): CategoryDraft => ({ key: ++draftKey, name: c.name, extensions: c.extensions.join(" "), folder: c.folder });
const toQueueDraft = (q: Queue): QueueDraft => ({ key: ++draftKey, id: q.id, name: q.name, max: String(q.max_concurrent) });
const fromQueueDraft = (d: QueueDraft): Queue => ({
  id: d.id,
  name: d.name.trim(),
  max_concurrent: Math.min(16, Math.max(1, Number.parseInt(d.max, 10) || 1)),
});
const fromDraft = (d: CategoryDraft): Category => ({
  name: d.name.trim(),
  extensions: d.extensions
    .split(/[\s,;]+/)
    .map((e) => e.trim().replace(/^\./, "").toLowerCase())
    .filter(Boolean),
  folder: d.folder.trim(),
});

export function SettingsDialog({
  settings,
  initialTab = "downloads",
  onClose,
  onSaved,
}: {
  settings: Settings;
  initialTab?: SettingsTab;
  onClose: () => void;
  onSaved: (s: Settings) => void;
}) {
  const initialRate = bytesToRate(settings.speed_limit);
  const [tab, setTab] = useState<SettingsTab>(initialTab);
  const [dir, setDir] = useState(settings.download_dir);
  const [concurrent, setConcurrent] = useState(String(settings.max_concurrent_downloads));
  const [connections, setConnections] = useState(String(settings.max_connections_per_download));
  const [limit, setLimit] = useState(initialRate.value > 0 ? String(initialRate.value) : "");
  const [unit, setUnit] = useState<RateUnit>(initialRate.unit);
  const [registerHost, setRegisterHost] = useState(settings.register_browser_host);
  const [notify, setNotify] = useState(settings.notify_on_finish);
  const [closeToTray, setCloseToTray] = useState(settings.close_to_tray);
  const [startWithSystem, setStartWithSystem] = useState(settings.start_with_system);
  const [proxy, setProxy] = useState(settings.proxy);
  const [userAgent, setUserAgent] = useState(settings.user_agent);
  const [useCategories, setUseCategories] = useState(settings.use_categories);
  const [categories, setCategories] = useState<CategoryDraft[]>(() => settings.categories.map(toDraft));
  const [schedule, setSchedule] = useState(settings.schedule);
  const [queues, setQueues] = useState<QueueDraft[]>(() => settings.queues.map(toQueueDraft));
  const [watchClipboard, setWatchClipboard] = useState(settings.watch_clipboard);
  const [keepAwake, setKeepAwake] = useState(settings.keep_awake);
  const [confirmBrowser, setConfirmBrowser] = useState(settings.confirm_browser_downloads);
  const [ffmpegPath, setFfmpegPath] = useState(settings.ffmpeg_path);
  const [error, setError] = useState<string | null>(null);
  const tabIds = useId();

  const save = async () => {
    try {
      const saved = await api.updateSettings({
        download_dir: dir.trim(),
        max_concurrent_downloads: Number.parseInt(concurrent, 10) || settings.max_concurrent_downloads,
        max_connections_per_download: Number.parseInt(connections, 10) || settings.max_connections_per_download,
        speed_limit: rateToBytes(Number.parseFloat(limit), unit),
        register_browser_host: registerHost,
        notify_on_finish: notify,
        close_to_tray: closeToTray,
        start_with_system: startWithSystem,
        proxy: proxy.trim(),
        user_agent: userAgent.trim(),
        use_categories: useCategories,
        categories: categories.map(fromDraft).filter((c) => c.name),
        schedule,
        watch_clipboard: watchClipboard,
        keep_awake: keepAwake,
        confirm_browser_downloads: confirmBrowser,
        queues: queues.map(fromQueueDraft).filter((q) => q.id === MAIN_QUEUE || q.name),
        ffmpeg_path: ffmpegPath.trim(),
      });
      onSaved(saved);
      onClose();
    } catch (err) {
      setError(errorMessage(err));
    }
  };

  const updateCategory = (key: number, patch: Partial<CategoryDraft>) =>
    setCategories((list) => list.map((c) => (c.key === key ? { ...c, ...patch } : c)));
  const updateQueue = (key: number, patch: Partial<QueueDraft>) => setQueues((list) => list.map((q) => (q.key === key ? { ...q, ...patch } : q)));
  // Only saved queues can be scheduled; a new one gets its id when saved.
  const schedulable = queues.filter((q) => q.id && q.name.trim());

  return (
    <Dialog
      size="lg"
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
      <div role="tablist" aria-label="Settings sections" className="scrollbar-none -mx-5 -mt-5 mb-5 flex overflow-x-auto border-b border-line px-3">
        {TABS.map((t) => (
          <button
            key={t.id}
            type="button"
            role="tab"
            id={`${tabIds}-${t.id}`}
            aria-selected={tab === t.id}
            aria-controls={`${tabIds}-${t.id}-panel`}
            onClick={() => setTab(t.id)}
            className={`-mb-px shrink-0 border-b-2 px-3 py-2.5 text-[13px] transition-colors duration-150 ${
              tab === t.id ? "border-primary text-ink" : "border-transparent text-ink-secondary hover:text-ink"
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>

      <div role="tabpanel" id={`${tabIds}-${tab}-panel`} aria-labelledby={`${tabIds}-${tab}`} className="flex min-h-[380px] flex-col gap-5">
        {tab === "downloads" && (
          <>
            <Field label="Default folder" hint="Categories below create their folders inside this one.">
              {(id) => <FolderInput id={id} value={dir} onChange={setDir} />}
            </Field>
            <div className="grid grid-cols-1 gap-5 sm:grid-cols-2">
              <Field label="Downloads at once" hint="Others wait in the queue. 1 to 16.">
                {(id) => <NumberInput id={id} min={1} max={16} value={concurrent} onChange={setConcurrent} />}
              </Field>
              <Field label="Connections per download" hint="Upper limit. Downpipe starts with 2 and adds more only while they help. 1 to 32.">
                {(id) => <NumberInput id={id} min={1} max={32} value={connections} onChange={setConnections} />}
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
          </>
        )}

        {tab === "queues" && (
          <>
            <p className="text-[13px] text-ink-secondary">
              Each queue runs its own number of downloads at once, next to the others; inside a queue, downloads start top to bottom. Move downloads
              between queues and change their order from the right-click menu. Downloads in a removed queue move to the main queue.
            </p>
            <ul aria-label="Queues" className="flex flex-col gap-3">
              {queues.map((q) => {
                const main = q.id === MAIN_QUEUE;
                return (
                  <li key={q.key} className="grid grid-cols-[minmax(0,1fr)_96px_32px] items-center gap-2 rounded-md border border-line p-3">
                    <input
                      aria-label="Queue name"
                      value={q.name}
                      placeholder="Name"
                      onChange={(e) => updateQueue(q.key, { name: e.target.value })}
                      className={`${inputClass} h-8 text-[13px]`}
                    />
                    {main ? (
                      <span className="num text-right font-mono text-[12px] text-ink-muted" title="Set in Downloads, Downloads at once">
                        {concurrent} at once
                      </span>
                    ) : (
                      <input
                        aria-label={`${q.name || "Queue"} downloads at once`}
                        title="Downloads at once, 1 to 16"
                        type="number"
                        min={1}
                        max={16}
                        value={q.max}
                        onChange={(e) => updateQueue(q.key, { max: e.target.value })}
                        className={`${inputClass} num h-8 font-mono text-[13px]`}
                      />
                    )}
                    {main ? (
                      <span aria-hidden="true" />
                    ) : (
                      <IconButton label={`Remove ${q.name || "queue"}`} tone="danger" onClick={() => setQueues((list) => list.filter((x) => x.key !== q.key))}>
                        <Trash2 size={16} strokeWidth={1.75} />
                      </IconButton>
                    )}
                  </li>
                );
              })}
            </ul>
            <div>
              <Button
                size="sm"
                onClick={() => setQueues((list) => [...list, { key: ++draftKey, id: "", name: `Queue ${list.length + 1}`, max: "1" }])}
              >
                <Plus size={16} strokeWidth={1.75} />
                Add queue
              </Button>
            </div>
          </>
        )}

        {tab === "categories" && (
          <>
            <Switch
              label="Sort downloads into category folders"
              hint="A new download goes to its category's folder by file type. When the server's file name shows a different type, the finished file moves to the right folder. Folders you pick yourself are never changed."
              checked={useCategories}
              onChange={setUseCategories}
            />
            <ul aria-label="Categories" className={`flex flex-col gap-3 ${useCategories ? "" : "opacity-50"}`}>
              {categories.map((c) => (
                <li key={c.key} className="grid grid-cols-1 gap-2 rounded-md border border-line p-3 sm:grid-cols-[140px_minmax(0,1fr)_auto]">
                  <input
                    aria-label="Category name"
                    value={c.name}
                    placeholder="Name"
                    onChange={(e) => updateCategory(c.key, { name: e.target.value })}
                    className={`${inputClass} h-8 text-[13px]`}
                  />
                  <input
                    aria-label={`${c.name || "Category"} file types`}
                    value={c.extensions}
                    placeholder="zip rar 7z"
                    spellCheck={false}
                    onChange={(e) => updateCategory(c.key, { extensions: e.target.value })}
                    className={`${inputClass} h-8 font-mono text-[12px]`}
                  />
                  <IconButton label={`Remove ${c.name || "category"}`} tone="danger" onClick={() => setCategories((list) => list.filter((x) => x.key !== c.key))}>
                    <Trash2 size={16} strokeWidth={1.75} />
                  </IconButton>
                  <input
                    aria-label={`${c.name || "Category"} folder`}
                    value={c.folder}
                    placeholder={`Folder: ${c.name || "name"} inside the default folder`}
                    spellCheck={false}
                    onChange={(e) => updateCategory(c.key, { folder: e.target.value })}
                    className={`${inputClass} h-8 font-mono text-[12px] sm:col-span-2`}
                  />
                </li>
              ))}
            </ul>
            <div>
              <Button size="sm" onClick={() => setCategories((list) => [...list, { key: ++draftKey, name: "", extensions: "", folder: "" }])}>
                <Plus size={16} strokeWidth={1.75} />
                Add category
              </Button>
            </div>
          </>
        )}

        {tab === "network" && (
          <>
            <Field label="Proxy" hint="http://, https:// or socks5://host:port. Empty uses the system proxy. Used by downloads started after saving.">
              {(id) => (
                <input
                  id={id}
                  value={proxy}
                  placeholder="System proxy"
                  spellCheck={false}
                  onChange={(e) => setProxy(e.target.value)}
                  className={`${inputClass} h-9 font-mono text-[13px]`}
                />
              )}
            </Field>
            <Field label="User agent" hint="Some servers only answer browsers. Empty sends Downpipe's own.">
              {(id) => (
                <input
                  id={id}
                  value={userAgent}
                  placeholder="Downpipe"
                  spellCheck={false}
                  onChange={(e) => setUserAgent(e.target.value)}
                  className={`${inputClass} h-9 font-mono text-[13px]`}
                />
              )}
            </Field>
          </>
        )}

        {tab === "media" && (
          <>
            <Field
              label="FFmpeg"
              hint="Merges the separate video and sound of a stream into one file, without re-encoding. Empty uses ffmpeg from PATH. Downpipe does not include FFmpeg."
            >
              {(id) => (
                <input
                  id={id}
                  value={ffmpegPath}
                  placeholder="ffmpeg on PATH"
                  spellCheck={false}
                  onChange={(e) => setFfmpegPath(e.target.value)}
                  className={`${inputClass} h-9 font-mono text-[13px]`}
                />
              )}
            </Field>
            <FfmpegStatus />
            <Banner tone="neutral">
              Streams come from the browser extension&apos;s media detection, or from .m3u8 and .mpd links added here.
              DRM-protected and live streams are not downloaded.
            </Banner>
          </>
        )}

        {tab === "schedule" && (
          <>
            <Switch
              label="Start and stop downloads on a schedule"
              hint="At the start time, paused and failed downloads resume. At the stop time, they pause. Downpipe has to be running, in the window or the tray."
              checked={schedule.enabled}
              onChange={(enabled) => setSchedule({ ...schedule, enabled })}
            />
            {schedulable.length > 1 && (
              <div className={schedule.enabled ? "" : "opacity-50"}>
                <Field label="Queue" hint={schedule.queue ? "The other queues are not touched." : undefined}>
                  {(id) => (
                    <select id={id} value={schedule.queue} onChange={(e) => setSchedule({ ...schedule, queue: e.target.value })} className={`${inputClass} h-9`}>
                      <option value="">Every queue</option>
                      {schedulable.map((q) => (
                        <option key={q.id} value={q.id}>
                          {q.name}
                        </option>
                      ))}
                    </select>
                  )}
                </Field>
              </div>
            )}
            <div className={`grid grid-cols-2 gap-5 ${schedule.enabled ? "" : "opacity-50"}`}>
              <Field label="Start at">
                {(id) => (
                  <input
                    id={id}
                    type="time"
                    value={schedule.start}
                    onChange={(e) => setSchedule({ ...schedule, start: e.target.value })}
                    className={`${inputClass} num h-9 font-mono`}
                  />
                )}
              </Field>
              <Field label="Stop at" hint="Optional">
                {(id) => (
                  <input
                    id={id}
                    type="time"
                    value={schedule.stop}
                    onChange={(e) => setSchedule({ ...schedule, stop: e.target.value })}
                    className={`${inputClass} num h-9 font-mono`}
                  />
                )}
              </Field>
            </div>
            <fieldset className={schedule.enabled ? "" : "opacity-50"}>
              <legend className="label mb-2">Days</legend>
              <div className="flex flex-wrap gap-1.5">
                {DAYS.map((day, i) => {
                  const on = schedule.days[i] ?? false;
                  return (
                    <button
                      key={day}
                      type="button"
                      aria-pressed={on}
                      onClick={() => setSchedule({ ...schedule, days: schedule.days.map((d, j) => (j === i ? !d : d)) })}
                      className={`h-8 w-12 rounded-md border font-mono text-[12px] transition-colors duration-150 ${
                        on ? "border-primary bg-primary-soft text-ink" : "border-line-strong text-ink-secondary hover:bg-surface-muted"
                      }`}
                    >
                      {day}
                    </button>
                  );
                })}
              </div>
            </fieldset>
          </>
        )}

        {tab === "app" && (
          <>
            <Switch
              label="Notify when a download finishes"
              hint="A system notification when a download completes or fails while the window is not in front."
              checked={notify}
              onChange={setNotify}
            />
            <Switch
              label="Keep running in the tray when the window closes"
              hint="Downloads continue, and the browser extension can still send links. Quit from the tray icon."
              checked={closeToTray}
              onChange={setCloseToTray}
            />
            <Switch
              label="Start Downpipe when you sign in"
              hint="Starts in the tray, so captured downloads always have somewhere to go."
              checked={startWithSystem}
              onChange={setStartWithSystem}
            />
            <Switch
              label="Offer to download copied links"
              hint="When you copy a link to a file type listed under Categories, Downpipe opens the Add dialog with it. Links already in the list are skipped."
              checked={watchClipboard}
              onChange={setWatchClipboard}
            />
            <Switch
              label="Keep the computer awake while downloading"
              hint="Sleep waits until downloads stop running. The screen can still turn off."
              checked={keepAwake}
              onChange={setKeepAwake}
            />
            <Switch
              label="Ask before downloads from the browser start"
              hint="Choose the file name, folder and queue first. Off: they start at once with the automatic folder."
              checked={confirmBrowser}
              onChange={setConfirmBrowser}
            />
            <BrowserBridge registerHost={registerHost} onRegisterHostChange={setRegisterHost} />
          </>
        )}
      </div>
    </Dialog>
  );
}

/** Which FFmpeg the saved settings lead to. */
function FfmpegStatus() {
  const [found, setFound] = useState<string | null | undefined>(undefined);
  useEffect(() => {
    let current = true;
    api
      .ffmpegStatus()
      .then((path) => current && setFound(path))
      .catch(() => current && setFound(null));
    return () => {
      current = false;
    };
  }, []);
  if (found === undefined) return null;
  return found ? (
    <div className="flex min-w-0 items-center gap-2">
      <Badge tone="positive">Found</Badge>
      <span className="truncate font-mono text-[12px] text-ink-secondary select-text" title={found}>
        {found}
      </span>
    </div>
  ) : (
    <div className="flex flex-col gap-1.5">
      <div>
        <Badge tone="warning">Not found</Badge>
      </div>
      <p className="text-[13px] text-ink-secondary">
        Streams whose video and sound come separately are saved as two files. Install FFmpeg and add it to PATH, or enter the
        path to ffmpeg above, then save.
      </p>
    </div>
  );
}

function FolderInput({ id, value, onChange }: { id: string; value: string; onChange: (v: string) => void }) {
  return (
    <div className="flex gap-2">
      <input id={id} value={value} onChange={(e) => onChange(e.target.value)} spellCheck={false} className={`${inputClass} h-9 font-mono text-[13px]`} />
      <Button
        onClick={async () => {
          const picked = await api.pickFolder(value || null).catch(() => null);
          if (picked) onChange(picked);
        }}
      >
        <FolderOpen size={16} strokeWidth={1.75} />
        Browse
      </Button>
    </div>
  );
}

function NumberInput({
  id,
  min,
  max,
  value,
  onChange,
}: {
  id: string;
  min: number;
  max: number;
  value: string;
  onChange: (v: string) => void;
}): ReactNode {
  return (
    <input
      id={id}
      type="number"
      min={min}
      max={max}
      value={value}
      onChange={(e) => onChange(e.target.value)}
      className={`${inputClass} num h-9 font-mono`}
    />
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
