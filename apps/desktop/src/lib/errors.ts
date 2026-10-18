// Failures in the user's language. The engine and the core explain errors
// in English for logs and the CLI; the UI words each stable kind or code
// itself and keeps the engine's text as the detail, for the details dialog
// and for reports.
import { t, type MessageKey } from "../i18n/core";
import { sentence } from "./format";

/** `DownloadError::kind()` in crates/http-engine/src/error.rs. */
export const FAILURE_KINDS = [
  "invalid_url",
  "invalid_header",
  "auth_required",
  "forbidden",
  "not_found",
  "throttled",
  "http_status",
  "resource_changed",
  "different_file",
  "expired_link",
  "login_required",
  "range_not_honoured",
  "network",
  "checksum",
  "io",
  "corrupt_meta",
  "drm_protected",
  "live_stream",
  "invalid_manifest",
  "unsupported_media",
  "tool_missing",
  "unavailable",
  "extractor",
] as const;

export type FailureKind = (typeof FAILURE_KINDS)[number];

/**
 * Codes only commands return (src-tauri/src/main.rs and `CoreError::code`);
 * the engine's kinds can come back from commands too.
 */
export const COMMAND_CODES = [
  "duplicate",
  "gone",
  "invalid_state",
  "invalid_setting",
  "internal",
  "open_failed",
  "autostart",
  "drag",
  "bridge_status",
] as const;

export type CommandCode = (typeof COMMAND_CODES)[number];

const isFailureKind = (code: string): code is FailureKind => (FAILURE_KINDS as readonly string[]).includes(code);
const isCommandCode = (code: string): code is CommandCode => (COMMAND_CODES as readonly string[]).includes(code);

/** The status an `http_status` message names ("unexpected HTTP status 502"). */
function statusIn(raw: string | null): string | null {
  return raw?.match(/\b([1-5]\d\d)\b/)?.[1] ?? null;
}

/** A failure kind in words, or null for a kind this version does not know. */
export function failureText(kind: string | null, raw: string | null): string | null {
  if (kind === null || !isFailureKind(kind)) return null;
  if (kind === "http_status") {
    const status = statusIn(raw);
    return status ? t("errors.kind.http_status", { status }) : t("errors.serverError");
  }
  const key: MessageKey = `errors.kind.${kind}`;
  return t(key);
}

export interface Described {
  /** In the user's language when the code is known, else the raw message. */
  text: string;
  /** The engine's or the app's own words, when they say more than `text`. */
  detail: string | null;
}

/** For a failure with a kind and the engine's message. */
export function describeFailure(kind: string | null, raw: string | null): Described | null {
  const clean = raw ? sentence(raw) : null;
  const text = failureText(kind, raw);
  if (text) return { text, detail: clean };
  return clean ? { text: clean, detail: null } : null;
}

function isCoded(err: unknown): err is { code: string; message: string } {
  if (typeof err !== "object" || err === null) return false;
  const v = err as Record<string, unknown>;
  return typeof v.code === "string" && typeof v.message === "string";
}

/** A rejected command (or any thrown value) in words. */
export function describeError(err: unknown): Described {
  if (isCoded(err)) {
    const raw = err.message || null;
    if (isCommandCode(err.code)) return { text: t(`errors.command.${err.code}`), detail: raw && sentence(raw) };
    const failure = describeFailure(err.code, err.message);
    if (failure) return failure;
    return { text: raw ?? t("confirm.unknownError"), detail: null };
  }
  if (err instanceof Error && err.message) return { text: err.message, detail: null };
  if (typeof err === "string" && err) return { text: err, detail: null };
  return { text: t("confirm.unknownError"), detail: null };
}
