// Small dialogs: removing downloads, and fetching a finished file again.
import type { Messages } from "../core";

export const dialogs: Messages["dialogs"] = {
  remove: {
    deleteFile: "ファイルごと削除",
    deleteFiles: "ファイルごと削除",
    deletePartial: "途中までのデータごと削除",
    fromList: "一覧から削除",
    doneOne: "一覧から削除しても、ファイルはディスクに残ります。ファイルごと削除すると、ファイル自体も削除されます。",
    doneMany: "一覧から削除しても、各ファイルはディスクに残ります。ファイルごと削除すると、ファイル自体も削除されます。",
    partialOne:
      "一覧から削除しても途中までのファイルは残るため、同じリンクから後で再開できます。データごと削除すると、途中までのデータも削除されます。",
    partialMany:
      "一覧から削除しても、完了したファイルと途中までのデータはディスクに残ります。データごと削除すると、その両方が削除されます。",
  },
  redownload: {
    eyebrow: "再ダウンロード",
    body: "同じリンクから同じフォルダーに、ファイルをもう一度ダウンロードします。サーバー上でファイルが変更された場合や、ディスク上のコピーが破損している場合などに使用します。",
    replace: "既存のファイルを置き換える",
    replaceOn: "現在のファイルを先に削除し、新しいファイルがその名前を引き継ぎます。",
    replaceOff: "オフ：現在のファイルは残り、新しいコピーは重複しない名前で同じフォルダーに保存されます。",
    confirm: "再ダウンロード",
    confirmReplace: "削除して再ダウンロード",
  },
  duplicate: {
    eyebrow: "一覧に登録済み",
    titleMany: { other: "{count} 件のリンクは既に一覧にあります" },
    body: "このアドレスのダウンロードは既に一覧にあります（{state}）。どうしますか？",
    bodyMany: {
      other: "これらのアドレスのダウンロードは既に一覧にあります。{count} 件すべてをどうしますか？",
    },
    question: "リンクの処理方法",
    copy: "番号付きのファイル名でもう一度追加する",
    overwrite: "もう一度追加して既存のファイルを置き換える",
    existing: "既存のダウンロードを表示する（未完了の場合は再開する）",
    ok: "OK",
    remember: "今後は確認せず、常にこうする",
    rememberHint: "「設定」の「ダウンロード」で元に戻せます。",
  },
  /** A download's own speed limit, under the global one. {limit} is like "2 MiB/s". */
  speedLimit: {
    eyebrow: "速度制限",
    body: "このダウンロードだけに適用され、全体の制限 {limit} の範囲内で動作します。",
    bodyNoGlobal: "このダウンロードだけに適用されます。全体の制限はありません。",
    bodyMany: "それぞれに個別に適用され、全体の制限 {limit} の範囲内で動作します。",
    bodyManyNoGlobal: "それぞれに個別に適用されます。全体の制限はありません。",
    label: "制限",
    unit: "速度制限の単位",
    placeholder: "制限なし",
    video: "動画は次に開始したときに新しい制限を使います。",
    remove: "制限を解除",
    save: "制限を設定",
  },
  /** "Pause until…" with a time of the user's choice. */
  pauseUntil: {
    eyebrow: "指定時刻まで一時停止",
    label: "再開時刻",
    hint: "今すぐ一時停止し、Downpipe の実行中ならこの時刻に、そうでなければ次に起動したときに自動で再開します。",
    past: "これから先の時刻を選んでください。",
    invalid: "日付と時刻を入力してください。",
    submit: "一時停止",
  },
};
