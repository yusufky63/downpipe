// Small dialogs: removing downloads, and fetching a finished file again.
import type { Messages } from "../core";

export const dialogs: Messages["dialogs"] = {
  // "Retirer" takes a download off the list; "supprimer" deletes the file on disk.
  remove: {
    deleteFile: "Retirer et supprimer le fichier",
    deleteFiles: "Retirer et supprimer les fichiers",
    deletePartial: "Retirer et supprimer les données partielles",
    fromList: "Retirer de la liste",
    doneOne: "Si vous le retirez de la liste, le fichier reste sur le disque. Si vous le supprimez, le fichier lui-même est aussi effacé.",
    doneMany: "Si vous les retirez de la liste, les fichiers restent sur le disque. Si vous les supprimez, les fichiers eux-mêmes sont aussi effacés.",
    partialOne:
      "Si vous le retirez de la liste, le fichier partiel est conservé et le téléchargement pourra reprendre plus tard avec le même lien. Si vous le supprimez, les données partielles sont aussi effacées.",
    partialMany:
      "Si vous les retirez de la liste, les fichiers terminés et les données partielles restent sur le disque. Si vous les supprimez, les deux sont effacés.",
  },
  redownload: {
    eyebrow: "Télécharger à nouveau",
    body: "Télécharge de nouveau le fichier depuis le même lien dans le même dossier, par exemple s'il a changé sur le serveur ou si la copie sur le disque est endommagée.",
    replace: "Remplacer le fichier existant",
    replaceOn: "Le fichier actuel est d'abord supprimé, et le nouveau prend son nom.",
    replaceOff: "Désactivé : le fichier actuel est conservé, et la nouvelle copie est enregistrée à côté sous un nom disponible.",
    confirm: "Télécharger à nouveau",
    confirmReplace: "Supprimer et télécharger à nouveau",
  },
  duplicate: {
    eyebrow: "Déjà dans la liste",
    titleMany: {
      one: "{count} lien figure déjà dans la liste",
      many: "{count} liens figurent déjà dans la liste",
      other: "{count} liens figurent déjà dans la liste",
    },
    body: "Un téléchargement avec cette adresse figure déjà dans la liste ({state}). Que voulez-vous faire ?",
    bodyMany: {
      one: "Un téléchargement avec la même adresse figure déjà dans la liste. Que voulez-vous faire ?",
      many: "Des téléchargements avec ces adresses figurent déjà dans la liste. Que voulez-vous faire de ces {count} liens ?",
      other: "Des téléchargements avec ces adresses figurent déjà dans la liste. Que voulez-vous faire de ces {count} liens ?",
    },
    question: "Que faire du lien",
    copy: "L'ajouter à nouveau avec un nom de fichier numéroté",
    overwrite: "L'ajouter à nouveau et remplacer le fichier existant",
    existing: "Afficher le téléchargement existant s'il est terminé, sinon le reprendre",
    ok: "OK",
  },
};
