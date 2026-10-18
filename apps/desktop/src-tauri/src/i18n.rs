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
};

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
