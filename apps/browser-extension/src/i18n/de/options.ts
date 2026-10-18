// The options page (Filters). In the help texts, {example}, {tarGz} and
// {wildcard} are code samples the page shows as they are.
import type { Messages } from "../core";

export const options: Messages["options"] = {
  pageTitle: "Filter · Downpipe",
  headLabel: "Browsererweiterung",
  lede:
    "Legen Sie fest, welche Downloads der Browser an Downpipe übergibt, solange „Browser-Downloads übernehmen“ eingeschaltet ist, und welche Medien die Erweiterung auflistet, solange „Medien auf dieser Seite“ eingeschaltet ist. Alles andere bleibt wie gewohnt im Browser.",
  captureOff:
    "Die Übernahme ist ausgeschaltet. Diese Filter gelten, sobald Sie „Browser-Downloads übernehmen“ im Downpipe-Popup der Symbolleiste einschalten.",
  loadError: "Die Einstellungen konnten nicht geladen werden. Laden Sie diese Seite neu, um es erneut zu versuchen.",
  typesHeading: "01 — Dateitypen",
  captureAll: "Alle Dateitypen übernehmen",
  captureAllDetail: "Jeder Download, den der Browser startet, geht an Downpipe, unabhängig vom Typ.",
  typesLabel: "Nur diese Dateitypen",
  typesHelp:
    "Dateierweiterungen, durch Leerzeichen oder Kommas getrennt, z. B. {example}. Ein Download wird übernommen, wenn sein Dateiname auf eine davon endet; {tarGz} funktioniert ebenfalls.",
  restoreDefault: "Standardliste wiederherstellen",
  sitesHeading: "02 — Ausgeschlossene Websites",
  sitesLabel: "Auf diesen Websites nie übernehmen oder Medien auflisten",
  sitesHelp:
    "Eine Website pro Zeile. {wildcard} umfasst zusätzlich alle Subdomains. Ein Download bleibt im Browser, wenn er von einer dieser Websites stammt oder auf einer ihrer Seiten gestartet wird, und Medien, die sie abspielen, werden nicht aufgelistet. Ports und Pfade werden ignoriert.",
  sizeHeading: "03 — Mindestgröße",
  sizeLabel: "Kleinere Dateien dem Browser überlassen",
  sizeHelp: "0 schaltet dies aus. Wenn der Server die Größe einer Datei nicht angibt, geht sie an Downpipe.",
  mediaHeading: "04 — Medien auf Seiten",
  mediaHelp:
    "Gilt, solange „Medien auf dieser Seite“ im Popup der Symbolleiste eingeschaltet ist. HLS- und DASH-Streams werden immer aufgelistet; die oben ausgeschlossenen Websites gelten auch hier.",
  overlay: "Schaltfläche „Herunterladen“ auf Videos anzeigen",
  overlayDetail: "Auf einem laufenden Video, für einige Sekunden und solange sich der Mauszeiger darüber befindet.",
  mediaTypesLabel: "Mediendateien dieser Typen auflisten",
  mediaTypesHelp:
    "Dateierweiterungen, durch Leerzeichen oder Kommas getrennt. Eine Datei ohne Erweiterung wird nach dem Typ beurteilt, den der Server sendet. Bei leerer Liste werden nur Streams aufgelistet.",
  mediaSizeLabel: "Mediendateien auslassen, die kleiner sind als",
  mediaSizeHelp:
    "Hält Benachrichtigungstöne und kurze Vorschauen fern. 0 listet jede Größe auf; Dateien, deren Größe der Server nicht angibt, werden immer aufgelistet.",
  bypassHeading: "05 — Im Browser herunterladen",
  bypassLabel: "Den Browser einen Link herunterladen lassen, wenn er angeklickt wird mit",
  bypassAlt: "Alt (Wahltaste auf dem Mac)",
  bypassShift: "Umschalt",
  bypassOff: "Aus",
  bypassHelp: "Solange Downloads abgefangen werden, bleibt ein Download beim Browser, wenn der Link mit gedrückter Taste angeklickt wird. Andere Klicks und die Tastenkürzel des Browsers bleiben, wie sie sind.",
  bypassAccess: "Dafür ist Zugriff auf die besuchten Websites nötig, um den Klick zu bemerken. Erst wenn Sie einen Link so anklicken, wird etwas gelesen oder gesendet.",
  bypassAllow: "Zugriff erlauben",
  save: "Speichern",
  moreErrors: { one: "Und {count} weiterer.", other: "Und {count} weitere." },
  unsaved: "Nicht gespeicherte Änderungen",
  notSavedFix: "Nicht gespeichert. Korrigieren Sie die rot markierten Felder.",
  saving: "Wird gespeichert…",
  notSavedNoAnswer: "Nicht gespeichert. Die Erweiterung hat nicht geantwortet; versuchen Sie es erneut.",
  saved: "Gespeichert",
  errors: {
    notExtension: "„{value}“ ist keine Dateierweiterung.",
    tooManyTypes: { one: "Geben Sie höchstens {count} Dateityp an.", other: "Geben Sie höchstens {count} Dateitypen an." },
    notSite: "Zeile {line}: „{value}“ ist keine Website. Verwenden Sie einen Host wie example.com oder *.example.com.",
    tooManySites: { one: "Geben Sie höchstens {count} Website an.", other: "Geben Sie höchstens {count} Websites an." },
    sizeNotNumber: "Geben Sie eine Zahl wie 2 oder 1.5 ein, oder 0, um dies auszuschalten.",
    sizeTooBig: "Geben Sie eine Größe unter 1 TiB ein.",
    noTypes: "Fügen Sie mindestens einen Dateityp hinzu, oder schalten Sie „Alle Dateitypen übernehmen“ ein.",
  },
};
