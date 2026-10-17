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
  /**
   * Waiting for the user to pick its name and folder: sent by the browser,
   * or a video page added here whose quality is still to be chosen.
   */
  awaiting_confirmation: boolean;
  /**
   * Added in this window (Add dialog, paste, drag, clipboard) rather than
   * sent by the browser. While it waits, the main window asks for it; the
   * confirmation window asks for the rest. Resets once it runs.
   */
  from_app: boolean;
  /** Set when the link is a stream (HLS or DASH) rather than a file. */
  media: MediaInfo | null;
  /** Set when the link is a video page (YouTube) downloaded with yt-dlp. */
  site: SiteInfo | null;
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

export interface SiteInfo {
  /** "YouTube". */
  site: string;
  /** Missing: the best quality is taken when it starts. */
  selection?: MediaSelection;
  /** "1080p", "Audio only" and the like, once known. */
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

/** The qualities to choose from, whether a stream's or a video page's. */
export interface ChoiceOptions {
  /** Best first; a page's "Audio only" entries last. */
  videos: VideoOption[];
  audios: AudioOption[];
  duration_ms?: number;
}

export interface MediaOptions extends ChoiceOptions {
  kind: MediaKind;
}

/** What a stream or a video page offers, for the quality chooser. */
export interface Choices {
  options: ChoiceOptions;
  selection: MediaSelection | null;
  /** FFmpeg can merge separate video and audio into one file. */
  ffmpeg: boolean;
}

/** What a stream offers, for the quality chooser. */
export interface MediaChoices extends Choices {
  options: MediaOptions;
}

/** What a video page offers, read by yt-dlp. */
export interface SiteChoices extends Choices {
  site: string;
  /** The page's title, offered as the file name. */
  title: string | null;
}

export type ToolSource = "settings" | "managed" | "path";

export interface ToolStatus {
  path: string | null;
  source: ToolSource | null;
  version: string | null;
  /** Found but not usable, or a set path that does not exist. */
  problem: string | null;
}

/** yt-dlp, its JavaScript runtime and FFmpeg as the settings find them. */
export interface ToolsStatus {
  yt_dlp: ToolStatus;
  /** Deno, or Node when no Deno is found (`js_kind`). */
  js: ToolStatus;
  js_kind: "deno" | "node" | null;
  ffmpeg: ToolStatus;
  /** Where the download button puts yt-dlp and Deno. */
  managed_dir: string | null;
}

export type SiteTool = "yt_dlp" | "deno";

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
  /** yt-dlp for video pages. Empty: Downpipe's tools folder, then PATH. */
  yt_dlp_path: string;
  /** Deno for yt-dlp. Empty: the tools folder, then PATH, then Node 22+. */
  deno_path: string;
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
  /** Hold it in the confirmation dialog (a video page's quality) before it starts. */
  confirm?: boolean;
  /** Added in the main window, which then asks itself (`Task.from_app`). */
  from_app?: boolean;
}

/** What the user chose for a download from the browser. */
export interface ConfirmOptions {
  file_name: string | null;
  /** Null keeps the automatic folder. */
  dir: string | null;
  queue: string | null;
  /** False: keep it paused ("Download later"). */
  start: boolean;
  /** Streams and video pages: the quality and audio to download. */
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
  /** Asks yt-dlp what a video page offers; downloads no video. */
  inspectSite: (id: string) => invoke<SiteChoices>("inspect_site", { id }),
  siteTools: () => invoke<ToolsStatus>("site_tools"),
  /** Downloads yt-dlp and/or Deno from their official releases, checked by SHA-256. */
  installSiteTools: (tools: SiteTool[]) => invoke<ToolsStatus>("install_site_tools", { tools }),
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
  /** The confirmation window drew a question and may be shown. */
  confirmWindowDrawn: () => invoke<null>("confirm_window_drawn"),
  /** The confirmation window's content needs this height, in CSS pixels. */
  fitConfirmWindow: (height: number) => invoke<null>("fit_confirm_window", { height }),
  /** From the confirmation window: the main window opens on Settings, Media. */
  openMediaSettings: () => invoke<null>("open_media_settings"),
  /** The main window's Settings closed; a question waiting for them returns. */
  settingsClosed: () => invoke<null>("settings_closed"),
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

/** To the main window: open Settings, Media (asked from the confirmation window). */
export function onMediaSettings(cb: () => void): Promise<UnlistenFn> {
  return listen("downpipe://media-settings", () => cb());
}

/** To the confirmation window: its close button was clicked. */
export function onConfirmClose(cb: () => void): Promise<UnlistenFn> {
  return listen("downpipe://confirm-close", () => cb());
}

/** To the confirmation window: back from Settings, read the download again. */
export function onConfirmAgain(cb: () => void): Promise<UnlistenFn> {
  return listen("downpipe://confirm-again", () => cb());
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
