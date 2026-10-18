import { Plus, Trash2 } from "lucide-react";
import { Fragment, useEffect, useId, useState, type ReactNode } from "react";

import { LOCALES, systemLocaleName, useI18n } from "../i18n";
import {
  MAIN_QUEUE,
  api,
  errorMessage,
  type BridgeRegistration,
  type Category,
  type DuplicateAnswer,
  type Queue,
  type Settings,
  type SiteRulesCheck,
  type ToolsStatus,
} from "../lib/api";
import { BROWSER_NAMES, bridgeStateInfo, canRegister } from "../lib/bridge";
import { fromRuleDraft, toRuleDraft, type RuleDraft } from "../lib/siteRules";
import { bytesToRate, decimalField, parseDecimal, rateToBytes, type RateUnit } from "../lib/format";
import { categoryLabel, storedCategoryName } from "../lib/categories";
import { installButtonLabel, toolRows, toolsToInstall } from "../lib/tools";
import { fromTorrentDraft, torrentDraft } from "../lib/torrent";
import { FolderInput } from "./FolderInput";
import { Badge, Banner, Button, Dialog, Field, IconButton, Switch, inputClass } from "./primitives";
import { RemoteSettingsPanel } from "./RemoteSettings";
import { SiteRules } from "./SiteRules";
import { TorrentSettingsPanel } from "./TorrentSettings";

export type SettingsTab = "downloads" | "queues" | "categories" | "network" | "sites" | "media" | "torrent" | "schedule" | "app" | "remote";

const TABS: readonly SettingsTab[] = ["downloads", "queues", "categories", "network", "sites", "media", "torrent", "schedule", "app", "remote"];

/** Short day names, Monday first like `schedule.days`; 1 January 2024 was a Monday. */
function weekdays(locale: string): string[] {
  const format = new Intl.DateTimeFormat(locale, { weekday: "short" });
  return Array.from({ length: 7 }, (_, i) => format.format(new Date(2024, 0, 1 + i)));
}

/**
 * A translated sentence with elements, such as a path in monospace, at its
 * {placeholders}, so translators can move them where their grammar needs.
 */
function fill(text: string, parts: Record<string, ReactNode>): ReactNode {
  return text.split(/\{(\w+)\}/).map((piece, i) => (i % 2 === 1 ? <Fragment key={i}>{parts[piece] ?? `{${piece}}`}</Fragment> : piece));
}

/** Categories as edited: extensions stay a free-form string until saved. */
interface CategoryDraft {
  key: number;
  /** The name in the settings, which a built-in category shows translated. */
  stored: string;
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
const toDraft = (c: Category): CategoryDraft => ({
  key: ++draftKey,
  stored: c.name,
  name: categoryLabel(c.name),
  extensions: c.extensions.join(" "),
  folder: c.folder,
});
const toQueueDraft = (q: Queue): QueueDraft => ({ key: ++draftKey, id: q.id, name: q.name, max: String(q.max_concurrent) });
const fromQueueDraft = (d: QueueDraft): Queue => ({
  id: d.id,
  name: d.name.trim(),
  max_concurrent: Math.min(16, Math.max(1, Number.parseInt(d.max, 10) || 1)),
});
const fromDraft = (d: CategoryDraft): Category => ({
  name: storedCategoryName(d.stored, d.name),
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
  const { t, locale } = useI18n();
  const initialRate = bytesToRate(settings.speed_limit);
  const [tab, setTab] = useState<SettingsTab>(initialTab);
  const [dir, setDir] = useState(settings.download_dir);
  const [concurrent, setConcurrent] = useState(String(settings.max_concurrent_downloads));
  const [connections, setConnections] = useState(String(settings.max_connections_per_download));
  const [limit, setLimit] = useState(initialRate.value > 0 ? decimalField(initialRate.value) : "");
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
  const [ytDlpPath, setYtDlpPath] = useState(settings.yt_dlp_path);
  const [denoPath, setDenoPath] = useState(settings.deno_path);
  const [language, setLanguage] = useState(settings.language);
  const [ruleDrafts, setRuleDrafts] = useState<RuleDraft[]>(() => settings.site_rules.map(toRuleDraft));
  const [ruleAddress, setRuleAddress] = useState("");
  const [ruleCheck, setRuleCheck] = useState<SiteRulesCheck | null>(null);
  const [duplicateAnswer, setDuplicateAnswer] = useState<DuplicateAnswer>(settings.duplicate_answer);
  const [torrent, setTorrent] = useState(() => torrentDraft(settings.torrent));
  const [error, setError] = useState<string | null>(null);
  const tabIds = useId();

  const limitValue = limit.trim() === "" ? 0 : parseDecimal(limit);
  const limitInvalid = Number.isNaN(limitValue);

  // Drafts are checked by the same Rust code that applies saved rules, a
  // moment after each change, so the tester never disagrees with downloads.
  useEffect(() => {
    let live = true;
    const timer = window.setTimeout(() => {
      api
        .checkSiteRules(ruleDrafts.map(fromRuleDraft), ruleAddress.trim() || null)
        .then((check) => live && setRuleCheck(check))
        .catch(() => live && setRuleCheck(null));
    }, 120);
    return () => {
      live = false;
      window.clearTimeout(timer);
    };
  }, [ruleDrafts, ruleAddress]);

  const save = async () => {
    if (ruleCheck && ruleCheck.problems.length > 0) {
      setTab("sites");
      setError(t("sites.fix"));
      return;
    }
    if (limitInvalid) {
      setTab("downloads");
      return;
    }
    try {
      const saved = await api.updateSettings({
        language,
        download_dir: dir.trim(),
        max_concurrent_downloads: Number.parseInt(concurrent, 10) || settings.max_concurrent_downloads,
        max_connections_per_download: Number.parseInt(connections, 10) || settings.max_connections_per_download,
        speed_limit: rateToBytes(limitValue, unit),
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
        yt_dlp_path: ytDlpPath.trim(),
        deno_path: denoPath.trim(),
        site_rules: ruleDrafts.map(fromRuleDraft),
        duplicate_answer: duplicateAnswer,
        torrent: fromTorrentDraft(torrent, settings.torrent),
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
      eyebrow={t("settings.eyebrow")}
      title={t("settings.title")}
      onClose={onClose}
      footer={
        <>
          {error && <p className="mr-auto text-[13px] text-danger-fg">{error}</p>}
          <Button variant="ghost" onClick={onClose}>
            {t("common.cancel")}
          </Button>
          <Button variant="primary" onClick={() => void save()}>
            {t("common.save")}
          </Button>
        </>
      }
    >
      <div role="tablist" aria-label={t("settings.tabsLabel")} className="scrollbar-none -mx-5 -mt-5 mb-5 flex overflow-x-auto border-b border-line px-3">
        {TABS.map((id) => (
          <button
            key={id}
            type="button"
            role="tab"
            id={`${tabIds}-${id}`}
            aria-selected={tab === id}
            aria-controls={`${tabIds}-${id}-panel`}
            onClick={() => setTab(id)}
            className={`-mb-px shrink-0 border-b-2 px-3 py-2.5 text-[13px] transition-colors duration-150 ${
              tab === id ? "border-primary text-ink" : "border-transparent text-ink-secondary hover:text-ink"
            }`}
          >
            {id === "remote" ? t("remote.settings.tab") : id === "torrent" ? t("torrent.settings.tab") : t(`settings.tabs.${id}`)}
          </button>
        ))}
      </div>

      <div role="tabpanel" id={`${tabIds}-${tab}-panel`} aria-labelledby={`${tabIds}-${tab}`} className="flex min-h-[380px] flex-col gap-5">
        {tab === "downloads" && (
          <>
            <Field label={t("settings.downloads.folder")} hint={t("settings.downloads.folderHint")}>
              {(id) => <FolderInput id={id} value={dir} onChange={setDir} />}
            </Field>
            <div className="grid grid-cols-1 gap-5 sm:grid-cols-2">
              <Field label={t("settings.downloads.concurrent")} hint={t("settings.downloads.concurrentHint")}>
                {(id) => <NumberInput id={id} min={1} max={16} value={concurrent} onChange={setConcurrent} />}
              </Field>
              <Field label={t("settings.downloads.connections")} hint={t("settings.downloads.connectionsHint")}>
                {(id) => <NumberInput id={id} min={1} max={32} value={connections} onChange={setConnections} />}
              </Field>
            </div>
            <Field
              label={t("settings.downloads.speedLimit")}
              hint={t("settings.downloads.speedLimitHint")}
              error={limitInvalid ? t("settings.downloads.speedLimitInvalid") : null}
            >
              {(id) => (
                <div className="flex gap-2">
                  <input
                    id={id}
                    // Text, not a number field: Chromium's number field drops a
                    // comma unless the system's language uses one.
                    type="text"
                    inputMode="decimal"
                    value={limit}
                    placeholder={t("settings.downloads.noLimit")}
                    onChange={(e) => setLimit(e.target.value)}
                    className={`${inputClass} num h-9 font-mono`}
                  />
                  <select
                    aria-label={t("settings.downloads.unit")}
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
            <Field label={t("settings.downloads.duplicate")} hint={t("settings.downloads.duplicateHint")}>
              {(id) => (
                <select
                  id={id}
                  value={duplicateAnswer}
                  onChange={(e) => setDuplicateAnswer(e.target.value as DuplicateAnswer)}
                  className={`${inputClass} h-9`}
                >
                  <option value="ask">{t("settings.downloads.duplicateAsk")}</option>
                  <option value="copy">{t("dialogs.duplicate.copy")}</option>
                  <option value="overwrite">{t("dialogs.duplicate.overwrite")}</option>
                  <option value="existing">{t("dialogs.duplicate.existing")}</option>
                </select>
              )}
            </Field>
          </>
        )}

        {tab === "queues" && (
          <>
            <p className="text-[13px] text-ink-secondary">{t("settings.queues.intro")}</p>
            <ul aria-label={t("settings.queues.label")} className="flex flex-col gap-3">
              {queues.map((q) => {
                const main = q.id === MAIN_QUEUE;
                return (
                  <li key={q.key} className="grid grid-cols-[minmax(0,1fr)_96px_32px] items-center gap-2 rounded-md border border-line p-3">
                    <input
                      aria-label={t("settings.queues.name")}
                      value={q.name}
                      placeholder={t("settings.queues.namePlaceholder")}
                      onChange={(e) => updateQueue(q.key, { name: e.target.value })}
                      className={`${inputClass} h-8 text-[13px]`}
                    />
                    {main ? (
                      <span className="num text-right font-mono text-[12px] text-ink-muted" title={t("settings.queues.mainLimitTitle")}>
                        {t("settings.queues.mainLimit", { count: concurrent })}
                      </span>
                    ) : (
                      <input
                        aria-label={q.name ? t("settings.queues.limit", { name: q.name }) : t("settings.queues.limitUnnamed")}
                        title={t("settings.queues.limitTitle")}
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
                      <IconButton
                        label={q.name ? t("settings.queues.remove", { name: q.name }) : t("settings.queues.removeUnnamed")}
                        tone="danger"
                        onClick={() => setQueues((list) => list.filter((x) => x.key !== q.key))}
                      >
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
                onClick={() =>
                  setQueues((list) => [...list, { key: ++draftKey, id: "", name: t("settings.queues.newName", { number: list.length + 1 }), max: "1" }])
                }
              >
                <Plus size={16} strokeWidth={1.75} />
                {t("settings.queues.add")}
              </Button>
            </div>
          </>
        )}

        {tab === "categories" && (
          <>
            <Switch label={t("settings.categories.use")} hint={t("settings.categories.useHint")} checked={useCategories} onChange={setUseCategories} />
            <ul aria-label={t("settings.categories.label")} className={`flex flex-col gap-3 ${useCategories ? "" : "opacity-50"}`}>
              {categories.map((c) => (
                <li key={c.key} className="grid grid-cols-1 gap-2 rounded-md border border-line p-3 sm:grid-cols-[140px_minmax(0,1fr)_auto]">
                  <input
                    aria-label={t("settings.categories.name")}
                    value={c.name}
                    placeholder={t("settings.categories.namePlaceholder")}
                    onChange={(e) => updateCategory(c.key, { name: e.target.value })}
                    className={`${inputClass} h-8 text-[13px]`}
                  />
                  <input
                    aria-label={c.name ? t("settings.categories.types", { name: c.name }) : t("settings.categories.typesUnnamed")}
                    value={c.extensions}
                    placeholder="zip rar 7z"
                    spellCheck={false}
                    onChange={(e) => updateCategory(c.key, { extensions: e.target.value })}
                    className={`${inputClass} h-8 font-mono text-[12px]`}
                  />
                  <IconButton
                    label={c.name ? t("settings.categories.remove", { name: c.name }) : t("settings.categories.removeUnnamed")}
                    tone="danger"
                    onClick={() => setCategories((list) => list.filter((x) => x.key !== c.key))}
                  >
                    <Trash2 size={16} strokeWidth={1.75} />
                  </IconButton>
                  <input
                    aria-label={c.name ? t("settings.categories.folder", { name: c.name }) : t("settings.categories.folderUnnamed")}
                    value={c.folder}
                    // The folder is named after the stored name, which stays English for a built-in category.
                    placeholder={
                      c.name
                        ? t("settings.categories.folderPlaceholder", { name: storedCategoryName(c.stored, c.name) })
                        : t("settings.categories.folderPlaceholderUnnamed")
                    }
                    spellCheck={false}
                    onChange={(e) => updateCategory(c.key, { folder: e.target.value })}
                    className={`${inputClass} h-8 font-mono text-[12px] sm:col-span-2`}
                  />
                </li>
              ))}
            </ul>
            <div>
              <Button size="sm" onClick={() => setCategories((list) => [...list, { key: ++draftKey, stored: "", name: "", extensions: "", folder: "" }])}>
                <Plus size={16} strokeWidth={1.75} />
                {t("settings.categories.add")}
              </Button>
            </div>
          </>
        )}

        {tab === "network" && (
          <>
            <Field label={t("settings.network.proxy")} hint={t("settings.network.proxyHint")}>
              {(id) => (
                <input
                  id={id}
                  value={proxy}
                  placeholder={t("settings.network.proxyPlaceholder")}
                  spellCheck={false}
                  onChange={(e) => setProxy(e.target.value)}
                  className={`${inputClass} h-9 font-mono text-[13px]`}
                />
              )}
            </Field>
            <Field label={t("settings.network.userAgent")} hint={t("settings.network.userAgentHint")}>
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

        {tab === "sites" && (
          <SiteRules
            drafts={ruleDrafts}
            onChange={setRuleDrafts}
            address={ruleAddress}
            onAddress={setRuleAddress}
            check={ruleCheck}
            queues={schedulable.map((q) => ({ id: q.id, name: q.name.trim() }))}
          />
        )}

        {tab === "media" && (
          <>
            <Field label="FFmpeg" hint={t("settings.media.ffmpegHint")}>
              {(id) => (
                <input
                  id={id}
                  value={ffmpegPath}
                  placeholder={t("settings.media.ffmpegPlaceholder")}
                  spellCheck={false}
                  onChange={(e) => setFfmpegPath(e.target.value)}
                  className={`${inputClass} h-9 font-mono text-[13px]`}
                />
              )}
            </Field>
            <FfmpegStatus />
            <Banner tone="neutral">{t("settings.media.streams")}</Banner>
            <SiteTools
              ytDlpPath={ytDlpPath}
              denoPath={denoPath}
              onYtDlpPath={setYtDlpPath}
              onDenoPath={setDenoPath}
              denoConfigured={settings.deno_path !== ""}
            />
          </>
        )}

        {tab === "torrent" && <TorrentSettingsPanel draft={torrent} onChange={setTorrent} />}

        {tab === "schedule" && (
          <>
            <Switch
              label={t("settings.schedule.enable")}
              hint={t("settings.schedule.enableHint")}
              checked={schedule.enabled}
              onChange={(enabled) => setSchedule({ ...schedule, enabled })}
            />
            {schedulable.length > 1 && (
              <div className={schedule.enabled ? "" : "opacity-50"}>
                <Field label={t("settings.schedule.queue")} hint={schedule.queue ? t("settings.schedule.queueHint") : undefined}>
                  {(id) => (
                    <select id={id} value={schedule.queue} onChange={(e) => setSchedule({ ...schedule, queue: e.target.value })} className={`${inputClass} h-9`}>
                      <option value="">{t("settings.schedule.everyQueue")}</option>
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
              <Field label={t("settings.schedule.start")}>
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
              <Field label={t("settings.schedule.stop")} hint={t("settings.schedule.stopHint")}>
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
              <legend className="label mb-2">{t("settings.schedule.days")}</legend>
              <div className="flex flex-wrap gap-1.5">
                {weekdays(locale).map((day, i) => {
                  const on = schedule.days[i] ?? false;
                  return (
                    <button
                      key={i}
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
            <Field label={t("settings.app.language")} hint={t("settings.app.languageHint")}>
              {(id) => (
                <select id={id} value={language} onChange={(e) => setLanguage(e.target.value)} className={`${inputClass} h-9`}>
                  <option value="">{t("settings.app.systemDefault", { language: systemLocaleName() })}</option>
                  {LOCALES.map((l) => (
                    <option key={l.code} value={l.code}>
                      {l.name}
                    </option>
                  ))}
                </select>
              )}
            </Field>
            <Switch label={t("settings.app.notify")} hint={t("settings.app.notifyHint")} checked={notify} onChange={setNotify} />
            <Switch label={t("settings.app.tray")} hint={t("settings.app.trayHint")} checked={closeToTray} onChange={setCloseToTray} />
            <Switch label={t("settings.app.startup")} hint={t("settings.app.startupHint")} checked={startWithSystem} onChange={setStartWithSystem} />
            <Switch label={t("settings.app.clipboard")} hint={t("settings.app.clipboardHint")} checked={watchClipboard} onChange={setWatchClipboard} />
            <Switch label={t("settings.app.awake")} hint={t("settings.app.awakeHint")} checked={keepAwake} onChange={setKeepAwake} />
            <Switch
              label={t("settings.app.confirmBrowser")}
              hint={t("settings.app.confirmBrowserHint")}
              checked={confirmBrowser}
              onChange={setConfirmBrowser}
            />
            <BrowserBridge registerHost={registerHost} onRegisterHostChange={setRegisterHost} />
          </>
        )}

        {tab === "remote" && <RemoteSettingsPanel />}
      </div>
    </Dialog>
  );
}

/** Which FFmpeg the saved settings lead to. */
function FfmpegStatus() {
  const { t } = useI18n();
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
      <Badge tone="positive">{t("settings.media.found")}</Badge>
      <span className="truncate font-mono text-[12px] text-ink-secondary select-text" title={found}>
        {found}
      </span>
    </div>
  ) : (
    <div className="flex flex-col gap-1.5">
      <div>
        <Badge tone="warning">{t("settings.media.notFound")}</Badge>
      </div>
      <p className="text-[13px] text-ink-secondary">{t("settings.media.ffmpegMissing")}</p>
    </div>
  );
}

/**
 * Videos on pages (YouTube): yt-dlp and Deno, found or downloaded on request.
 * The status describes the saved settings, like FFmpeg's above.
 */
function SiteTools({
  ytDlpPath,
  denoPath,
  onYtDlpPath,
  onDenoPath,
  denoConfigured,
}: {
  ytDlpPath: string;
  denoPath: string;
  onYtDlpPath: (v: string) => void;
  onDenoPath: (v: string) => void;
  /** A Deno path is saved in the settings, so a download would not be used. */
  denoConfigured: boolean;
}) {
  const { t } = useI18n();
  const headingId = useId();
  const [status, setStatus] = useState<ToolsStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let live = true;
    api
      .siteTools()
      .then((s) => live && setStatus(s))
      .catch((err: unknown) => live && setError(errorMessage(err)));
    return () => {
      live = false;
    };
  }, []);

  const wanted = status ? toolsToInstall(status, denoConfigured) : [];
  const install = async () => {
    setBusy(true);
    setError(null);
    try {
      setStatus(await api.installSiteTools(wanted));
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section aria-labelledby={headingId} className="flex flex-col gap-4 border-t border-line pt-5">
      <h3 id={headingId} className="label">
        {t("settings.siteTools.heading")}
      </h3>
      <p className="text-[13px] text-ink-secondary">{t("settings.siteTools.intro")}</p>
      <Field label="yt-dlp" hint={t("settings.siteTools.ytDlpHint")}>
        {(id) => (
          <input
            id={id}
            value={ytDlpPath}
            placeholder={t("settings.siteTools.ytDlpPlaceholder")}
            spellCheck={false}
            onChange={(e) => onYtDlpPath(e.target.value)}
            className={`${inputClass} h-9 font-mono text-[13px]`}
          />
        )}
      </Field>
      <Field label="Deno" hint={t("settings.siteTools.denoHint")}>
        {(id) => (
          <input
            id={id}
            value={denoPath}
            placeholder={t("settings.siteTools.denoPlaceholder")}
            spellCheck={false}
            onChange={(e) => onDenoPath(e.target.value)}
            className={`${inputClass} h-9 font-mono text-[13px]`}
          />
        )}
      </Field>

      {error && <Banner tone="danger">{error}</Banner>}
      {!status && !error && <p className="text-[13px] text-ink-muted">{t("settings.siteTools.looking")}</p>}
      {status && (
        <ul aria-label={t("settings.siteTools.listLabel")} className="divide-y divide-line rounded-md border border-line">
          {toolRows(status).map((row) => (
            <li key={row.name} className="flex flex-col gap-1 px-3 py-2">
              <div className="flex items-center justify-between gap-3">
                <span className="text-[13px] text-ink">{row.name}</span>
                <Badge tone={row.tone}>{row.badge}</Badge>
              </div>
              {row.detail && <p className="font-mono text-[12px] break-all text-ink-muted select-text">{row.detail}</p>}
              {row.problem && <p className="text-[12px] text-warning-fg">{row.problem}</p>}
            </li>
          ))}
        </ul>
      )}
      {status && wanted.length > 0 && (
        <div className="flex flex-col gap-2">
          <div>
            <Button size="sm" onClick={() => void install()} disabled={busy}>
              {busy ? t("settings.siteTools.downloading") : installButtonLabel(status, wanted)}
            </Button>
          </div>
          <p className="text-[12px] text-ink-muted">
            {fill(t("settings.siteTools.installNote"), {
              folder: <span className="font-mono break-all">{status.managed_dir ?? t("settings.siteTools.toolsFolder")}</span>,
            })}
          </p>
        </div>
      )}
      <Banner tone="neutral">{t("settings.siteTools.cookies")}</Banner>
    </section>
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
  const { t } = useI18n();
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
        {t("settings.bridge.heading")}
      </h3>
      <Switch label={t("settings.bridge.register")} hint={t("settings.bridge.registerHint")} checked={registerHost} onChange={onRegisterHostChange} />

      {error && <Banner tone="danger">{error}</Banner>}
      {!status && !error && <p className="text-[13px] text-ink-muted">{t("settings.bridge.checking")}</p>}
      {status && !status.host_found && (
        <Banner tone="warning">
          {fill(t("settings.bridge.noHost"), {
            host: <span className="font-mono break-all">{status.host}</span>,
            command: <span className="font-mono">pnpm sidecar</span>,
          })}
        </Banner>
      )}
      {status && (
        <ul aria-label={t("settings.bridge.listLabel")} className="divide-y divide-line rounded-md border border-line">
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
            {busy ? t("settings.bridge.registering") : t("settings.bridge.registerNow")}
          </Button>
          <span className="text-[12px] text-ink-muted">{t("settings.bridge.pickUp")}</span>
        </div>
      )}
    </section>
  );
}
