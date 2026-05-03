import { afterEach, describe, expect, it, vi } from "vitest"
import { confirmDestructive } from "./confirmTwice"

describe("confirmDestructive", () => {
  afterEach(() => {
    vi.restoreAllMocks()
    Reflect.deleteProperty(window, "__TAURI_INTERNALS__")
  })

  it("uses window.confirm when not in Tauri", async () => {
    const spy = vi.spyOn(window, "confirm").mockReturnValue(true)
    await expect(confirmDestructive("删除？")).resolves.toBe(true)
    expect(spy).toHaveBeenCalledWith("删除？")
  })

  it("returns false when window.confirm returns false", async () => {
    vi.spyOn(window, "confirm").mockReturnValue(false)
    await expect(confirmDestructive("x")).resolves.toBe(false)
  })
})
