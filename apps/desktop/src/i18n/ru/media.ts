// Stream and video page qualities, their audio tracks, and the site tools
// (yt-dlp, Deno or Node, FFmpeg) in the settings. Codec, tool and site names,
// resolutions and units stay as they are in every language.
import type { Messages } from "../core";

export const media: Messages["media"] = {
  audioOnly: "Только звук",
  defaultQuality: "По умолчанию",
  aboutSize: "около {size}",
  audio: "Звук",
  tools: {
    found: "Найден",
    notFound: "Не найден",
    problem: "Проблема",
    version: "версия {version}",
    sources: {
      settings: "задан в настройках",
      managed: "загружен Downpipe",
      path: "в PATH",
    },
    missing: {
      ytDlp: "Нужен для видео на страницах.",
      runtime: "Без Deno (или Node) yt-dlp не видит некоторые варианты качества YouTube.",
      ffmpeg: "Без него доступны только «Только звук» и варианты качества со своим звуком.",
    },
    // {tools} is joined by Intl.ListFormat, e.g. "yt-dlp и Deno".
    download: "Загрузить {tools}",
    update: "Обновить {tools}",
  },
};
