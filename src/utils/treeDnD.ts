/** 谱库树内拖移：Tauri/WKWebView 不同页内 HTML5 dragover/drop，故用指针拖动 + 内存 payload */

export type TreeDndPayload =
  | { kind: "sheet"; id: string }
  | { kind: "folder"; id: string }

let activeLibraryDrag: TreeDndPayload | null = null

export function setLibraryPointerPayload(p: TreeDndPayload): void {
  activeLibraryDrag = p
}

export function takeLibraryPointerPayload(): TreeDndPayload | null {
  const v = activeLibraryDrag
  activeLibraryDrag = null
  return v
}

export function clearLibraryPointerPayload(): void {
  activeLibraryDrag = null
}

type HoverCb = (x: number, y: number) => void
type DropCb = (x: number, y: number) => void | Promise<void>

let hoverCb: HoverCb | null = null
let dropCb: DropCb | null = null

export function registerLibraryPointerUi(hover: HoverCb | null, drop: DropCb | null): void {
  hoverCb = hover
  dropCb = drop
}

export function notifyLibraryPointerHover(x: number, y: number): void {
  hoverCb?.(x, y)
}

export async function notifyLibraryPointerDrop(x: number, y: number): Promise<void> {
  await dropCb?.(x, y)
}
