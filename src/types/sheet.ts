export interface SheetMeta {
  id: string;
  display_title: string;
  kind: string;
  local_rel_path: string;
  absolute_path: string;
  local_content_hash: string;
  remote_path: string | null;
}

export interface ConflictEntry {
  sheet_id: string;
  display_title: string;
  remote_path: string;
  remote_sha: string;
  local_disk_hash: string;
  last_known_remote_sha: string | null;
}

export interface SyncOutcome {
  ok: boolean;
  conflicts: ConflictEntry[];
  pulled: number;
  pushed: number;
}
