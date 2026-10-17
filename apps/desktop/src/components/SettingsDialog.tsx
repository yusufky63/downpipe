import { FolderOpen } from "lucide-react";
import { useState } from "react";

import { api, errorMessage, type Settings } from "../lib/api";
import { bytesToRate, rateToBytes, type RateUnit } from "../lib/format";
import { Button, Dialog, Field, inputClass } from "./primitives";

export function SettingsDialog({
  settings,
  onClose,
  onSaved,
}: {
  settings: Settings;
  onClose: () => void;
  onSaved: (s: Settings) => void;
}) {
  const initialRate = bytesToRate(settings.speed_limit);
  const [dir, setDir] = useState(settings.download_dir);
  const [concurrent, setConcurrent] = useState(String(settings.max_concurrent_downloads));
  const [connections, setConnections] = useState(String(settings.max_connections_per_download));
  const [limit, setLimit] = useState(initialRate.value > 0 ? String(initialRate.value) : "");
  const [unit, setUnit] = useState<RateUnit>(initialRate.unit);
  const [error, setError] = useState<string | null>(null);

  const save = async () => {
    try {
      const saved = await api.updateSettings({
        download_dir: dir.trim(),
        max_concurrent_downloads: Number.parseInt(concurrent, 10) || settings.max_concurrent_downloads,
        max_connections_per_download: Number.parseInt(connections, 10) || settings.max_connections_per_download,
        speed_limit: rateToBytes(Number.parseFloat(limit), unit),
      });
      onSaved(saved);
      onClose();
    } catch (err) {
      setError(errorMessage(err));
    }
  };

  return (
    <Dialog
      eyebrow="Settings"
      title="Downloads"
      onClose={onClose}
      footer={
        <>
          {error && <p className="mr-auto text-[13px] text-danger-fg">{error}</p>}
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" onClick={() => void save()}>
            Save
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-5">
        <Field label="Default folder">
          {(id) => (
            <div className="flex gap-2">
              <input id={id} value={dir} onChange={(e) => setDir(e.target.value)} spellCheck={false} className={`${inputClass} h-9 font-mono text-[13px]`} />
              <Button
                onClick={async () => {
                  const picked = await api.pickFolder(dir || null).catch(() => null);
                  if (picked) setDir(picked);
                }}
              >
                <FolderOpen size={16} strokeWidth={1.75} />
                Browse
              </Button>
            </div>
          )}
        </Field>

        <div className="grid grid-cols-1 gap-5 sm:grid-cols-2">
          <Field label="Downloads at once" hint="Others wait in the queue. 1 to 16.">
            {(id) => (
              <input
                id={id}
                type="number"
                min={1}
                max={16}
                value={concurrent}
                onChange={(e) => setConcurrent(e.target.value)}
                className={`${inputClass} num h-9 font-mono`}
              />
            )}
          </Field>
          <Field label="Connections per download" hint="Upper limit. Downpipe starts with 2 and adds more only while they help. 1 to 32.">
            {(id) => (
              <input
                id={id}
                type="number"
                min={1}
                max={32}
                value={connections}
                onChange={(e) => setConnections(e.target.value)}
                className={`${inputClass} num h-9 font-mono`}
              />
            )}
          </Field>
        </div>

        <Field label="Speed limit" hint="Shared by all downloads. Leave empty for no limit. Applies immediately.">
          {(id) => (
            <div className="flex gap-2">
              <input
                id={id}
                type="number"
                min={0}
                step="any"
                value={limit}
                placeholder="No limit"
                onChange={(e) => setLimit(e.target.value)}
                className={`${inputClass} num h-9 font-mono`}
              />
              <select
                aria-label="Speed limit unit"
                value={unit}
                onChange={(e) => setUnit(e.target.value as RateUnit)}
                className={`${inputClass} h-9 w-28 font-mono`}
              >
                <option value="KiB">KiB/s</option>
                <option value="MiB">MiB/s</option>
              </select>
            </div>
          )}
        </Field>
      </div>
    </Dialog>
  );
}
