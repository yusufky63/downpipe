// Typed bridge to the Rust side (src-tauri/src/main.rs). Field names mirror
// the serde output of downpipe-core, hence snake_case.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import { describeError } from "./errors";

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

/** Why a running download is waiting (downpipe_http::WaitReason). */
export type WaitReason = "no_data" | "network" | "throttled" | "server";

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
  /** Why it waits, while its state is "waiting". */
  waiting: WaitReason | null;
  /**
   * Waiting for the user to pick its name and folder: sent by the browser,
   * or a video page added here whose quality is still to be chosen.
   */
  awaiting_confirmation: boolean;
  /** Held because its link is already in the list: that download's id. */
  duplicate_of: string | null;
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
  /** Pattern of the site rule it was added or last started with. */
  site_rule: string | null;
  /** Bytes per second of its own, under the global limit; null: only the global one. */
  speed_limit: number | null;
  /** "Pause until": Unix seconds when the core queues it again. */
  resume_at: number | null;
  /** Set for BitTorrent downloads (a magnet link or a .torrent file). */
  torrent: TorrentInfo | null;
  /** BitTorrent: peers and uploads while it downloads or seeds. */
  swarm: Swarm | null;
  /** Stream downloads while they run: fragments done and in all. */
  fragments: { done: number; total: number } | null;
  eta_secs: number | null;
  /** Its own command and extraction choice; empty values follow the queue, category and settings. */
  post_actions: PostActions;
  /** What the virus scan, the extraction and the command found, once it completed. */
  post: PostReport | null;
  /** Completed, and a post-download step runs now. */
  post_step: PostStep | null;
}

export interface PostActions {
  /** Command line run when it completes. Empty: the queue's, else the one in Settings. */
  command: string;
  /** Extract it if it is an archive; null lets its category decide. */
  extract: boolean | null;
}

export type PostStep = "scanning" | "extracting" | "running_command";

export type ScanResult = "clean" | "threat" | "missing" | "failed";

export interface ScanReport {
  result: ScanResult;
  /** "Microsoft Defender", or the scanner program's file name. */
  scanner: string;
  /** The threat's name, or why the scan failed. */
  detail?: string;
}

export interface ExtractReport {
  result: "done" | "failed";
  folder?: string;
  files: number;
  bytes: number;
  /** Entries left out: unsafe names, links, special files, duplicates. */
  skipped: number;
  archive_deleted: boolean;
  error?: "encrypted" | "too_large" | "too_many_entries" | "corrupt" | "io";
  detail?: string;
}

export interface CommandReport {
  result: "ok" | "failed" | "timed_out" | "not_started" | "invalid";
  program: string;
  exit_code?: number;
  /** The end of what it printed. */
  output: string;
  detail?: string;
}

export interface PostReport {
  scan?: ScanReport;
  extract?: ExtractReport;
  command?: CommandReport;
  /** A threat was found, so nothing was extracted and the command did not run. */
  stopped_by_threat: boolean;
}

/** Settings for what happens after downloads complete. */
export interface PostSettings {
  /** Run after every completed download whose queue or own choice sets none. */
  command: string;
  /** Seconds a command or a scan may take, 5 to 86400. */
  timeout_secs: number;
  scan: boolean;
  /** Scanner command line; empty is Microsoft Defender on Windows. */
  scanner: string;
  /** Delete an archive once everything in it was extracted. */
  delete_archive: boolean;
}

export type MediaKind = "hls" | "dash";

/** What a torrent download keeps (downpipe_core::TorrentInfo). */
export interface TorrentInfo {
  /** BitTorrent v1 info hash, lower-case hex; empty for a .torrent link not fetched yet. */
  info_hash: string;
  /** The torrent's name: the magnet link's until the metadata is known. */
  name?: string;
  /** Files in the torrent; 0 while the metadata is not known. */
  files: number;
  /** Indices of the files to download; missing for all. */
  only_files?: number[];
  /** Bytes uploaded to peers in all. */
  uploaded: number;
  private: boolean;
}

export interface Swarm {
  /** Peers connected right now. */
  peers: number;
  /** Peers heard of, connected or not. */
  known_peers: number;
  upload_speed: number;
  /** Finished and still uploading. */
  seeding: boolean;
  /** Asking peers for a magnet link's file list. */
  fetching_metadata: boolean;
}

export interface TorrentFileChoice {
  index: number;
  /** Inside the torrent, "/"-separated. */
  path: string;
  size: number;
}

/** A torrent's files, for choosing which to download. */
export interface TorrentChoices {
  info_hash: string;
  name: string;
  files: TorrentFileChoice[];
  total: number;
  /** Chosen so far; null for all. */
  selection: number[] | null;
  /** The files go into a folder named after the torrent. */
  folder: boolean;
  private: boolean;
}

export interface TorrentSettings {
  /** Keep uploading finished torrents until a limit is reached. */
  seed: boolean;
  /** 100 = ratio 1.0; 0 for no ratio limit. */
  seed_ratio_percent: number;
  /** 0 for no time limit. */
  seed_minutes: number;
  /** Bytes per second; 0 means unlimited. */
  upload_limit: number;
  /** 0 picks a free port at every start. */
  listen_port: number;
  dht: boolean;
  lsd: boolean;
  upnp: boolean;
}

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
  /** Missing: the best quality (or the preset) is taken when it starts. */
  selection?: MediaSelection;
  /** "1080p", "Audio only" and the like, once known. */
  quality?: string;
  /** A playlist's one quality for its videos, until their formats are known. */
  preset?: Preset;
  subtitles?: SubtitleRef[];
  /** The title from the playlist, shown until the file is named. */
  title?: string;
  /** Position in the playlist it was added from, and that playlist's size. */
  index?: number;
  count?: number;
}

/** A quality for every video of a playlist. */
export type Preset = "best" | "1080p" | "720p" | "audio";

/** A subtitle to save: the uploader's, or the site's automatic captions (`auto`). */
export interface SubtitleRef {
  language: string;
  auto: boolean;
}

/** A subtitle language a video page offers. */
export interface SubtitleOption {
  /** The site's code: "en", "pt-BR", "en-orig" (the original of automatic captions). */
  language: string;
  /** The site's name for it, in English. */
  name?: string | null;
  auto: boolean;
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
  title: string | null;
  /** The file name the media name template makes, offered in the dialog. */
  name: string | null;
  /** The uploader's subtitles first, then automatic captions. */
  subtitles: SubtitleOption[];
  chosen_subtitles: SubtitleRef[];
}

/** One video of a playlist or channel. */
export interface ListEntry {
  id: string;
  url: string;
  title: string | null;
  channel: string | null;
  duration_ms: number | null;
  /** Position in the list, 1 first. */
  index: number;
  /** False for private, deleted, members-only and live videos. */
  available: boolean;
}

/** A playlist's or channel's videos, read by yt-dlp from the list pages. */
export interface ListChoices {
  site: string;
  kind: "playlist" | "channel";
  /** The list's own page. */
  url: string;
  title: string | null;
  channel: string | null;
  entries: ListEntry[];
  /** The list has more videos than were read. */
  truncated: boolean;
  /** Where the list's folder goes unless another folder is chosen. */
  dir: string;
  ffmpeg: boolean;
}

export interface AddListOptions {
  url: string;
  /** Names the folder the videos go into. */
  title: string | null;
  entries: { id: string; title: string | null; index: number }[];
  /** Videos in the whole list, for {index}'s zero padding. */
  count: number;
  preset: Preset;
  subtitles: SubtitleRef[];
  /** Null: the video category's folder. */
  dir: string | null;
  queue: string | null;
  paused: boolean;
}

export interface AddListResult {
  added: Task[];
  /** Videos already in the list, left as they were. */
  skipped: number;
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
  /** Where the download buttons put the tools. */
  managed_dir: string | null;
  /** About how many bytes each download is; null where Downpipe has no build for this system (FFmpeg on macOS). */
  sizes: Record<SiteTool, number | null>;
  /** Seconds since the epoch of the last look for a newer yt-dlp; null before the first. */
  yt_dlp_checked: number | null;
}

export type SiteTool = "yt_dlp" | "deno" | "ffmpeg";

/** How far a tool download has come; every window hears it. */
export interface ToolsProgress {
  tool: SiteTool;
  done: number;
  total: number | null;
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
  /** Interface language code such as "tr" or "pt-BR"; empty follows the system. */
  language: string;
  /** The first is always the main queue, whose limit is `max_concurrent_downloads`. */
  queues: Queue[];
  /** FFmpeg for merging the video and sound of streams. Empty: `ffmpeg` on PATH. */
  ffmpeg_path: string;
  /** yt-dlp for video pages. Empty: Downpipe's tools folder, then PATH. */
  yt_dlp_path: string;
  /** Deno for yt-dlp. Empty: the tools folder, then PATH, then Node 22+. */
  deno_path: string;
  /** Once a day, when a video download starts, replace the yt-dlp Downpipe downloaded with a newer one. */
  yt_dlp_auto_update: boolean;
  /** Names site downloads: {title}, {channel}, {date}, {index}, {id}. */
  media_name_template: string;
  /** Per-site overrides; the most specific matching pattern applies. */
  site_rules: SiteRule[];
  /** The remembered answer to "this link is already in the list"; "ask" asks every time. */
  duplicate_answer: DuplicateAnswer;
  /** The remote control; changed only with `api.setRemote` and `api.newRemoteToken`. */
  remote: RemoteSettings;
  /** After a download completes: the default command, the virus scan, archives. */
  post: PostSettings;
  torrent: TorrentSettings;
}

export interface RemoteSettings {
  enabled: boolean;
  /** IP address it listens on: "127.0.0.1" unless the user picks a network address. */
  bind: string;
  port: number;
  /** SHA-256 of the access token; the token itself is never stored. */
  token_hash: string;
}

/** The remote control as it runs now (src-tauri/src/remote.rs). */
export interface RemoteStatus {
  enabled: boolean;
  bind: string;
  port: number;
  has_token: boolean;
  running: boolean;
  /** Why it is not running although switched on. */
  error: "port_in_use" | "bind_failed" | null;
  /** The page's address, for the browser and the QR code. */
  url: string | null;
  /** This computer's address on the local network. */
  lan_address: string | null;
}

export interface RemoteUpdate {
  status: RemoteStatus;
  /** A token just made: the only time it is ever shown. */
  token: string | null;
}

export type RuleProxy = "inherit" | "direct" | "custom";

/** downpipe_core::SiteRule. Empty strings and nulls keep the global setting. */
export interface SiteRule {
  /** Empty for a new rule; the app makes one. */
  id: string;
  /** `example.com` (that host) or `*.example.com` (it and every host under it). */
  pattern: string;
  enabled: boolean;
  max_connections: number | null;
  user_agent: string;
  referer: string;
  proxy: RuleProxy;
  /** The proxy when `proxy` is "custom". */
  proxy_url: string;
  /** Bytes per second, shared by the downloads the rule applies to. */
  speed_limit: number | null;
  max_retries: number | null;
  /** New downloads; relative to the download folder unless absolute. */
  download_dir: string;
  /** Queue id for new downloads. */
  queue: string;
}

/** Stable codes from downpipe_core::rules, translated in the settings dialog. */
export type RuleProblemCode =
  | "pattern_empty"
  | "pattern_url"
  | "pattern_wildcard"
  | "pattern_too_broad"
  | "pattern_port"
  | "pattern_invalid"
  | "pattern_duplicate"
  | "user_agent_invalid"
  | "referer_invalid"
  | "proxy_missing"
  | "proxy_invalid";

export interface RuleProblem {
  /** The rule's id. */
  id: string;
  field: "pattern" | "user_agent" | "referer" | "proxy_url";
  code: RuleProblemCode;
}

export interface RuleTest {
  /** Null when the text is not an address. */
  host: string | null;
  /** Id of the rule that applies. */
  rule: string | null;
  /** Ids of rules that match but are switched off. */
  disabled: string[];
}

export interface SiteRulesCheck {
  problems: RuleProblem[];
  test: RuleTest | null;
}

/** A host that refused extra connections this session, not yet held by a rule. */
export interface RefusedHost {
  host: string;
  connections: number;
}

/** "ask", or one of the duplicate question's answers given with "Don't ask again". */
export type DuplicateAnswer = "ask" | "copy" | "overwrite" | "existing";

export const MAIN_QUEUE = "main";

export interface Queue {
  /** Empty for a new queue; the app makes one from the name. */
  id: string;
  name: string;
  /** Downloads at once, 1 to 16. Ignored for the main queue. */
  max_concurrent: number;
  /** Run after each of its downloads completes, instead of the one in Settings. */
  command: string;
}

export interface Category {
  name: string;
  /** Lower case, no dots. */
  extensions: string[];
  /** Empty: a folder named after the category inside the download folder. */
  folder: string;
  /** Extract its archives (zip, tar, 7z) after they download. */
  extract: boolean;
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
  /** A link already in the list: refuse (the app then asks), add a copy, or ask in the confirmation window. */
  on_duplicate?: OnDuplicate;
  /** Sent with every request; kept in memory only, never written to disk. */
  headers?: [string, string][];
  /** Its own command and extraction choice. */
  post?: PostActions;
}

export type OnDuplicate = "refuse" | "add_copy" | "ask";

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
  /** Replaces its own command and extraction choice. */
  post?: PostActions | null;
  /** Video pages: the subtitles to save; null keeps the choice. */
  subtitles?: SubtitleRef[] | null;
  /** Torrents: indices of the files to download. */
  files?: number[] | null;
}

export type QueueMove = "up" | "down" | "top" | "bottom";

/** What the site grabber reads and collects (downpipe_crawl::GrabOptions). */
export interface GrabOptions {
  url: string;
  /** 1 to 5: 1 reads only the start page. */
  depth: number;
  /** Pages read at most, the start page included. */
  max_pages: number;
  /** Lower case, without dots. */
  extensions: string[];
  /** Follow only pages of the start page's site. */
  same_host: boolean;
  respect_robots: boolean;
  /** Wait between two requests to the same host. */
  delay_ms: number;
  /** Ask each file's size with a HEAD request. */
  check_sizes: boolean;
}

export interface FoundFile {
  url: string;
  name: string;
  extension: string;
  size: number | null;
  /** The page the link was first seen on. */
  found_on: string;
}

export type GrabPhase = "pages" | "sizes" | "done";

export interface GrabProgress {
  phase: GrabPhase;
  pages_done: number;
  pages_total: number;
  files: number;
  sized: number;
  to_size: number;
  current: string | null;
  /** Left out because robots.txt disallows them. */
  blocked: number;
  failed: number;
}

export interface GrabReport {
  files: FoundFile[];
  pages: number;
  blocked: number;
  failed: number;
  /** Stopped by the user; `files` holds what was found until then. */
  canceled: boolean;
  hit_page_limit: boolean;
}

export type GrabEvent =
  | { kind: "progress"; id: string; progress: GrabProgress }
  | { kind: "done"; id: string; report: GrabReport }
  | { kind: "error"; id: string; code: string; message: string };

export interface FeedFilter {
  /** Lower case, without dots. Empty: any file. */
  extensions: string[];
  /** A regular expression titles must contain, case ignored. Empty: any title. */
  title_pattern: string;
}

/** An RSS or Atom subscription (downpipe_core::feeds::Feed). */
export interface Feed {
  id: string;
  url: string;
  /** The feed's own title, from the last good check. */
  title: string;
  filter: FeedFilter;
  interval_minutes: number;
  queue: string;
  enabled: boolean;
  created_at: number;
  last_checked: number | null;
  last_error: string | null;
  last_added: number;
  added_total: number;
}

export interface FeedInput {
  url: string;
  filter: FeedFilter;
  interval_minutes: number;
  queue: string;
  enabled: boolean;
  /** When subscribing: also add the matching items the feed holds now. */
  download_existing: boolean;
}

export interface FeedCheck {
  items: number;
  /** Ids of the downloads added. */
  added: string[];
}

export type ManagerEvent =
  | { type: "updated"; task: Task }
  | { type: "removed"; id: string }
  | { type: "settings"; settings: Settings }
  /** The browser sent a playlist or channel to choose videos from. */
  | { type: "list_offered"; url: string };

export interface CommandError {
  code: string;
  message: string;
  id: string | null;
}

/** What a backup file holds, read and checked by Rust for the import preview. */
export interface BackupPreview {
  /** The file's name, without its folder. */
  file: string;
  /** The Downpipe that wrote it; empty when unknown. */
  app_version: string;
  /** Seconds since the epoch; 0 when unknown. */
  exported_at: number;
  downloads: number;
  /** Of `downloads`, the ones whose link is not in the list yet. */
  new_downloads: number;
  /** Damaged entries that will be left out. */
  invalid: number;
  /** Queues besides the main queue. */
  queues: number;
  categories: number;
  site_rules: number;
  /** Feed subscriptions in the file, and of those the ones not subscribed to yet. */
  feeds: number;
  new_feeds: number;
}

/** What to take from a backup. */
export interface ImportChoice {
  settings: boolean;
  downloads: boolean;
  feeds: boolean;
}

export interface ImportReport {
  settings: boolean;
  added: number;
  /** Links already in the list (or twice in the file), left alone. */
  duplicates: number;
  invalid: number;
  /** Feeds subscribed to, and feeds that could not be read now. */
  feeds: number;
  feeds_failed: number;
}

/** One download from IDM's list (downpipe_core::idm::IdmEntry). */
export interface IdmRow {
  url: string;
  file_name: string | null;
  /** The folder IDM saves to; exported lists have none. */
  dir: string | null;
  size: number | null;
  /** IDM downloaded every byte. */
  finished: boolean;
  /** The link is in Downpipe's list already. */
  listed: boolean;
}

export interface IdmList {
  /** The exported file's name; null for the list on this computer. */
  file: string | null;
  rows: IdmRow[];
}

export type IdmSource = "computer" | "file";

export const api = {
  list: () => invoke<Task[]>("list_downloads"),
  add: (options: AddOptions) => invoke<Task>("add_download", { options }),
  pause: (id: string) => invoke<null>("pause_download", { id }),
  resume: (id: string) => invoke<null>("resume_download", { id }),
  /** `at` in Unix seconds; the core queues it again then. */
  pauseUntil: (id: string, at: number) => invoke<null>("pause_download_until", { id, at }),
  /** Bytes per second; null removes the download's own limit. */
  setSpeedLimit: (id: string, limit: number | null) => invoke<null>("set_download_speed_limit", { id, limit }),
  cancel: (id: string) => invoke<null>("cancel_download", { id }),
  remove: (id: string, deleteFiles: boolean) => invoke<null>("remove_download", { id, deleteFiles }),
  pauseAll: () => invoke<null>("pause_all"),
  resumeAll: () => invoke<null>("resume_all"),
  clearFinished: () => invoke<null>("clear_finished"),
  settings: () => invoke<Settings>("get_settings"),
  /** The remote control part is ignored; it has commands of its own. */
  updateSettings: (settings: Omit<Settings, "remote">) => invoke<Settings>("update_settings", { settings }),
  /** Checks unsaved rules and tests an address against them, with the download code itself. */
  checkSiteRules: (rules: SiteRule[], address: string | null) =>
    invoke<SiteRulesCheck>("check_site_rules", { rules, address }),
  refusedHosts: () => invoke<RefusedHost[]>("refused_hosts"),
  remoteStatus: () => invoke<RemoteStatus>("remote_status"),
  /** Switching it on the first time makes the token, returned once. */
  setRemote: (enabled: boolean, bind: string, port: number) => invoke<RemoteUpdate>("set_remote", { enabled, bind, port }),
  /** A new token; the old one stops working at once. */
  newRemoteToken: () => invoke<RemoteUpdate>("new_remote_token"),
  bridgeStatus: () => invoke<BridgeRegistration>("browser_bridge_status"),
  registerBridge: () => invoke<BridgeRegistration>("register_browser_bridge"),
  pickFolder: (initial: string | null) => invoke<string | null>("pick_folder", { initial }),
  reveal: (id: string) => invoke<null>("reveal_download", { id }),
  open: (id: string) => invoke<null>("open_download", { id }),
  /** The folder its archive was extracted into. */
  openExtracted: (id: string) => invoke<null>("open_extracted", { id }),
  setPostActions: (id: string, actions: PostActions) => invoke<null>("set_post_actions", { id, actions }),
  changeUrl: (id: string, url: string) => invoke<null>("change_download_url", { id, url }),
  confirm: (id: string, options: ConfirmOptions) => invoke<null>("confirm_download", { id, options }),
  /** Reads the stream's manifest; downloads no media. */
  inspectMedia: (id: string) => invoke<MediaChoices>("inspect_media", { id }),
  /** The FFmpeg streams would use now, or null when there is none. */
  ffmpegStatus: () => invoke<string | null>("ffmpeg_status"),
  /** Asks yt-dlp what a video page offers; downloads no video. */
  inspectSite: (id: string) => invoke<SiteChoices>("inspect_site", { id }),
  /** Asks yt-dlp for a playlist's or channel's videos; downloads no video. */
  inspectList: (url: string) => invoke<ListChoices>("inspect_list", { url }),
  addList: (options: AddListOptions) => invoke<AddListResult>("add_list", { options }),
  siteTools: () => invoke<ToolsStatus>("site_tools"),
  /** A torrent's files; a magnet link asks peers for them, which can take up to two minutes. */
  inspectTorrent: (id: string) => invoke<TorrentChoices>("inspect_torrent", { id }),
  stopSeeding: (id: string) => invoke<null>("stop_seeding", { id }),
  /** A .torrent file dropped on the window, as its bytes. */
  addTorrentFile: (bytes: Uint8Array, options: AddOptions) =>
    invoke<Task>("add_torrent_file", { bytes: Array.from(bytes), options }),
  /** Picks a .torrent file in the system dialog and adds it; null when the dialog was closed. */
  openTorrentFile: (options: AddOptions) => invoke<Task | null>("open_torrent_file", { options }),
  /** Downloads yt-dlp and/or Deno from their official releases, checked by SHA-256. */
  /** `onlyMissing` skips tools found by the time the install starts (another window got them). */
  installSiteTools: (tools: SiteTool[], onlyMissing = false) => invoke<ToolsStatus>("install_site_tools", { tools, onlyMissing }),
  /** With `replace`, the old file is deleted first and the new one takes its name. */
  redownload: (id: string, replace: boolean) => invoke<null>("redownload", { id, replace }),
  move: (ids: string[], to: QueueMove) => invoke<null>("move_downloads", { ids, to }),
  setQueue: (ids: string[], queue: string) => invoke<null>("set_download_queue", { ids, queue }),
  startQueue: (queue: string) => invoke<null>("start_queue", { queue }),
  stopQueue: (queue: string) => invoke<null>("stop_queue", { queue }),
  /** Number of links written, or null when the save dialog was closed. */
  exportLinks: () => invoke<number | null>("export_downloads"),
  /** Settings and list to a JSON file; null when the save dialog was closed. */
  exportBackup: () => invoke<{ file: string; downloads: number } | null>("export_backup"),
  /** Pick and check a backup; null when the dialog was closed. Rust keeps it for `importBackup`. */
  inspectBackup: () => invoke<BackupPreview | null>("inspect_backup"),
  importBackup: (choice: ImportChoice) => invoke<ImportReport>("import_backup", { choice }),
  /** IDM's list, read only; null when the file dialog was closed. Rust keeps it for `importIdm`. */
  readIdmList: (source: IdmSource) => invoke<IdmList | null>("read_idm_list", { source }),
  /** Adds the rows at these positions of the last list read, paused. */
  importIdm: (rows: number[]) => invoke<ImportReport>("import_idm", { rows }),
  startDrag: (id: string) => invoke<null>("start_drag", { id }),
  whenDone: () => invoke<WhenDoneView>("get_when_done"),
  setWhenDone: (action: WhenDone) => invoke<WhenDoneView>("set_when_done", { action }),
  whenDoneNow: () => invoke<null>("when_done_now"),
  /** The confirmation window drew a question and may be shown. */
  confirmWindowDrawn: () => invoke<null>("confirm_window_drawn"),
  /** This dialog window's content (confirmation or progress) needs this height, in CSS pixels at this `devicePixelRatio`. */
  fitWindow: (height: number) => invoke<null>("fit_window", { height, scale: window.devicePixelRatio }),
  /** After "Start download" in the confirmation window: follow it in a progress window of its own. */
  watchDownload: (id: string) => invoke<null>("watch_download", { id }),
  /** This progress window drew its download and may be shown. */
  progressWindowDrawn: () => invoke<null>("progress_window_drawn"),
  setProgressTitle: (title: string) => invoke<null>("set_progress_title", { title }),
  /** Close this progress window; the download goes on. */
  closeProgressWindow: () => invoke<null>("close_progress_window"),
  /** From the confirmation window: the main window opens on Settings, Media. */
  openMediaSettings: () => invoke<null>("open_media_settings"),
  /** The main window's Settings closed; a question waiting for them returns. */
  settingsClosed: () => invoke<null>("settings_closed"),
  /** Starts the site grabber; its progress and result arrive as `downpipe://grab` events with this id. */
  startGrab: (options: GrabOptions) => invoke<string>("start_grab", { options }),
  /** Stops a grab; it still reports what it found until then. */
  cancelGrab: (id: string) => invoke<null>("cancel_grab", { id }),
  feeds: () => invoke<Feed[]>("list_feeds"),
  /** Reads the feed once; refuses addresses that are not feeds. */
  addFeed: (input: FeedInput) => invoke<{ feed: Feed; check: FeedCheck }>("add_feed", { input }),
  updateFeed: (id: string, input: FeedInput) => invoke<Feed>("update_feed", { id, input }),
  removeFeed: (id: string) => invoke<null>("remove_feed", { id }),
  checkFeed: (id: string) => invoke<FeedCheck>("check_feed", { id }),
};

export function onGrabEvent(cb: (event: GrabEvent) => void): Promise<UnlistenFn> {
  return listen<GrabEvent>("downpipe://grab", (e) => cb(e.payload));
}

/** The whole list of feeds after any change, also from a check on its interval. */
export function onFeeds(cb: (feeds: Feed[]) => void): Promise<UnlistenFn> {
  return listen<Feed[]>("downpipe://feeds", (e) => cb(e.payload));
}

export function onWhenDone(cb: (view: WhenDoneView) => void): Promise<UnlistenFn> {
  return listen<WhenDoneView>("downpipe://when-done", (e) => cb(e.payload));
}

export function onToolsProgress(cb: (progress: ToolsProgress) => void): Promise<UnlistenFn> {
  return listen<ToolsProgress>("downpipe://tools-progress", (e) => cb(e.payload));
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

/** A rejected command in the user's language; lib/errors.ts has the details. */
export function errorMessage(err: unknown): string {
  return describeError(err).text;
}

/** False when the UI is opened in a plain browser instead of the app. */
export function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}
