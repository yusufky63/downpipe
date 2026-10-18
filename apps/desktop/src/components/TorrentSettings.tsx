import { useI18n } from "../i18n";
import type { RateUnit } from "../lib/format";
import type { TorrentDraft } from "../lib/torrent";
import { Field, Switch, inputClass } from "./primitives";

/** Settings, BitTorrent: seeding, the upload limit and how peers are found. */
export function TorrentSettingsPanel({ draft, onChange }: { draft: TorrentDraft; onChange: (draft: TorrentDraft) => void }) {
  const { t } = useI18n();
  const set = (patch: Partial<TorrentDraft>) => onChange({ ...draft, ...patch });
  const number = `${inputClass} num h-9 font-mono`;
  return (
    <>
      <Switch label={t("torrent.settings.seed")} hint={t("torrent.settings.seedHint")} checked={draft.seed} onChange={(seed) => set({ seed })} />
      <div className="grid grid-cols-1 gap-5 sm:grid-cols-2">
        <Field label={t("torrent.settings.ratio")} hint={t("torrent.settings.ratioHint")}>
          {(id) => (
            <input
              id={id}
              type="number"
              min={0}
              max={100}
              step={0.1}
              value={draft.ratio}
              disabled={!draft.seed}
              onChange={(e) => set({ ratio: e.target.value })}
              className={`${number} disabled:opacity-50`}
            />
          )}
        </Field>
        <Field label={t("torrent.settings.minutes")} hint={t("torrent.settings.minutesHint")}>
          {(id) => (
            <input
              id={id}
              type="number"
              min={0}
              max={43200}
              value={draft.minutes}
              disabled={!draft.seed}
              onChange={(e) => set({ minutes: e.target.value })}
              className={`${number} disabled:opacity-50`}
            />
          )}
        </Field>
      </div>
      <div className="grid grid-cols-1 gap-5 sm:grid-cols-2">
        <Field label={t("torrent.settings.uploadLimit")} hint={t("torrent.settings.uploadLimitHint")}>
          {(id) => (
            // Widths on wrappers: the shared input class sets its own.
            <div className="flex gap-2">
              <div className="min-w-0 flex-1">
                <input
                  id={id}
                  type="number"
                  min={0}
                  step="any"
                  value={draft.upload}
                  placeholder={t("settings.downloads.noLimit")}
                  onChange={(e) => set({ upload: e.target.value })}
                  className={number}
                />
              </div>
              <div className="w-28 shrink-0">
                <select
                  aria-label={t("settings.downloads.unit")}
                  value={draft.uploadUnit}
                  onChange={(e) => set({ uploadUnit: e.target.value as RateUnit })}
                  className={`${inputClass} h-9 font-mono`}
                >
                  <option value="KiB">KiB/s</option>
                  <option value="MiB">MiB/s</option>
                </select>
              </div>
            </div>
          )}
        </Field>
        <Field label={t("torrent.settings.port")} hint={t("torrent.settings.portHint")}>
          {(id) => (
            <input id={id} type="number" min={0} max={65535} value={draft.port} onChange={(e) => set({ port: e.target.value })} className={number} />
          )}
        </Field>
      </div>
      <Switch label={t("torrent.settings.dht")} hint={t("torrent.settings.dhtHint")} checked={draft.dht} onChange={(dht) => set({ dht })} />
      <Switch label={t("torrent.settings.lsd")} hint={t("torrent.settings.lsdHint")} checked={draft.lsd} onChange={(lsd) => set({ lsd })} />
      <Switch label={t("torrent.settings.upnp")} hint={t("torrent.settings.upnpHint")} checked={draft.upnp} onChange={(upnp) => set({ upnp })} />
      <div className="flex flex-col gap-2 border-t border-line pt-4 text-[12px] text-ink-muted">
        <p>{t("torrent.settings.pex")}</p>
        <p>{t("torrent.settings.privacy")}</p>
        <p>{t("torrent.settings.applies")}</p>
      </div>
    </>
  );
}
