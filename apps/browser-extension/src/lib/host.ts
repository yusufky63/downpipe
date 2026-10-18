import { HOST_NAME, parseResponse, type HostMessage, type HostResponse } from "./protocol";

/** `runtime.sendNativeMessage`, injected so tests can stand in for the browser. */
export type NativeSend = (host: string, message: object) => Promise<unknown>;

/** Ways the bridge itself can fail, before Downpipe gets a say. */
export type BridgeFailure =
  | "host_missing"
  | "host_forbidden"
  | "host_error"
  | "timeout"
  | "bad_response";

export type HostResult =
  | { ok: true; response: HostResponse }
  | { ok: false; failure: BridgeFailure; detail: string };

/**
 * Map the browser's error text to a failure kind. Chrome and Firefox word
 * these differently and neither offers error codes.
 */
export function classifyNativeError(message: string): BridgeFailure {
  const m = message.toLowerCase();
  if (m.includes("not found") || m.includes("no such native application")) return "host_missing";
  if (m.includes("forbidden") || m.includes("does not have permission")) return "host_forbidden";
  return "host_error";
}

/**
 * Send one message and wait at most `timeoutMs` for the answer. A host from
 * before `start_app` refuses the field as an invalid message; such a host
 * never starts Downpipe, so the message goes again without it.
 */
export async function callHost(
  send: NativeSend,
  message: HostMessage,
  timeoutMs: number,
): Promise<HostResult> {
  const result = await callOnce(send, message, timeoutMs);
  if (
    "start_app" in message &&
    result.ok &&
    result.response.type === "error" &&
    result.response.code === "invalid_message"
  ) {
    const without = { ...message };
    delete without.start_app;
    return callOnce(send, without, timeoutMs);
  }
  return result;
}

async function callOnce(send: NativeSend, message: HostMessage, timeoutMs: number): Promise<HostResult> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const timeout = new Promise<"timeout">((resolve) => {
    timer = setTimeout(() => resolve("timeout"), timeoutMs);
  });
  try {
    const raw = await Promise.race([send(HOST_NAME, message), timeout]);
    if (raw === "timeout") {
      return { ok: false, failure: "timeout", detail: `No answer within ${Math.round(timeoutMs / 1000)} s` };
    }
    const response = parseResponse(raw);
    if (!response || (response.id !== undefined && response.id !== message.id)) {
      return { ok: false, failure: "bad_response", detail: "The browser bridge sent an unexpected answer" };
    }
    return { ok: true, response };
  } catch (err) {
    const detail = err instanceof Error ? err.message : String(err);
    return { ok: false, failure: classifyNativeError(detail), detail: detail.slice(0, 300) };
  } finally {
    clearTimeout(timer);
  }
}
