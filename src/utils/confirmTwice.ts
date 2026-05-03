import { confirm as tauriConfirm } from "@tauri-apps/plugin-dialog"

function hasTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window
}

async function confirmOnce(message: string): Promise<boolean> {
  if (hasTauriRuntime()) {
    return tauriConfirm(message, { kind: "warning" })
  }
  return window.confirm(message)
}

/** 两次确认；任一次取消则返回 `false`。在 Tauri 中使用系统对话框（`window.confirm` 在 WKWebView 中不可靠）。 */
export async function confirmTwice(
  first: string,
  second: string,
): Promise<boolean> {
  if (!(await confirmOnce(first))) return false
  return confirmOnce(second)
}
