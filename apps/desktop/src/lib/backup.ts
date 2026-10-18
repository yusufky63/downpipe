// Texts and choices for Settings → App: backups and IDM's list. The files
// themselves are read and checked in Rust (src-tauri/src/backup.rs).
import { t } from "../i18n/core";
import { longDate } from "../i18n/format";
import { errorMessage, isCommandError, type BackupPreview, type IdmRow, type ImportReport } from "./api";
import { nameFromUrl } from "./format";

/** Why a picked file cannot be imported, in the user's words. */
export function importErrorText(err: unknown): string {
  if (isCommandError(err)) {
    switch (err.code) {
      case "not_a_backup":
        return t("settings.backup.errors.notABackup");
      case "backup_too_new":
        return t("settings.backup.errors.tooNew");
      case "backup_damaged":
        return t("settings.backup.errors.damaged");
      case "backup_too_large":
      case "file_too_large":
        return t("settings.backup.errors.tooLarge");
      case "idm_unreadable":
        return t("settings.idm.unreadable");
    }
  }
  return errorMessage(err);
}

/** "Made by Downpipe 0.4.0 on 18 Oct 2026, 14:02." with what is known. */
export function backupOrigin(preview: BackupPreview): string {
  const date = preview.exported_at > 0 ? longDate(preview.exported_at) : null;
  if (preview.app_version && date) return t("settings.backup.madeBy", { version: preview.app_version, date });
  if (date) return t("settings.backup.madeOn", { date });
  return t("settings.backup.madeUnknown");
}

/** What an import did, as one or more sentences. */
export function importReportText(report: ImportReport): string {
  const parts: string[] = [];
  if (report.settings) parts.push(t("settings.backup.result.settings"));
  parts.push(t("settings.backup.result.added", { count: report.added }));
  if (report.feeds > 0) parts.push(t("settings.backup.result.feeds", { count: report.feeds }));
  if (report.feeds_failed > 0) parts.push(t("settings.backup.result.feedsFailed", { count: report.feeds_failed }));
  if (report.duplicates > 0) parts.push(t("settings.backup.result.duplicates", { count: report.duplicates }));
  if (report.invalid > 0) parts.push(t("settings.backup.result.invalid", { count: report.invalid }));
  return parts.join(" ");
}

/**
 * Rows ticked when IDM's list opens: what is neither finished in IDM nor in
 * Downpipe's list already. Finished ones can still be ticked by hand.
 */
export function initialIdmSelection(rows: readonly IdmRow[]): Set<number> {
  const out = new Set<number>();
  rows.forEach((row, i) => {
    if (!row.listed && !row.finished) out.add(i);
  });
  return out;
}

/** Rows that can be imported: links Downpipe does not have yet. */
export function importableRows(rows: readonly IdmRow[]): number[] {
  return rows.flatMap((row, i) => (row.listed ? [] : [i]));
}

/** The name a row is shown under. */
export function idmRowName(row: IdmRow): string {
  return row.file_name ?? nameFromUrl(row.url);
}
