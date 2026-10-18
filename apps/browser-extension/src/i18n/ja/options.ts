// The options page (Filters). {example}, {tarGz} and {wildcard} are code samples shown as they are.
import type { Messages } from "../core";

export const options: Messages["options"] = {
  pageTitle: "フィルター · Downpipe",
  headLabel: "ブラウザー拡張機能",
  lede:
    "「ブラウザーのダウンロードを取り込む」がオンのときにブラウザーが Downpipe に渡すダウンロードと、「このページのメディア」がオンのときに拡張機能が一覧表示するメディアを選択します。それ以外はすべて、通常どおりブラウザーで処理されます。",
  captureOff:
    "取り込みはオフです。これらのフィルターは、Downpipe のツールバーのポップアップで「ブラウザーのダウンロードを取り込む」をオンにすると適用されます。",
  loadError: "設定を読み込めませんでした。このページを再読み込みして、もう一度お試しください。",
  typesHeading: "01 — ファイルの種類",
  captureAll: "すべての種類のファイルを取り込む",
  captureAllDetail: "ブラウザーが開始するすべてのダウンロードが、種類に関係なく Downpipe に送られます。",
  typesLabel: "次の種類のファイルのみ",
  typesHelp:
    "{example} のように、スペースまたはコンマで区切った拡張子です。ファイル名がいずれかで終わるダウンロードが取り込まれます。{tarGz} も使用できます。",
  restoreDefault: "既定の一覧に戻す",
  sitesHeading: "02 — 除外サイト",
  sitesLabel: "次のサイトでは取り込みもメディアの一覧表示も行わない",
  sitesHelp:
    "1 行に 1 つのサイトを入力します。{wildcard} はすべてのサブドメインも対象になります。これらのサイトからのダウンロードや、これらのサイトのページで開始したダウンロードはブラウザーで処理され、これらのサイトで再生されるメディアは一覧に表示されません。ポートとパスは無視されます。",
  sizeHeading: "03 — 最小サイズ",
  sizeLabel: "これより小さいファイルはブラウザーに任せる",
  sizeHelp: "0 にするとオフになります。サーバーがファイルのサイズを示さない場合、そのファイルは Downpipe に送られます。",
  mediaHeading: "04 — ページ内のメディア",
  mediaHelp:
    "ツールバーのポップアップで「このページのメディア」がオンのときに使用されます。HLS と DASH のストリームは常に一覧表示されます。上記の除外サイトはここにも適用されます。",
  overlay: "動画に「ダウンロード」ボタンを表示",
  overlayDetail: "再生中の動画で、数秒間と、ポインターが動画の上にある間に表示します。",
  overlayHover: "ポインターが動画の上にあるときだけ表示する",
  overlayHoverDetail: "オフにすると、動画の再生開始時にも数秒間表示されます。",
  overlaySitesLabel: "ボタンを表示しないサイト",
  overlaySitesHelp: "除外するサイトと同じく 1 行に 1 サイト。ボタンのメニューの「このサイトでは表示しない」でここに追加されます。これらのサイトで再生されるメディアは引き続きポップアップに表示されます。",
  mediaTypesLabel: "次の種類のメディアファイルを一覧表示",
  mediaTypesHelp:
    "スペースまたはコンマで区切った拡張子です。拡張子のないファイルは、サーバーが送る種類で判断します。一覧が空の場合は、ストリームのみを表示します。",
  mediaSizeLabel: "これより小さいメディアファイルを除外",
  mediaSizeHelp:
    "通知音や短いプレビューを除外します。0 にするとすべてのサイズを表示します。サーバーがサイズを示さないファイルは常に表示されます。",
  bypassHeading: "05 — ブラウザーでダウンロード",
  bypassLabel: "このキーを押しながらクリックしたリンクはブラウザーでダウンロード",
  bypassAlt: "Alt（Mac では Option）",
  bypassShift: "Shift",
  bypassOff: "オフ",
  bypassHelp: "キャプチャーがオンのとき、このキーを押しながらリンクをクリックすると、そのダウンロードだけブラウザーに任せます。ほかのクリックやブラウザー自身のショートカットは変わりません。",
  bypassAccess: "クリックを検知するには、閲覧するサイトへのアクセスが必要です。この方法でリンクをクリックするまで、何も読み取りや送信はしません。",
  bypassAllow: "アクセスを許可",
  appHeading: "06 — デスクトップアプリ",
  startApp: "Downpipe が閉じているときは起動する",
  startAppDetail:
    "ブラウザーから送ったダウンロードは、失敗する代わりにまず Downpipe を通知領域で起動します。オフにすると、Downpipe を開いておく必要があります。",
  save: "保存",
  moreErrors: { other: "ほかに {count} 件あります。" },
  unsaved: "未保存の変更があります",
  notSavedFix: "保存されていません。赤で示された項目を修正してください。",
  saving: "保存中…",
  notSavedNoAnswer: "保存されていません。拡張機能から応答がありませんでした。もう一度お試しください。",
  saved: "保存しました",
  errors: {
    notExtension: "「{value}」はファイルの拡張子ではありません。",
    tooManyTypes: { other: "指定できるファイルの種類は {count} 個までです。" },
    notSite: "{line} 行目：「{value}」はサイトではありません。example.com や *.example.com のようなホストを使用してください。",
    tooManySites: { other: "指定できるサイトは {count} 個までです。" },
    sizeNotNumber: "2 や 1.5 のような数値を入力するか、0 を入力してオフにしてください。",
    sizeTooBig: "1 TiB 未満のサイズを入力してください。",
    noTypes: "ファイルの種類を 1 つ以上追加するか、「すべての種類のファイルを取り込む」をオンにしてください。",
  },
};
