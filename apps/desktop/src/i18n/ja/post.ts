// After a download completes: the virus scan, extracting archives and a command.
import type { Messages } from "../core";

export const post: Messages["post"] = {
  steps: {
    scanning: "スキャン中",
    extracting: "展開中",
    running_command: "コマンド実行中",
  },
  threat: "脅威",
  summary: {
    threat: "脅威を検出: {threat}",
    notExtracted: "展開されていません",
    commandFailed: "ダウンロード後のコマンドが失敗しました",
  },
  section: "ダウンロード後",
  scan: {
    label: "ウイルススキャン",
    clean: "{scanner} は何も検出しませんでした。",
    threat:
      "{scanner} が{threat}を検出しました。展開や実行は行われず、ファイルはそのまま残されています。削除するかどうかはご自身で判断してください。",
    unnamed: "脅威",
    theScanner: "スキャナー",
    missing: "スキャンされていません: スキャナーが設定されていません。設定の「ダウンロード後」で選んでください。",
    missingNamed: "スキャンされていません: {scanner} が見つかりません。",
    failed: "{scanner} はファイルをスキャンできませんでした: {detail}",
  },
  extract: {
    label: "展開済み",
    done: { other: "{count} 個のファイルを新しいフォルダーに展開しました。" },
    skipped: { other: "{count} 個の項目を除外しました: 安全でない名前、リンク、または重複です。" },
    deleted: "その後、アーカイブを削除しました。",
    openFolder: "フォルダーを開く",
    failed: "展開されていません: {reason}。",
    errors: {
      encrypted: "アーカイブがパスワードで保護されています",
      too_large: "元のサイズをはるかに超えて展開されます (zip 爆弾の可能性)",
      too_many_entries: "項目が多すぎます",
      corrupt: "アーカイブが壊れているか、対応していない方式を使っています",
      io: "ファイルを書き込めませんでした",
    },
  },
  command: {
    label: "コマンド",
    ok: "{program} が完了しました。",
    failed: "{program} は終了コード {code} で終了しました。",
    ended: "{program} は完了前に終了されました。",
    timedOut: "{program} が制限時間を超えたため停止しました。",
    notStarted: "{program} を起動できませんでした: {detail}",
    invalid: "このコマンドは実行できません: {detail}",
    theCommand: "コマンド",
    output: "出力",
  },
  fields: {
    heading: "ダウンロード後",
    command: "完了後に実行",
    inherited: "既定: {command}",
    nothing: "なし",
    hint: "プログラムとその引数。例: notepad {path}。{path}、{name}、{folder}、{url} はそれぞれ 1 つの引数として埋め込まれ、シェルは使いません。ダウンロードが失敗したときは実行されません。",
    extract: "アーカイブを展開",
    extractHint: "完了したら、隣の新しいフォルダーに展開します。",
    save: "保存",
    saved: "保存しました。ダウンロードの完了時に適用されます。",
  },
  settings: {
    tab: "ダウンロード後",
    command: "各ダウンロードの後にコマンドを実行",
    commandHint:
      "プログラムとその引数。例: notepad {path}。{path}、{name}、{folder}、{url} はそれぞれ 1 つの引数として埋め込まれます。シェルを経由しないため、ファイル名やリンクがコマンドを追加することはできません。キューや個々のダウンロードごとに設定することもできます。ダウンロードが失敗したときは実行されません。",
    timeout: "制限時間",
    timeoutHint: "コマンドやスキャンが、起動したものと一緒に停止されるまで実行できる秒数。5～86400。",
    seconds: "秒",
    scan: "ダウンロードをウイルススキャンする",
    scanHintWindows:
      "各ダウンロードの完了後に Microsoft Defender でスキャンします。脅威が見つかるとダウンロードに印が付き、通知が表示されます。削除や検疫は行われず、展開や実行もされません。",
    scanHintOther:
      "各ダウンロードの完了後に下のスキャナーでスキャンします。脅威が見つかるとダウンロードに印が付き、通知が表示されます。削除、展開、実行は行われません。",
    scanner: "スキャナー",
    scannerPlaceholderWindows: "Microsoft Defender",
    scannerHint:
      "空欄の場合、Windows では Microsoft Defender を使います。ほかのスキャナーにはファイルが {path} として、または最後の引数として渡されます。ClamAV の clamscan と同じく、終了コード 0 は問題なし、1 は脅威を意味します。",
    archives: "アーカイブ",
    archivesHint:
      "zip、tar、tar.gz、tar.xz、7z のアーカイブは、カテゴリ (カテゴリ タブ) またはダウンロード自体で指定されていれば、隣の新しいフォルダーに展開されます。そのフォルダーの外に出る項目、リンク、サイズをはるかに超えて展開されるアーカイブは拒否されます。RAR には対応していません。7-Zip をコマンドとして使えば展開できます。",
    deleteArchive: "展開後にアーカイブを削除",
    deleteArchiveHint: "すべての項目が展開され、コマンドの実行が終わった場合のみ。",
    queueCommand: "{name} のダウンロード後のコマンド",
    queueCommandUnnamed: "キューのダウンロード後のコマンド",
    queueCommandPlaceholder: "完了後のコマンド。空欄: 「ダウンロード後」の設定",
    categoryExtract: "アーカイブを展開",
    categoryExtractTitle: "このカテゴリの zip、tar、7z アーカイブをダウンロード後に展開します",
  },
};
