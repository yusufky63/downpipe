import { FolderOpen } from "lucide-react";

import { useI18n } from "../i18n";
import { api } from "../lib/api";
import { Button, inputClass } from "./primitives";

/** A path field with a Browse button; the picker runs in Rust. */
export function FolderInput({
  id,
  value,
  placeholder,
  onChange,
}: {
  id: string;
  value: string;
  placeholder?: string;
  onChange: (v: string) => void;
}) {
  const { t } = useI18n();
  return (
    <div className="flex gap-2">
      <input id={id} value={value} placeholder={placeholder} onChange={(e) => onChange(e.target.value)} spellCheck={false} className={`${inputClass} h-9 font-mono text-[13px]`} />
      <Button
        onClick={async () => {
          const picked = await api.pickFolder(value || null).catch(() => null);
          if (picked) onChange(picked);
        }}
      >
        <FolderOpen size={16} strokeWidth={1.75} />
        {t("common.browse")}
      </Button>
    </div>
  );
}
