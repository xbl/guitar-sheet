import type { FolderNode } from "../types/folder"
import type { SheetMeta } from "../types/sheet"

export type LibraryTreeRow =
  | { kind: "folder"; id: string; name: string; children: LibraryTreeRow[] }
  | { kind: "sheet"; id: string; title: string; sheetKind: string }

function cmpZh(a: string, b: string) {
  return a.localeCompare(b, "zh-CN", { sensitivity: "accent" })
}

function groupSheetsByFolder(sheets: SheetMeta[]) {
  const m = new Map<string | null, SheetMeta[]>()
  for (const s of sheets) {
    const k = s.folder_id
    if (!m.has(k)) m.set(k, [])
    m.get(k)!.push(s)
  }
  for (const arr of m.values()) {
    arr.sort((a, b) => cmpZh(a.display_title, b.display_title))
  }
  return m
}

function folderToTree(
  n: FolderNode,
  byFolder: Map<string | null, SheetMeta[]>,
): LibraryTreeRow {
  const childFolders = n.children
    .map((c) => folderToTree(c, byFolder))
    .sort((a, b) => {
      if (a.kind !== "folder" || b.kind !== "folder") return 0
      return cmpZh(a.name, b.name)
    })
  const sheetsHere = (byFolder.get(n.id) ?? []).map(
    (s): LibraryTreeRow => ({
      kind: "sheet",
      id: s.id,
      title: s.display_title,
      sheetKind: s.kind,
    }),
  )
  const children = [...childFolders, ...sheetsHere]
  return { kind: "folder", id: n.id, name: n.name, children }
}

/** Folders (nested) plus sheets under each folder; root-level sheets (`folder_id == null`) after top-level folders. */
export function buildLibraryTree(
  folderRoots: FolderNode[],
  sheets: SheetMeta[],
): LibraryTreeRow[] {
  const byFolder = groupSheetsByFolder(sheets)
  const folderRows = folderRoots
    .map((r) => folderToTree(r, byFolder))
    .sort((a, b) => {
      if (a.kind !== "folder" || b.kind !== "folder") return 0
      return cmpZh(a.name, b.name)
    })
  const rootSheets = (byFolder.get(null) ?? []).map(
    (s): LibraryTreeRow => ({
      kind: "sheet",
      id: s.id,
      title: s.display_title,
      sheetKind: s.kind,
    }),
  )
  return [...folderRows, ...rootSheets]
}
