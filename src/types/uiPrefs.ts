/** Mirrors Rust `UiPrefs` (serde `camelCase`) from `get_ui_prefs` / `set_ui_prefs`. */
export interface UiPrefs {
  alwaysOnTop: boolean
  themeId: string
}

/** Partial update payload for `set_ui_prefs`. */
export type UiPrefsPatch = Partial<Pick<UiPrefs, "alwaysOnTop" | "themeId">>
