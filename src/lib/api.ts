import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Kind = "text" | "link" | "code" | "color";

export interface Clip {
  id: number;
  preview: string;
  length: number;
  lines: number;
  kind: Kind;
  pinned: boolean;
  last_used_at: number;
}

export const getHistory = (query: string) => invoke<Clip[]>("get_history", { query });
export const copyItem = (id: number) => invoke<void>("copy_item", { id });
export const setPinned = (id: number, pinned: boolean) => invoke<void>("set_pinned", { id, pinned });
export const deleteItem = (id: number) => invoke<void>("delete_item", { id });
export const clearAll = (includePinned: boolean) =>
  invoke<void>("clear_all", { includePinned });
export const getPaused = () => invoke<boolean>("get_paused");
export const setPaused = (paused: boolean) => invoke<boolean>("set_paused", { paused });
export const getAutostart = () => invoke<boolean>("get_autostart");
export const setAutostart = (enabled: boolean) => invoke<boolean>("set_autostart", { enabled });
export const hidePopup = () => invoke<void>("hide_popup");

export const onHistoryChanged = (cb: () => void): Promise<UnlistenFn> =>
  listen("history-changed", cb);
export const onPopupShown = (cb: () => void): Promise<UnlistenFn> =>
  listen("popup-shown", cb);
export const onPausedChanged = (cb: (paused: boolean) => void): Promise<UnlistenFn> =>
  listen<boolean>("paused-changed", (e) => cb(e.payload));
export const onAutostartChanged = (cb: (enabled: boolean) => void): Promise<UnlistenFn> =>
  listen<boolean>("autostart-changed", (e) => cb(e.payload));
