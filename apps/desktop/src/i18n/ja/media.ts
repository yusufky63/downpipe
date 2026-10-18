// Stream and video page qualities, their audio tracks, and the site tools
// (yt-dlp, Deno or Node, FFmpeg) in the settings.
import type { Messages } from "../core";

export const media: Messages["media"] = {
  subtitles: {
    label: "字幕",
    none: "なし",
    uploaded: "投稿者の字幕",
    auto: "自動字幕",
    original: "{language}（オリジナル）",
    embedded: "字幕は動画ファイルに埋め込まれます。",
    separate: "字幕は別のファイルとして動画の隣に保存されます。",
  },
  siteErrors: {
    forbidden: "YouTube にダウンロードを拒否されました。多くの場合、新しい yt-dlp で解決します。「設定」の「メディア」で更新してから、もう一度お試しください。",
    signIn: "このダウンロードには、YouTube にログインしたブラウザーが必要です。Downpipe はブラウザーの Cookie を使いません。しばらく待つか別のネットワークを使って、もう一度お試しください。",
    ageRestricted: "YouTube はこの動画をログインした成人にのみ表示します。Downpipe はアカウントもブラウザーの Cookie も使わないため、ダウンロードできません。",
    unavailable: "この動画は利用できません。非公開、削除済み、メンバー限定、またはお住まいの地域でブロックされています。",
    jsRuntime: "yt-dlp が YouTube の JavaScript チャレンジに応答できませんでした。「設定」の「メディア」で Deno をダウンロードしてから、もう一度お試しください。",
    poToken: "YouTube がこの動画に発信元証明（PO）トークンを求めました。「設定」の「メディア」で yt-dlp を更新するか、別の画質を選んでください。",
    outdated: "yt-dlp がこのページを読み取れなくなりました。YouTube がページを変更した可能性があります。「設定」の「メディア」で yt-dlp を更新してから、もう一度お試しください。",
  },
  audioOnly: "音声のみ",
  defaultQuality: "既定",
  aboutSize: "約 {size}",
  audio: "音声",
  tools: {
    found: "検出済み",
    notFound: "未検出",
    problem: "問題あり",
    version: "バージョン {version}",
    sources: {
      settings: "設定で指定",
      managed: "Downpipe がダウンロード",
      path: "PATH 上",
    },
    missing: {
      ytDlp: "ページ内の動画に必要です。",
      runtime: "Deno（または Node）がないと、yt-dlp は YouTube の一部の画質を取得できません。",
      ffmpeg: "これがないと、「音声のみ」と、音声を含む画質しか使用できません。",
    },
    download: "{tools} をダウンロード",
    update: "{tools} を更新",
  },
};
