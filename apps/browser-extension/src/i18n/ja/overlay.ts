// The Download button over playing videos on web pages.
import type { Messages } from "../core";

export const overlay: Messages["overlay"] = {
  buttonTitle: "この動画を Downpipe でダウンロード",
  noAnswer: "Downpipe の拡張機能から応答がありませんでした。ページを再読み込みして、もう一度お試しください。",
  nothingYet: "まだダウンロードできるものがありません。動画をしばらく再生してから、もう一度お試しください。",
  duplicate: "既に Downpipe にあります",
  dragHint: "ドラッグすると動画の中で移動できます。",
  more: "その他のオプション",
  hideSite: "このサイトでは表示しない",
  hideSiteDetail: "このサイトの動画は引き続き検出されます。",
  hidden: "このサイトでは非表示にしました。Downpipe 拡張機能の「フィルター」で再表示できます。",
};
