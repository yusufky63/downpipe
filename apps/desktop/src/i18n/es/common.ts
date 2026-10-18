// Words used across the app. {placeholders} are filled in at run time.
import type { Messages } from "../core";

export const common: Messages["common"] = {
  appName: "Downpipe",
  cancel: "Cancelar",
  save: "Guardar",
  close: "Cerrar",
  browse: "Examinar",
  dismiss: "Descartar",
  keep: "Conservar",
  open: "Abrir",
  pause: "Pausar",
  resume: "Reanudar",
  retry: "Reintentar",
  remove: "Quitar",
  showInFolder: "Mostrar en la carpeta",
  moreActions: "Más acciones",
  unknown: "Desconocido",
  none: "–",
  siteVideo: "Video de {site}",
  downloads: { one: "{count} descarga", many: "{count} descargas", other: "{count} descargas" },
  links: { one: "{count} enlace", many: "{count} enlaces", other: "{count} enlaces" },
};

export const states: Messages["states"] = {
  confirm: "Confirmar",
  queued: "En cola",
  starting: "Conectando",
  downloading: "Descargando",
  waiting: "Reintentando",
  verifying: "Verificando",
  paused: "En pausa",
  completed: "Completada",
  failed: "Fallida",
  canceled: "Cancelada",
  why: {
    noData: "El servidor no envía datos, reconectando",
    network: "Sin red, se reintentará cuando vuelva",
    throttled: "El servidor pidió ir más despacio, esperando",
    server: "Error del servidor, reintentando",
  },
  filters: {
    all: "Todas",
    active: "Activas",
    queued: "En cola",
    completed: "Completadas",
    failed: "Fallidas",
  },
};
