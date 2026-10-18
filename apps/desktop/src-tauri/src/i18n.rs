//! Texts the app shows outside the web view: the tray menu, its tooltip and
//! system notifications. The web view has its own translations; the language
//! is chosen the same way (Settings, else the system, else English).

use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Tr,
    Es,
    De,
    Fr,
    It,
    PtBr,
    Ru,
    ZhCn,
    Ja,
}

impl Lang {
    /// `setting` is the code saved in Settings; empty follows the system.
    pub fn resolve(setting: &str) -> Lang {
        let setting = setting.trim();
        let picked = (!setting.is_empty()).then(|| setting.to_string());
        picked
            .iter()
            .chain(system_languages())
            .find_map(|tag| Lang::from_tag(tag))
            .unwrap_or(Lang::En)
    }

    /// `pt` means Brazilian Portuguese and `zh` simplified Chinese;
    /// traditional Chinese is skipped rather than shown in the other script.
    fn from_tag(tag: &str) -> Option<Lang> {
        let tag = tag.trim().to_ascii_lowercase().replace('_', "-");
        let base = tag.split('-').next().unwrap_or_default();
        match base {
            "en" => Some(Lang::En),
            "tr" => Some(Lang::Tr),
            "es" => Some(Lang::Es),
            "de" => Some(Lang::De),
            "fr" => Some(Lang::Fr),
            "it" => Some(Lang::It),
            "pt" => Some(Lang::PtBr),
            "ru" => Some(Lang::Ru),
            "ja" => Some(Lang::Ja),
            "zh" if ["hant", "-tw", "-hk", "-mo"]
                .iter()
                .any(|t| tag.contains(t)) =>
            {
                None
            }
            "zh" => Some(Lang::ZhCn),
            _ => None,
        }
    }

    pub fn texts(self) -> &'static Texts {
        match self {
            Lang::En => &EN,
            Lang::Tr => &TR,
            Lang::Es => &ES,
            Lang::De => &DE,
            Lang::Fr => &FR,
            Lang::It => &IT,
            Lang::PtBr => &PT_BR,
            Lang::Ru => &RU,
            Lang::ZhCn => &ZH_CN,
            Lang::Ja => &JA,
        }
    }
}

/// The system's languages, read once: they only change with a new sign-in.
fn system_languages() -> &'static [String] {
    static LANGS: OnceLock<Vec<String>> = OnceLock::new();
    LANGS.get_or_init(|| sys_locale::get_locales().collect())
}

/// Texts for the language picked in `setting` (empty follows the system).
pub fn texts(setting: &str) -> &'static Texts {
    Lang::resolve(setting).texts()
}

/// Replace `{name}` placeholders.
pub fn fill(template: &str, vars: &[(&str, &str)]) -> String {
    vars.iter()
        .fold(template.to_string(), |text, (name, value)| {
            text.replace(&format!("{{{name}}}"), value)
        })
}

pub struct Texts {
    pub show: &'static str,
    pub pause_all: &'static str,
    pub resume_all: &'static str,
    pub quit: &'static str,
    pub still_running_title: &'static str,
    pub still_running_body: &'static str,
    pub all_done_title: &'static str,
    /// `{secs}`
    pub all_done_quit: &'static str,
    pub all_done_sleep: &'static str,
    pub all_done_shutdown: &'static str,
    pub job_failed_title: &'static str,
    /// `{err}`
    pub job_failed_body: &'static str,
    pub complete: &'static str,
    pub failed: &'static str,
    pub threat_title: &'static str,
    /// `{scanner}`, `{threat}`, `{name}`
    pub threat_body: &'static str,
    /// Stands for `{threat}` when the scanner gave no name.
    pub threat_unnamed: &'static str,
    pub schedule_started: &'static str,
    pub schedule_started_all: &'static str,
    /// `{name}`
    pub schedule_started_queue: &'static str,
    pub schedule_stopped: &'static str,
    pub schedule_stopped_all: &'static str,
    pub schedule_stopped_queue: &'static str,
    /// `{speed}`
    pub tooltip_one: &'static str,
    /// `{n}`, `{speed}`
    pub tooltip_many: &'static str,
    /// An `http_status` failure whose message names no status.
    pub server_error: &'static str,
    /// `(error kind, text)`, the web view's `errors.kind.*` and
    /// `media.siteErrors.*` word for word; `http_status` has `{status}`.
    pub failure_kinds: &'static [(&'static str, &'static str)],
}

/// A failure kind in words, as the web view words it (`failureText` in
/// src/lib/errors.ts), or `None` for a kind this version does not know,
/// so the caller can show the engine's own message instead.
pub fn failure_text(texts: &Texts, kind: &str, raw: &str) -> Option<String> {
    let (_, text) = texts.failure_kinds.iter().find(|(k, _)| *k == kind)?;
    if kind != "http_status" {
        return Some(text.to_string());
    }
    Some(match status_in(raw) {
        Some(status) => fill(text, &[("status", status)]),
        None => texts.server_error.to_string(),
    })
}

/// The status an `http_status` message names ("unexpected HTTP status 502"):
/// a whole word of three digits from 100 to 599, like `\b[1-5]\d\d\b`.
fn status_in(raw: &str) -> Option<&str> {
    raw.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .find(|w| {
            w.len() == 3
                && w.bytes().all(|b| b.is_ascii_digit())
                && (b'1'..=b'5').contains(&w.as_bytes()[0])
        })
}

static EN: Texts = Texts {
    show: "Show Downpipe",
    pause_all: "Pause all",
    resume_all: "Resume all",
    quit: "Quit Downpipe",
    still_running_title: "Downpipe is still running",
    still_running_body: "Downloads continue and the browser can still send links. Quit from the tray icon.",
    all_done_title: "All downloads finished",
    all_done_quit: "Downpipe will quit in {secs} seconds. Open Downpipe to cancel.",
    all_done_sleep: "Downpipe will put this computer to sleep in {secs} seconds. Open Downpipe to cancel.",
    all_done_shutdown: "Downpipe will shut down this computer in {secs} seconds. Open Downpipe to cancel.",
    job_failed_title: "Downpipe could not finish the job",
    job_failed_body: "The system did not allow it: {err}",
    complete: "Download complete",
    failed: "Download failed",
    threat_title: "Threat found in a download",
    threat_body: "{scanner} found {threat} in {name}. Nothing was extracted or run, and the file was not deleted.",
    threat_unnamed: "a threat",
    schedule_started: "Scheduled downloads started",
    schedule_started_all: "Paused and failed downloads are running again.",
    schedule_started_queue: "Paused and failed downloads in {name} are running again.",
    schedule_stopped: "Scheduled stop",
    schedule_stopped_all: "All downloads are paused until the next start time.",
    schedule_stopped_queue: "Downloads in {name} are paused until the next start time.",
    tooltip_one: "Downpipe: 1 download, {speed}/s",
    tooltip_many: "Downpipe: {n} downloads, {speed}/s",
    server_error: "The server answered with an error.",
    failure_kinds: EN_FAILURES,
};

static TR: Texts = Texts {
    show: "Downpipe'ı göster",
    pause_all: "Tümünü duraklat",
    resume_all: "Tümünü sürdür",
    quit: "Downpipe'tan çık",
    still_running_title: "Downpipe hâlâ çalışıyor",
    still_running_body: "İndirmeler sürüyor ve tarayıcı bağlantı göndermeye devam edebilir. Çıkmak için tepsi simgesini kullanın.",
    all_done_title: "Tüm indirmeler bitti",
    all_done_quit: "Downpipe {secs} saniye içinde kapanacak. İptal etmek için Downpipe'ı açın.",
    all_done_sleep: "Downpipe bu bilgisayarı {secs} saniye içinde uyku moduna alacak. İptal etmek için Downpipe'ı açın.",
    all_done_shutdown: "Downpipe bu bilgisayarı {secs} saniye içinde kapatacak. İptal etmek için Downpipe'ı açın.",
    job_failed_title: "Downpipe işlemi tamamlayamadı",
    job_failed_body: "Sistem buna izin vermedi: {err}",
    complete: "İndirme tamamlandı",
    failed: "İndirme başarısız oldu",
    threat_title: "İndirmede tehdit bulundu",
    threat_body: "{scanner}, {name} içinde {threat} buldu. Hiçbir şey çıkarılmadı veya çalıştırılmadı, dosya silinmedi.",
    threat_unnamed: "bir tehdit",
    schedule_started: "Zamanlanmış indirmeler başladı",
    schedule_started_all: "Duraklatılan ve başarısız olan indirmeler yeniden çalışıyor.",
    schedule_started_queue: "{name} kuyruğundaki duraklatılan ve başarısız olan indirmeler yeniden çalışıyor.",
    schedule_stopped: "Zamanlanmış durdurma",
    schedule_stopped_all: "Tüm indirmeler bir sonraki başlama saatine kadar duraklatıldı.",
    schedule_stopped_queue: "{name} kuyruğundaki indirmeler bir sonraki başlama saatine kadar duraklatıldı.",
    tooltip_one: "Downpipe: 1 indirme, {speed}/s",
    tooltip_many: "Downpipe: {n} indirme, {speed}/s",
    server_error: "Sunucu bir hatayla yanıt verdi.",
    failure_kinds: TR_FAILURES,
};

static ES: Texts = Texts {
    show: "Mostrar Downpipe",
    pause_all: "Pausar todo",
    resume_all: "Reanudar todo",
    quit: "Salir de Downpipe",
    still_running_title: "Downpipe sigue en ejecución",
    still_running_body: "Las descargas continúan y el navegador aún puede enviar enlaces. Sal desde el icono de la bandeja.",
    all_done_title: "Todas las descargas han terminado",
    all_done_quit: "Downpipe se cerrará en {secs} segundos. Abre Downpipe para cancelar.",
    all_done_sleep: "Downpipe suspenderá este equipo en {secs} segundos. Abre Downpipe para cancelar.",
    all_done_shutdown: "Downpipe apagará este equipo en {secs} segundos. Abre Downpipe para cancelar.",
    job_failed_title: "Downpipe no pudo completar la tarea",
    job_failed_body: "El sistema no lo permitió: {err}",
    complete: "Descarga completada",
    failed: "Descarga fallida",
    threat_title: "Amenaza encontrada en una descarga",
    threat_body: "{scanner} encontró {threat} en {name}. No se extrajo ni se ejecutó nada, y el archivo no se eliminó.",
    threat_unnamed: "una amenaza",
    schedule_started: "Descargas programadas iniciadas",
    schedule_started_all: "Las descargas pausadas y fallidas se están ejecutando de nuevo.",
    schedule_started_queue: "Las descargas pausadas y fallidas de {name} se están ejecutando de nuevo.",
    schedule_stopped: "Parada programada",
    schedule_stopped_all: "Todas las descargas están en pausa hasta la próxima hora de inicio.",
    schedule_stopped_queue: "Las descargas de {name} están en pausa hasta la próxima hora de inicio.",
    tooltip_one: "Downpipe: 1 descarga, {speed}/s",
    tooltip_many: "Downpipe: {n} descargas, {speed}/s",
    server_error: "El servidor respondió con un error.",
    failure_kinds: ES_FAILURES,
};

static DE: Texts = Texts {
    show: "Downpipe anzeigen",
    pause_all: "Alle pausieren",
    resume_all: "Alle fortsetzen",
    quit: "Downpipe beenden",
    still_running_title: "Downpipe läuft weiter",
    still_running_body: "Downloads laufen weiter und der Browser kann weiterhin Links senden. Beenden über das Symbol im Infobereich.",
    all_done_title: "Alle Downloads abgeschlossen",
    all_done_quit: "Downpipe wird in {secs} Sekunden beendet. Öffnen Sie Downpipe, um abzubrechen.",
    all_done_sleep: "Downpipe versetzt diesen Computer in {secs} Sekunden in den Energiesparmodus. Öffnen Sie Downpipe, um abzubrechen.",
    all_done_shutdown: "Downpipe fährt diesen Computer in {secs} Sekunden herunter. Öffnen Sie Downpipe, um abzubrechen.",
    job_failed_title: "Downpipe konnte den Vorgang nicht abschließen",
    job_failed_body: "Das System hat es nicht zugelassen: {err}",
    complete: "Download abgeschlossen",
    failed: "Download fehlgeschlagen",
    threat_title: "Bedrohung in einem Download gefunden",
    threat_body: "{scanner} hat {threat} in {name} gefunden. Nichts wurde entpackt oder ausgeführt, und die Datei wurde nicht gelöscht.",
    threat_unnamed: "eine Bedrohung",
    schedule_started: "Geplante Downloads gestartet",
    schedule_started_all: "Pausierte und fehlgeschlagene Downloads laufen wieder.",
    schedule_started_queue: "Pausierte und fehlgeschlagene Downloads in {name} laufen wieder.",
    schedule_stopped: "Geplanter Stopp",
    schedule_stopped_all: "Alle Downloads sind bis zur nächsten Startzeit pausiert.",
    schedule_stopped_queue: "Downloads in {name} sind bis zur nächsten Startzeit pausiert.",
    tooltip_one: "Downpipe: 1 Download, {speed}/s",
    tooltip_many: "Downpipe: {n} Downloads, {speed}/s",
    server_error: "Der Server hat mit einem Fehler geantwortet.",
    failure_kinds: DE_FAILURES,
};

static FR: Texts = Texts {
    show: "Afficher Downpipe",
    pause_all: "Tout mettre en pause",
    resume_all: "Tout reprendre",
    quit: "Quitter Downpipe",
    still_running_title: "Downpipe est toujours en cours d'exécution",
    still_running_body: "Les téléchargements continuent et le navigateur peut toujours envoyer des liens. Quittez depuis l'icône de la zone de notification.",
    all_done_title: "Tous les téléchargements sont terminés",
    all_done_quit: "Downpipe va se fermer dans {secs} secondes. Ouvrez Downpipe pour annuler.",
    all_done_sleep: "Downpipe va mettre cet ordinateur en veille dans {secs} secondes. Ouvrez Downpipe pour annuler.",
    all_done_shutdown: "Downpipe va éteindre cet ordinateur dans {secs} secondes. Ouvrez Downpipe pour annuler.",
    job_failed_title: "Downpipe n'a pas pu terminer l'opération",
    job_failed_body: "Le système ne l'a pas permis : {err}",
    complete: "Téléchargement terminé",
    failed: "Échec du téléchargement",
    threat_title: "Menace détectée dans un téléchargement",
    threat_body: "{scanner} a détecté {threat} dans {name}. Rien n'a été extrait ni exécuté, et le fichier n'a pas été supprimé.",
    threat_unnamed: "une menace",
    schedule_started: "Téléchargements programmés démarrés",
    schedule_started_all: "Les téléchargements en pause et en échec reprennent.",
    schedule_started_queue: "Les téléchargements en pause et en échec de {name} reprennent.",
    schedule_stopped: "Arrêt programmé",
    schedule_stopped_all: "Tous les téléchargements sont en pause jusqu'à la prochaine heure de début.",
    schedule_stopped_queue: "Les téléchargements de {name} sont en pause jusqu'à la prochaine heure de début.",
    tooltip_one: "Downpipe : 1 téléchargement, {speed}/s",
    tooltip_many: "Downpipe : {n} téléchargements, {speed}/s",
    server_error: "Le serveur a répondu par une erreur.",
    failure_kinds: FR_FAILURES,
};

static IT: Texts = Texts {
    show: "Mostra Downpipe",
    pause_all: "Metti tutto in pausa",
    resume_all: "Riprendi tutto",
    quit: "Esci da Downpipe",
    still_running_title: "Downpipe è ancora in esecuzione",
    still_running_body: "I download continuano e il browser può ancora inviare link. Esci dall'icona nell'area di notifica.",
    all_done_title: "Tutti i download sono terminati",
    all_done_quit: "Downpipe si chiuderà tra {secs} secondi. Apri Downpipe per annullare.",
    all_done_sleep: "Downpipe metterà in sospensione questo computer tra {secs} secondi. Apri Downpipe per annullare.",
    all_done_shutdown: "Downpipe spegnerà questo computer tra {secs} secondi. Apri Downpipe per annullare.",
    job_failed_title: "Downpipe non è riuscito a completare l'operazione",
    job_failed_body: "Il sistema non lo ha consentito: {err}",
    complete: "Download completato",
    failed: "Download non riuscito",
    threat_title: "Minaccia trovata in un download",
    threat_body: "{scanner} ha trovato {threat} in {name}. Non è stato estratto né eseguito nulla e il file non è stato eliminato.",
    threat_unnamed: "una minaccia",
    schedule_started: "Download programmati avviati",
    schedule_started_all: "I download in pausa e non riusciti sono di nuovo in corso.",
    schedule_started_queue: "I download in pausa e non riusciti in {name} sono di nuovo in corso.",
    schedule_stopped: "Arresto programmato",
    schedule_stopped_all: "Tutti i download sono in pausa fino al prossimo orario di avvio.",
    schedule_stopped_queue: "I download in {name} sono in pausa fino al prossimo orario di avvio.",
    tooltip_one: "Downpipe: 1 download, {speed}/s",
    tooltip_many: "Downpipe: {n} download, {speed}/s",
    server_error: "Il server ha risposto con un errore.",
    failure_kinds: IT_FAILURES,
};

static PT_BR: Texts = Texts {
    show: "Mostrar o Downpipe",
    pause_all: "Pausar tudo",
    resume_all: "Retomar tudo",
    quit: "Sair do Downpipe",
    still_running_title: "O Downpipe continua em execução",
    still_running_body: "Os downloads continuam e o navegador ainda pode enviar links. Saia pelo ícone na bandeja do sistema.",
    all_done_title: "Todos os downloads foram concluídos",
    all_done_quit: "O Downpipe será fechado em {secs} segundos. Abra o Downpipe para cancelar.",
    all_done_sleep: "O Downpipe colocará este computador em suspensão em {secs} segundos. Abra o Downpipe para cancelar.",
    all_done_shutdown: "O Downpipe desligará este computador em {secs} segundos. Abra o Downpipe para cancelar.",
    job_failed_title: "O Downpipe não conseguiu concluir a tarefa",
    job_failed_body: "O sistema não permitiu: {err}",
    complete: "Download concluído",
    failed: "Falha no download",
    threat_title: "Ameaça encontrada em um download",
    threat_body: "{scanner} encontrou {threat} em {name}. Nada foi extraído nem executado, e o arquivo não foi excluído.",
    threat_unnamed: "uma ameaça",
    schedule_started: "Downloads agendados iniciados",
    schedule_started_all: "Os downloads pausados e com falha estão em andamento novamente.",
    schedule_started_queue: "Os downloads pausados e com falha em {name} estão em andamento novamente.",
    schedule_stopped: "Parada agendada",
    schedule_stopped_all: "Todos os downloads estão pausados até o próximo horário de início.",
    schedule_stopped_queue: "Os downloads em {name} estão pausados até o próximo horário de início.",
    tooltip_one: "Downpipe: 1 download, {speed}/s",
    tooltip_many: "Downpipe: {n} downloads, {speed}/s",
    server_error: "O servidor respondeu com um erro.",
    failure_kinds: PT_BR_FAILURES,
};

static RU: Texts = Texts {
    show: "Показать Downpipe",
    pause_all: "Приостановить все",
    resume_all: "Возобновить все",
    quit: "Выйти из Downpipe",
    still_running_title: "Downpipe продолжает работать",
    still_running_body: "Загрузки продолжаются, и браузер по-прежнему может передавать ссылки. Выйти можно через значок в области уведомлений.",
    all_done_title: "Все загрузки завершены",
    all_done_quit: "Downpipe закроется через {secs} с. Откройте Downpipe, чтобы отменить.",
    all_done_sleep: "Downpipe переведёт компьютер в спящий режим через {secs} с. Откройте Downpipe, чтобы отменить.",
    all_done_shutdown: "Downpipe выключит компьютер через {secs} с. Откройте Downpipe, чтобы отменить.",
    job_failed_title: "Downpipe не удалось выполнить действие",
    job_failed_body: "Система не разрешила это: {err}",
    complete: "Загрузка завершена",
    failed: "Ошибка загрузки",
    threat_title: "В загрузке найдена угроза",
    threat_body: "{scanner} обнаружил {threat} в {name}. Ничего не было извлечено или запущено, файл не удалён.",
    threat_unnamed: "угрозу",
    schedule_started: "Запланированные загрузки запущены",
    schedule_started_all: "Приостановленные и неудавшиеся загрузки снова выполняются.",
    schedule_started_queue: "Приостановленные и неудавшиеся загрузки в очереди «{name}» снова выполняются.",
    schedule_stopped: "Запланированная остановка",
    schedule_stopped_all: "Все загрузки приостановлены до следующего времени запуска.",
    schedule_stopped_queue: "Загрузки в очереди «{name}» приостановлены до следующего времени запуска.",
    tooltip_one: "Downpipe: загрузок: 1, {speed}/s",
    tooltip_many: "Downpipe: загрузок: {n}, {speed}/s",
    server_error: "Сервер ответил ошибкой.",
    failure_kinds: RU_FAILURES,
};

static ZH_CN: Texts = Texts {
    show: "显示 Downpipe",
    pause_all: "全部暂停",
    resume_all: "全部继续",
    quit: "退出 Downpipe",
    still_running_title: "Downpipe 仍在运行",
    still_running_body: "下载会继续进行，浏览器仍可发送链接。可从托盘图标退出。",
    all_done_title: "所有下载已完成",
    all_done_quit: "Downpipe 将在 {secs} 秒后退出。打开 Downpipe 可取消。",
    all_done_sleep: "Downpipe 将在 {secs} 秒后让此电脑进入睡眠状态。打开 Downpipe 可取消。",
    all_done_shutdown: "Downpipe 将在 {secs} 秒后关闭此电脑。打开 Downpipe 可取消。",
    job_failed_title: "Downpipe 无法完成此操作",
    job_failed_body: "系统不允许：{err}",
    complete: "下载完成",
    failed: "下载失败",
    threat_title: "下载中发现威胁",
    threat_body: "{scanner} 在 {name} 中发现了{threat}。未解压或运行任何内容，文件也未被删除。",
    threat_unnamed: "威胁",
    schedule_started: "计划的下载已开始",
    schedule_started_all: "已暂停和失败的下载正在重新运行。",
    schedule_started_queue: "{name} 中已暂停和失败的下载正在重新运行。",
    schedule_stopped: "计划停止",
    schedule_stopped_all: "所有下载已暂停，直到下一个开始时间。",
    schedule_stopped_queue: "{name} 中的下载已暂停，直到下一个开始时间。",
    tooltip_one: "Downpipe：1 个下载，{speed}/s",
    tooltip_many: "Downpipe：{n} 个下载，{speed}/s",
    server_error: "服务器返回了错误。",
    failure_kinds: ZH_CN_FAILURES,
};

static JA: Texts = Texts {
    show: "Downpipe を表示",
    pause_all: "すべて一時停止",
    resume_all: "すべて再開",
    quit: "Downpipe を終了",
    still_running_title: "Downpipe は実行中です",
    still_running_body: "ダウンロードは続行され、ブラウザーからリンクを送ることもできます。終了するにはトレイアイコンを使ってください。",
    all_done_title: "すべてのダウンロードが完了しました",
    all_done_quit: "Downpipe は {secs} 秒後に終了します。キャンセルするには Downpipe を開いてください。",
    all_done_sleep: "Downpipe は {secs} 秒後にこのコンピューターをスリープさせます。キャンセルするには Downpipe を開いてください。",
    all_done_shutdown: "Downpipe は {secs} 秒後にこのコンピューターをシャットダウンします。キャンセルするには Downpipe を開いてください。",
    job_failed_title: "Downpipe は処理を完了できませんでした",
    job_failed_body: "システムが許可しませんでした: {err}",
    complete: "ダウンロード完了",
    failed: "ダウンロード失敗",
    threat_title: "ダウンロードで脅威が見つかりました",
    threat_body: "{scanner} が {name} で{threat}を検出しました。展開や実行は行われず、ファイルも削除されていません。",
    threat_unnamed: "脅威",
    schedule_started: "スケジュールされたダウンロードを開始しました",
    schedule_started_all: "一時停止中と失敗したダウンロードを再開しました。",
    schedule_started_queue: "{name} の一時停止中と失敗したダウンロードを再開しました。",
    schedule_stopped: "スケジュールによる停止",
    schedule_stopped_all: "次の開始時刻まですべてのダウンロードを一時停止しました。",
    schedule_stopped_queue: "次の開始時刻まで {name} のダウンロードを一時停止しました。",
    tooltip_one: "Downpipe: 1 件のダウンロード、{speed}/s",
    tooltip_many: "Downpipe: {n} 件のダウンロード、{speed}/s",
    server_error: "サーバーがエラーを返しました。",
    failure_kinds: JA_FAILURES,
};

static EN_FAILURES: &[(&str, &str)] = &[
    (
        "invalid_url",
        "This link is not a valid http or https address.",
    ),
    (
        "invalid_header",
        "One of this download's custom headers is not valid.",
    ),
    (
        "auth_required",
        "The server asks for a user name and password.",
    ),
    (
        "forbidden",
        "The server refused the download. The link may have expired, or the site may need you to sign in.",
    ),
    (
        "not_found",
        "The file was not found on the server. Check the link or use a new one.",
    ),
    (
        "throttled",
        "The server is limiting downloads. Try again later or with fewer connections.",
    ),
    (
        "http_status",
        "The server answered with an error (HTTP {status}).",
    ),
    (
        "resource_changed",
        "The file on the server changed during the download. Download it again from the start.",
    ),
    (
        "different_file",
        "The new link leads to a different file. The downloaded part was kept.",
    ),
    (
        "expired_link",
        "The link has expired: the server sent a web page instead of the file. Use a new link; the downloaded part is kept.",
    ),
    (
        "login_required",
        "The server asks you to sign in. Sign in in your browser, then use a new link; the downloaded part is kept.",
    ),
    (
        "private_file",
        "The file is not shared publicly. Ask its owner to share it with anyone who has the link, or download it in your browser while signed in.",
    ),
    (
        "range_not_honoured",
        "The server stopped sending the file in parts. Try again with one connection.",
    ),
    (
        "network",
        "The connection to the server failed. Check your network and try again.",
    ),
    (
        "checksum",
        "The downloaded file does not match its checksum. It may be damaged; download it again.",
    ),
    (
        "io",
        "The file could not be written. Check that the folder exists and the disk has free space.",
    ),
    (
        "corrupt_meta",
        "The resume data of this download is damaged. Download it again from the start.",
    ),
    (
        "drm_protected",
        "This stream is protected by DRM. Downpipe does not download protected media.",
    ),
    (
        "live_stream",
        "This is a live stream. Only finished streams can be downloaded.",
    ),
    (
        "invalid_manifest",
        "The stream's playlist could not be read.",
    ),
    ("unsupported_media", "This kind of stream is not supported."),
    (
        "tool_missing",
        "A program this download needs (yt-dlp, Deno or FFmpeg) is not set up. Set it up under Preferences, Media.",
    ),
    (
        "unavailable",
        "This video is unavailable: it is private, removed, members-only or blocked in your region.",
    ),
    ("extractor", "yt-dlp could not read this video page."),
    (
        "site_forbidden",
        "YouTube refused the download. A newer yt-dlp usually fixes this: update it in Settings, Media, then try again.",
    ),
    (
        "site_sign_in",
        "YouTube wants a signed-in browser for this download. Downpipe does not use browser cookies: wait a while or use another network, then try again.",
    ),
    (
        "site_age_restricted",
        "YouTube shows this video only to signed-in adults. Downpipe does not use accounts or browser cookies, so it cannot download it.",
    ),
    (
        "site_js_runtime",
        "yt-dlp could not answer YouTube's JavaScript challenges. Download Deno in Settings, Media, then try again.",
    ),
    (
        "site_po_token",
        "YouTube asked for a proof-of-origin (PO) token for this video. Update yt-dlp in Settings, Media, or choose another quality.",
    ),
    (
        "site_outdated",
        "yt-dlp no longer understands this page; YouTube probably changed it. Update yt-dlp in Settings, Media, then try again.",
    ),
];

static TR_FAILURES: &[(&str, &str)] = &[
    (
        "invalid_url",
        "Bu bağlantı geçerli bir http veya https adresi değil.",
    ),
    (
        "invalid_header",
        "Bu indirmenin özel başlıklarından biri geçersiz.",
    ),
    ("auth_required", "Sunucu kullanıcı adı ve parola istiyor."),
    (
        "forbidden",
        "Sunucu indirmeyi reddetti. Bağlantının süresi dolmuş olabilir ya da site oturum açmanızı istiyor olabilir.",
    ),
    (
        "not_found",
        "Dosya sunucuda bulunamadı. Bağlantıyı denetleyin ya da yeni bir bağlantı kullanın.",
    ),
    (
        "throttled",
        "Sunucu indirmeleri sınırlıyor. Daha sonra ya da daha az bağlantıyla yeniden deneyin.",
    ),
    (
        "http_status",
        "Sunucu bir hatayla yanıt verdi (HTTP {status}).",
    ),
    (
        "resource_changed",
        "Sunucudaki dosya indirme sırasında değişti. Dosyayı baştan yeniden indirin.",
    ),
    (
        "different_file",
        "Yeni bağlantı başka bir dosyaya gidiyor. İndirilen kısım korundu.",
    ),
    (
        "expired_link",
        "Bağlantının süresi dolmuş: sunucu dosya yerine bir web sayfası gönderdi. Yeni bir bağlantı kullanın; indirilen kısım korunur.",
    ),
    (
        "login_required",
        "Sunucu oturum açmanızı istiyor. Tarayıcıda oturum açın, ardından yeni bir bağlantı kullanın; indirilen kısım korunur.",
    ),
    (
        "private_file",
        "Dosya herkese açık paylaşılmamış. Sahibinden bağlantıya sahip herkesle paylaşmasını isteyin ya da oturum açıkken tarayıcınızda indirin.",
    ),
    (
        "range_not_honoured",
        "Sunucu dosyayı parça parça göndermeyi bıraktı. Tek bağlantıyla yeniden deneyin.",
    ),
    (
        "network",
        "Sunucuyla bağlantı kurulamadı. Ağınızı denetleyip yeniden deneyin.",
    ),
    (
        "checksum",
        "İndirilen dosya sağlama toplamıyla eşleşmiyor. Dosya bozulmuş olabilir; yeniden indirin.",
    ),
    (
        "io",
        "Dosya yazılamadı. Klasörün var olduğunu ve diskte boş yer olduğunu denetleyin.",
    ),
    (
        "corrupt_meta",
        "Bu indirmenin sürdürme verisi bozuk. Dosyayı baştan yeniden indirin.",
    ),
    (
        "drm_protected",
        "Bu akış DRM ile korunuyor. Downpipe korumalı medya indirmez.",
    ),
    (
        "live_stream",
        "Bu bir canlı yayın. Yalnızca bitmiş akışlar indirilebilir.",
    ),
    ("invalid_manifest", "Akışın oynatma listesi okunamadı."),
    ("unsupported_media", "Bu tür akış desteklenmiyor."),
    (
        "tool_missing",
        "Bu indirmenin gerektirdiği bir program (yt-dlp, Deno ya da FFmpeg) kurulu değil. Tercihler'de Medya bölümünden kurun.",
    ),
    (
        "unavailable",
        "Bu video kullanılamıyor: gizli, kaldırılmış, yalnızca üyelere açık ya da bölgenizde engellenmiş.",
    ),
    ("extractor", "yt-dlp bu video sayfasını okuyamadı."),
    (
        "site_forbidden",
        "YouTube indirmeyi reddetti. Genellikle daha yeni bir yt-dlp bunu çözer: Ayarlar'ın Medya sekmesinden güncelleyip yeniden deneyin.",
    ),
    (
        "site_sign_in",
        "YouTube bu indirme için oturum açılmış bir tarayıcı istiyor. Downpipe tarayıcı çerezlerini kullanmaz: biraz bekleyin ya da başka bir ağ kullanıp yeniden deneyin.",
    ),
    (
        "site_age_restricted",
        "YouTube bu videoyu yalnızca oturum açmış yetişkinlere gösteriyor. Downpipe hesap ya da tarayıcı çerezi kullanmadığı için indiremez.",
    ),
    (
        "site_js_runtime",
        "yt-dlp, YouTube'un JavaScript sınamalarını yanıtlayamadı. Ayarlar'ın Medya sekmesinden Deno'yu indirip yeniden deneyin.",
    ),
    (
        "site_po_token",
        "YouTube bu video için bir kaynak kanıtı (PO) belirteci istedi. Ayarlar'ın Medya sekmesinden yt-dlp'yi güncelleyin ya da başka bir kalite seçin.",
    ),
    (
        "site_outdated",
        "yt-dlp bu sayfayı artık anlamıyor; YouTube muhtemelen sayfayı değiştirdi. Ayarlar'ın Medya sekmesinden yt-dlp'yi güncelleyip yeniden deneyin.",
    ),
];

static ES_FAILURES: &[(&str, &str)] = &[
    (
        "invalid_url",
        "Este enlace no es una dirección http o https válida.",
    ),
    (
        "invalid_header",
        "Uno de los encabezados personalizados de esta descarga no es válido.",
    ),
    (
        "auth_required",
        "El servidor pide un nombre de usuario y una contraseña.",
    ),
    (
        "forbidden",
        "El servidor rechazó la descarga. Puede que el enlace haya caducado o que el sitio te pida iniciar sesión.",
    ),
    (
        "not_found",
        "No se encontró el archivo en el servidor. Comprueba el enlace o usa uno nuevo.",
    ),
    (
        "throttled",
        "El servidor está limitando las descargas. Vuelve a intentarlo más tarde o con menos conexiones.",
    ),
    (
        "http_status",
        "El servidor respondió con un error (HTTP {status}).",
    ),
    (
        "resource_changed",
        "El archivo del servidor cambió durante la descarga. Descárgalo de nuevo desde el principio.",
    ),
    (
        "different_file",
        "El enlace nuevo lleva a otro archivo. La parte descargada se conservó.",
    ),
    (
        "expired_link",
        "El enlace ha caducado: el servidor envió una página web en lugar del archivo. Usa un enlace nuevo; la parte descargada se conserva.",
    ),
    (
        "login_required",
        "El servidor pide iniciar sesión. Inicia sesión en el navegador y luego usa un enlace nuevo; la parte descargada se conserva.",
    ),
    (
        "private_file",
        "El archivo no se ha compartido públicamente. Pide a su propietario que lo comparta con cualquiera que tenga el enlace, o descárgalo en el navegador con tu sesión iniciada.",
    ),
    (
        "range_not_honoured",
        "El servidor dejó de enviar el archivo por partes. Vuelve a intentarlo con una sola conexión.",
    ),
    (
        "network",
        "Falló la conexión con el servidor. Comprueba tu red y vuelve a intentarlo.",
    ),
    (
        "checksum",
        "El archivo descargado no coincide con su suma de comprobación. Puede estar dañado; descárgalo de nuevo.",
    ),
    (
        "io",
        "No se pudo escribir el archivo. Comprueba que la carpeta existe y que el disco tiene espacio libre.",
    ),
    (
        "corrupt_meta",
        "Los datos para reanudar esta descarga están dañados. Descárgala de nuevo desde el principio.",
    ),
    (
        "drm_protected",
        "Esta transmisión está protegida con DRM. Downpipe no descarga contenido protegido.",
    ),
    (
        "live_stream",
        "Es una transmisión en directo. Solo se pueden descargar transmisiones terminadas.",
    ),
    (
        "invalid_manifest",
        "No se pudo leer la lista de reproducción de la transmisión.",
    ),
    (
        "unsupported_media",
        "Este tipo de transmisión no es compatible.",
    ),
    (
        "tool_missing",
        "Falta un programa que esta descarga necesita (yt-dlp, Deno o FFmpeg). Instálalo en Preferencias, Multimedia.",
    ),
    (
        "unavailable",
        "Este video no está disponible: es privado, se eliminó, es exclusivo para miembros o está bloqueado en tu región.",
    ),
    ("extractor", "yt-dlp no pudo leer esta página de video."),
    (
        "site_forbidden",
        "YouTube rechazó la descarga. Una versión más reciente de yt-dlp suele solucionarlo: actualízala en Configuración, Multimedia y vuelve a intentarlo.",
    ),
    (
        "site_sign_in",
        "YouTube pide un navegador con la sesión iniciada para esta descarga. Downpipe no usa las cookies del navegador: espera un rato o usa otra red y vuelve a intentarlo.",
    ),
    (
        "site_age_restricted",
        "YouTube muestra este video solo a adultos con la sesión iniciada. Downpipe no usa cuentas ni cookies del navegador, así que no puede descargarlo.",
    ),
    (
        "site_js_runtime",
        "yt-dlp no pudo responder a los desafíos de JavaScript de YouTube. Descarga Deno en Configuración, Multimedia y vuelve a intentarlo.",
    ),
    (
        "site_po_token",
        "YouTube pidió un token de prueba de origen (PO) para este video. Actualiza yt-dlp en Configuración, Multimedia o elige otra calidad.",
    ),
    (
        "site_outdated",
        "yt-dlp ya no entiende esta página; probablemente YouTube la cambió. Actualiza yt-dlp en Configuración, Multimedia y vuelve a intentarlo.",
    ),
];

static DE_FAILURES: &[(&str, &str)] = &[
    (
        "invalid_url",
        "Dieser Link ist keine gültige http- oder https-Adresse.",
    ),
    (
        "invalid_header",
        "Einer der eigenen Header dieses Downloads ist ungültig.",
    ),
    (
        "auth_required",
        "Der Server verlangt einen Benutzernamen und ein Passwort.",
    ),
    (
        "forbidden",
        "Der Server hat den Download abgelehnt. Der Link ist vielleicht abgelaufen, oder die Website verlangt eine Anmeldung.",
    ),
    (
        "not_found",
        "Die Datei wurde auf dem Server nicht gefunden. Prüfen Sie den Link oder verwenden Sie einen neuen.",
    ),
    (
        "throttled",
        "Der Server begrenzt Downloads. Versuchen Sie es später oder mit weniger Verbindungen erneut.",
    ),
    (
        "http_status",
        "Der Server hat mit einem Fehler geantwortet (HTTP {status}).",
    ),
    (
        "resource_changed",
        "Die Datei auf dem Server hat sich während des Downloads geändert. Laden Sie sie von vorn herunter.",
    ),
    (
        "different_file",
        "Der neue Link führt zu einer anderen Datei. Der heruntergeladene Teil wurde behalten.",
    ),
    (
        "expired_link",
        "Der Link ist abgelaufen: Der Server hat statt der Datei eine Webseite geschickt. Verwenden Sie einen neuen Link; der heruntergeladene Teil bleibt erhalten.",
    ),
    (
        "login_required",
        "Der Server verlangt eine Anmeldung. Melden Sie sich im Browser an und verwenden Sie dann einen neuen Link; der heruntergeladene Teil bleibt erhalten.",
    ),
    (
        "private_file",
        "Die Datei ist nicht öffentlich freigegeben. Bitten Sie den Eigentümer, sie für alle mit dem Link freizugeben, oder laden Sie sie angemeldet im Browser herunter.",
    ),
    (
        "range_not_honoured",
        "Der Server sendet die Datei nicht mehr in Teilen. Versuchen Sie es mit einer Verbindung erneut.",
    ),
    (
        "network",
        "Die Verbindung zum Server ist fehlgeschlagen. Prüfen Sie Ihr Netzwerk und versuchen Sie es erneut.",
    ),
    (
        "checksum",
        "Die heruntergeladene Datei passt nicht zu ihrer Prüfsumme. Sie ist vielleicht beschädigt; laden Sie sie erneut herunter.",
    ),
    (
        "io",
        "Die Datei konnte nicht geschrieben werden. Prüfen Sie, ob der Ordner existiert und auf dem Datenträger Platz frei ist.",
    ),
    (
        "corrupt_meta",
        "Die Fortsetzungsdaten dieses Downloads sind beschädigt. Laden Sie die Datei von vorn herunter.",
    ),
    (
        "drm_protected",
        "Dieser Stream ist mit DRM geschützt. Downpipe lädt keine geschützten Medien herunter.",
    ),
    (
        "live_stream",
        "Das ist ein Livestream. Nur abgeschlossene Streams können heruntergeladen werden.",
    ),
    (
        "invalid_manifest",
        "Die Wiedergabeliste des Streams konnte nicht gelesen werden.",
    ),
    (
        "unsupported_media",
        "Diese Art von Stream wird nicht unterstützt.",
    ),
    (
        "tool_missing",
        "Ein Programm, das dieser Download braucht (yt-dlp, Deno oder FFmpeg), ist nicht eingerichtet. Richten Sie es unter Optionen, Medien ein.",
    ),
    (
        "unavailable",
        "Dieses Video ist nicht verfügbar: Es ist privat, entfernt, nur für Mitglieder oder in Ihrer Region gesperrt.",
    ),
    ("extractor", "yt-dlp konnte diese Videoseite nicht lesen."),
    (
        "site_forbidden",
        "YouTube hat den Download abgelehnt. Ein neueres yt-dlp behebt das meist: Aktualisieren Sie es unter Einstellungen, Medien, und versuchen Sie es erneut.",
    ),
    (
        "site_sign_in",
        "YouTube verlangt für diesen Download einen angemeldeten Browser. Downpipe verwendet keine Browser-Cookies: Warten Sie eine Weile oder nutzen Sie ein anderes Netzwerk, und versuchen Sie es erneut.",
    ),
    (
        "site_age_restricted",
        "YouTube zeigt dieses Video nur angemeldeten Erwachsenen. Downpipe verwendet weder Konten noch Browser-Cookies und kann es daher nicht herunterladen.",
    ),
    (
        "site_js_runtime",
        "yt-dlp konnte die JavaScript-Abfragen von YouTube nicht beantworten. Laden Sie Deno unter Einstellungen, Medien herunter, und versuchen Sie es erneut.",
    ),
    (
        "site_po_token",
        "YouTube hat für dieses Video ein Herkunftsnachweis-Token (PO-Token) verlangt. Aktualisieren Sie yt-dlp unter Einstellungen, Medien, oder wählen Sie eine andere Qualität.",
    ),
    (
        "site_outdated",
        "yt-dlp versteht diese Seite nicht mehr; YouTube hat sie wahrscheinlich geändert. Aktualisieren Sie yt-dlp unter Einstellungen, Medien, und versuchen Sie es erneut.",
    ),
];

static FR_FAILURES: &[(&str, &str)] = &[
    (
        "invalid_url",
        "Ce lien n'est pas une adresse http ou https valide.",
    ),
    (
        "invalid_header",
        "L'un des en-têtes personnalisés de ce téléchargement n'est pas valide.",
    ),
    (
        "auth_required",
        "Le serveur demande un nom d'utilisateur et un mot de passe.",
    ),
    (
        "forbidden",
        "Le serveur a refusé le téléchargement. Le lien a peut-être expiré, ou le site demande de vous connecter.",
    ),
    (
        "not_found",
        "Le fichier est introuvable sur le serveur. Vérifiez le lien ou utilisez-en un nouveau.",
    ),
    (
        "throttled",
        "Le serveur limite les téléchargements. Réessayez plus tard ou avec moins de connexions.",
    ),
    (
        "http_status",
        "Le serveur a répondu par une erreur (HTTP {status}).",
    ),
    (
        "resource_changed",
        "Le fichier a changé sur le serveur pendant le téléchargement. Téléchargez-le à nouveau depuis le début.",
    ),
    (
        "different_file",
        "Le nouveau lien mène à un autre fichier. La partie téléchargée a été conservée.",
    ),
    (
        "expired_link",
        "Le lien a expiré : le serveur a envoyé une page web au lieu du fichier. Utilisez un nouveau lien ; la partie téléchargée est conservée.",
    ),
    (
        "login_required",
        "Le serveur demande de vous connecter. Connectez-vous dans le navigateur, puis utilisez un nouveau lien ; la partie téléchargée est conservée.",
    ),
    (
        "private_file",
        "Le fichier n'est pas partagé publiquement. Demandez à son propriétaire de le partager avec toute personne disposant du lien, ou téléchargez-le dans votre navigateur en étant connecté.",
    ),
    (
        "range_not_honoured",
        "Le serveur n'envoie plus le fichier par morceaux. Réessayez avec une seule connexion.",
    ),
    (
        "network",
        "La connexion au serveur a échoué. Vérifiez votre réseau et réessayez.",
    ),
    (
        "checksum",
        "Le fichier téléchargé ne correspond pas à sa somme de contrôle. Il est peut-être endommagé ; téléchargez-le à nouveau.",
    ),
    (
        "io",
        "Le fichier n'a pas pu être écrit. Vérifiez que le dossier existe et que le disque a de l'espace libre.",
    ),
    (
        "corrupt_meta",
        "Les données de reprise de ce téléchargement sont endommagées. Téléchargez-le à nouveau depuis le début.",
    ),
    (
        "drm_protected",
        "Ce flux est protégé par DRM. Downpipe ne télécharge pas les médias protégés.",
    ),
    (
        "live_stream",
        "C'est un direct. Seuls les flux terminés peuvent être téléchargés.",
    ),
    (
        "invalid_manifest",
        "La liste de lecture du flux n'a pas pu être lue.",
    ),
    (
        "unsupported_media",
        "Ce type de flux n'est pas pris en charge.",
    ),
    (
        "tool_missing",
        "Un programme dont ce téléchargement a besoin (yt-dlp, Deno ou FFmpeg) n'est pas installé. Installez-le dans Préférences, Médias.",
    ),
    (
        "unavailable",
        "Cette vidéo n'est pas disponible : elle est privée, supprimée, réservée aux membres ou bloquée dans votre région.",
    ),
    ("extractor", "yt-dlp n'a pas pu lire cette page vidéo."),
    (
        "site_forbidden",
        "YouTube a refusé le téléchargement. Une version plus récente de yt-dlp règle souvent le problème : mettez-la à jour dans Paramètres, Médias, puis réessayez.",
    ),
    (
        "site_sign_in",
        "YouTube demande un navigateur connecté pour ce téléchargement. Downpipe n'utilise pas les cookies du navigateur : patientez un moment ou utilisez un autre réseau, puis réessayez.",
    ),
    (
        "site_age_restricted",
        "YouTube ne montre cette vidéo qu'aux adultes connectés. Downpipe n'utilise ni compte ni cookies du navigateur ; il ne peut donc pas la télécharger.",
    ),
    (
        "site_js_runtime",
        "yt-dlp n'a pas pu répondre aux défis JavaScript de YouTube. Téléchargez Deno dans Paramètres, Médias, puis réessayez.",
    ),
    (
        "site_po_token",
        "YouTube a demandé un jeton de preuve d'origine (PO) pour cette vidéo. Mettez à jour yt-dlp dans Paramètres, Médias, ou choisissez une autre qualité.",
    ),
    (
        "site_outdated",
        "yt-dlp ne comprend plus cette page ; YouTube l'a probablement modifiée. Mettez à jour yt-dlp dans Paramètres, Médias, puis réessayez.",
    ),
];

static IT_FAILURES: &[(&str, &str)] = &[
    (
        "invalid_url",
        "Questo link non è un indirizzo http o https valido.",
    ),
    (
        "invalid_header",
        "Una delle intestazioni personalizzate di questo download non è valida.",
    ),
    (
        "auth_required",
        "Il server chiede un nome utente e una password.",
    ),
    (
        "forbidden",
        "Il server ha rifiutato il download. Il link potrebbe essere scaduto o il sito potrebbe chiederti di accedere.",
    ),
    (
        "not_found",
        "Il file non è stato trovato sul server. Controlla il link o usane uno nuovo.",
    ),
    (
        "throttled",
        "Il server sta limitando i download. Riprova più tardi o con meno connessioni.",
    ),
    (
        "http_status",
        "Il server ha risposto con un errore (HTTP {status}).",
    ),
    (
        "resource_changed",
        "Il file sul server è cambiato durante il download. Scaricalo di nuovo dall'inizio.",
    ),
    (
        "different_file",
        "Il nuovo link porta a un file diverso. La parte scaricata è stata conservata.",
    ),
    (
        "expired_link",
        "Il link è scaduto: il server ha inviato una pagina web invece del file. Usa un nuovo link; la parte scaricata viene conservata.",
    ),
    (
        "login_required",
        "Il server chiede di accedere. Accedi nel browser, poi usa un nuovo link; la parte scaricata viene conservata.",
    ),
    (
        "private_file",
        "Il file non è condiviso pubblicamente. Chiedi al proprietario di condividerlo con chiunque abbia il link, oppure scaricalo nel browser dopo aver effettuato l'accesso.",
    ),
    (
        "range_not_honoured",
        "Il server ha smesso di inviare il file a parti. Riprova con una sola connessione.",
    ),
    (
        "network",
        "La connessione al server non è riuscita. Controlla la rete e riprova.",
    ),
    (
        "checksum",
        "Il file scaricato non corrisponde al suo checksum. Potrebbe essere danneggiato; scaricalo di nuovo.",
    ),
    (
        "io",
        "Non è stato possibile scrivere il file. Controlla che la cartella esista e che il disco abbia spazio libero.",
    ),
    (
        "corrupt_meta",
        "I dati di ripresa di questo download sono danneggiati. Scaricalo di nuovo dall'inizio.",
    ),
    (
        "drm_protected",
        "Questo stream è protetto da DRM. Downpipe non scarica contenuti protetti.",
    ),
    (
        "live_stream",
        "Questa è una diretta. Si possono scaricare solo gli stream terminati.",
    ),
    (
        "invalid_manifest",
        "Non è stato possibile leggere la playlist dello stream.",
    ),
    (
        "unsupported_media",
        "Questo tipo di stream non è supportato.",
    ),
    (
        "tool_missing",
        "Manca un programma che serve a questo download (yt-dlp, Deno o FFmpeg). Installalo in Preferenze, Media.",
    ),
    (
        "unavailable",
        "Questo video non è disponibile: è privato, rimosso, riservato agli abbonati o bloccato nella tua zona.",
    ),
    (
        "extractor",
        "yt-dlp non è riuscito a leggere questa pagina video.",
    ),
    (
        "site_forbidden",
        "YouTube ha rifiutato il download. Di solito una versione più recente di yt-dlp risolve: aggiornala in Impostazioni, Media, poi riprova.",
    ),
    (
        "site_sign_in",
        "Per questo download YouTube chiede un browser con accesso effettuato. Downpipe non usa i cookie del browser: aspetta un po' o usa un'altra rete, poi riprova.",
    ),
    (
        "site_age_restricted",
        "YouTube mostra questo video solo agli adulti che hanno effettuato l'accesso. Downpipe non usa account né cookie del browser, quindi non può scaricarlo.",
    ),
    (
        "site_js_runtime",
        "yt-dlp non è riuscito a rispondere alle verifiche JavaScript di YouTube. Scarica Deno in Impostazioni, Media, poi riprova.",
    ),
    (
        "site_po_token",
        "YouTube ha chiesto un token di prova di origine (PO) per questo video. Aggiorna yt-dlp in Impostazioni, Media, o scegli un'altra qualità.",
    ),
    (
        "site_outdated",
        "yt-dlp non capisce più questa pagina; probabilmente YouTube l'ha cambiata. Aggiorna yt-dlp in Impostazioni, Media, poi riprova.",
    ),
];

static PT_BR_FAILURES: &[(&str, &str)] = &[
    (
        "invalid_url",
        "Este link não é um endereço http ou https válido.",
    ),
    (
        "invalid_header",
        "Um dos cabeçalhos personalizados deste download não é válido.",
    ),
    (
        "auth_required",
        "O servidor pede um nome de usuário e uma senha.",
    ),
    (
        "forbidden",
        "O servidor recusou o download. O link pode ter expirado, ou o site pode pedir que você entre na sua conta.",
    ),
    (
        "not_found",
        "O arquivo não foi encontrado no servidor. Confira o link ou use um novo.",
    ),
    (
        "throttled",
        "O servidor está limitando os downloads. Tente de novo mais tarde ou com menos conexões.",
    ),
    (
        "http_status",
        "O servidor respondeu com um erro (HTTP {status}).",
    ),
    (
        "resource_changed",
        "O arquivo no servidor mudou durante o download. Baixe-o de novo desde o início.",
    ),
    (
        "different_file",
        "O novo link leva a outro arquivo. A parte baixada foi mantida.",
    ),
    (
        "expired_link",
        "O link expirou: o servidor enviou uma página da web em vez do arquivo. Use um link novo; a parte baixada é mantida.",
    ),
    (
        "login_required",
        "O servidor pede que você entre na sua conta. Entre no navegador e depois use um link novo; a parte baixada é mantida.",
    ),
    (
        "private_file",
        "O arquivo não está compartilhado publicamente. Peça ao dono para compartilhá-lo com qualquer pessoa que tenha o link, ou baixe-o no navegador com sua conta conectada.",
    ),
    (
        "range_not_honoured",
        "O servidor parou de enviar o arquivo em partes. Tente de novo com uma conexão só.",
    ),
    (
        "network",
        "A conexão com o servidor falhou. Confira sua rede e tente de novo.",
    ),
    (
        "checksum",
        "O arquivo baixado não confere com o checksum. Ele pode estar danificado; baixe-o de novo.",
    ),
    (
        "io",
        "Não foi possível gravar o arquivo. Confira se a pasta existe e se o disco tem espaço livre.",
    ),
    (
        "corrupt_meta",
        "Os dados para retomar este download estão danificados. Baixe-o de novo desde o início.",
    ),
    (
        "drm_protected",
        "Esta transmissão é protegida por DRM. O Downpipe não baixa mídia protegida.",
    ),
    (
        "live_stream",
        "Esta é uma transmissão ao vivo. Só transmissões encerradas podem ser baixadas.",
    ),
    (
        "invalid_manifest",
        "Não foi possível ler a playlist da transmissão.",
    ),
    (
        "unsupported_media",
        "Este tipo de transmissão não é suportado.",
    ),
    (
        "tool_missing",
        "Falta um programa de que este download precisa (yt-dlp, Deno ou FFmpeg). Instale-o em Preferências, Mídia.",
    ),
    (
        "unavailable",
        "Este vídeo não está disponível: ele é privado, foi removido, é exclusivo para membros ou está bloqueado na sua região.",
    ),
    (
        "extractor",
        "O yt-dlp não conseguiu ler esta página de vídeo.",
    ),
    (
        "site_forbidden",
        "O YouTube recusou o download. Uma versão mais nova do yt-dlp costuma resolver: atualize-o em Configurações, Mídia e tente de novo.",
    ),
    (
        "site_sign_in",
        "O YouTube quer um navegador conectado para este download. O Downpipe não usa cookies do navegador: espere um pouco ou use outra rede e tente de novo.",
    ),
    (
        "site_age_restricted",
        "O YouTube mostra este vídeo apenas para adultos conectados. O Downpipe não usa contas nem cookies do navegador, então não consegue baixá-lo.",
    ),
    (
        "site_js_runtime",
        "O yt-dlp não conseguiu responder aos desafios de JavaScript do YouTube. Baixe o Deno em Configurações, Mídia e tente de novo.",
    ),
    (
        "site_po_token",
        "O YouTube pediu um token de prova de origem (PO) para este vídeo. Atualize o yt-dlp em Configurações, Mídia ou escolha outra qualidade.",
    ),
    (
        "site_outdated",
        "O yt-dlp não entende mais esta página; provavelmente o YouTube a mudou. Atualize o yt-dlp em Configurações, Mídia e tente de novo.",
    ),
];

static RU_FAILURES: &[(&str, &str)] = &[
    (
        "invalid_url",
        "Эта ссылка не является правильным адресом http или https.",
    ),
    (
        "invalid_header",
        "Один из пользовательских заголовков этой загрузки недопустим.",
    ),
    (
        "auth_required",
        "Сервер запрашивает имя пользователя и пароль.",
    ),
    (
        "forbidden",
        "Сервер отказал в загрузке. Возможно, ссылка устарела или сайт требует войти в аккаунт.",
    ),
    (
        "not_found",
        "Файл не найден на сервере. Проверьте ссылку или используйте новую.",
    ),
    (
        "throttled",
        "Сервер ограничивает загрузки. Повторите попытку позже или с меньшим числом подключений.",
    ),
    ("http_status", "Сервер ответил ошибкой (HTTP {status})."),
    (
        "resource_changed",
        "Файл на сервере изменился во время загрузки. Загрузите его заново с начала.",
    ),
    (
        "different_file",
        "Новая ссылка ведёт к другому файлу. Загруженная часть сохранена.",
    ),
    (
        "expired_link",
        "Срок действия ссылки истёк: сервер прислал веб-страницу вместо файла. Используйте новую ссылку; загруженная часть сохранена.",
    ),
    (
        "login_required",
        "Сервер требует войти в аккаунт. Войдите в браузере, затем используйте новую ссылку; загруженная часть сохранена.",
    ),
    (
        "private_file",
        "Файл не открыт для общего доступа. Попросите владельца открыть его всем, у кого есть ссылка, или скачайте его в браузере, войдя в аккаунт.",
    ),
    (
        "range_not_honoured",
        "Сервер перестал отдавать файл по частям. Повторите попытку с одним подключением.",
    ),
    (
        "network",
        "Не удалось подключиться к серверу. Проверьте сеть и повторите попытку.",
    ),
    (
        "checksum",
        "Загруженный файл не совпадает с контрольной суммой. Возможно, он повреждён; загрузите его заново.",
    ),
    (
        "io",
        "Не удалось записать файл. Проверьте, что папка существует и на диске есть свободное место.",
    ),
    (
        "corrupt_meta",
        "Данные для возобновления этой загрузки повреждены. Загрузите файл заново с начала.",
    ),
    (
        "drm_protected",
        "Этот поток защищён DRM. Downpipe не загружает защищённые медиа.",
    ),
    (
        "live_stream",
        "Это прямая трансляция. Загрузить можно только завершённые потоки.",
    ),
    ("invalid_manifest", "Не удалось прочитать плейлист потока."),
    ("unsupported_media", "Такой поток не поддерживается."),
    (
        "tool_missing",
        "Не настроена программа, нужная этой загрузке (yt-dlp, Deno или FFmpeg). Установите её в разделе «Параметры», «Медиа».",
    ),
    (
        "unavailable",
        "Это видео недоступно: оно личное, удалено, доступно только спонсорам или заблокировано в вашем регионе.",
    ),
    (
        "extractor",
        "yt-dlp не смог прочитать эту страницу с видео.",
    ),
    (
        "site_forbidden",
        "YouTube отказал в загрузке. Обычно помогает более новая версия yt-dlp: обновите ее в настройках, в разделе «Медиа», и попробуйте снова.",
    ),
    (
        "site_sign_in",
        "Для этой загрузки YouTube требует браузер с выполненным входом. Downpipe не использует файлы cookie браузера: подождите немного или подключитесь к другой сети и попробуйте снова.",
    ),
    (
        "site_age_restricted",
        "YouTube показывает это видео только взрослым пользователям, выполнившим вход. Downpipe не использует аккаунты и файлы cookie браузера, поэтому не может его загрузить.",
    ),
    (
        "site_js_runtime",
        "yt-dlp не смог ответить на JavaScript-проверки YouTube. Загрузите Deno в настройках, в разделе «Медиа», и попробуйте снова.",
    ),
    (
        "site_po_token",
        "YouTube запросил для этого видео токен подтверждения происхождения (PO). Обновите yt-dlp в настройках, в разделе «Медиа», или выберите другое качество.",
    ),
    (
        "site_outdated",
        "yt-dlp больше не понимает эту страницу; вероятно, YouTube ее изменил. Обновите yt-dlp в настройках, в разделе «Медиа», и попробуйте снова.",
    ),
];

static ZH_CN_FAILURES: &[(&str, &str)] = &[
    ("invalid_url", "此链接不是有效的 http 或 https 地址。"),
    ("invalid_header", "此下载的某个自定义标头无效。"),
    ("auth_required", "服务器要求提供用户名和密码。"),
    (
        "forbidden",
        "服务器拒绝了下载。链接可能已过期，或者网站要求登录。",
    ),
    (
        "not_found",
        "服务器上找不到该文件。请检查链接或使用新链接。",
    ),
    (
        "throttled",
        "服务器正在限制下载。请稍后再试，或减少连接数后再试。",
    ),
    ("http_status", "服务器返回了错误（HTTP {status}）。"),
    (
        "resource_changed",
        "下载过程中服务器上的文件发生了变化。请从头重新下载。",
    ),
    (
        "different_file",
        "新链接指向另一个文件。已下载的部分已保留。",
    ),
    (
        "expired_link",
        "链接已过期：服务器返回了网页而不是文件。请使用新链接；已下载的部分会保留。",
    ),
    (
        "login_required",
        "服务器要求登录。请先在浏览器中登录，然后使用新链接；已下载的部分会保留。",
    ),
    (
        "private_file",
        "该文件未公开共享。请让所有者将其共享给任何拥有链接的人，或在浏览器中登录后下载。",
    ),
    (
        "range_not_honoured",
        "服务器不再分段发送文件。请改用单个连接重试。",
    ),
    ("network", "无法连接到服务器。请检查网络后重试。"),
    (
        "checksum",
        "下载的文件与校验和不符，可能已损坏。请重新下载。",
    ),
    ("io", "无法写入文件。请确认文件夹存在且磁盘有可用空间。"),
    ("corrupt_meta", "此下载的续传数据已损坏。请从头重新下载。"),
    (
        "drm_protected",
        "此流受 DRM 保护。Downpipe 不下载受保护的媒体。",
    ),
    ("live_stream", "这是直播。只能下载已结束的流。"),
    ("invalid_manifest", "无法读取该流的播放列表。"),
    ("unsupported_media", "不支持此类流。"),
    (
        "tool_missing",
        "此下载所需的程序（yt-dlp、Deno 或 FFmpeg）尚未设置。请在“首选项 > 媒体”中设置。",
    ),
    (
        "unavailable",
        "此视频不可用：它是私享视频、已被移除、仅限会员观看或在你所在的地区被屏蔽。",
    ),
    ("extractor", "yt-dlp 无法读取此视频页面。"),
    (
        "site_forbidden",
        "YouTube 拒绝了此次下载。更新 yt-dlp 通常可以解决：请在“设置”的“媒体”中更新，然后重试。",
    ),
    (
        "site_sign_in",
        "YouTube 要求使用已登录的浏览器进行此次下载。Downpipe 不使用浏览器 Cookie：请稍等片刻或换一个网络，然后重试。",
    ),
    (
        "site_age_restricted",
        "YouTube 只向已登录的成年用户显示此视频。Downpipe 不使用账号或浏览器 Cookie，因此无法下载。",
    ),
    (
        "site_js_runtime",
        "yt-dlp 无法应答 YouTube 的 JavaScript 验证。请在“设置”的“媒体”中下载 Deno，然后重试。",
    ),
    (
        "site_po_token",
        "YouTube 要求为此视频提供来源证明（PO）令牌。请在“设置”的“媒体”中更新 yt-dlp，或选择其他画质。",
    ),
    (
        "site_outdated",
        "yt-dlp 已无法解析此页面，YouTube 可能更改了页面。请在“设置”的“媒体”中更新 yt-dlp，然后重试。",
    ),
];

static JA_FAILURES: &[(&str, &str)] = &[
    (
        "invalid_url",
        "このリンクは有効な http または https のアドレスではありません。",
    ),
    (
        "invalid_header",
        "このダウンロードのカスタムヘッダーのひとつが無効です。",
    ),
    (
        "auth_required",
        "サーバーがユーザー名とパスワードを求めています。",
    ),
    (
        "forbidden",
        "サーバーがダウンロードを拒否しました。リンクの有効期限が切れたか、サイトへのログインが必要な可能性があります。",
    ),
    (
        "not_found",
        "サーバー上にファイルが見つかりません。リンクを確認するか、新しいリンクを使ってください。",
    ),
    (
        "throttled",
        "サーバーがダウンロードを制限しています。時間をおくか、接続数を減らしてやり直してください。",
    ),
    (
        "http_status",
        "サーバーがエラーを返しました（HTTP {status}）。",
    ),
    (
        "resource_changed",
        "ダウンロード中にサーバー上のファイルが変わりました。最初からダウンロードし直してください。",
    ),
    (
        "different_file",
        "新しいリンクは別のファイルを指しています。ダウンロード済みの部分は保持されました。",
    ),
    (
        "expired_link",
        "リンクの有効期限が切れています。サーバーはファイルではなくウェブページを返しました。新しいリンクを使ってください。ダウンロード済みの部分は保持されます。",
    ),
    (
        "login_required",
        "サーバーがログインを求めています。ブラウザーでログインしてから新しいリンクを使ってください。ダウンロード済みの部分は保持されます。",
    ),
    (
        "private_file",
        "このファイルは一般公開されていません。リンクを知っている全員に共有するよう所有者に依頼するか、ログインした状態でブラウザからダウンロードしてください。",
    ),
    (
        "range_not_honoured",
        "サーバーがファイルの分割送信をやめました。接続数 1 でやり直してください。",
    ),
    (
        "network",
        "サーバーに接続できませんでした。ネットワークを確認してやり直してください。",
    ),
    (
        "checksum",
        "ダウンロードしたファイルがチェックサムと一致しません。破損している可能性があるので、ダウンロードし直してください。",
    ),
    (
        "io",
        "ファイルを書き込めませんでした。フォルダーがあり、ディスクに空きがあるか確認してください。",
    ),
    (
        "corrupt_meta",
        "このダウンロードの再開用データが壊れています。最初からダウンロードし直してください。",
    ),
    (
        "drm_protected",
        "このストリームは DRM で保護されています。Downpipe は保護されたメディアをダウンロードしません。",
    ),
    (
        "live_stream",
        "これはライブ配信です。ダウンロードできるのは終了した配信だけです。",
    ),
    (
        "invalid_manifest",
        "ストリームのプレイリストを読み取れませんでした。",
    ),
    (
        "unsupported_media",
        "この種類のストリームには対応していません。",
    ),
    (
        "tool_missing",
        "このダウンロードに必要なプログラム（yt-dlp、Deno、FFmpeg）が設定されていません。環境設定の「メディア」で設定してください。",
    ),
    (
        "unavailable",
        "この動画は利用できません。非公開、削除済み、メンバー限定、またはお住まいの地域でブロックされています。",
    ),
    (
        "extractor",
        "yt-dlp はこの動画ページを読み取れませんでした。",
    ),
    (
        "site_forbidden",
        "YouTube にダウンロードを拒否されました。多くの場合、新しい yt-dlp で解決します。「設定」の「メディア」で更新してから、もう一度お試しください。",
    ),
    (
        "site_sign_in",
        "このダウンロードには、YouTube にログインしたブラウザーが必要です。Downpipe はブラウザーの Cookie を使いません。しばらく待つか別のネットワークを使って、もう一度お試しください。",
    ),
    (
        "site_age_restricted",
        "YouTube はこの動画をログインした成人にのみ表示します。Downpipe はアカウントもブラウザーの Cookie も使わないため、ダウンロードできません。",
    ),
    (
        "site_js_runtime",
        "yt-dlp が YouTube の JavaScript チャレンジに応答できませんでした。「設定」の「メディア」で Deno をダウンロードしてから、もう一度お試しください。",
    ),
    (
        "site_po_token",
        "YouTube がこの動画に発信元証明（PO）トークンを求めました。「設定」の「メディア」で yt-dlp を更新するか、別の画質を選んでください。",
    ),
    (
        "site_outdated",
        "yt-dlp がこのページを読み取れなくなりました。YouTube がページを変更した可能性があります。「設定」の「メディア」で yt-dlp を更新してから、もう一度お試しください。",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_setting_then_the_system_then_english() {
        assert_eq!(Lang::resolve("tr"), Lang::Tr);
        assert_eq!(Lang::resolve(" pt-PT "), Lang::PtBr);
        assert_eq!(Lang::from_tag("zh-Hans-CN"), Some(Lang::ZhCn));
        assert_eq!(
            Lang::from_tag("zh_TW"),
            None,
            "traditional Chinese is not guessed"
        );
        assert_eq!(Lang::from_tag("de-AT"), Some(Lang::De));
        assert_eq!(Lang::from_tag("xx"), None);
    }

    /// `FAILURE_KINDS` in src/lib/errors.ts and the `site_*` codes of
    /// src/lib/siteErrors.ts.
    const FAILURE_KINDS: &[&str] = &[
        "invalid_url",
        "invalid_header",
        "auth_required",
        "forbidden",
        "not_found",
        "throttled",
        "http_status",
        "resource_changed",
        "different_file",
        "expired_link",
        "login_required",
        "private_file",
        "range_not_honoured",
        "network",
        "checksum",
        "io",
        "corrupt_meta",
        "drm_protected",
        "live_stream",
        "invalid_manifest",
        "unsupported_media",
        "tool_missing",
        "unavailable",
        "extractor",
        "site_forbidden",
        "site_sign_in",
        "site_age_restricted",
        "site_js_runtime",
        "site_po_token",
        "site_outdated",
    ];

    const ALL: [Lang; 10] = [
        Lang::En,
        Lang::Tr,
        Lang::Es,
        Lang::De,
        Lang::Fr,
        Lang::It,
        Lang::PtBr,
        Lang::Ru,
        Lang::ZhCn,
        Lang::Ja,
    ];

    #[test]
    fn every_failure_kind_has_a_text_in_every_language() {
        for lang in ALL {
            let t = lang.texts();
            assert_eq!(
                t.failure_kinds.len(),
                FAILURE_KINDS.len(),
                "{lang:?} has a kind no other language has"
            );
            for kind in FAILURE_KINDS {
                let text = failure_text(t, kind, "unexpected HTTP status 502")
                    .unwrap_or_else(|| panic!("{lang:?} has no text for {kind}"));
                assert!(!text.trim().is_empty(), "{lang:?}: {kind}");
                assert!(!text.contains('{'), "{lang:?}: {kind}: {text}");
            }
            let status = failure_text(t, "http_status", "unexpected HTTP status 502").unwrap();
            assert!(status.contains("502"), "{lang:?}: {status}");
            assert!(!t.server_error.trim().is_empty(), "{lang:?}");
        }
    }

    #[test]
    fn failures_read_as_in_the_web_view() {
        let en = Lang::En.texts();
        assert_eq!(
            failure_text(en, "not_found", "HTTP 404 Not Found").as_deref(),
            Some("The file was not found on the server. Check the link or use a new one.")
        );
        assert_eq!(
            failure_text(
                Lang::Tr.texts(),
                "http_status",
                "unexpected HTTP status 503"
            )
            .as_deref(),
            Some("Sunucu bir hatayla yanıt verdi (HTTP 503).")
        );
        assert_eq!(
            failure_text(en, "http_status", "the server closed the connection").as_deref(),
            Some(en.server_error),
            "no status in the message"
        );
        assert_eq!(
            failure_text(en, "http_status", "status 6000 or 0502").as_deref(),
            Some(en.server_error),
            "only a whole three-digit status counts"
        );
    }

    #[test]
    fn unknown_kinds_are_left_to_the_engine() {
        assert_eq!(
            failure_text(Lang::De.texts(), "torrent", "tracker said no"),
            None
        );
        assert_eq!(failure_text(Lang::De.texts(), "", ""), None);
    }

    #[test]
    fn fills_placeholders() {
        assert_eq!(
            fill(
                Lang::Tr.texts().tooltip_many,
                &[("n", "3"), ("speed", "2.0 MiB")]
            ),
            "Downpipe: 3 indirme, 2.0 MiB/s"
        );
        for lang in [
            Lang::En,
            Lang::Tr,
            Lang::Es,
            Lang::De,
            Lang::Fr,
            Lang::It,
            Lang::PtBr,
            Lang::Ru,
            Lang::ZhCn,
            Lang::Ja,
        ] {
            let t = lang.texts();
            for (text, names) in [
                (t.all_done_quit, &["secs"][..]),
                (t.all_done_sleep, &["secs"]),
                (t.all_done_shutdown, &["secs"]),
                (t.job_failed_body, &["err"]),
                (t.schedule_started_queue, &["name"]),
                (t.schedule_stopped_queue, &["name"]),
                (t.tooltip_one, &["speed"]),
                (t.tooltip_many, &["n", "speed"]),
            ] {
                for name in names {
                    assert!(text.contains(&format!("{{{name}}}")), "{lang:?}: {text}");
                }
            }
        }
    }
}
