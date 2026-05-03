import { confirm as tauriConfirm } from "@tauri-apps/plugin-dialog"

function hasTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window
}

/** 破坏性操作一次确认；在 Tauri 中使用系统对话框（`window.confirm` 在 WKWebView 中不可靠）。 */
export async function confirmDestructive(message: string): Promise<boolean> {
  if (hasTauriRuntime()) {
    return tauriConfirm(message, { kind: "warning" })
  }
  return window.confirm(message)
}
