// The media file name template (Settings, Media), for its preview. The app
// names files with the same rules (crates/providers/src/naming.rs), checked
// against the same cases (crates/providers/tests/fixtures/naming.json).

export const DEFAULT_TEMPLATE = "{title}";

/** The placeholders a template knows, in the order the settings list them. */
export const PLACEHOLDERS = ["title", "channel", "date", "index", "id"] as const;

export interface NameFields {
  title?: string | null;
  channel?: string | null;
  /** yt-dlp's upload_date, YYYYMMDD. */
  upload_date?: string | null;
  /** Position in the playlist, 1 first. */
  index?: number | null;
  /** Videos in that playlist, for zero padding. */
  count?: number | null;
  id?: string | null;
}

const SEPARATORS = /^[\s\-_.,|·–—]*$/;

function field(name: string, f: NameFields): string | undefined {
  const clean = (v: string | null | undefined) => (v ?? "").trim();
  switch (name) {
    case "title":
      return clean(f.title);
    case "channel":
      return clean(f.channel);
    case "id":
      return clean(f.id);
    case "date": {
      const raw = clean(f.upload_date);
      return /^\d{8}$/.test(raw) ? `${raw.slice(0, 4)}-${raw.slice(4, 6)}-${raw.slice(6)}` : "";
    }
    case "index": {
      if (f.index === null || f.index === undefined) return "";
      const width = String(Math.max(f.count ?? f.index, f.index)).length;
      return String(f.index).padStart(width, "0");
    }
    default:
      return undefined;
  }
}

/**
 * The template with the fields filled in. Unknown `{words}` stay. Text made
 * only of separators is kept only between two filled parts, so
 * "{index} - {title}" of a single video is just its title.
 */
export function renderTemplate(template: string, fields: NameFields): string {
  type Part = { text: string; field: boolean };
  const parts: Part[] = [];
  let text = "";
  let rest = template;
  for (let open = rest.indexOf("{"); open !== -1; open = rest.indexOf("{")) {
    text += rest.slice(0, open);
    const after = rest.slice(open + 1);
    const close = after.indexOf("}");
    const value = close === -1 ? undefined : field(after.slice(0, close), fields);
    if (value === undefined) {
      text += "{";
      rest = after;
      continue;
    }
    if (text) parts.push({ text, field: false });
    text = "";
    parts.push({ text: value, field: true });
    rest = after.slice(close + 1);
  }
  text += rest;
  if (text) parts.push({ text, field: false });

  let out = "";
  let pending: string | null = null;
  for (const part of parts) {
    if (!part.field && SEPARATORS.test(part.text)) {
      if (out && pending === null) pending = part.text;
      continue;
    }
    if (part.field && !part.text) continue;
    if (pending !== null) out += pending;
    pending = null;
    out += part.text;
  }
  return out.trim();
}

/** Whether every video would get the same name (no title or id in it). */
export function namesAlike(template: string): boolean {
  return !template.includes("{title}") && !template.includes("{id}");
}
