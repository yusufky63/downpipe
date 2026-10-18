// Small dialogs: removing downloads, and fetching a finished file again.
import type { Messages } from "../core";

export const dialogs: Messages["dialogs"] = {
  remove: {
    deleteFile: "Quitar y eliminar archivo",
    deleteFiles: "Quitar y eliminar archivos",
    deletePartial: "Quitar y eliminar datos parciales",
    fromList: "Quitar de la lista",
    doneOne: "Quitarla de la lista conserva el archivo en el disco. Eliminar también borra el propio archivo.",
    doneMany: "Quitarlas de la lista conserva los archivos en el disco. Eliminar también borra los propios archivos.",
    partialOne:
      "Quitarla de la lista conserva el archivo parcial, así que el mismo enlace puede reanudarse más tarde. Eliminar también borra los datos parciales.",
    partialMany: "Quitarlas de la lista conserva en el disco los archivos terminados y los datos parciales. Eliminar borra ambos.",
  },
  redownload: {
    eyebrow: "Volver a descargar",
    body: "Descarga de nuevo el archivo desde el mismo enlace en la misma carpeta, por ejemplo, cuando cambió en el servidor o la copia en el disco está dañada.",
    replace: "Reemplazar el archivo existente",
    replaceOn: "Primero se elimina el archivo actual y el nuevo toma su nombre.",
    replaceOff: "Desactivado: el archivo actual se conserva y la nueva copia se guarda junto a él con un nombre libre.",
    confirm: "Volver a descargar",
    confirmReplace: "Eliminar y volver a descargar",
  },
  duplicate: {
    eyebrow: "Ya está en la lista",
    titleMany: {
      one: "{count} enlace ya está en la lista",
      many: "{count} enlaces ya están en la lista",
      other: "{count} enlaces ya están en la lista",
    },
    body: "Ya hay una descarga con esta dirección en la lista ({state}). ¿Qué quieres hacer?",
    bodyMany: {
      one: "Ya hay una descarga con la misma dirección en la lista. ¿Qué quieres hacer?",
      many: "Ya hay descargas con estas direcciones en la lista. ¿Qué quieres hacer con las {count}?",
      other: "Ya hay descargas con estas direcciones en la lista. ¿Qué quieres hacer con las {count}?",
    },
    question: "Qué hacer con el enlace",
    copy: "Volver a agregarlo con un nombre de archivo numerado",
    overwrite: "Volver a agregarlo y reemplazar el archivo existente",
    existing: "Mostrar la descarga existente si terminó; si no, reanudarla",
    ok: "Aceptar",
  },
};
