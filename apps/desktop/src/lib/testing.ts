// Test fixtures. Only tests import this file.
import type { Task } from "./api";

export function makeTask(id: string, patch: Partial<Task> = {}): Task {
  return {
    id,
    url: `https://example.org/${id}.bin`,
    dir: "C:/Downloads",
    file_name: null,
    path: null,
    state: "queued",
    total: null,
    downloaded: 0,
    error: null,
    error_kind: null,
    checksum: null,
    max_connections: null,
    category: null,
    auto_dir: true,
    queue: "main",
    position: 0,
    created_at: 1,
    updated_at: 1,
    completed_at: null,
    speed: 0,
    connections: 0,
    target_connections: 0,
    resumable: false,
    segments: [],
    notices: [],
    awaiting_confirmation: false,
    from_app: false,
    media: null,
    site: null,
    fragments: null,
    eta_secs: null,
    ...patch,
  };
}
