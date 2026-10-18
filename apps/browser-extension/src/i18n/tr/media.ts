// Videos and audio detected on pages, in the popup's media list and the
// overlay's menu. {site} is a site's own name (YouTube), {format} a file
// format in capitals (MP4).
import type { Messages } from "../core";

export const media: Messages["media"] = {
  hls: "HLS akışı",
  dash: "DASH akışı",
  hlsQualities: { one: "HLS · {count} kalite", other: "HLS · {count} kalite" },
  fileAudio: "{format} ses",
  fileVideo: "{format} video",
  audio: "Ses",
  video: "Video",
  siteVideo: "{site} videosu",
  thisSiteVideo: "Bu {site} videosu",
  siteVideoQuality: "{site} videosu · kalite Downpipe'ta seçilir",
  updateForStreams: "Akış indirmek için Downpipe'ı güncelleyin",
  updateForPages: "Sayfalardaki videoları indirmek için Downpipe'ı güncelleyin",
  emptyOff: "Sayfaların oynattığı video ve sesleri listelemek için açın. Tarayıcı önce tüm sitelere erişim izni ister.",
  emptyExcluded: "Burada listelenmez: {site}, Filtreler'de hariç tutulan bir site.",
  emptyNothing: "Henüz bir şey yok. Videoyu oynatın; zaten oynuyorsa sayfayı yeniden yükleyin.",
};
