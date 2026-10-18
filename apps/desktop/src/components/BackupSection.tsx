// Settings → App: backups of the settings and list, and IDM's list. Rust
// opens and reads the files (src-tauri/src/backup.rs); this only shows what
// it found and passes the user's choice back.
import { useId, useState } from "react";

import { useI18n } from "../i18n";
import { api, type BackupPreview, type IdmList, type IdmSource, type Settings } from "../lib/api";
import { backupOrigin, idmRowName, importErrorText, importReportText, importableRows, initialIdmSelection } from "../lib/backup";
import { formatBytes } from "../lib/format";
import { Badge, Banner, Button } from "./primitives";

/** Message shown under a section after an action, or its error. */
type Note = { tone: "neutral" | "danger"; text: string } | null;

function Checkbox({
  label,
  hint,
  checked,
  onChange,
}: {
  label: string;
  hint?: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  const id = useId();
  return (
    <label htmlFor={id} className="flex cursor-pointer items-start gap-2.5">
      <input id={id} type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} className="mt-0.5 size-4 shrink-0 accent-primary" />
      <span className="flex min-w-0 flex-col gap-0.5">
        <span className="text-[14px] text-ink">{label}</span>
        {hint && <span className="text-[12px] text-ink-muted">{hint}</span>}
      </span>
    </label>
  );
}

/**
 * Export and import of a backup. Importing settings replaces what the
 * Settings dialog shows, so `onSettingsImported` closes it with the new
 * settings and the result.
 */
export function BackupSection({ onSettingsImported }: { onSettingsImported: (settings: Settings, message: string) => void }) {
  const { t } = useI18n();
  const headingId = useId();
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<Note>(null);
  const [preview, setPreview] = useState<BackupPreview | null>(null);
  const [takeSettings, setTakeSettings] = useState(true);
  const [takeDownloads, setTakeDownloads] = useState(true);

  const exportBackup = async () => {
    setBusy(true);
    setNote(null);
    try {
      const saved = await api.exportBackup();
      if (saved) setNote({ tone: "neutral", text: t("settings.backup.exported", { count: saved.downloads, file: saved.file }) });
    } catch (err) {
      setNote({ tone: "danger", text: importErrorText(err) });
    } finally {
      setBusy(false);
    }
  };

  const pick = async () => {
    setBusy(true);
    setNote(null);
    try {
      const read = await api.inspectBackup();
      if (read) {
        setPreview(read);
        setTakeSettings(true);
        setTakeDownloads(read.new_downloads > 0);
      }
    } catch (err) {
      setPreview(null);
      setNote({ tone: "danger", text: importErrorText(err) });
    } finally {
      setBusy(false);
    }
  };

  const importNow = async () => {
    setBusy(true);
    try {
      const report = await api.importBackup({ settings: takeSettings, downloads: takeDownloads });
      setPreview(null);
      const message = importReportText(report);
      if (report.settings) {
        onSettingsImported(await api.settings(), message);
        return;
      }
      setNote({ tone: "neutral", text: message });
    } catch (err) {
      setNote({ tone: "danger", text: importErrorText(err) });
    } finally {
      setBusy(false);
    }
  };

  return (
    <section aria-labelledby={headingId} className="flex flex-col gap-4 border-t border-line pt-5">
      <h3 id={headingId} className="label">
        {t("settings.backup.heading")}
      </h3>
      <p className="text-[13px] text-ink-secondary">{t("settings.backup.intro")}</p>
      <div className="flex flex-wrap gap-2">
        <Button size="sm" onClick={() => void exportBackup()} disabled={busy}>
          {t("settings.backup.export")}
        </Button>
        <Button size="sm" onClick={() => void pick()} disabled={busy}>
          {t("settings.backup.import")}
        </Button>
      </div>

      {preview && (
        <div className="flex flex-col gap-3 rounded-md border border-line p-3">
          <div className="flex flex-col gap-0.5">
            <p className="text-[14px] font-medium break-all text-ink">{t("settings.backup.previewHeading", { file: preview.file })}</p>
            <p className="text-[12px] text-ink-muted">{backupOrigin(preview)}</p>
          </div>
          <Checkbox
            label={t("settings.backup.settings")}
            hint={t("settings.backup.settingsHint", { queues: preview.queues, categories: preview.categories })}
            checked={takeSettings}
            onChange={setTakeSettings}
          />
          <Checkbox
            label={t("settings.backup.downloads")}
            hint={t("settings.backup.downloadsHint", { count: preview.new_downloads, total: preview.downloads })}
            checked={takeDownloads}
            onChange={setTakeDownloads}
          />
          {preview.invalid > 0 && <Banner tone="warning">{t("settings.backup.invalid", { count: preview.invalid })}</Banner>}
          <div className="flex flex-wrap justify-end gap-2">
            <Button size="sm" variant="ghost" onClick={() => setPreview(null)} disabled={busy}>
              {t("common.cancel")}
            </Button>
            <Button size="sm" variant="primary" onClick={() => void importNow()} disabled={busy || (!takeSettings && !takeDownloads)}>
              {t("settings.backup.importNow")}
            </Button>
          </div>
        </div>
      )}

      {busy && !preview && <p className="text-[13px] text-ink-muted">{t("settings.backup.reading")}</p>}
      {note && (note.tone === "danger" ? <Banner tone="danger">{note.text}</Banner> : <p className="text-[13px] text-ink-secondary">{note.text}</p>)}
    </section>
  );
}

/** IDM's list, from this computer (Windows) or an exported file, as a checklist. */
export function IdmImport({ windows }: { windows: boolean }) {
  const { t } = useI18n();
  const headingId = useId();
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<Note>(null);
  const [list, setList] = useState<IdmList | null>(null);
  const [chosen, setChosen] = useState<Set<number>>(new Set());

  const read = async (source: IdmSource) => {
    setBusy(true);
    setNote(null);
    try {
      const found = await api.readIdmList(source);
      if (found) {
        setList(found);
        setChosen(initialIdmSelection(found.rows));
      }
    } catch (err) {
      setList(null);
      setNote({ tone: "danger", text: importErrorText(err) });
    } finally {
      setBusy(false);
    }
  };

  const add = async () => {
    setBusy(true);
    try {
      const report = await api.importIdm([...chosen].sort((a, b) => a - b));
      setList(null);
      setNote({ tone: "neutral", text: importReportText(report) });
    } catch (err) {
      setNote({ tone: "danger", text: importErrorText(err) });
    } finally {
      setBusy(false);
    }
  };

  const toggle = (i: number, on: boolean) =>
    setChosen((current) => {
      const next = new Set(current);
      if (on) next.add(i);
      else next.delete(i);
      return next;
    });

  return (
    <section aria-labelledby={headingId} className="flex flex-col gap-4 border-t border-line pt-5">
      <h3 id={headingId} className="label">
        {t("settings.idm.heading")}
      </h3>
      <p className="text-[13px] text-ink-secondary">{t("settings.idm.intro")}</p>
      <div className="flex flex-col gap-2">
        <div className="flex flex-wrap gap-2">
          {windows && (
            <Button size="sm" onClick={() => void read("computer")} disabled={busy}>
              {t("settings.idm.fromComputer")}
            </Button>
          )}
          <Button size="sm" onClick={() => void read("file")} disabled={busy}>
            {t("settings.idm.fromFile")}
          </Button>
        </div>
        <p className="text-[12px] text-ink-muted">{t("settings.idm.fileHint")}</p>
      </div>

      {list && (
        <div className="flex flex-col gap-3 rounded-md border border-line p-3">
          <p className="text-[14px] font-medium break-all text-ink">
            {list.file ? t("settings.idm.sourceFile", { file: list.file }) : t("settings.idm.sourceComputer")}
          </p>
          {list.rows.length === 0 ? (
            <p className="text-[13px] text-ink-muted">{t("settings.idm.empty")}</p>
          ) : (
            <>
              <div className="flex flex-wrap gap-x-3 gap-y-1">
                <button type="button" className="text-[13px] text-primary hover:underline" onClick={() => setChosen(new Set(importableRows(list.rows)))}>
                  {t("settings.idm.selectAll")}
                </button>
                <button type="button" className="text-[13px] text-primary hover:underline" onClick={() => setChosen(new Set())}>
                  {t("settings.idm.selectNone")}
                </button>
              </div>
              <ul aria-label={t("settings.idm.listLabel")} className="max-h-72 divide-y divide-line overflow-y-auto rounded-md border border-line">
                {list.rows.map((row, i) => {
                  const id = `${headingId}-row-${i}`;
                  return (
                    <li key={`${i}-${row.url}`} className="flex items-start gap-2.5 px-3 py-2">
                      <input
                        id={id}
                        type="checkbox"
                        checked={chosen.has(i)}
                        disabled={row.listed}
                        onChange={(e) => toggle(i, e.target.checked)}
                        className="mt-0.5 size-4 shrink-0 accent-primary"
                      />
                      <label htmlFor={id} className="flex min-w-0 flex-1 flex-col gap-0.5">
                        <span className="flex min-w-0 items-center gap-2">
                          <span className="truncate text-[13px] text-ink" title={idmRowName(row)}>
                            {idmRowName(row)}
                          </span>
                          {row.listed ? (
                            <Badge tone="neutral">{t("settings.idm.listed")}</Badge>
                          ) : row.finished ? (
                            <Badge tone="positive">{t("settings.idm.finished")}</Badge>
                          ) : null}
                        </span>
                        <span className="truncate font-mono text-[12px] text-ink-muted" title={row.url}>
                          {row.url}
                        </span>
                        <span className="truncate text-[12px] text-ink-muted" title={row.dir ?? undefined}>
                          {row.dir ?? t("settings.idm.defaultFolder")} · {row.size !== null ? formatBytes(row.size) : t("settings.idm.sizeUnknown")}
                        </span>
                      </label>
                    </li>
                  );
                })}
              </ul>
            </>
          )}
          <div className="flex flex-wrap justify-end gap-2">
            <Button size="sm" variant="ghost" onClick={() => setList(null)} disabled={busy}>
              {t("common.cancel")}
            </Button>
            {list.rows.length > 0 && (
              <Button size="sm" variant="primary" onClick={() => void add()} disabled={busy || chosen.size === 0}>
                {t("settings.idm.add", { count: chosen.size })}
              </Button>
            )}
          </div>
        </div>
      )}

      {busy && !list && <p className="text-[13px] text-ink-muted">{t("settings.backup.reading")}</p>}
      {note && (note.tone === "danger" ? <Banner tone="danger">{note.text}</Banner> : <p className="text-[13px] text-ink-secondary">{note.text}</p>)}
    </section>
  );
}
