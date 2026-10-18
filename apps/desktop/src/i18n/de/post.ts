// After a download completes: the virus scan, extracting archives and a command.
import type { Messages } from "../core";

export const post: Messages["post"] = {
  steps: {
    scanning: "Wird geprüft",
    extracting: "Wird entpackt",
    running_command: "Befehl läuft",
  },
  threat: "Bedrohung",
  summary: {
    threat: "Bedrohung gefunden: {threat}",
    notExtracted: "Nicht entpackt",
    commandFailed: "Der Befehl nach dem Download ist fehlgeschlagen",
  },
  section: "Nach dem Download",
  scan: {
    label: "Virenprüfung",
    clean: "{scanner} hat nichts gefunden.",
    threat:
      "{scanner} hat {threat} gefunden. Nichts wurde entpackt oder ausgeführt, und die Datei blieb unverändert: Ob sie gelöscht wird, entscheiden Sie.",
    unnamed: "eine Bedrohung",
    theScanner: "Der Scanner",
    missing: "Nicht geprüft: Kein Scanner eingerichtet. Wählen Sie einen unter Einstellungen, Nach dem Download.",
    missingNamed: "Nicht geprüft: {scanner} wurde nicht gefunden.",
    failed: "{scanner} konnte die Datei nicht prüfen: {detail}",
  },
  extract: {
    label: "Entpackt",
    done: { one: "{count} Datei in einen neuen Ordner.", other: "{count} Dateien in einen neuen Ordner." },
    skipped: {
      one: "{count} Eintrag wurde ausgelassen: ein unsicherer Name, eine Verknüpfung oder ein Duplikat.",
      other: "{count} Einträge wurden ausgelassen: unsichere Namen, Verknüpfungen oder Duplikate.",
    },
    deleted: "Das Archiv wurde danach gelöscht.",
    openFolder: "Ordner öffnen",
    failed: "Nicht entpackt: {reason}.",
    errors: {
      encrypted: "Das Archiv ist passwortgeschützt",
      too_large: "Es würde weit über seine eigene Größe hinaus anwachsen (möglicherweise eine Zip-Bombe)",
      too_many_entries: "Es hat zu viele Einträge",
      corrupt: "Das Archiv ist beschädigt oder nutzt ein nicht unterstütztes Verfahren",
      io: "Die Dateien konnten nicht geschrieben werden",
    },
  },
  command: {
    label: "Befehl",
    ok: "{program} ist fertig.",
    failed: "{program} endete mit dem Exit-Code {code}.",
    ended: "{program} wurde beendet, bevor es fertig war.",
    timedOut: "{program} hat das Zeitlimit überschritten und wurde gestoppt.",
    notStarted: "{program} konnte nicht gestartet werden: {detail}",
    invalid: "Der Befehl kann nicht ausgeführt werden: {detail}",
    theCommand: "Der Befehl",
    output: "Ausgabe",
  },
  fields: {
    heading: "Nach dem Download",
    command: "Danach ausführen",
    inherited: "Standard: {command}",
    nothing: "Nichts",
    hint: "Ein Programm mit seinen Argumenten, etwa notepad {path}. {path}, {name}, {folder} und {url} werden eingesetzt, jeweils als ein Argument, ohne Shell. Läuft nicht, wenn der Download fehlschlägt.",
    extract: "Archiv entpacken",
    extractHint: "In einen neuen Ordner daneben, wenn er fertig ist.",
    save: "Speichern",
    saved: "Gespeichert. Gilt, wenn der Download fertig ist.",
  },
  settings: {
    tab: "Nach dem Download",
    command: "Nach jedem Download einen Befehl ausführen",
    commandHint:
      "Ein Programm mit seinen Argumenten, etwa notepad {path}. {path}, {name}, {folder} und {url} werden eingesetzt, jeweils als ein Argument; nichts läuft über eine Shell, daher können Dateinamen und Links keine Befehle hinzufügen. Warteschlangen und einzelne Downloads können eigene festlegen. Läuft nicht, wenn ein Download fehlschlägt.",
    timeout: "Zeitlimit",
    timeoutHint: "Sekunden, die ein Befehl oder eine Prüfung laufen darf, bevor sie samt allem, was sie gestartet hat, gestoppt wird. 5 bis 86400.",
    seconds: "s",
    scan: "Downloads auf Viren prüfen",
    scanHintWindows:
      "Mit Microsoft Defender, nachdem jeder Download fertig ist. Eine Bedrohung markiert den Download und zeigt eine Benachrichtigung; nichts wird gelöscht oder in Quarantäne verschoben, nichts entpackt oder ausgeführt.",
    scanHintOther:
      "Mit dem Scanner unten, nachdem jeder Download fertig ist. Eine Bedrohung markiert den Download und zeigt eine Benachrichtigung; nichts wird gelöscht, entpackt oder ausgeführt.",
    scanner: "Scanner",
    scannerPlaceholderWindows: "Microsoft Defender",
    scannerHint:
      "Leer verwendet unter Windows Microsoft Defender. Jeder andere Scanner erhält die Datei als {path} oder als letztes Argument; Exit-Code 0 bedeutet sauber und 1 eine Bedrohung, wie bei clamscan von ClamAV.",
    archives: "Archive",
    archivesHint:
      "Zip-, tar-, tar.gz-, tar.xz- und 7z-Archive werden in einen neuen Ordner daneben entpackt, wenn ihre Kategorie (Kategorien) oder der Download es vorsieht. Einträge, die außerhalb dieses Ordners landen würden, Verknüpfungen und Archive, die weit über ihre Größe hinaus anwachsen, werden abgelehnt. RAR wird nicht unterstützt; 7-Zip kann es als Befehl entpacken.",
    deleteArchive: "Archiv nach dem Entpacken löschen",
    deleteArchiveHint: "Nur wenn jeder Eintrag entpackt wurde, und nachdem der Befehl lief.",
    queueCommand: "Befehl nach Downloads von {name}",
    queueCommandUnnamed: "Befehl nach den Downloads der Warteschlange",
    queueCommandPlaceholder: "Befehl danach; leer: der unter Nach dem Download",
    categoryExtract: "Archive entpacken",
    categoryExtractTitle: "Zip-, tar- und 7z-Archive dieser Kategorie nach dem Download entpacken",
  },
};
