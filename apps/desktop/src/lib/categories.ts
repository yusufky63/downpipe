// The categories Downpipe creates are stored in the settings under their
// English names, which are also their folders' names on disk. The UI shows
// them in the user's language until the user renames them; the stored name
// and the folders stay as they are.
import { t, type MessageKey } from "../i18n/core";

/** `default_categories()` in crates/core/src/settings.rs. */
const BUILTIN: Readonly<Record<string, MessageKey>> = {
  Compressed: "builtin.categories.compressed",
  Documents: "builtin.categories.documents",
  Music: "builtin.categories.music",
  Video: "builtin.categories.video",
  Programs: "builtin.categories.programs",
  "Disk images": "builtin.categories.diskImages",
};

/** A category's name as the UI shows it. */
export function categoryLabel(name: string): string {
  const key = Object.hasOwn(BUILTIN, name) ? BUILTIN[name] : undefined;
  return key ? t(key) : name;
}

/**
 * The name to store for what the settings field holds: a built-in name the
 * user left as shown keeps its stored English name, so its folder does not
 * move; anything else is the user's own name.
 */
export function storedCategoryName(stored: string, typed: string): string {
  const name = typed.trim();
  return name === categoryLabel(stored) ? stored : name;
}
