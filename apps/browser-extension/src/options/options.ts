// Capture filters and media detection settings. Changes are saved only with
// the Save button, through the service worker, which writes the settings one
// change at a time.
import { t } from "../i18n";
import { ext } from "../lib/browser";
import {
  DEFAULT_CAPTURE_TYPES,
  filtersFromForm,
  parseMinSize,
  sizeFields,
  type FormErrors,
  type SizeUnit,
} from "../lib/filters";
import { parseBypassKey } from "../lib/bypass";
import { DEFAULT_MEDIA_TYPES, MAX_MEDIA_MIN_BYTES, WEB_ORIGINS, parseMediaTypeList, type MediaFilters } from "../lib/media";
import { isSettings, type Settings } from "../lib/messages";
import { byId, startPage } from "../ui/dom";

startPage();

const DOWNLOADS_PERMISSION: chrome.permissions.Permissions = { permissions: ["downloads"] };
/** What the click script for Alt+click needs: to run in the pages. */
const SITES_PERMISSION: chrome.permissions.Permissions = { origins: WEB_ORIGINS };
/** Longer lists end in "and N more" so one bad paste does not fill the page. */
const MAX_SHOWN_ERRORS = 5;

const els = {
  form: byId<HTMLFormElement>("form"),
  captureOff: byId("capture-off"),
  loadError: byId("load-error"),
  captureAll: byId<HTMLInputElement>("capture-all"),
  types: byId<HTMLTextAreaElement>("types"),
  typesField: byId<HTMLTextAreaElement>("types").closest<HTMLElement>(".field"),
  typesDefault: byId<HTMLButtonElement>("types-default"),
  typesErrors: byId<HTMLUListElement>("types-errors"),
  sites: byId<HTMLTextAreaElement>("sites"),
  sitesErrors: byId<HTMLUListElement>("sites-errors"),
  size: byId<HTMLInputElement>("size"),
  unit: byId<HTMLSelectElement>("unit"),
  sizeErrors: byId<HTMLUListElement>("size-errors"),
  mediaOverlay: byId<HTMLInputElement>("media-overlay"),
  mediaTypes: byId<HTMLTextAreaElement>("media-types"),
  mediaTypesDefault: byId<HTMLButtonElement>("media-types-default"),
  mediaTypesErrors: byId<HTMLUListElement>("media-types-errors"),
  mediaSize: byId<HTMLInputElement>("media-size"),
  mediaUnit: byId<HTMLSelectElement>("media-unit"),
  mediaSizeErrors: byId<HTMLUListElement>("media-size-errors"),
  bypassKey: byId<HTMLSelectElement>("bypass-key"),
  bypassAccess: byId("bypass-access"),
  bypassAllow: byId<HTMLButtonElement>("bypass-allow"),
  save: byId<HTMLButtonElement>("save"),
  state: byId("save-state"),
};

/** Edits not saved yet; while there are any, outside changes are not loaded over them. */
let dirty = false;

function setState(text: string, tone: "muted" | "positive" | "danger" = "muted"): void {
  els.state.textContent = text;
  els.state.dataset.tone = tone;
}

function showErrors(list: HTMLUListElement, field: HTMLElement, errors: readonly string[]): void {
  const shown = errors.slice(0, MAX_SHOWN_ERRORS).map((text) => {
    const li = document.createElement("li");
    li.textContent = text;
    return li;
  });
  if (errors.length > MAX_SHOWN_ERRORS) {
    const li = document.createElement("li");
    li.textContent = t("options.moreErrors", { count: errors.length - MAX_SHOWN_ERRORS });
    shown.push(li);
  }
  list.replaceChildren(...shown);
  field.setAttribute("aria-invalid", String(errors.length > 0));
}

function paintErrors(errors: FormErrors): void {
  showErrors(els.typesErrors, els.types, errors.types);
  showErrors(els.sitesErrors, els.sites, errors.sites);
  showErrors(els.sizeErrors, els.size, errors.size);
}

const NO_ERRORS: FormErrors = { types: [], sites: [], size: [] };

function paintMediaErrors(types: readonly string[], size: readonly string[]): void {
  showErrors(els.mediaTypesErrors, els.mediaTypes, types);
  showErrors(els.mediaSizeErrors, els.mediaSize, size);
}

/** The media card as stored, or what to say under its fields. */
function mediaFromForm(): { ok: true; media: MediaFilters } | { ok: false; types: string[]; size: string[] } {
  const types = parseMediaTypeList(els.mediaTypes.value);
  const size = parseMinSize(els.mediaSize.value, els.mediaUnit.value as SizeUnit);
  const sizeErrors = size.ok ? (size.bytes > MAX_MEDIA_MIN_BYTES ? [t("options.errors.sizeTooBig")] : []) : [size.error];
  if (types.errors.length || !size.ok || sizeErrors.length) return { ok: false, types: types.errors, size: sizeErrors };
  return { ok: true, media: { mediaOverlay: els.mediaOverlay.checked, mediaTypes: types.values, mediaMinBytes: size.bytes } };
}

function syncTypes(): void {
  const off = els.captureAll.checked;
  els.types.disabled = off;
  els.typesDefault.disabled = off;
  els.typesField?.setAttribute("data-off", String(off));
}

function fill(s: Settings): void {
  els.captureAll.checked = s.captureAll;
  els.types.value = s.captureTypes.join(" ");
  els.sites.value = s.excludedSites.join("\n");
  const size = sizeFields(s.minSizeBytes);
  els.size.value = size.value;
  els.unit.value = size.unit;
  els.mediaOverlay.checked = s.mediaOverlay;
  els.mediaTypes.value = s.mediaTypes.join(" ");
  const mediaSize = sizeFields(s.mediaMinBytes);
  els.mediaSize.value = mediaSize.value;
  els.mediaUnit.value = mediaSize.unit;
  els.bypassKey.value = s.bypassKey;
  syncTypes();
  void loadAccess();
  paintErrors(NO_ERRORS);
  paintMediaErrors([], []);
  dirty = false;
}

async function loadSettings(): Promise<void> {
  const reply: unknown = await ext.runtime.sendMessage({ kind: "get_settings" }).catch(() => null);
  const ok = isSettings(reply);
  els.loadError.hidden = ok;
  els.save.disabled = !ok;
  if (ok) fill(reply);
}

async function loadCapture(): Promise<void> {
  els.captureOff.hidden = await ext.permissions.contains(DOWNLOADS_PERMISSION).catch(() => true);
}

/** The key needs access to the sites; say so, with a button, while it is missing. */
async function loadAccess(): Promise<void> {
  const granted = await ext.permissions.contains(SITES_PERMISSION).catch(() => false);
  els.bypassAccess.hidden = granted || els.bypassKey.value === "off";
}

function edited(): void {
  dirty = true;
  setState(t("options.unsaved"));
}

els.form.addEventListener("input", edited);
els.captureAll.addEventListener("change", () => {
  syncTypes();
  edited();
});
els.types.addEventListener("input", () => showErrors(els.typesErrors, els.types, []));
els.sites.addEventListener("input", () => showErrors(els.sitesErrors, els.sites, []));
els.size.addEventListener("input", () => showErrors(els.sizeErrors, els.size, []));
els.mediaTypes.addEventListener("input", () => showErrors(els.mediaTypesErrors, els.mediaTypes, []));
els.mediaSize.addEventListener("input", () => showErrors(els.mediaSizeErrors, els.mediaSize, []));
els.mediaOverlay.addEventListener("change", edited);
els.bypassKey.addEventListener("change", () => {
  edited();
  void loadAccess();
});
els.bypassAllow.addEventListener("click", () => {
  // Asked from the click itself: browsers grant permissions only during one.
  void ext.permissions
    .request(SITES_PERMISSION)
    .catch(() => false)
    .then(() => loadAccess());
});

els.mediaTypesDefault.addEventListener("click", () => {
  els.mediaTypes.value = DEFAULT_MEDIA_TYPES.join(" ");
  showErrors(els.mediaTypesErrors, els.mediaTypes, []);
  edited();
});

els.typesDefault.addEventListener("click", () => {
  els.types.value = DEFAULT_CAPTURE_TYPES.join(" ");
  showErrors(els.typesErrors, els.types, []);
  edited();
});

els.form.addEventListener("submit", (event) => {
  event.preventDefault();
  const result = filtersFromForm({
    captureAll: els.captureAll.checked,
    typesText: els.types.value,
    sitesText: els.sites.value,
    sizeText: els.size.value,
    unit: els.unit.value as SizeUnit,
  });
  const media = mediaFromForm();
  paintErrors(result.ok ? NO_ERRORS : result.errors);
  paintMediaErrors(media.ok ? [] : media.types, media.ok ? [] : media.size);
  if (!result.ok || !media.ok) {
    setState(t("options.notSavedFix"), "danger");
    if (!result.ok) {
      const { types, sites } = result.errors;
      (types.length ? els.types : sites.length ? els.sites : els.size).focus();
    } else if (!media.ok) {
      (media.types.length ? els.mediaTypes : els.mediaSize).focus();
    }
    return;
  }
  els.save.disabled = true;
  setState(t("options.saving"));
  // Three writes, one after the other: the service worker chains them anyway.
  void ext.runtime
    .sendMessage({ kind: "save_filters", filters: result.filters })
    .then((reply: unknown) => (isSettings(reply) ? ext.runtime.sendMessage({ kind: "save_media", media: media.media }) : null))
    .then((reply: unknown) =>
      isSettings(reply) ? ext.runtime.sendMessage({ kind: "set_bypass_key", key: parseBypassKey(els.bypassKey.value) }) : null,
    )
    .catch(() => null)
    .then((reply: unknown) => {
      if (!isSettings(reply)) {
        setState(t("options.notSavedNoAnswer"), "danger");
        return;
      }
      // Shows the lists as stored: deduplicated, lower case, hosts in ASCII.
      fill(reply);
      setState(t("options.saved"), "positive");
    })
    .finally(() => {
      els.save.disabled = false;
    });
});

// The popup's "Pause capture on this site" also changes the settings.
ext.storage.onChanged.addListener((changes, area) => {
  if (area === "local" && changes.settings && !dirty) void loadSettings();
});
ext.permissions.onAdded.addListener(() => {
  void loadCapture();
  void loadAccess();
});
ext.permissions.onRemoved.addListener(() => {
  void loadCapture();
  void loadAccess();
});

window.addEventListener("beforeunload", (event) => {
  if (dirty) event.preventDefault();
});

void loadSettings();
void loadCapture();
