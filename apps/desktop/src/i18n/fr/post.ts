// After a download completes: the virus scan, extracting archives and a command.
import type { Messages } from "../core";

export const post: Messages["post"] = {
  steps: {
    scanning: "Analyse",
    extracting: "Extraction",
    running_command: "Commande en cours",
  },
  threat: "Menace",
  summary: {
    threat: "Menace détectée : {threat}",
    notExtracted: "Non extrait",
    commandFailed: "La commande après le téléchargement a échoué",
  },
  section: "Après le téléchargement",
  scan: {
    label: "Analyse antivirus",
    clean: "{scanner} n'a rien trouvé.",
    threat:
      "{scanner} a détecté {threat}. Rien n'a été extrait ni exécuté, et le fichier a été laissé tel quel : c'est à vous de décider de le supprimer.",
    unnamed: "une menace",
    theScanner: "L'antivirus",
    missing: "Non analysé : aucun antivirus n'est configuré. Choisissez-en un dans Paramètres, Après le téléchargement.",
    missingNamed: "Non analysé : {scanner} est introuvable.",
    failed: "{scanner} n'a pas pu analyser le fichier : {detail}",
  },
  extract: {
    label: "Extrait",
    done: { one: "{count} fichier dans un nouveau dossier.", many: "{count} fichiers dans un nouveau dossier.", other: "{count} fichiers dans un nouveau dossier." },
    skipped: {
      one: "{count} élément a été ignoré : un nom dangereux, un lien ou un doublon.",
      many: "{count} éléments ont été ignorés : noms dangereux, liens ou doublons.",
      other: "{count} éléments ont été ignorés : noms dangereux, liens ou doublons.",
    },
    deleted: "L'archive a ensuite été supprimée.",
    openFolder: "Ouvrir le dossier",
    failed: "Non extrait : {reason}.",
    errors: {
      encrypted: "l'archive est protégée par un mot de passe",
      too_large: "elle prendrait bien plus que sa propre taille (une bombe zip possible)",
      too_many_entries: "elle contient trop d'éléments",
      corrupt: "l'archive est endommagée ou utilise une méthode non prise en charge",
      io: "les fichiers n'ont pas pu être écrits",
    },
  },
  command: {
    label: "Commande",
    ok: "{program} a terminé.",
    failed: "{program} s'est arrêté avec le code de sortie {code}.",
    ended: "{program} a été arrêté avant la fin.",
    timedOut: "{program} a dépassé le délai et a été arrêté.",
    notStarted: "Impossible de lancer {program} : {detail}",
    invalid: "La commande ne peut pas s'exécuter : {detail}",
    theCommand: "La commande",
    output: "Sortie",
  },
  fields: {
    heading: "Après le téléchargement",
    command: "Exécuter à la fin",
    inherited: "Par défaut : {command}",
    nothing: "Rien",
    hint: "Un programme et ses arguments, comme notepad {path}. {path}, {name}, {folder} et {url} sont remplis, chacun comme un seul argument, sans shell. Ne s'exécute pas si le téléchargement échoue.",
    extract: "Extraire l'archive",
    extractHint: "Dans un nouveau dossier à côté, une fois terminée.",
    save: "Enregistrer",
    saved: "Enregistré. S'applique à la fin du téléchargement.",
  },
  settings: {
    tab: "Après le téléchargement",
    command: "Exécuter une commande après chaque téléchargement",
    commandHint:
      "Un programme et ses arguments, comme notepad {path}. {path}, {name}, {folder} et {url} sont remplis, chacun comme un seul argument ; rien ne passe par un shell, donc les noms de fichiers et les liens ne peuvent pas ajouter de commandes. Les files d'attente et chaque téléchargement peuvent avoir la leur. Ne s'exécute pas si un téléchargement échoue.",
    timeout: "Délai",
    timeoutHint: "Secondes pendant lesquelles une commande ou une analyse peut tourner avant d'être arrêtée, avec tout ce qu'elle a lancé. De 5 à 86400.",
    seconds: "s",
    scan: "Analyser les téléchargements contre les virus",
    scanHintWindows:
      "Avec Microsoft Defender, après chaque téléchargement. Une menace marque le téléchargement et affiche une notification ; rien n'est supprimé ni mis en quarantaine, et rien n'est extrait ni exécuté.",
    scanHintOther:
      "Avec l'antivirus ci-dessous, après chaque téléchargement. Une menace marque le téléchargement et affiche une notification ; rien n'est supprimé, extrait ni exécuté.",
    scanner: "Antivirus",
    scannerPlaceholderWindows: "Microsoft Defender",
    scannerHint:
      "Vide utilise Microsoft Defender sous Windows. Tout autre antivirus reçoit le fichier comme {path} ou en dernier argument ; le code de sortie 0 signifie sain et 1 une menace, comme avec clamscan de ClamAV.",
    archives: "Archives",
    archivesHint:
      "Les archives zip, tar, tar.gz, tar.xz et 7z sont extraites dans un nouveau dossier à côté d'elles quand leur catégorie le prévoit (Catégories) ou que le téléchargement le demande. Les éléments qui atterriraient hors de ce dossier, les liens et les archives qui prendraient bien plus que leur taille sont refusés. RAR n'est pas pris en charge ; 7-Zip peut l'extraire en tant que commande.",
    deleteArchive: "Supprimer l'archive après l'extraction",
    deleteArchiveHint: "Seulement si tous les éléments ont été extraits, et après l'exécution de la commande.",
    queueCommand: "Commande après les téléchargements de {name}",
    queueCommandUnnamed: "Commande après les téléchargements de la file",
    queueCommandPlaceholder: "Commande à la fin ; vide : celle d'Après le téléchargement",
    categoryExtract: "Extraire les archives",
    categoryExtractTitle: "Extraire les archives zip, tar et 7z de cette catégorie après leur téléchargement",
  },
};
