// Videos and audio detected on pages. {site} is a site's own name (YouTube),
// {format} a file format in capitals (MP4).
import type { Messages } from "../core";

export const media: Messages["media"] = {
  hls: "Поток HLS",
  dash: "Поток DASH",
  hlsQualities: {
    one: "HLS · {count} вариант качества",
    few: "HLS · {count} варианта качества",
    many: "HLS · {count} вариантов качества",
    other: "HLS · {count} варианта качества",
  },
  fileAudio: "Аудио {format}",
  fileVideo: "Видео {format}",
  audio: "Аудио",
  video: "Видео",
  siteVideo: "Видео {site}",
  thisSiteVideo: "Это видео {site}",
  siteVideoQuality: "Видео {site} · качество выбирается в Downpipe",
  updateForStreams: "Обновите Downpipe, чтобы загружать потоки",
  updateForPages: "Обновите Downpipe, чтобы загружать видео со страниц",
  emptyOff: "Включите, чтобы видеть видео и аудио, которые воспроизводят страницы. Сначала браузер запросит доступ ко всем сайтам.",
  emptyExcluded: "Здесь не показывается: {site} входит в исключенные сайты в разделе «Фильтры».",
  emptyNothing: "Пока ничего. Воспроизведите видео; если оно уже воспроизводилось, перезагрузите страницу.",
};
