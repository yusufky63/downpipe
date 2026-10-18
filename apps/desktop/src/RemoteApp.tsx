import { LogOut, Pause, Play, Plus, Trash2 } from "lucide-react";
import { useCallback, useEffect, useId, useState, type FormEvent } from "react";

import { Badge, Banner, Button, Field, IconButton, inputClass } from "./components/primitives";
import { LocaleProvider, useI18n, useLocale } from "./i18n";
import { formatBytes, formatDuration, formatSpeed, percent } from "./lib/format";
import {
  RemoteError,
  TOKEN_KEY,
  cleanToken,
  remoteCanPause,
  remoteCanResume,
  remoteClient,
  remoteErrorText,
  remoteStateInfo,
  tokenFromHash,
  type RemoteClient,
  type RemoteDownload,
  type RemoteQueue,
} from "./lib/remote";

/** How often the list is read while the page is visible. */
const POLL_MS = 2000;

function readToken(): string | null {
  try {
    return localStorage.getItem(TOKEN_KEY);
  } catch {
    return null;
  }
}

function storeToken(token: string | null) {
  try {
    if (token) localStorage.setItem(TOKEN_KEY, token);
    else localStorage.removeItem(TOKEN_KEY);
  } catch {
    // Storage blocked (private mode): the token lives until the tab closes.
  }
}

/** A token in the link (from the QR code) wins, and leaves the address bar at once. */
function initialToken(): string | null {
  const fromLink = tokenFromHash(window.location.hash);
  if (fromLink) {
    storeToken(fromLink);
    window.history.replaceState(null, "", window.location.pathname);
    return fromLink;
  }
  return readToken();
}

export function RemoteApp() {
  // The phone's own language, not the computer's.
  const locale = useLocale("");
  const [token, setToken] = useState<string | null>(initialToken);
  const [signInError, setSignInError] = useState<string | null>(null);

  const signOut = useCallback((reason: string | null) => {
    storeToken(null);
    setToken(null);
    setSignInError(reason);
  }, []);

  // A link with a new token opened while the page is already open changes
  // only the fragment, which does not load the page again.
  useEffect(() => {
    const onHash = () => {
      const next = initialToken();
      if (next) {
        setSignInError(null);
        setToken(next);
      }
    };
    window.addEventListener("hashchange", onHash);
    return () => window.removeEventListener("hashchange", onHash);
  }, []);

  return (
    <LocaleProvider locale={locale}>
      <div className="h-full overflow-y-auto">
        <div className="mx-auto flex min-h-full w-full max-w-xl flex-col gap-5 px-4 pt-5 pb-10">
          {token ? (
            <Remote key={token} token={token} onSignOut={signOut} />
          ) : (
            <SignIn
              error={signInError}
              onToken={(t) => {
                storeToken(t);
                setSignInError(null);
                setToken(t);
              }}
            />
          )}
        </div>
      </div>
    </LocaleProvider>
  );
}

function Header({ onSignOut }: { onSignOut?: () => void }) {
  const { t } = useI18n();
  return (
    <header className="flex items-center justify-between gap-3">
      <div className="flex min-w-0 flex-col gap-1">
        {/* A name, not a word: no locale's capitals (Turkish dotted İ) apply. */}
        <span className="eyebrow" lang="en" translate="no">
          Downpipe
        </span>
        <h1 className="display text-[22px] leading-tight text-ink">{t("remote.page.title")}</h1>
      </div>
      {onSignOut && (
        <IconButton label={t("remote.page.signOut")} onClick={onSignOut}>
          <LogOut size={18} strokeWidth={1.75} />
        </IconButton>
      )}
    </header>
  );
}

function SignIn({ error, onToken }: { error: string | null; onToken: (token: string) => void }) {
  const { t } = useI18n();
  const [value, setValue] = useState("");
  const submit = (e: FormEvent) => {
    e.preventDefault();
    const token = cleanToken(value);
    if (token) onToken(token);
  };
  return (
    <>
      <Header />
      <form onSubmit={submit} className="ticks flex flex-col gap-4 rounded-md border border-line bg-surface p-4">
        <h2 className="display-medium text-[17px] text-ink">{t("remote.page.signIn.title")}</h2>
        <p className="text-[13px] text-ink-secondary">{t("remote.page.signIn.hint")}</p>
        {error && <Banner tone="danger">{error}</Banner>}
        <Field label={t("remote.page.signIn.label")}>
          {(id) => (
            <input
              id={id}
              value={value}
              autoComplete="off"
              autoCapitalize="none"
              autoCorrect="off"
              spellCheck={false}
              onChange={(e) => setValue(e.target.value)}
              className={`${inputClass} h-11 font-mono text-[15px]`}
            />
          )}
        </Field>
        <Button type="submit" variant="primary" className="h-11" disabled={cleanToken(value) === ""}>
          {t("remote.page.signIn.submit")}
        </Button>
      </form>
    </>
  );
}

function Remote({ token, onSignOut }: { token: string; onSignOut: (reason: string | null) => void }) {
  const { t } = useI18n();
  const [client] = useState<RemoteClient>(() => remoteClient(token));
  const [downloads, setDownloads] = useState<RemoteDownload[] | null>(null);
  const [queues, setQueues] = useState<RemoteQueue[]>([]);
  const [problem, setProblem] = useState<string | null>(null);

  /** Signs out on a rejected token; anything else is shown above the list. */
  const fail = useCallback(
    (err: unknown) => {
      const code = err instanceof RemoteError ? err.code : "failed";
      if (code === "unauthorized" || code === "too_many_attempts") onSignOut(remoteErrorText(code));
      else setProblem(remoteErrorText(code));
    },
    [onSignOut],
  );

  const refresh = useCallback(async () => {
    try {
      setDownloads(await client.list());
      setProblem(null);
    } catch (err) {
      fail(err);
    }
  }, [client, fail]);

  useEffect(() => {
    let live = true;
    const load = () =>
      client.list().then(
        (list) => {
          if (!live) return;
          setDownloads(list);
          setProblem(null);
        },
        (err: unknown) => live && fail(err),
      );
    client.settings().then(
      (s) => live && setQueues(s.queues),
      (err: unknown) => live && fail(err),
    );
    void load();
    // Only while the page is visible: a phone in a pocket should not keep
    // the radio busy.
    const timer = window.setInterval(() => {
      if (document.visibilityState === "visible") void load();
    }, POLL_MS);
    const onVisible = () => {
      if (document.visibilityState === "visible") void load();
    };
    document.addEventListener("visibilitychange", onVisible);
    return () => {
      live = false;
      window.clearInterval(timer);
      document.removeEventListener("visibilitychange", onVisible);
    };
  }, [client, fail]);

  const act = async (action: () => Promise<unknown>) => {
    try {
      await action();
    } catch (err) {
      fail(err);
    }
    await refresh();
  };

  return (
    <>
      <Header onSignOut={() => onSignOut(null)} />
      <AddLink client={client} queues={queues} onAdded={refresh} onError={fail} />
      {problem && <Banner tone="danger">{problem}</Banner>}
      <section aria-labelledby="remote-list" className="flex flex-col gap-2">
        <h2 id="remote-list" className="label">
          {t("remote.page.list")}
        </h2>
        {downloads?.length === 0 && <p className="py-6 text-center text-[14px] text-ink-muted">{t("remote.page.empty")}</p>}
        <ul className="flex flex-col gap-2">
          {downloads?.map((d) => (
            <Row
              key={d.id}
              download={d}
              onPause={() => void act(() => client.pause(d.id))}
              onResume={() => void act(() => client.resume(d.id))}
              onRemove={() => void act(() => client.remove(d.id))}
            />
          ))}
        </ul>
      </section>
    </>
  );
}

function AddLink({
  client,
  queues,
  onAdded,
  onError,
}: {
  client: RemoteClient;
  queues: RemoteQueue[];
  onAdded: () => Promise<void>;
  onError: (err: unknown) => void;
}) {
  const { t } = useI18n();
  const formId = useId();
  const [url, setUrl] = useState("");
  const [folder, setFolder] = useState("");
  const [queue, setQueue] = useState("");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<{ tone: "neutral" | "danger"; text: string } | null>(null);
  /** The link is in the list already; the user may add it once more. */
  const [duplicate, setDuplicate] = useState(false);

  const add = async (paused: boolean, addCopy = false) => {
    setBusy(true);
    setMessage(null);
    try {
      const download = await client.add({
        url: url.trim(),
        folder: folder.trim() || undefined,
        queue: queue || undefined,
        paused,
        add_copy: addCopy,
      });
      setUrl("");
      setDuplicate(false);
      setMessage({ tone: "neutral", text: t("remote.page.add.added", { name: download.name }) });
      await onAdded();
    } catch (err) {
      const code = err instanceof RemoteError ? err.code : "failed";
      if (code === "unauthorized" || code === "too_many_attempts") {
        onError(err);
        return;
      }
      setDuplicate(code === "duplicate");
      setMessage({ tone: "danger", text: remoteErrorText(code) });
    } finally {
      setBusy(false);
    }
  };

  return (
    <form
      aria-labelledby={formId}
      onSubmit={(e) => {
        e.preventDefault();
        void add(false);
      }}
      className="ticks flex flex-col gap-4 rounded-md border border-line bg-surface p-4"
    >
      <h2 id={formId} className="display-medium text-[17px] text-ink">
        {t("remote.page.add.title")}
      </h2>
      <Field label={t("remote.page.add.url")}>
        {(id) => (
          <input
            id={id}
            type="url"
            inputMode="url"
            required
            value={url}
            placeholder="https://"
            autoCapitalize="none"
            autoCorrect="off"
            spellCheck={false}
            onChange={(e) => {
              setUrl(e.target.value);
              setDuplicate(false);
            }}
            className={`${inputClass} h-11 font-mono text-[14px]`}
          />
        )}
      </Field>
      <div className={`grid grid-cols-1 gap-4 ${queues.length > 1 ? "sm:grid-cols-[minmax(0,1fr)_160px]" : ""}`}>
        <Field label={t("remote.page.add.folder")} hint={t("remote.page.add.folderHint")}>
          {(id) => (
            <input
              id={id}
              value={folder}
              autoCapitalize="none"
              spellCheck={false}
              onChange={(e) => setFolder(e.target.value)}
              className={`${inputClass} h-11 text-[15px]`}
            />
          )}
        </Field>
        {queues.length > 1 && (
          <Field label={t("remote.page.add.queue")}>
            {(id) => (
              <select id={id} value={queue} onChange={(e) => setQueue(e.target.value)} className={`${inputClass} h-11 text-[15px]`}>
                {queues.map((q, i) => (
                  <option key={q.id} value={i === 0 ? "" : q.id}>
                    {q.name}
                  </option>
                ))}
              </select>
            )}
          </Field>
        )}
      </div>
      {message && <Banner tone={message.tone}>{message.text}</Banner>}
      <div className="flex flex-wrap gap-2">
        {duplicate ? (
          <Button variant="primary" className="h-11 flex-1" disabled={busy} onClick={() => void add(false, true)}>
            <Plus size={16} strokeWidth={1.75} />
            {t("remote.page.add.addAgain")}
          </Button>
        ) : (
          <Button type="submit" variant="primary" className="h-11 flex-1" disabled={busy || url.trim() === ""}>
            <Plus size={16} strokeWidth={1.75} />
            {t("remote.page.add.start")}
          </Button>
        )}
        <Button className="h-11" disabled={busy || url.trim() === ""} onClick={() => void add(true, duplicate)}>
          {t("remote.page.add.later")}
        </Button>
      </div>
    </form>
  );
}

function Row({
  download: d,
  onPause,
  onResume,
  onRemove,
}: {
  download: RemoteDownload;
  onPause: () => void;
  onResume: () => void;
  onRemove: () => void;
}) {
  const { t } = useI18n();
  const [asking, setAsking] = useState(false);
  const state = remoteStateInfo(d);
  const share = percent(d.downloaded, d.total);
  const running = d.state === "downloading" || d.state === "starting" || d.state === "waiting" || d.state === "verifying";
  const amount = d.total !== null ? t("remote.page.of", { done: formatBytes(d.downloaded), total: formatBytes(d.total) }) : formatBytes(d.downloaded);
  const details = [d.host, d.quality].filter(Boolean).join(" · ");

  return (
    <li className="flex flex-col gap-2 rounded-md border border-line bg-canvas p-3">
      <div className="flex min-w-0 items-start gap-2">
        <div className="flex min-w-0 flex-1 flex-col gap-0.5">
          <span className="truncate text-[15px] font-medium text-ink" title={d.name}>
            {d.name}
          </span>
          {details && <span className="truncate text-[12px] text-ink-muted">{details}</span>}
        </div>
        <Badge tone={state.tone}>{state.label}</Badge>
      </div>
      {d.state !== "completed" && (
        <div
          role="progressbar"
          aria-label={d.name}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={share === null ? undefined : Math.round(share)}
          className="h-1.5 overflow-hidden rounded-full bg-surface-muted"
        >
          <div
            className={`h-full rounded-full ${d.state === "failed" ? "bg-danger" : "bg-primary"} transition-[width] duration-300`}
            style={{ width: `${share ?? (running ? 100 : 0)}%`, opacity: share === null && running ? 0.35 : 1 }}
          />
        </div>
      )}
      <div className="flex items-center gap-2">
        {/* Wraps rather than cuts: the time left is what a glance is for. */}
        <span className="num flex min-w-0 flex-1 flex-wrap gap-x-2 font-mono text-[12px] text-ink-secondary">
          <span>{d.state === "completed" ? formatBytes(d.total ?? d.downloaded) : amount}</span>
          {running && d.speed > 0 && <span>{formatSpeed(d.speed)}</span>}
          {running && d.eta_secs !== null && <span>{t("remote.page.left", { time: formatDuration(d.eta_secs) })}</span>}
        </span>
        {remoteCanPause(d) && (
          <IconButton label={t("remote.page.pause", { name: d.name })} className="size-10" onClick={onPause}>
            <Pause size={18} strokeWidth={1.75} />
          </IconButton>
        )}
        {remoteCanResume(d) && (
          <IconButton label={t("remote.page.resume", { name: d.name })} className="size-10" onClick={onResume}>
            <Play size={18} strokeWidth={1.75} />
          </IconButton>
        )}
        <IconButton label={t("remote.page.remove", { name: d.name })} tone="danger" className="size-10" onClick={() => setAsking(!asking)}>
          <Trash2 size={18} strokeWidth={1.75} />
        </IconButton>
      </div>
      {asking && (
        <div className="flex flex-col gap-2 rounded-md bg-surface-muted p-3">
          <p className="text-[13px] text-ink-secondary">{t("remote.page.removeQuestion", { name: d.name })}</p>
          <div className="flex gap-2">
            <Button variant="danger" className="h-10 flex-1" onClick={onRemove}>
              {t("common.remove")}
            </Button>
            <Button variant="ghost" className="h-10 flex-1" onClick={() => setAsking(false)}>
              {t("common.cancel")}
            </Button>
          </div>
        </div>
      )}
    </li>
  );
}
