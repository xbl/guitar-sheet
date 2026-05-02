/** Backend filter for sheets only under library root (no `folder_id`). */
export const FOLDER_ROOT_SENTINEL = "__root__"

export interface SheetMeta {
  id: string;
  display_title: string;
  kind: string;
  local_rel_path: string;
  absolute_path: string;
  local_content_hash: string;
  remote_path: string | null;
  folder_id: string | null;
  artist: string | null;
  tags: string[];
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
