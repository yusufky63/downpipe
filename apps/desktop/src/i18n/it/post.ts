// After a download completes: the virus scan, extracting archives and a command.
import type { Messages } from "../core";

export const post: Messages["post"] = {
  steps: {
    scanning: "Scansione",
    extracting: "Estrazione",
    running_command: "Comando in corso",
  },
  threat: "Minaccia",
  summary: {
    threat: "Minaccia trovata: {threat}",
    notExtracted: "Non estratto",
    commandFailed: "Il comando dopo il download non è riuscito",
  },
  section: "Dopo il download",
  scan: {
    label: "Scansione antivirus",
    clean: "{scanner} non ha trovato nulla.",
    threat:
      "{scanner} ha trovato {threat}. Non è stato estratto né eseguito nulla e il file è rimasto com'era: eliminarlo spetta a te.",
    unnamed: "una minaccia",
    theScanner: "L'antivirus",
    missing: "Non scansionato: nessun antivirus configurato. Scegline uno in Impostazioni, Dopo il download.",
    missingNamed: "Non scansionato: {scanner} non è stato trovato.",
    failed: "{scanner} non è riuscito a scansionare il file: {detail}",
  },
  extract: {
    label: "Estratto",
    done: { one: "{count} file in una nuova cartella.", many: "{count} file in una nuova cartella.", other: "{count} file in una nuova cartella." },
    skipped: {
      one: "{count} voce è stata esclusa: un nome non sicuro, un collegamento o un duplicato.",
      many: "{count} voci sono state escluse: nomi non sicuri, collegamenti o duplicati.",
      other: "{count} voci sono state escluse: nomi non sicuri, collegamenti o duplicati.",
    },
    deleted: "L'archivio è stato poi eliminato.",
    openFolder: "Apri cartella",
    failed: "Non estratto: {reason}.",
    errors: {
      encrypted: "l'archivio è protetto da password",
      too_large: "si espanderebbe ben oltre la sua dimensione (una possibile zip bomb)",
      too_many_entries: "ha troppe voci",
      corrupt: "l'archivio è danneggiato o usa un metodo non supportato",
      io: "non è stato possibile scrivere i file",
    },
  },
  command: {
    label: "Comando",
    ok: "{program} ha terminato.",
    failed: "{program} è terminato con il codice di uscita {code}.",
    ended: "{program} è stato interrotto prima di finire.",
    timedOut: "{program} ha superato il limite di tempo ed è stato fermato.",
    notStarted: "Impossibile avviare {program}: {detail}",
    invalid: "Il comando non può essere eseguito: {detail}",
    theCommand: "Il comando",
    output: "Output",
  },
  fields: {
    heading: "Dopo il download",
    command: "Esegui al termine",
    inherited: "Predefinito: {command}",
    nothing: "Niente",
    hint: "Un programma e i suoi argomenti, come notepad {path}. {path}, {name}, {folder} e {url} vengono compilati, ognuno come un solo argomento, senza shell. Non viene eseguito se il download non riesce.",
    extract: "Estrai l'archivio",
    extractHint: "In una nuova cartella accanto, al termine.",
    save: "Salva",
    saved: "Salvato. Si applica al termine del download.",
  },
  settings: {
    tab: "Dopo il download",
    command: "Esegui un comando dopo ogni download",
    commandHint:
      "Un programma e i suoi argomenti, come notepad {path}. {path}, {name}, {folder} e {url} vengono compilati, ognuno come un solo argomento; nulla passa da una shell, quindi nomi di file e link non possono aggiungere comandi. Le code e i singoli download possono avere il proprio. Non viene eseguito se un download non riesce.",
    timeout: "Limite di tempo",
    timeoutHint: "Secondi in cui un comando o una scansione può restare in esecuzione prima di essere fermato, insieme a tutto ciò che ha avviato. Da 5 a 86400.",
    seconds: "s",
    scan: "Scansiona i download alla ricerca di virus",
    scanHintWindows:
      "Con Microsoft Defender, al termine di ogni download. Una minaccia contrassegna il download e mostra una notifica; nulla viene eliminato o messo in quarantena, e nulla viene estratto o eseguito.",
    scanHintOther:
      "Con l'antivirus qui sotto, al termine di ogni download. Una minaccia contrassegna il download e mostra una notifica; nulla viene eliminato, estratto o eseguito.",
    scanner: "Antivirus",
    scannerPlaceholderWindows: "Microsoft Defender",
    scannerHint:
      "Vuoto usa Microsoft Defender su Windows. Qualsiasi altro antivirus riceve il file come {path} o come ultimo argomento; il codice di uscita 0 significa pulito e 1 una minaccia, come con clamscan di ClamAV.",
    archives: "Archivi",
    archivesHint:
      "Gli archivi zip, tar, tar.gz, tar.xz e 7z vengono estratti in una nuova cartella accanto quando la loro categoria lo prevede (Categorie) o lo chiede il download. Le voci che finirebbero fuori da quella cartella, i collegamenti e gli archivi che si espandono ben oltre la loro dimensione vengono rifiutati. RAR non è supportato; 7-Zip può estrarlo come comando.",
    deleteArchive: "Elimina l'archivio dopo l'estrazione",
    deleteArchiveHint: "Solo se tutte le voci sono state estratte, e dopo l'esecuzione del comando.",
    queueCommand: "Comando dopo i download di {name}",
    queueCommandUnnamed: "Comando dopo i download della coda",
    queueCommandPlaceholder: "Comando al termine; vuoto: quello di Dopo il download",
    categoryExtract: "Estrai archivi",
    categoryExtractTitle: "Estrai gli archivi zip, tar e 7z di questa categoria dopo il download",
  },
};
