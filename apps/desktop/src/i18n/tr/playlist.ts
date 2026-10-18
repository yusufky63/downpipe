// The dialog that lists a playlist's or channel's videos to choose from.
import type { Messages } from "../core";

export const playlist: Messages["playlist"] = {
  eyebrow: {
    playlist: "{site} oynatma listesi",
    channel: "{site} kanalı",
  },
  untitled: "Oynatma listesi",
  reading: "Videolar yt-dlp'ye soruluyor…",
  unavailable: "Bu oynatma listesi ya da kanal kullanılamıyor: gizli, kaldırılmış ya da bölgenizde engellenmiş.",
  empty: "yt-dlp bu listede video bulamadı.",
  truncated: { one: "Yalnızca ilk {count} video listeleniyor.", other: "Yalnızca ilk {count} video listeleniyor." },
  filter: "Başlığa göre süz",
  selectAll: "Tümünü seç",
  selectNone: "Seçimi kaldır",
  selected: { one: "{count} / {total} seçili", other: "{count} / {total} seçili" },
  rowsLabel: "Videolar",
  notAvailable: "Kullanılamıyor",
  untitledVideo: "Başlıksız video",
  quality: "Kalite",
  qualityHint: "Tüm videolar için tek kalite; bu kalitesi olmayan video bir alttakini alır.",
  presets: {
    best: "En iyi",
    "1080p": "En fazla 1080p",
    "720p": "En fazla 720p",
    audio: "Yalnızca ses",
  },
  autoCaptions: "Altyazı olmayan videolarda otomatik altyazıları kullan",
  noFfmpeg: "FFmpeg bulunamadı: yalnızca kendi sesi olan kaliteler indirilebilir; bu, YouTube'da en fazla 360p demektir.",
  folder: "Klasör",
  folderHint: "Yanındaki klasörün içinde açılır. Boş bırakılırsa ayrı klasör açılmaz.",
  saveIn: "Kayıt yeri",
  submit: { one: "{count} videoyu indir", other: "{count} videoyu indir" },
  addToList: { one: "{count} videoyu ekle", other: "{count} videoyu ekle" },
  added: { one: "{count} video eklendi.", other: "{count} video eklendi." },
  skipped: { one: "{count} tanesi zaten listedeydi.", other: "{count} tanesi zaten listedeydi." },
};
