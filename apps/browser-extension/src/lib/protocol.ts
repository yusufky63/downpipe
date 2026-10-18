// Messages to and from downpipe-native-host. The host validates everything
// again (crates/native-host/src/protocol.rs); these builders make sure the
// extension never even asks for something the host would refuse, and that
// optional hints which do not fit are dropped instead of failing the request.
import { siteList, sitePage } from "./sites";
import { MAX_URL_LENGTH, baseName, webUrl, withoutSecrets } from "./url";

export const HOST_NAME = "dev.downpipe.native_host";
export const PROTOCOL_VERSION = 1;
export const MAX_FILE_NAME_LENGTH = 255;
export const MAX_MIME_LENGTH = 255;
/** Matches `MAX_TASK_IDS` in crates/native-host/src/protocol.rs. */
export const MAX_TASK_IDS = 20;
const ID_PATTERN = /^[A-Za-z0-9_-]{1,64}$/;

export interface PingMessage {
  protocol_version: typeof PROTOCOL_VERSION;
  type: "ping";
  id: string;
}

export interface DownloadMessage {
  protocol_version: typeof PROTOCOL_VERSION;
  type: "download";
  id: string;
  resource_url: string;
  page_url?: string;
  referrer?: string;
  mime_type?: string;
  file_name?: string;
}

/** A stream the page played: its manifest, for Downpipe to read and download. */
export interface MediaMessage {
  protocol_version: typeof PROTOCOL_VERSION;
  type: "media";
  id: string;
  manifest_url: string;
  kind: StreamKind;
  page_url?: string;
  referrer?: string;
  /** The page title; Downpipe names the file after it. */
  title?: string;
  /** The `Origin` the page's player sent, if it sent one. */
  origin?: string;
}

/**
 * A video page (YouTube) for Downpipe's site provider: the page and its
 * title only, never the media requests its player made, and never cookies.
 */
export interface PageMessage {
  protocol_version: typeof PROTOCOL_VERSION;
  type: "page";
  id: string;
  page_url: string;
  title?: string;
}

/** Streams the `media` message carries. Single media files go as `download`. */
export type StreamKind = "hls" | "dash";

/** Asks where downloads the extension sent stand now. */
export interface TasksMessage {
  protocol_version: typeof PROTOCOL_VERSION;
  type: "tasks";
  id: string;
  task_ids: string[];
}

export type HostMessage = PingMessage | DownloadMessage | MediaMessage | PageMessage | TasksMessage;

/** What the extension knows about a download before it is sent. */
export interface DownloadFields {
  resourceUrl: string;
  pageUrl?: string;
  referrer?: string;
  mimeType?: string;
  fileName?: string;
}

export function newMessageId(): string {
  return crypto.randomUUID();
}

export function pingMessage(id: string): PingMessage {
  return { protocol_version: PROTOCOL_VERSION, type: "ping", id };
}

/** Null when there is nothing valid to ask about. */
export function tasksMessage(taskIds: readonly string[], id: string): TasksMessage | null {
  const ids = [...new Set(taskIds.filter((t) => ID_PATTERN.test(t)))].slice(0, MAX_TASK_IDS);
  return ids.length ? { protocol_version: PROTOCOL_VERSION, type: "tasks", id, task_ids: ids } : null;
}

/** Null when the resource itself cannot be downloaded (not http/https). */
export function downloadMessage(fields: DownloadFields, id: string): DownloadMessage | null {
  const resource = webUrl(fields.resourceUrl);
  if (!resource) return null;
  resource.hash = "";
  if (resource.href.length > MAX_URL_LENGTH) return null;
  const message: DownloadMessage = {
    protocol_version: PROTOCOL_VERSION,
    type: "download",
    id,
    resource_url: resource.href,
  };
  const page = webUrl(fields.pageUrl);
  if (page) message.page_url = withoutSecrets(page).href;
  const referrer = webUrl(fields.referrer);
  if (referrer) message.referrer = withoutSecrets(referrer).href;
  const mime = mimeEssence(fields.mimeType);
  if (mime) message.mime_type = mime;
  const name = fileNameHint(fields.fileName);
  if (name) message.file_name = name;
  return message;
}

/** What the extension knows about a stream before it is sent. */
export interface MediaFields {
  manifestUrl: string;
  kind: StreamKind;
  pageUrl?: string;
  referrer?: string;
  title?: string;
  origin?: string;
}

/** Matches `MAX_TITLE_LEN` in crates/native-host/src/protocol.rs. */
export const MAX_TITLE_LENGTH = 200;

/** Null when the manifest URL is not http(s) or too long. */
export function mediaMessage(fields: MediaFields, id: string): MediaMessage | null {
  const manifest = webUrl(fields.manifestUrl);
  if (!manifest) return null;
  manifest.hash = "";
  if (manifest.href.length > MAX_URL_LENGTH) return null;
  const message: MediaMessage = {
    protocol_version: PROTOCOL_VERSION,
    type: "media",
    id,
    manifest_url: manifest.href,
    kind: fields.kind,
  };
  const page = webUrl(fields.pageUrl);
  if (page) message.page_url = withoutSecrets(page).href;
  const referrer = webUrl(fields.referrer);
  if (referrer) message.referrer = withoutSecrets(referrer).href;
  const title = titleHint(fields.title);
  if (title) message.title = title;
  const origin = webUrl(fields.origin);
  if (origin) message.origin = origin.origin;
  return message;
}

/**
 * Null unless the page is one the site provider takes (`sitePage`) or a
 * playlist or channel it lists (`siteList`), which the app opens for the
 * user to choose its videos.
 */
export function pageMessage(fields: { pageUrl: string; title?: string }, id: string): PageMessage | null {
  const page = sitePage(fields.pageUrl) ?? siteList(fields.pageUrl);
  if (!page) return null;
  const message: PageMessage = { protocol_version: PROTOCOL_VERSION, type: "page", id, page_url: page.url };
  const title = titleHint(fields.title);
  if (title) message.title = title;
  return message;
}

/** A title the host accepts: no control characters, capped; otherwise undefined. */
export function titleHint(raw: string | undefined): string | undefined {
  const title = raw?.trim();
  if (!title || [...title].length > MAX_TITLE_LENGTH) return undefined;
  for (const ch of title) {
    const code = ch.codePointAt(0) ?? 0;
    if (code < 0x20 || (code >= 0x7f && code <= 0x9f)) return undefined;
  }
  return title;
}

const MIME_TOKEN = /^[a-z0-9!#$&^_.+-]{1,127}$/;

/** `type/subtype` in lower case, without parameters, or undefined. */
export function mimeEssence(raw: string | undefined): string | undefined {
  if (!raw || raw.length > MAX_MIME_LENGTH) return undefined;
  const essence = (raw.split(";")[0] ?? "").trim().toLowerCase();
  const [type, subtype, ...rest] = essence.split("/");
  if (rest.length || !type || !subtype) return undefined;
  return MIME_TOKEN.test(type) && MIME_TOKEN.test(subtype) ? essence : undefined;
}

/** A bare file name the host will accept, or undefined. */
export function fileNameHint(raw: string | undefined): string | undefined {
  if (!raw) return undefined;
  const name = baseName(raw).trim();
  if (!name || name === "." || name === "..") return undefined;
  if ([...name].length > MAX_FILE_NAME_LENGTH) return undefined;
  for (const ch of name) {
    const code = ch.codePointAt(0) ?? 0;
    if (code < 0x20 || (code >= 0x7f && code <= 0x9f)) return undefined;
  }
  return name;
}

export const TASK_STATES = [
  "queued",
  "starting",
  "downloading",
  "waiting",
  "verifying",
  "paused",
  "completed",
  "failed",
  "canceled",
] as const;

export type TaskState = (typeof TASK_STATES)[number];

/** Where one download stands in the app. */
export interface TaskStatus {
  taskId: string;
  state: TaskState;
  fileName?: string;
  downloaded: number;
  total?: number;
  /** Bytes per second. */
  speed: number;
  error?: string;
  /** The failure's stable kind (`not_found`, `network`), from apps that send it. */
  errorKind?: string;
  /**
   * Paused in the app until the user confirms its name, folder and queue.
   * A flag next to `paused` rather than a state of its own, so extensions
   * that predate it still read the entry.
   */
  awaitingConfirmation?: boolean;
}

export type HostResponse =
  | {
      type: "pong";
      id?: string;
      hostVersion: string;
      appRunning: boolean;
      appVersion?: string;
      /** What both the host and the app support beyond version 1's basics, such as `media`. */
      features: string[];
    }
  | { type: "accepted"; id?: string; taskId: string; fileName?: string; awaitingConfirmation?: boolean }
  | { type: "error"; id?: string; code: string; message: string; taskId?: string }
  | { type: "tasks"; id?: string; tasks: TaskStatus[] }
  /** A playlist or channel is open in the app, to choose its videos there. */
  | { type: "list_opened"; id?: string; pageUrl: string };

function text(value: unknown, max = 300): string | undefined {
  return typeof value === "string" ? value.slice(0, max) : undefined;
}

function count(value: unknown): number | undefined {
  return typeof value === "number" && Number.isFinite(value) && value >= 0 ? value : undefined;
}

/** The host leaves optional flags out unless they are set. */
function flag(value: unknown): true | undefined {
  return value === true ? true : undefined;
}

function parseTask(value: unknown): TaskStatus | null {
  if (typeof value !== "object" || value === null) return null;
  const v = value as Record<string, unknown>;
  const taskId = text(v.task_id, 64);
  const state = TASK_STATES.find((s) => s === v.state);
  const downloaded = count(v.downloaded);
  const speed = count(v.speed);
  if (taskId === undefined || !state || downloaded === undefined || speed === undefined) return null;
  return {
    taskId,
    state,
    fileName: text(v.file_name, MAX_FILE_NAME_LENGTH),
    downloaded,
    total: count(v.total),
    speed,
    error: text(v.error),
    errorKind: typeof v.error_kind === "string" && /^[a-z_]{1,32}$/.test(v.error_kind) ? v.error_kind : undefined,
    awaitingConfirmation: flag(v.awaiting_confirmation),
  };
}

/**
 * Check the shape of what the host sent. Anything unexpected is treated as
 * a broken bridge rather than trusted, and strings are length-capped
 * because they end up in the popup.
 */
export function parseResponse(value: unknown): HostResponse | null {
  if (typeof value !== "object" || value === null) return null;
  const v = value as Record<string, unknown>;
  if (v.protocol_version !== PROTOCOL_VERSION) return null;
  const id = text(v.id, 64);
  switch (v.type) {
    case "pong": {
      const hostVersion = text(v.host_version, 32);
      if (hostVersion === undefined || typeof v.app_running !== "boolean") return null;
      const features = Array.isArray(v.features)
        ? v.features.slice(0, 16).flatMap((f) => (typeof f === "string" && f.length <= 32 ? [f] : []))
        : [];
      return { type: "pong", id, hostVersion, appRunning: v.app_running, appVersion: text(v.app_version, 32), features };
    }
    case "accepted": {
      const taskId = text(v.task_id, 64);
      if (taskId === undefined) return null;
      return {
        type: "accepted",
        id,
        taskId,
        fileName: text(v.file_name, MAX_FILE_NAME_LENGTH),
        awaitingConfirmation: flag(v.awaiting_confirmation),
      };
    }
    case "error": {
      const code = text(v.code, 64);
      const message = text(v.message);
      if (code === undefined || message === undefined) return null;
      return { type: "error", id, code, message, taskId: text(v.task_id, 64) };
    }
    case "list_opened": {
      const pageUrl = text(v.page_url, MAX_URL_LENGTH);
      if (pageUrl === undefined) return null;
      return { type: "list_opened", id, pageUrl };
    }
    case "tasks": {
      if (!Array.isArray(v.tasks)) return null;
      const tasks = v.tasks
        .slice(0, MAX_TASK_IDS)
        .map(parseTask)
        .filter((t): t is TaskStatus => t !== null);
      return { type: "tasks", id, tasks };
    }
    default:
      return null;
  }
}
