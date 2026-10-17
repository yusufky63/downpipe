// The light or dark choice, shared by every window: they load from the same
// origin, so they read and write the same storage. public/theme.js applies
// it before first paint.

export type Theme = "light" | "dark";

export const THEME_KEY = "downpipe:theme";

/** A stored choice; null follows the system. */
export function parseTheme(value: string | null): Theme | null {
  return value === "light" || value === "dark" ? value : null;
}

/** Apply a stored choice to this window, or follow the system. */
export function applyTheme(theme: Theme | null) {
  if (theme) document.documentElement.setAttribute("data-theme", theme);
  else document.documentElement.removeAttribute("data-theme");
}
