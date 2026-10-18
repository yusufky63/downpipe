// BitTorrent downloads in the list and the dialogs: pure functions, tested
// in torrent.test.ts.
import { currentLocale, t } from "../i18n/core";
import type { Task, TorrentChoices, TorrentSettings } from "./api";
import { bytesToRate, formatBytes, formatSpeed, rateToBytes, type RateUnit } from "./format";

/** The core refuses bigger ones; real .torrent files are far smaller. */
export const MAX_TORRENT_FILE = 16 * 1024 * 1024;

export function isMagnet(text: string): boolean {
  return /^magnet:\?/i.test(text.trim());
}

/** A file name or link path that ends in `.torrent`. */
export function isTorrentName(name: string): boolean {
  return /\.torrent$/i.test(name.trim());
}

/** An http(s) link to a `.torrent` file. */
export function isTorrentLink(url: string): boolean {
  try {
    const u = new URL(url);
    return (u.protocol === "http:" || u.protocol === "https:") && isTorrentName(u.pathname);
  } catch {
    return false;
  }
}

/** Links the core turns into torrent downloads, whose files are chosen next. */
export function isTorrentUrl(url: string): boolean {
  return isMagnet(url) || isTorrentLink(url);
}

/** A magnet link's `dn`, the name it gives the torrent. */
export function magnetName(url: string): string | null {
  if (!isMagnet(url)) return null;
  const name = new URLSearchParams(url.slice(url.indexOf("?") + 1)).get("dn")?.trim();
  return name || null;
}

export function isSeeding(task: Task): boolean {
  return task.swarm?.seeding === true;
}

/** Uploaded divided by downloaded, once something was downloaded. */
export function ratio(task: Task): number | null {
  if (!task.torrent) return null;
  const size = task.total ?? task.downloaded;
  return size > 0 ? task.torrent.uploaded / size : null;
}

export function formatRatio(value: number): string {
  return new Intl.NumberFormat(currentLocale(), { minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(value);
}

/** What the list says under a torrent's name, or null to show the usual. */
export function torrentDetail(task: Task): string | null {
  if (!task.torrent) return null;
  const swarm = task.swarm;
  if (swarm?.fetching_metadata) return t("torrent.row.fetching");
  const parts: string[] = [];
  if (swarm) {
    parts.push(t("torrent.row.peers", { count: swarm.peers }));
    if (swarm.upload_speed > 0) parts.push(t("torrent.row.upload", { speed: formatSpeed(swarm.upload_speed) }));
  }
  const r = ratio(task);
  if (r !== null && (task.torrent.uploaded > 0 || swarm?.seeding)) parts.push(t("torrent.row.ratio", { ratio: formatRatio(r) }));
  return parts.length > 0 ? parts.join(" · ") : null;
}

/** The files chosen in the file list, or null when all of them are. */
export function chosenFiles(choices: TorrentChoices, chosen: ReadonlySet<number>): number[] | null {
  const all = choices.files.every((f) => chosen.has(f.index));
  return all ? null : choices.files.filter((f) => chosen.has(f.index)).map((f) => f.index);
}

/** The line under the file list: "2 of 3 files · 1.2 GiB". */
export function chosenSummary(choices: TorrentChoices, chosen: ReadonlySet<number>): string {
  const picked = choices.files.filter((f) => chosen.has(f.index));
  const size = picked.reduce((sum, f) => sum + f.size, 0);
  return t("torrent.confirm.chosen", { count: picked.length, total: choices.files.length, size: formatBytes(size) });
}

/** The files to start with: what was chosen before, else all. */
export function initialChosen(choices: TorrentChoices): Set<number> {
  const all = choices.files.map((f) => f.index);
  return new Set(choices.selection ?? all);
}

/** The BitTorrent settings as the form edits them: numbers as typed. */
export interface TorrentDraft {
  seed: boolean;
  /** "1.0"; empty or 0 for no ratio limit. */
  ratio: string;
  minutes: string;
  upload: string;
  uploadUnit: RateUnit;
  port: string;
  dht: boolean;
  lsd: boolean;
  upnp: boolean;
}

export function torrentDraft(s: TorrentSettings): TorrentDraft {
  const rate = bytesToRate(s.upload_limit);
  return {
    seed: s.seed,
    ratio: s.seed_ratio_percent > 0 ? String(s.seed_ratio_percent / 100) : "0",
    minutes: String(s.seed_minutes),
    upload: rate.value > 0 ? String(rate.value) : "",
    uploadUnit: rate.unit,
    port: String(s.listen_port),
    dht: s.dht,
    lsd: s.lsd,
    upnp: s.upnp,
  };
}

/** The draft as settings; what does not parse keeps the saved value. */
export function fromTorrentDraft(d: TorrentDraft, saved: TorrentSettings): TorrentSettings {
  const ratio = Number.parseFloat(d.ratio.replace(",", "."));
  const minutes = Number.parseInt(d.minutes, 10);
  const port = Number.parseInt(d.port, 10);
  return {
    seed: d.seed,
    seed_ratio_percent: Number.isFinite(ratio) && ratio >= 0 ? Math.round(ratio * 100) : saved.seed_ratio_percent,
    seed_minutes: Number.isFinite(minutes) && minutes >= 0 ? minutes : saved.seed_minutes,
    upload_limit: rateToBytes(Number.parseFloat(d.upload), d.uploadUnit),
    listen_port: Number.isFinite(port) && port >= 0 && port <= 65535 ? port : saved.listen_port,
    dht: d.dht,
    lsd: d.lsd,
    upnp: d.upnp,
  };
}
