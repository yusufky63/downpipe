// Small dialogs: removing downloads, fetching a finished file again, and links already in the list.
import type { Messages } from "../core";

export const dialogs: Messages["dialogs"] = {
  remove: {
    deleteFile: "Entfernen und Datei löschen",
    deleteFiles: "Entfernen und Dateien löschen",
    deletePartial: "Entfernen und Teildaten löschen",
    fromList: "Aus Liste entfernen",
    doneOne: "Beim Entfernen aus der Liste bleibt die Datei auf dem Datenträger. Beim Löschen wird auch die Datei selbst entfernt.",
    doneMany: "Beim Entfernen aus der Liste bleiben die Dateien auf dem Datenträger. Beim Löschen werden auch die Dateien selbst entfernt.",
    partialOne:
      "Beim Entfernen aus der Liste bleibt die unvollständige Datei erhalten, sodass derselbe Link später noch fortgesetzt werden kann. Beim Löschen werden auch die Teildaten entfernt.",
    partialMany: "Beim Entfernen aus der Liste bleiben fertige Dateien und Teildaten auf dem Datenträger. Beim Löschen wird beides entfernt.",
  },
  redownload: {
    eyebrow: "Erneut herunterladen",
    body: "Lädt die Datei über denselben Link erneut in denselben Ordner herunter, etwa wenn sie sich auf dem Server geändert hat oder die Kopie auf dem Datenträger beschädigt ist.",
    replace: "Vorhandene Datei ersetzen",
    replaceOn: "Die aktuelle Datei wird zuerst gelöscht, und die neue erhält ihren Namen.",
    replaceOff: "Aus: Die aktuelle Datei bleibt erhalten, und die neue Kopie wird daneben unter einem freien Namen gespeichert.",
    confirm: "Erneut herunterladen",
    confirmReplace: "Löschen und erneut herunterladen",
  },
  duplicate: {
    eyebrow: "Bereits in der Liste",
    titleMany: { one: "{count} Link ist bereits in der Liste", other: "{count} Links sind bereits in der Liste" },
    body: "Ein Download mit dieser Adresse ist bereits in der Liste ({state}). Wie möchten Sie fortfahren?",
    bodyMany: {
      one: "Ein Download mit derselben Adresse ist bereits in der Liste. Wie möchten Sie fortfahren?",
      other: "Downloads mit diesen Adressen sind bereits in der Liste. Was soll mit allen {count} geschehen?",
    },
    question: "Umgang mit dem Link",
    copy: "Erneut mit nummeriertem Dateinamen hinzufügen",
    overwrite: "Erneut hinzufügen und vorhandene Datei ersetzen",
    existing: "Vorhandenen Download anzeigen, wenn er fertig ist, sonst fortsetzen",
    ok: "OK",
    remember: "Nicht mehr fragen, immer so verfahren",
    rememberHint: "Unter Einstellungen, Downloads lässt es sich wieder ändern.",
  },
  /** A download's own speed limit, under the global one. {limit} is like "2 MiB/s". */
  speedLimit: {
    eyebrow: "Geschwindigkeitslimit",
    body: "Nur für diesen Download, unter dem globalen Limit von {limit}.",
    bodyNoGlobal: "Nur für diesen Download. Es gibt kein globales Limit.",
    bodyMany: "Für jeden einzeln, unter dem globalen Limit von {limit}.",
    bodyManyNoGlobal: "Für jeden einzeln. Es gibt kein globales Limit.",
    label: "Limit",
    unit: "Einheit des Geschwindigkeitslimits",
    placeholder: "Kein Limit",
    video: "Ein Video übernimmt ein neues Limit beim nächsten Start.",
    remove: "Limit entfernen",
    save: "Limit setzen",
  },
  /** "Pause until…" with a time of the user's choice. */
  pauseUntil: {
    eyebrow: "Pausieren bis",
    label: "Fortsetzen um",
    hint: "Pausiert jetzt und setzt zu dieser Zeit von selbst fort, solange Downpipe läuft, sonst sobald es danach startet.",
    past: "Wählen Sie eine Zeit in der Zukunft.",
    invalid: "Geben Sie ein Datum und eine Uhrzeit ein.",
    submit: "Pausieren",
  },
};
