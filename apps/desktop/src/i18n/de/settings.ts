// The settings dialog, one group per tab, and the browser bridge's states.
// {placeholders} that hold an element (a path, a command) are put in by the dialog.
import type { Messages } from "../core";

export const settings: Messages["settings"] = {
  eyebrow: "Einstellungen",
  title: "Optionen",
  tabsLabel: "Einstellungsbereiche",
  tabs: {
    downloads: "Downloads",
    queues: "Warteschlangen",
    categories: "Kategorien",
    network: "Netzwerk",
    media: "Medien",
    schedule: "Zeitplan",
    app: "App",
  },
  downloads: {
    folder: "Standardordner",
    folderHint: "Die Kategorien unten legen ihre Ordner in diesem Ordner an.",
    concurrent: "Gleichzeitige Downloads",
    concurrentHint: "Weitere warten in der Warteschlange. 1 bis 16.",
    connections: "Verbindungen pro Download",
    connectionsHint: "Obergrenze. Downpipe beginnt mit 2 und fügt weitere nur hinzu, solange sie helfen. 1 bis 32.",
    speedLimit: "Geschwindigkeitslimit",
    speedLimitHint: "Gilt für alle Downloads zusammen. Leer lassen für kein Limit. Wirkt sofort.",
    noLimit: "Kein Limit",
    unit: "Einheit des Geschwindigkeitslimits",
  },
  queues: {
    intro:
      "Jede Warteschlange führt ihre eigene Anzahl an Downloads gleichzeitig aus, parallel zu den anderen; innerhalb einer Warteschlange starten Downloads von oben nach unten. Über das Kontextmenü verschieben Sie Downloads zwischen Warteschlangen und ändern ihre Reihenfolge. Downloads einer entfernten Warteschlange werden in die Hauptwarteschlange verschoben.",
    label: "Warteschlangen",
    name: "Name der Warteschlange",
    namePlaceholder: "Name",
    mainLimit: { one: "{count} gleichzeitig", other: "{count} gleichzeitig" },
    mainLimitTitle: "Festgelegt unter Downloads > Gleichzeitige Downloads",
    limit: "Gleichzeitige Downloads in {name}",
    limitUnnamed: "Gleichzeitige Downloads der Warteschlange",
    limitTitle: "Gleichzeitige Downloads, 1 bis 16",
    remove: "{name} entfernen",
    removeUnnamed: "Warteschlange entfernen",
    newName: "Warteschlange {number}",
    add: "Warteschlange hinzufügen",
  },
  categories: {
    use: "Downloads in Kategorieordner einsortieren",
    useHint:
      "Ein neuer Download landet je nach Dateityp im Ordner seiner Kategorie. Zeigt der Dateiname vom Server einen anderen Typ, wird die fertige Datei in den passenden Ordner verschoben. Ordner, die Sie selbst wählen, werden nie geändert.",
    label: "Kategorien",
    name: "Name der Kategorie",
    namePlaceholder: "Name",
    types: "Dateitypen von {name}",
    typesUnnamed: "Dateitypen der Kategorie",
    remove: "{name} entfernen",
    removeUnnamed: "Kategorie entfernen",
    folder: "Ordner von {name}",
    folderUnnamed: "Ordner der Kategorie",
    folderPlaceholder: "Ordner: {name} im Standardordner",
    folderPlaceholderUnnamed: "Ordner: Name im Standardordner",
    add: "Kategorie hinzufügen",
  },
  network: {
    proxy: "Proxy",
    proxyHint: "http://, https:// oder socks5://host:port. Ohne Eintrag wird der Systemproxy verwendet. Gilt für Downloads, die nach dem Speichern starten.",
    proxyPlaceholder: "Systemproxy",
    userAgent: "User-Agent",
    userAgentHint: "Manche Server antworten nur Browsern. Ohne Eintrag sendet Downpipe seinen eigenen.",
  },
  media: {
    ffmpegHint:
      "Führt getrenntes Video und getrennten Ton eines Streams ohne Neukodierung zu einer Datei zusammen. Ohne Eintrag wird ffmpeg aus dem PATH verwendet. FFmpeg ist nicht in Downpipe enthalten.",
    ffmpegPlaceholder: "ffmpeg im PATH",
    found: "Gefunden",
    notFound: "Nicht gefunden",
    ffmpegMissing:
      "Streams mit getrenntem Video und Ton werden als zwei Dateien gespeichert. Installieren Sie FFmpeg und fügen Sie es dem PATH hinzu oder geben Sie oben den Pfad zu ffmpeg ein, und speichern Sie dann.",
    streams:
      "Streams stammen aus der Medienerkennung der Browsererweiterung oder aus hier hinzugefügten .m3u8- und .mpd-Links. DRM-geschützte Streams und Livestreams werden nicht heruntergeladen.",
  },
  siteTools: {
    heading: "Videos auf Webseiten",
    // The quoted menu item is the browser extension's (its menus.link in German).
    intro:
      "Auf einer YouTube-Seite übergibt „Mit Downpipe herunterladen“ die Seite an yt-dlp, das das Video daraus ermittelt. yt-dlp benötigt Deno für YouTube und FFmpeg, um Video und Ton oberhalb von 360p zusammenzuführen.",
    ytDlpHint: "Ohne Eintrag wird die unten heruntergeladene Kopie verwendet, sonst yt-dlp aus dem PATH.",
    ytDlpPlaceholder: "Heruntergeladene Kopie, sonst yt-dlp im PATH",
    denoHint: "Version 2.3.0 oder neuer. Ohne Eintrag wird die unten heruntergeladene Kopie verwendet, sonst deno aus dem PATH, sonst Node 22 oder neuer.",
    denoPlaceholder: "Heruntergeladene Kopie, sonst deno im PATH",
    looking: "yt-dlp und Deno werden gesucht…",
    listLabel: "Tools für Videos auf Webseiten",
    downloading: "Wird heruntergeladen…",
    installNote:
      "Nur wenn Sie darauf klicken: Die offiziellen Builds von github.com/yt-dlp/yt-dlp und github.com/denoland/deno werden, jeweils anhand der im Release veröffentlichten SHA-256-Prüfsumme geprüft, in {folder} abgelegt. YouTube ändert sich oft; klicken Sie erneut, um das neueste yt-dlp zu erhalten.",
    toolsFolder: "Tools-Ordner von Downpipe",
    cookies:
      "Downpipe sendet yt-dlp keine Cookies, daher werden private, nur für Mitglieder freigegebene und altersbeschränkte Videos nicht heruntergeladen, ebenso wenig geschützte (DRM) und Live-Videos. Für die Einhaltung der YouTube-Nutzungsbedingungen sind Sie selbst verantwortlich.",
  },
  schedule: {
    enable: "Downloads nach Zeitplan starten und anhalten",
    enableHint:
      "Zur Startzeit werden pausierte und fehlgeschlagene Downloads fortgesetzt. Zur Stoppzeit werden sie pausiert. Downpipe muss dafür laufen, im Fenster oder im Infobereich.",
    queue: "Warteschlange",
    queueHint: "Die anderen Warteschlangen bleiben unberührt.",
    everyQueue: "Alle Warteschlangen",
    start: "Starten um",
    stop: "Anhalten um",
    stopHint: "Optional",
    days: "Tage",
  },
  app: {
    language: "Sprache",
    languageHint: "Gilt sofort für das Fenster, das Infobereichsmenü und Benachrichtigungen.",
    systemDefault: "Systemstandard ({language})",
    notify: "Benachrichtigen, wenn ein Download abgeschlossen ist",
    notifyHint: "Eine Systembenachrichtigung, wenn ein Download abgeschlossen ist oder fehlschlägt, während das Fenster nicht im Vordergrund ist.",
    tray: "Beim Schließen des Fensters im Infobereich weiterlaufen",
    trayHint: "Downloads laufen weiter, und die Browsererweiterung kann weiterhin Links senden. Beenden über das Symbol im Infobereich.",
    startup: "Downpipe bei der Anmeldung starten",
    startupHint: "Startet im Infobereich, damit erfasste Downloads immer ein Ziel haben.",
    clipboard: "Download kopierter Links anbieten",
    clipboardHint:
      "Wenn Sie einen Link auf einen unter Kategorien aufgeführten Dateityp oder auf ein YouTube-Video kopieren, öffnet Downpipe damit das Dialogfeld „Link hinzufügen“. Links, die bereits in der Liste sind, werden übersprungen.",
    awake: "Computer während des Downloads aktiv halten",
    awakeHint: "Der Energiesparmodus wartet, bis keine Downloads mehr laufen. Der Bildschirm kann sich trotzdem ausschalten.",
    confirmBrowser: "Vor dem Start von Downloads aus dem Browser nachfragen",
    confirmBrowserHint:
      "Wählen Sie zuerst Dateiname, Ordner und Warteschlange in einem kleinen Fenster über dem Browser. Aus: Downloads starten sofort im automatischen Ordner.",
  },
  bridge: {
    heading: "Browser-Anbindung",
    register: "Beim Start bei den Browsern registrieren",
    registerHint:
      "Ermöglicht der Downpipe-Erweiterung in Chrome, Edge, Chromium und Firefox, diese Kopie der App zu erreichen. Betrifft nur Ihr Benutzerkonto. Deaktivieren Sie die Option, um die Registrierung selbst mit downpipe-native-host zu verwalten.",
    checking: "Registrierung wird geprüft…",
    noHost: "Diese Kopie von Downpipe hat keine Browser-Anbindung unter {host}. Installieren Sie Downpipe neu; führen Sie in einem Entwicklungsbuild {command} aus.",
    listLabel: "Registrierung pro Browser",
    registering: "Wird registriert…",
    registerNow: "Jetzt registrieren",
    pickUp: "Geöffnete Browser übernehmen sie, wenn die Erweiterung das nächste Mal prüft.",
    states: {
      registered: "Registriert",
      notInstalled: "Nicht installiert",
      missing: "Nicht registriert",
      invalid: "Nicht lesbar",
      otherHost: "Andere Kopie",
      extensionNotAllowed: "Andere Erweiterung",
      failed: "Fehler",
    },
  },
};
