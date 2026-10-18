// Stream and video page qualities, their audio tracks, and the site tools
// (yt-dlp, Deno or Node, FFmpeg) in the settings. Codec, tool and site names,
// resolutions and units stay as they are in every language.
import type { Messages } from "../core";

export const media: Messages["media"] = {
  subtitles: {
    label: "Altyazı",
    none: "Yok",
    uploaded: "Yükleyenin altyazıları",
    auto: "Otomatik altyazılar",
    original: "{language} (orijinal)",
    embedded: "Altyazı video dosyasının içine eklenir.",
    separate: "Altyazı, videonun yanına ayrı bir dosya olarak kaydedilir.",
  },
  siteErrors: {
    forbidden: "YouTube indirmeyi reddetti. Genellikle daha yeni bir yt-dlp bunu çözer: Ayarlar'ın Medya sekmesinden güncelleyip yeniden deneyin.",
    signIn: "YouTube bu indirme için oturum açılmış bir tarayıcı istiyor. Downpipe tarayıcı çerezlerini kullanmaz: biraz bekleyin ya da başka bir ağ kullanıp yeniden deneyin.",
    ageRestricted: "YouTube bu videoyu yalnızca oturum açmış yetişkinlere gösteriyor. Downpipe hesap ya da tarayıcı çerezi kullanmadığı için indiremez.",
    unavailable: "Bu video kullanılamıyor: gizli, kaldırılmış, yalnızca üyelere açık ya da bölgenizde engellenmiş.",
    jsRuntime: "yt-dlp, YouTube'un JavaScript sınamalarını yanıtlayamadı. Ayarlar'ın Medya sekmesinden Deno'yu indirip yeniden deneyin.",
    poToken: "YouTube bu video için bir kaynak kanıtı (PO) belirteci istedi. Ayarlar'ın Medya sekmesinden yt-dlp'yi güncelleyin ya da başka bir kalite seçin.",
    outdated: "yt-dlp bu sayfayı artık anlamıyor; YouTube muhtemelen sayfayı değiştirdi. Ayarlar'ın Medya sekmesinden yt-dlp'yi güncelleyip yeniden deneyin.",
  },
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
      ffmpeg: "Bu olmadan sadece “Yalnızca ses” seçeneği ve kendi sesi olan kaliteler çalışır.",
    },
    download: "{tools} indir",
    update: "{tools} güncelle",
  },
};
