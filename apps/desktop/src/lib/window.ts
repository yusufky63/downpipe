// What every Downpipe window does, the main one and the confirmation window.
import { useEffect } from "react";

import { isBrowserShortcut, isTextField } from "./shortcuts";
import { THEME_KEY, applyTheme, parseTheme } from "./theme";

/**
 * An app, not a web page: no browser menu (Back, Reload, Inspect) and no
 * browser keys. Text fields keep their own menu for cut, copy and paste.
 */
export function useAppWindow() {
  useEffect(() => {
    const onContextMenu = (e: MouseEvent) => {
      if (!isTextField(e.target)) e.preventDefault();
    };
    // Capture: runs before any handler can stop the key from reaching the window.
    const onKey = (e: KeyboardEvent) => {
      if (isBrowserShortcut(e, import.meta.env.DEV)) e.preventDefault();
    };
    window.addEventListener("contextmenu", onContextMenu);
    window.addEventListener("keydown", onKey, true);
    return () => {
      window.removeEventListener("contextmenu", onContextMenu);
      window.removeEventListener("keydown", onKey, true);
    };
  }, []);
}

/** Follow a theme picked in another window. */
export function useThemeFromOtherWindows() {
  useEffect(() => {
    const onStorage = (e: StorageEvent) => {
      if (e.key === THEME_KEY) applyTheme(parseTheme(e.newValue));
    };
    window.addEventListener("storage", onStorage);
    return () => window.removeEventListener("storage", onStorage);
  }, []);
}
