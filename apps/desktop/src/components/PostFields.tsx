import { useId } from "react";

import { useI18n } from "../i18n";
import { commandPlaceholder } from "../lib/post";
import { inputClass } from "./primitives";

/**
 * A download's own post-download choices: a command line, and whether to
 * extract it when it is an archive. Empty values follow the queue, the
 * category and Settings, which the placeholder and the box's start say.
 */
export function PostFields({
  command,
  onCommand,
  inherited,
  archive,
  extract,
  onExtract,
  compact = false,
}: {
  command: string;
  onCommand: (command: string) => void;
  /** What runs when the field stays empty. */
  inherited: string;
  /** The file is an archive Downpipe can extract. */
  archive: boolean;
  extract: boolean;
  onExtract: (extract: boolean) => void;
  /** One line, for the confirmation dialog's grid. */
  compact?: boolean;
}) {
  const { t } = useI18n();
  const id = useId();
  const input = (
    <input
      id={id}
      value={command}
      onChange={(e) => onCommand(e.target.value)}
      spellCheck={false}
      placeholder={commandPlaceholder(inherited)}
      title={t("post.fields.hint")}
      className={`${inputClass} ${compact ? "h-8 text-[12px]" : "h-9 text-[13px]"} min-w-0 flex-1 font-mono`}
    />
  );
  const box = archive && (
    <label className="flex shrink-0 items-center gap-1.5 text-[12px] text-ink-secondary" title={t("post.fields.extractHint")}>
      <input type="checkbox" checked={extract} onChange={(e) => onExtract(e.target.checked)} className="size-3.5 accent-primary" />
      {t("post.fields.extract")}
    </label>
  );

  if (compact) {
    return (
      <>
        <label htmlFor={id} className="label">
          {t("post.fields.command")}
        </label>
        <div className="flex min-w-0 items-center gap-2">
          {input}
          {box}
        </div>
      </>
    );
  }
  return (
    <div className="flex flex-col gap-1.5">
      <label htmlFor={id} className="label">
        {t("post.fields.command")}
      </label>
      <div className="flex min-w-0 items-center gap-3">{input}</div>
      <p className="text-[12px] text-ink-muted">{t("post.fields.hint")}</p>
      {box && <div className="pt-1">{box}</div>}
    </div>
  );
}
