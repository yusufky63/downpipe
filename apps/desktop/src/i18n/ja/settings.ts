// The settings dialog, one group per tab, and the browser bridge's states.
// {placeholders} that hold an element (a path, a command) are put in by the dialog.
import type { Messages } from "../core";

export const settings: Messages["settings"] = {
  eyebrow: "設定",
  title: "環境設定",
  tabsLabel: "設定のセクション",
  tabs: {
    downloads: "ダウンロード",
    queues: "キュー",
    categories: "カテゴリ",
    network: "ネットワーク",
    media: "メディア",
    schedule: "スケジュール",
    app: "アプリ",
  },
  downloads: {
    folder: "既定のフォルダー",
    folderHint: "カテゴリのフォルダーは、このフォルダーの中に作成されます。",
    concurrent: "同時ダウンロード数",
    concurrentHint: "それ以外はキューで待機します。1～16 の範囲で指定します。",
    connections: "ダウンロードごとの接続数",
    connectionsHint: "上限です。Downpipe は 2 接続から始め、速くなる間だけ接続を増やします。1～32 の範囲で指定します。",
    speedLimit: "速度制限",
    speedLimitHint: "すべてのダウンロードで共有されます。空欄にすると制限なしになります。すぐに適用されます。",
    noLimit: "制限なし",
    unit: "速度制限の単位",
    duplicate: "リンクが既に一覧にあるとき",
    duplicateHint: "確認画面の「今後は確認しない」でもこの設定が変わります。",
    duplicateAsk: "毎回確認する",
  },
  queues: {
    intro:
      "各キューは、ほかのキューと並行して、それぞれに設定された数のダウンロードを同時に実行します。キュー内では、ダウンロードは上から順に開始されます。キュー間の移動や順序の変更は、右クリックメニューから行えます。削除したキューのダウンロードはメインキューに移動します。",
    label: "キュー",
    name: "キュー名",
    namePlaceholder: "名前",
    mainLimit: { other: "同時に {count} 件" },
    mainLimitTitle: "「ダウンロード」タブの「同時ダウンロード数」で設定します",
    limit: "{name} の同時ダウンロード数",
    limitUnnamed: "キューの同時ダウンロード数",
    limitTitle: "同時ダウンロード数（1～16）",
    remove: "{name} を削除",
    removeUnnamed: "キューを削除",
    newName: "キュー {number}",
    add: "キューを追加",
  },
  categories: {
    use: "ダウンロードをカテゴリ別のフォルダーに振り分ける",
    useHint:
      "新しいダウンロードは、ファイルの種類に応じてカテゴリのフォルダーに保存されます。サーバーが返すファイル名が別の種類を示している場合、完了したファイルは適切なフォルダーに移動されます。自分で選んだフォルダーが変更されることはありません。",
    label: "カテゴリ",
    name: "カテゴリ名",
    namePlaceholder: "名前",
    types: "{name} のファイルの種類",
    typesUnnamed: "カテゴリのファイルの種類",
    remove: "{name} を削除",
    removeUnnamed: "カテゴリを削除",
    folder: "{name} のフォルダー",
    folderUnnamed: "カテゴリのフォルダー",
    folderPlaceholder: "フォルダー：既定のフォルダー内の {name}",
    folderPlaceholderUnnamed: "フォルダー：既定のフォルダー内の「名前」",
    add: "カテゴリを追加",
  },
  network: {
    proxy: "プロキシ",
    proxyHint:
      "http://、https:// または socks5://host:port。空欄の場合はシステムのプロキシを使用します。保存後に開始したダウンロードに適用されます。",
    proxyPlaceholder: "システムのプロキシ",
    userAgent: "ユーザーエージェント",
    userAgentHint: "一部のサーバーはブラウザーにしか応答しません。空欄の場合は Downpipe 独自のものを送信します。",
  },
  media: {
    ffmpegHint:
      "ストリームの別々の映像と音声を、再エンコードせずに 1 つのファイルに結合します。空欄の場合は PATH 上の ffmpeg を使用します。Downpipe には FFmpeg は含まれていません。",
    ffmpegPlaceholder: "PATH 上の ffmpeg",
    found: "検出済み",
    notFound: "未検出",
    ffmpegMissing:
      "映像と音声が別々に配信されるストリームは、2 つのファイルとして保存されます。FFmpeg をインストールして PATH に追加するか、上に ffmpeg のパスを入力してから保存してください。",
    streams:
      "ストリームは、ブラウザー拡張機能のメディア検出、またはここで追加した .m3u8 や .mpd のリンクから取得されます。DRM で保護されたストリームやライブストリームはダウンロードされません。",
  },
  siteTools: {
    heading: "ページ内の動画",
    intro:
      "YouTube のページで「Downpipe でダウンロード」を選ぶと、ページが yt-dlp に渡され、yt-dlp が動画を取り出します。yt-dlp には、YouTube のために Deno が、360p を超える画質で映像と音声を結合するために FFmpeg が必要です。",
    ytDlpHint: "空欄の場合は下でダウンロードしたコピーを使用し、なければ PATH 上の yt-dlp を使用します。",
    ytDlpPlaceholder: "ダウンロードしたコピー、なければ PATH 上の yt-dlp",
    denoHint:
      "バージョン 2.3.0 以降。空欄の場合は下でダウンロードしたコピー、なければ PATH 上の deno、それもなければ Node 22 以降を使用します。",
    denoPlaceholder: "ダウンロードしたコピー、なければ PATH 上の deno",
    looking: "yt-dlp と Deno を探しています…",
    listLabel: "ページ内の動画用ツール",
    downloading: "ダウンロード中…",
    installNote:
      "ボタンを押したときのみ実行されます：github.com/yt-dlp/yt-dlp と github.com/denoland/deno の公式ビルドを、それぞれのリリースが公開している SHA-256 で検証したうえで {folder} に保存します。YouTube は頻繁に変更されるため、最新の yt-dlp を入手するにはもう一度押してください。",
    toolsFolder: "Downpipe のツールフォルダー",
    cookies:
      "Downpipe は yt-dlp に Cookie を送信しないため、非公開、メンバー限定、年齢制限付きの動画はダウンロードされません。保護された（DRM）動画やライブ動画も同様です。YouTube の利用規約を守るのはご自身の責任です。",
  },
  schedule: {
    enable: "スケジュールに従ってダウンロードを開始・停止する",
    enableHint:
      "開始時刻になると、一時停止中および失敗したダウンロードが再開されます。停止時刻になると一時停止されます。Downpipe がウィンドウまたは通知領域で実行されている必要があります。",
    queue: "キュー",
    queueHint: "ほかのキューには影響しません。",
    everyQueue: "すべてのキュー",
    start: "開始時刻",
    stop: "停止時刻",
    stopHint: "省略可",
    days: "曜日",
  },
  app: {
    language: "言語",
    languageHint: "ウィンドウ、通知領域のメニュー、通知にすぐ反映されます。",
    systemDefault: "システムの既定（{language}）",
    notify: "ダウンロードの完了時に通知する",
    notifyHint: "ウィンドウが前面にないときにダウンロードが完了または失敗すると、システム通知を表示します。",
    tray: "ウィンドウを閉じても通知領域で実行を続ける",
    trayHint: "ダウンロードは続行され、ブラウザー拡張機能からリンクを送ることもできます。終了するには通知領域のアイコンを使用します。",
    startup: "サインイン時に Downpipe を起動する",
    startupHint: "通知領域で起動するため、取り込んだダウンロードを常に受け取れます。",
    clipboard: "コピーしたリンクのダウンロードを提案する",
    clipboardHint:
      "「カテゴリ」に登録されたファイルの種類へのリンクや YouTube 動画のリンクをコピーすると、Downpipe がそのリンクで「リンクを追加」ダイアログを開きます。すでに一覧にあるリンクはスキップされます。",
    awake: "ダウンロード中はコンピューターをスリープさせない",
    awakeHint: "ダウンロードが止まるまでスリープを待機します。画面はオフになることがあります。",
    confirmBrowser: "ブラウザーからのダウンロードを開始する前に確認する",
    confirmBrowserHint:
      "ブラウザーの上に表示される小さなウィンドウで、先にファイル名、フォルダー、キューを選択します。オフ：自動のフォルダーですぐに開始されます。",
  },
  bridge: {
    heading: "ブラウザー連携",
    register: "起動時にブラウザーへ登録する",
    registerHint:
      "Chrome、Edge、Chromium、Firefox の Downpipe 拡張機能が、この Downpipe に接続できるようにします。変更されるのはお使いのユーザーアカウントのみです。downpipe-native-host で登録を自分で管理する場合はオフにしてください。",
    checking: "登録を確認しています…",
    noHost: "この Downpipe のブラウザー連携が {host} に見つかりません。Downpipe を再インストールしてください。開発ビルドの場合は {command} を実行してください。",
    listLabel: "ブラウザーごとの登録状況",
    registering: "登録しています…",
    registerNow: "今すぐ登録",
    pickUp: "開いているブラウザーには、拡張機能が次に確認したときに反映されます。",
    states: {
      registered: "登録済み",
      notInstalled: "未インストール",
      missing: "未登録",
      invalid: "読み取り不可",
      otherHost: "別のコピー",
      extensionNotAllowed: "別の拡張機能",
      failed: "エラー",
    },
  },
};
