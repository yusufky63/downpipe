// Stream and video page qualities, their audio tracks, and the site tools
// (yt-dlp, Deno or Node, FFmpeg) in the settings. Codec, tool and site names,
// resolutions and units stay as they are in every language.
import type { Messages } from "../core";

export const media: Messages["media"] = {
  audioOnly: "Yalnızca ses",
  defaultQuality: "Varsayılan",
  aboutSize: "yaklaşık {size}",
  audio: "Ses",
  tools: {
    found: "Bulundu",
    notFound: "Bulunamadı",
    problem: "Sorun",
    version: "sürüm {version}",
    sources: {
      settings: "Ayarlar'da belirlendi",
      managed: "Downpipe tarafından indirildi",
      path: "PATH'te",
    },
    missing: {
      ytDlp: "Sayfalardaki videolar için gerekli.",
      runtime: "Deno (ya da Node) olmadan yt-dlp bazı YouTube kalitelerini kaçırır.",
      ffmpeg: "Bu olmadan sadece Yalnızca ses ve kendi sesi olan kaliteler çalışır.",
    },
    download: "{tools} indir",
    update: "{tools} güncelle",
  },
};
