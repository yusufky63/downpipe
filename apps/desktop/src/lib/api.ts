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
  /** Category the file was sorted into. */
  category: string | null;
  /** Downpipe picked the folder (it may move the file into its category). */
  auto_dir: boolean;
  /** Queue id; see `Settings.queues`. */
  queue: string;
  /** Order in the queue: lower starts first. */
  position: number;
  created_at: number;
  updated_at: number;
  completed_at: number | null;
  speed: number;
  connections: number;
  target_connections: number;
  resumable: boolean;
  segments: SegmentView[];
  notices: string[];
  /** Sent by the browser and waiting for the user to pick its name and folder. */
  awaiting_confirmation: boolean;
  /** Set when the link is a stream (HLS or DASH) rather than a file. */
  media: MediaInfo | null;
  /** Stream downloads while they run: fragments done and in all. */
  fragments: { done: number; total: number } | null;
  eta_secs: number | null;
}

export type MediaKind = "hls" | "dash";

/** A track picked by the user, with enough detail to find it again. */
export interface TrackRef {
  id: string;
  height?: number;
  bandwidth?: number;
  language?: string;
}

export interface MediaSelection {
  video?: TrackRef;
  audio?: TrackRef;
}

export interface MediaInfo {
  kind: MediaKind;
  /** Missing: the best quality is taken when it starts. */
  selection?: MediaSelection;
  /** "1080p" and the like, once the stream was read. */
  quality?: string;
}

export interface VideoOption {
  id: string;
  width?: number;
  height?: number;
  /** Bits per second, as announced. */
  bandwidth?: number;
  codecs?: string;
  frame_rate?: string;
  /** The video has its own sound; no audio track goes with it. */
  has_audio: boolean;
  /** HLS: the audio tracks that go with it (`AudioOption.group`). */
  audio_group?: string;
  audio_only: boolean;
  estimated_size?: number;
}

export interface AudioOption {
  id: string;
  name?: string;
  language?: string;
  bandwidth?: number;
  codecs?: string;
  default: boolean;
  group?: string;
}

export interface MediaOptions {
  kind: MediaKind;
  /** Best first. */
  videos: VideoOption[];
  audios: AudioOption[];
  duration_ms?: number;
}

/** What a stream offers, for the quality chooser. */
export interface MediaChoices {
  options: MediaOptions;
  selection: MediaSelection | null;
  /** FFmpeg can merge separate video and audio into one file. */
  ffmpeg: boolean;
}

export interface Settings {
  download_dir: string;
  max_concurrent_downloads: number;
  max_connections_per_download: number;
  /** Bytes per second; 0 means unlimited. */
  speed_limit: number;
  /** Re-register the browser bridge for this user at every start. */
  register_browser_host: boolean;
  notify_on_finish: boolean;
  close_to_tray: boolean;
  start_with_system: boolean;
  /** Empty: the system proxy. */
  proxy: string;
  /** Empty: Downpipe's own. */
  user_agent: string;
  use_categories: boolean;
  categories: Category[];
  schedule: Schedule;
  /** Offer to download links copied to the clipboard. */
  watch_clipboard: boolean;
  /** Keep the computer from sleeping while downloads run. */
  keep_awake: boolean;
  /** Ask for name, folder and queue before a download from the browser starts. */
  confirm_browser_downloads: boolean;
  /** The first is always the main queue, whose limit is `max_concurrent_downloads`. */
  queues: Queue[];
  /** FFmpeg for merging the video and sound of streams. Empty: `ffmpeg` on PATH. */
  ffmpeg_path: string;
}

export const MAIN_QUEUE = "main";

export interface Queue {
  /** Empty for a new queue; the app makes one from the name. */
  id: string;
  name: string;
  /** Downloads at once, 1 to 16. Ignored for the main queue. */
  max_concurrent: number;
}

export interface Category {
  name: string;
  /** Lower case, no dots. */
  extensions: string[];
  /** Empty: a folder named after the category inside the download folder. */
  folder: string;
}

export interface Schedule {
  enabled: boolean;
  /** HH:MM */
  start: string;
  /** HH:MM or empty */
  stop: string;
  /** Monday first. */
  days: boolean[];
  /** Queue id to start and stop; empty for every queue. */
  queue: string;
}

export type WhenDone = "nothing" | "quit" | "sleep" | "shutdown";

export interface WhenDoneView {
  action: WhenDone;
  /** Set while counting down to the action. */
  seconds_left: number | null;
}

export type BridgeBrowser = "chrome" | "edge" | "chromium" | "firefox";

export type BridgeState =
  | "registered"
  | "not_installed"
  | "missing"
  | "invalid"
  | "other_host"
  | "extension_not_allowed"
  | "failed";

export interface BridgeRegistration {
  /** The native host this copy of the app ships. */
  host: string;
  host_found: boolean;
  browsers: { browser: BridgeBrowser; state: BridgeState; detail: string | null }[];
}

export interface AddOptions {
  url: string;
  dir?: string | null;
  file_name?: string | null;
  checksum?: string | null;
  max_connections?: number | null;
  paused?: boolean;
  queue?: string | null;
}

/** What the user chose for a download from the browser. */
export interface ConfirmOptions {
  file_name: string | null;
  /** Null keeps the automatic folder. */
  dir: string | null;
  queue: string | null;
  /** False: keep it paused ("Download later"). */
  start: boolean;
  /** Streams: the quality and audio to download. */
  media?: MediaSelection | null;
}

export type QueueMove = "up" | "down" | "top" | "bottom";

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
  bridgeStatus: () => invoke<BridgeRegistration>("browser_bridge_status"),
  registerBridge: () => invoke<BridgeRegistration>("register_browser_bridge"),
  pickFolder: (initial: string | null) => invoke<string | null>("pick_folder", { initial }),
  reveal: (id: string) => invoke<null>("reveal_download", { id }),
  open: (id: string) => invoke<null>("open_download", { id }),
  changeUrl: (id: string, url: string) => invoke<null>("change_download_url", { id, url }),
  confirm: (id: string, options: ConfirmOptions) => invoke<null>("confirm_download", { id, options }),
  /** Reads the stream's manifest; downloads no media. */
  inspectMedia: (id: string) => invoke<MediaChoices>("inspect_media", { id }),
  /** The FFmpeg streams would use now, or null when there is none. */
  ffmpegStatus: () => invoke<string | null>("ffmpeg_status"),
  /** With `replace`, the old file is deleted first and the new one takes its name. */
  redownload: (id: string, replace: boolean) => invoke<null>("redownload", { id, replace }),
  move: (ids: string[], to: QueueMove) => invoke<null>("move_downloads", { ids, to }),
  setQueue: (ids: string[], queue: string) => invoke<null>("set_download_queue", { ids, queue }),
  startQueue: (queue: string) => invoke<null>("start_queue", { queue }),
  stopQueue: (queue: string) => invoke<null>("stop_queue", { queue }),
  /** Number of links written, or null when the save dialog was closed. */
  exportLinks: () => invoke<number | null>("export_downloads"),
  startDrag: (id: string) => invoke<null>("start_drag", { id }),
  whenDone: () => invoke<WhenDoneView>("get_when_done"),
  setWhenDone: (action: WhenDone) => invoke<WhenDoneView>("set_when_done", { action }),
  whenDoneNow: () => invoke<null>("when_done_now"),
};

export function onWhenDone(cb: (view: WhenDoneView) => void): Promise<UnlistenFn> {
  return listen<WhenDoneView>("downpipe://when-done", (e) => cb(e.payload));
}

export function onManagerEvent(cb: (event: ManagerEvent) => void): Promise<UnlistenFn> {
  return listen<ManagerEvent>("downpipe://event", (e) => cb(e.payload));
}

/** A download link was copied to the clipboard (when watching is on). */
export function onClipboardLink(cb: (url: string) => void): Promise<UnlistenFn> {
  return listen<{ url: string }>("downpipe://clipboard", (e) => cb(e.payload.url));
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
