// Stream and video page qualities, their audio tracks, and the site tools
// (yt-dlp, Deno or Node, FFmpeg) in the settings. Codec, tool and site names,
// resolutions and units stay as they are in every language.
import type { Messages } from "../core";

export const media: Messages["media"] = {
  subtitles: {
    label: "Субтитры",
    none: "Нет",
    uploaded: "От автора",
    auto: "Автоматические субтитры",
    original: "{language} (оригинал)",
    embedded: "Субтитры встраиваются в файл видео.",
    separate: "Субтитры сохраняются рядом с видео отдельным файлом.",
  },
  siteErrors: {
    forbidden: "YouTube отказал в загрузке. Обычно помогает более новая версия yt-dlp: обновите ее в настройках, в разделе «Медиа», и попробуйте снова.",
    signIn: "Для этой загрузки YouTube требует браузер с выполненным входом. Downpipe не использует файлы cookie браузера: подождите немного или подключитесь к другой сети и попробуйте снова.",
    ageRestricted: "YouTube показывает это видео только взрослым пользователям, выполнившим вход. Downpipe не использует аккаунты и файлы cookie браузера, поэтому не может его загрузить.",
    unavailable: "Это видео недоступно: оно личное, удалено, доступно только спонсорам или заблокировано в вашем регионе.",
    jsRuntime: "yt-dlp не смог ответить на JavaScript-проверки YouTube. Загрузите Deno в настройках, в разделе «Медиа», и попробуйте снова.",
    poToken: "YouTube запросил для этого видео токен подтверждения происхождения (PO). Обновите yt-dlp в настройках, в разделе «Медиа», или выберите другое качество.",
    outdated: "yt-dlp больше не понимает эту страницу; вероятно, YouTube ее изменил. Обновите yt-dlp в настройках, в разделе «Медиа», и попробуйте снова.",
  },
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
