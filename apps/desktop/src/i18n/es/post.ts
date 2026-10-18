// After a download completes: the virus scan, extracting archives and a command.
import type { Messages } from "../core";

export const post: Messages["post"] = {
  steps: {
    scanning: "Analizando",
    extracting: "Extrayendo",
    running_command: "Ejecutando comando",
  },
  threat: "Amenaza",
  summary: {
    threat: "Amenaza encontrada: {threat}",
    notExtracted: "No extraído",
    commandFailed: "El comando posterior a la descarga falló",
  },
  section: "Al terminar la descarga",
  scan: {
    label: "Análisis antivirus",
    clean: "{scanner} no encontró nada.",
    threat:
      "{scanner} encontró {threat}. No se extrajo ni se ejecutó nada y el archivo se dejó como estaba: eliminarlo es decisión tuya.",
    unnamed: "una amenaza",
    theScanner: "El analizador",
    missing: "No analizado: no hay ningún analizador configurado. Elige uno en Ajustes, Al terminar la descarga.",
    missingNamed: "No analizado: no se encontró {scanner}.",
    failed: "{scanner} no pudo analizar el archivo: {detail}",
  },
  extract: {
    label: "Extraído",
    done: { one: "{count} archivo en una carpeta nueva.", many: "{count} archivos en una carpeta nueva.", other: "{count} archivos en una carpeta nueva." },
    skipped: {
      one: "Se omitió {count} entrada: un nombre inseguro, un enlace o un duplicado.",
      many: "Se omitieron {count} entradas: nombres inseguros, enlaces o duplicados.",
      other: "Se omitieron {count} entradas: nombres inseguros, enlaces o duplicados.",
    },
    deleted: "Después se eliminó el archivo comprimido.",
    openFolder: "Abrir carpeta",
    failed: "No extraído: {reason}.",
    errors: {
      encrypted: "el archivo comprimido está protegido con contraseña",
      too_large: "se expandiría mucho más allá de su propio tamaño (posible bomba zip)",
      too_many_entries: "tiene demasiadas entradas",
      corrupt: "el archivo comprimido está dañado o usa un método no compatible",
      io: "no se pudieron escribir los archivos",
    },
  },
  command: {
    label: "Comando",
    ok: "{program} terminó.",
    failed: "{program} terminó con el código de salida {code}.",
    ended: "{program} se detuvo antes de terminar.",
    timedOut: "{program} superó el límite de tiempo y se detuvo.",
    notStarted: "No se pudo iniciar {program}: {detail}",
    invalid: "El comando no se puede ejecutar: {detail}",
    theCommand: "El comando",
    output: "Salida",
  },
  fields: {
    heading: "Al terminar la descarga",
    command: "Ejecutar al terminar",
    inherited: "Predeterminado: {command}",
    nothing: "Nada",
    hint: "Un programa y sus argumentos, como notepad {path}. {path}, {name}, {folder} y {url} se rellenan, cada uno como un solo argumento, sin shell. No se ejecuta si la descarga falla.",
    extract: "Extraer el archivo comprimido",
    extractHint: "En una carpeta nueva junto a él, cuando termine.",
    save: "Guardar",
    saved: "Guardado. Se aplica cuando termine la descarga.",
  },
  settings: {
    tab: "Al terminar la descarga",
    command: "Ejecutar un comando después de cada descarga",
    commandHint:
      "Un programa y sus argumentos, como notepad {path}. {path}, {name}, {folder} y {url} se rellenan, cada uno como un solo argumento; nada pasa por un shell, así que los nombres de archivo y los enlaces no pueden añadir comandos. Las colas y cada descarga pueden tener el suyo. No se ejecuta si una descarga falla.",
    timeout: "Límite de tiempo",
    timeoutHint: "Segundos que un comando o un análisis puede durar antes de detenerse, junto con todo lo que inició. De 5 a 86400.",
    seconds: "s",
    scan: "Analizar las descargas en busca de virus",
    scanHintWindows:
      "Con Microsoft Defender, después de cada descarga. Una amenaza marca la descarga y muestra una notificación; nada se elimina ni se pone en cuarentena, y nada se extrae ni se ejecuta.",
    scanHintOther:
      "Con el analizador de abajo, después de cada descarga. Una amenaza marca la descarga y muestra una notificación; nada se elimina, se extrae ni se ejecuta.",
    scanner: "Analizador",
    scannerPlaceholderWindows: "Microsoft Defender",
    scannerHint:
      "Vacío usa Microsoft Defender en Windows. Cualquier otro analizador recibe el archivo como {path} o como último argumento; el código de salida 0 significa limpio y 1 una amenaza, como en clamscan de ClamAV.",
    archives: "Archivos comprimidos",
    archivesHint:
      "Los archivos zip, tar, tar.gz, tar.xz y 7z se extraen en una carpeta nueva junto a ellos cuando su categoría lo indica (Categorías) o la descarga lo pide. Se rechazan las entradas que quedarían fuera de esa carpeta, los enlaces y los archivos que se expanden mucho más allá de su tamaño. RAR no es compatible; 7-Zip puede extraerlo como comando.",
    deleteArchive: "Eliminar el archivo comprimido después de extraerlo",
    deleteArchiveHint: "Solo cuando se extrajeron todas las entradas, y después de ejecutar el comando.",
    queueCommand: "Comando después de las descargas de {name}",
    queueCommandUnnamed: "Comando después de las descargas de la cola",
    queueCommandPlaceholder: "Comando al terminar; vacío: el de Al terminar la descarga",
    categoryExtract: "Extraer archivos comprimidos",
    categoryExtractTitle: "Extraer los archivos zip, tar y 7z de esta categoría después de descargarlos",
  },
};
