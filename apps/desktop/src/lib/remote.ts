// The remote control's web page (remote.html) and the Remote settings tab:
// the HTTP API's shapes, a small client and pure helpers. Nothing here may
// import Tauri; the page runs in a phone's browser.
import { t, type MessageKey } from "../i18n/core";
import type { TaskState, WaitReason } from "./api";
import type { Tone } from "./tasks";

/** One row as the API sends it (downpipe-remote `DownloadView`). */
export interface RemoteDownload {
  id: string;
  name: string;
  host: string | null;
  state: TaskState;
  total: number | null;
  downloaded: number;
  speed: number;
  eta_secs: number | null;
  error_kind: string | null;
  waiting: WaitReason | null;
  queue: string;
  category: string | null;
  awaiting_confirmation: boolean;
  quality: string | null;
  created_at: number;
  completed_at: number | null;
}

export interface RemoteQueue {
  id: string;
  name: string;
}

/** The settings the API lets the page read. */
export interface RemoteSettingsView {
  version: string;
  language: string;
  main_queue: string;
  queues: RemoteQueue[];
  max_concurrent_downloads: number;
  max_connections_per_download: number;
  speed_limit: number;
  use_categories: boolean;
}

export interface RemoteAdd {
  url: string;
  folder?: string;
  queue?: string;
  paused?: boolean;
  add_copy?: boolean;
}

/** An answer other than success: `code` is the API's `error` field. */
export class RemoteError extends Error {
  constructor(
    readonly status: number,
    readonly code: string,
    readonly id: string | null = null,
  ) {
    super(code);
  }
}

/** Where the token is kept in the phone's browser. */
export const TOKEN_KEY = "downpipe:remote-token";

/**
 * The token from a link the settings made (`.../#token=abcd`). It travels in
 * the fragment, which browsers never send to a server or put in a referrer.
 */
export function tokenFromHash(hash: string): string | null {
  const params = new URLSearchParams(hash.replace(/^#/, ""));
  const token = cleanToken(params.get("token") ?? "");
  return token === "" ? null : token;
}

/** What the user typed or pasted: no spaces or dashes, lower case. */
export function cleanToken(text: string): string {
  return text.replace(/[\s-]/g, "").toLowerCase();
}

/** `abcd-efgh-…`, easier to read and copy by hand. */
export function groupToken(token: string): string {
  return (token.match(/.{1,4}/g) ?? []).join("-");
}

/** The page's address with the token in its fragment, for the QR code. */
export function linkWithToken(url: string, token: string): string {
  return `${url.replace(/#.*$/, "")}#token=${token}`;
}

/** One SVG path for the dark modules of a QR code, one unit per module. */
export function qrPath(modules: readonly (readonly boolean[])[]): string {
  const parts: string[] = [];
  modules.forEach((row, y) => {
    row.forEach((dark, x) => {
      if (dark) parts.push(`M${x} ${y}h1v1h-1z`);
    });
  });
  return parts.join("");
}

const RUNNING: ReadonlySet<TaskState> = new Set(["starting", "downloading", "waiting", "verifying"]);

export const remoteCanPause = (d: RemoteDownload) => !d.awaiting_confirmation && (RUNNING.has(d.state) || d.state === "queued");
export const remoteCanResume = (d: RemoteDownload) =>
  d.awaiting_confirmation || d.state === "paused" || d.state === "failed" || d.state === "canceled";

const STATE_TONES: Record<TaskState, Tone> = {
  queued: "neutral",
  starting: "primary",
  downloading: "primary",
  waiting: "warning",
  verifying: "primary",
  paused: "neutral",
  completed: "positive",
  failed: "danger",
  canceled: "neutral",
};

export function remoteStateInfo(d: RemoteDownload): { label: string; tone: Tone } {
  if (d.awaiting_confirmation) return { label: t("states.confirm"), tone: "warning" };
  return { label: t(`states.${d.state}`), tone: STATE_TONES[d.state] };
}

const ERROR_KEYS: Record<string, MessageKey> = {
  unauthorized: "remote.page.errors.unauthorized",
  too_many_attempts: "remote.page.errors.tooMany",
  duplicate: "remote.page.errors.duplicate",
  invalid_url: "remote.page.errors.invalidUrl",
  invalid_folder: "remote.page.errors.invalidFolder",
  unknown_queue: "remote.page.errors.unknownQueue",
  not_found: "remote.page.errors.notFound",
  invalid_state: "remote.page.errors.invalidState",
  too_large: "remote.page.errors.invalidUrl",
  busy: "remote.page.errors.busy",
  offline: "remote.page.errors.offline",
};

/** A sentence for an API error code; unknown codes get a general one. */
export function remoteErrorText(code: string): string {
  return t(ERROR_KEYS[code] ?? "remote.page.errors.failed");
}

type Fetch = (input: string, init?: RequestInit) => Promise<Response>;

/** The API under `/api/v1` of the page's own server. */
export function remoteClient(token: string, fetchImpl: Fetch = (input, init) => fetch(input, init)) {
  const call = async <T>(method: string, path: string, body?: unknown): Promise<T> => {
    let res: Response;
    try {
      res = await fetchImpl(`/api/v1${path}`, {
        method,
        headers: {
          Authorization: `Bearer ${token}`,
          ...(body === undefined ? {} : { "Content-Type": "application/json" }),
        },
        body: body === undefined ? undefined : JSON.stringify(body),
        cache: "no-store",
        credentials: "omit",
      });
    } catch {
      throw new RemoteError(0, "offline");
    }
    if (res.status === 204) return undefined as T;
    let data: unknown = null;
    try {
      data = await res.json();
    } catch {
      // Not JSON: an error page from something between the phone and Downpipe.
    }
    if (!res.ok) {
      const err = (data ?? {}) as { error?: unknown; id?: unknown };
      throw new RemoteError(res.status, typeof err.error === "string" ? err.error : "failed", typeof err.id === "string" ? err.id : null);
    }
    return data as T;
  };
  const one = (id: string) => `/downloads/${encodeURIComponent(id)}`;
  return {
    list: () => call<{ downloads: RemoteDownload[] }>("GET", "/downloads").then((r) => r.downloads),
    settings: () => call<RemoteSettingsView>("GET", "/settings"),
    add: (options: RemoteAdd) => call<{ download: RemoteDownload }>("POST", "/downloads", options).then((r) => r.download),
    pause: (id: string) => call<undefined>("POST", `${one(id)}/pause`),
    resume: (id: string) => call<undefined>("POST", `${one(id)}/resume`),
    remove: (id: string) => call<undefined>("DELETE", one(id)),
  };
}

export type RemoteClient = ReturnType<typeof remoteClient>;
