// Typed bridge to the Rust side (src-tauri/src/main.rs). Field names mirror
// the serde output of downpipe-core, hence snake_case.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type TaskState =
  | "queued"
  | "starting"
  | "downloading"
  | "waiting"
  | "verifying"
  | "paused"
  | "completed"
  | "failed"
  | "canceled";

export interface SegmentView {
  start: number;
  end: number;
  written: number;
  active: boolean;
}

export interface Task {
  id: string;
  url: string;
  dir: string;
  file_name: string | null;
  path: string | null;
  state: TaskState;
  total: number | null;
  downloaded: number;
  error: string | null;
  error_kind: string | null;
  checksum: string | null;
  max_connections: number | null;
  created_at: number;
  updated_at: number;
  completed_at: number | null;
  speed: number;
  connections: number;
  target_connections: number;
  resumable: boolean;
  segments: SegmentView[];
  notices: string[];
  eta_secs: number | null;
}

export interface Settings {
  download_dir: string;
  max_concurrent_downloads: number;
  max_connections_per_download: number;
  /** Bytes per second; 0 means unlimited. */
  speed_limit: number;
}

export interface AddOptions {
  url: string;
  dir?: string | null;
  file_name?: string | null;
  checksum?: string | null;
  max_connections?: number | null;
  paused?: boolean;
}

export type ManagerEvent =
  | { type: "updated"; task: Task }
  | { type: "removed"; id: string }
  | { type: "settings"; settings: Settings };

export interface CommandError {
  code: string;
  message: string;
  id: string | null;
}

export const api = {
  list: () => invoke<Task[]>("list_downloads"),
  add: (options: AddOptions) => invoke<Task>("add_download", { options }),
  pause: (id: string) => invoke<null>("pause_download", { id }),
  resume: (id: string) => invoke<null>("resume_download", { id }),
  cancel: (id: string) => invoke<null>("cancel_download", { id }),
  remove: (id: string, deleteFiles: boolean) => invoke<null>("remove_download", { id, deleteFiles }),
  pauseAll: () => invoke<null>("pause_all"),
  resumeAll: () => invoke<null>("resume_all"),
  clearFinished: () => invoke<null>("clear_finished"),
  settings: () => invoke<Settings>("get_settings"),
  updateSettings: (settings: Settings) => invoke<Settings>("update_settings", { settings }),
  pickFolder: (initial: string | null) => invoke<string | null>("pick_folder", { initial }),
  reveal: (id: string) => invoke<null>("reveal_download", { id }),
  open: (id: string) => invoke<null>("open_download", { id }),
};

export function onManagerEvent(cb: (event: ManagerEvent) => void): Promise<UnlistenFn> {
  return listen<ManagerEvent>("downpipe://event", (e) => cb(e.payload));
}

/** The event stream fell behind; the full list should be reloaded. */
export function onResync(cb: () => void): Promise<UnlistenFn> {
  return listen("downpipe://resync", () => cb());
}

export function isCommandError(err: unknown): err is CommandError {
  return typeof err === "object" && err !== null && "code" in err && "message" in err;
}

export function errorMessage(err: unknown): string {
  if (isCommandError(err)) return err.message;
  if (err instanceof Error) return err.message;
  if (typeof err === "string") return err;
  return "Something went wrong";
}

/** False when the UI is opened in a plain browser instead of the app. */
export function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}
