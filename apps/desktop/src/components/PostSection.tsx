import { Check, FolderOpen, ShieldAlert, TriangleAlert } from "lucide-react";
import { useState, type ReactNode } from "react";

import { useI18n } from "../i18n";
import { errorMessage, type PostActions, type Settings, type Task } from "../lib/api";
import {
  categoryExtracts,
  commandText,
  extractText,
  inheritedCommand,
  isArchive,
  reportTone,
  scanText,
} from "../lib/post";
import { displayName, type Tone } from "../lib/tasks";
import { PostFields } from "./PostFields";
import { Banner, Button } from "./primitives";

/**
 * The details dialog's "After download": what the scan, the extraction and
 * the command found once the download completed, and before that its own
 * command and extraction choice.
 */
export function PostSection({
  task,
  settings,
  onOpenExtracted,
  onSave,
}: {
  task: Task;
  settings: Settings | null;
  onOpenExtracted: (id: string) => void;
  onSave: (id: string, actions: PostActions) => Promise<void>;
}) {
  const { t } = useI18n();
  const report = task.post;
  const editable = task.state !== "completed" && settings !== null;
  if (!report && !task.post_step && !editable) return null;

  return (
    <section className="flex flex-col gap-2.5" aria-label={t("post.section")}>
      <span className="label">{t("post.section")}</span>
      {task.post_step && <Banner tone="neutral">{t(`post.steps.${task.post_step}`)}…</Banner>}
      {report && (report.scan || report.extract || report.command) && (
        <dl className="divide-y divide-line rounded-lg border border-line">
          {report.scan && (
            <ReportRow label={t("post.scan.label")} tone={reportTone(report.scan.result)}>
              {scanText(report.scan)}
            </ReportRow>
          )}
          {report.extract && (
            <ReportRow label={t("post.extract.label")} tone={reportTone(report.extract.result)}>
              <span>{extractText(report.extract)}</span>
              {report.extract.folder && (
                <span className="flex min-w-0 items-center gap-1">
                  <span className="truncate font-mono text-[12px] text-ink-secondary select-text" title={report.extract.folder}>
                    {report.extract.folder}
                  </span>
                  <Button size="sm" variant="ghost" onClick={() => onOpenExtracted(task.id)}>
                    <FolderOpen size={14} strokeWidth={1.75} />
                    {t("post.extract.openFolder")}
                  </Button>
                </span>
              )}
            </ReportRow>
          )}
          {report.command && (
            <ReportRow label={t("post.command.label")} tone={reportTone(report.command.result)}>
              <span>{commandText(report.command)}</span>
              {report.command.output && (
                <details>
                  <summary className="cursor-pointer text-[12px] text-ink-secondary">{t("post.command.output")}</summary>
                  <pre className="mt-1 max-h-40 overflow-auto rounded-md bg-surface-muted p-2 font-mono text-[11px] whitespace-pre-wrap text-ink-secondary select-text">
                    {report.command.output}
                  </pre>
                </details>
              )}
            </ReportRow>
          )}
        </dl>
      )}
      {editable && <EditPost key={task.id} task={task} settings={settings} onSave={onSave} />}
    </section>
  );
}

const TONE_ICONS = {
  positive: { Icon: Check, cls: "text-positive-fg" },
  danger: { Icon: ShieldAlert, cls: "text-danger-fg" },
  warning: { Icon: TriangleAlert, cls: "text-warning-fg" },
  primary: { Icon: Check, cls: "text-primary" },
  neutral: { Icon: TriangleAlert, cls: "text-ink-muted" },
} as const;

function ReportRow({ label, tone, children }: { label: string; tone: Tone; children: ReactNode }) {
  const { Icon, cls } = TONE_ICONS[tone];
  return (
    <div className="grid grid-cols-1 gap-1 px-4 py-2.5 sm:grid-cols-[96px_minmax(0,1fr)] sm:items-start sm:gap-3">
      <dt className="label pt-0.5">{label}</dt>
      <dd className="m-0 flex min-w-0 items-start gap-2 text-[13px] text-ink">
        <Icon size={16} strokeWidth={1.75} aria-hidden="true" className={`mt-0.5 shrink-0 ${cls}`} />
        <span className="flex min-w-0 flex-col gap-1">{children}</span>
      </dd>
    </div>
  );
}

/** The download's own command and extraction choice, before it completes. */
function EditPost({ task, settings, onSave }: { task: Task; settings: Settings; onSave: (id: string, actions: PostActions) => Promise<void> }) {
  const { t } = useI18n();
  const [command, setCommand] = useState(task.post_actions.command);
  const [extract, setExtract] = useState<boolean | null>(task.post_actions.extract);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const name = displayName(task);
  const changed = command.trim() !== task.post_actions.command || extract !== task.post_actions.extract;

  const save = async () => {
    setError(null);
    try {
      await onSave(task.id, { command: command.trim(), extract });
      setSaved(true);
    } catch (err) {
      setError(errorMessage(err));
    }
  };

  return (
    <div className="flex flex-col gap-2 rounded-lg border border-line p-4">
      <PostFields
        command={command}
        onCommand={(c) => {
          setCommand(c);
          setSaved(false);
        }}
        inherited={inheritedCommand(settings, task.queue)}
        archive={task.media === null && task.site === null && isArchive(name)}
        extract={extract ?? categoryExtracts(settings, name)}
        onExtract={(x) => {
          setExtract(x);
          setSaved(false);
        }}
      />
      <div className="flex items-center gap-3">
        <Button size="sm" onClick={() => void save()} disabled={!changed}>
          {t("post.fields.save")}
        </Button>
        {error ? (
          <span className="text-[12px] text-danger-fg">{error}</span>
        ) : (
          saved && <span className="text-[12px] text-ink-muted">{t("post.fields.saved")}</span>
        )}
      </div>
    </div>
  );
}
