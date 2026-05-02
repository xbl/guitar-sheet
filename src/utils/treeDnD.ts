/** HTML5 drag payload for library tree reorder */
export const TREE_DND_MIME = "application/x-guitar-sheet-tree"

export type TreeDndPayload =
  | { kind: "sheet"; id: string }
  | { kind: "folder"; id: string }

export function setTreeDragPayload(e: DragEvent, payload: TreeDndPayload): void {
  const raw = JSON.stringify(payload)
  e.dataTransfer?.setData(TREE_DND_MIME, raw)
  e.dataTransfer?.setData("text/plain", `${payload.kind}:${payload.id}`)
  if (e.dataTransfer) e.dataTransfer.effectAllowed = "move"
}

export function readTreeDragPayload(e: DragEvent): TreeDndPayload | null {
  const raw = e.dataTransfer?.getData(TREE_DND_MIME)
  if (!raw) return null
  try {
    const v = JSON.parse(raw) as TreeDndPayload
    if (
      v &&
      (v.kind === "sheet" || v.kind === "folder") &&
      typeof v.id === "string"
    ) {
      return v
    }
  } catch {
    /* ignore */
  }
  return null
}
