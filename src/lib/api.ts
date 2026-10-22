import { invoke } from "@tauri-apps/api/core";

export type Status = {
  exists: boolean;
  unlocked: boolean;
  biometricAvailable: boolean;
  biometricEnabled: boolean;
  biometricError: string | null;
  biometricMethod: "windows" | "mobile" | "none";
  driveConnected: boolean;
  lastBackup: number | null;
};
export type SyncResult = { uploaded: number; downloaded: number; removed: number; unchanged: number; at: number };
export type PropertyType = "text" | "number" | "select";
export type JournalProperty = { id: string; name: string; type: PropertyType; show: boolean; options: string[] };
export type RawEntry = { id: string; title: string; body: string; values: Record<string, string>; created_at: number; updated_at: number };

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export const errMsg = (e: unknown) =>
  typeof e === "string" ? e : (e as { message?: string })?.message || "Something went wrong";

const call = <T>(cmd: string, args?: Record<string, unknown>) => invoke<T>(cmd, args);

// Biometric prompts are shown and verified in Rust (src-tauri/src/biometric.rs).
// The webview only asks; it never decides whether the user passed.
export const api = {
  status: () => call<Status>("vault_status"),
  setup: (password: string) => call<void>("setup_vault", { password }),
  unlock: (password: string) => call<void>("unlock", { password }),
  unlockBiometric: () => call<void>("biometric_unlock"),
  lock: () => call<void>("lock"),
  setBiometric: (enabled: boolean) =>
    call<void>(enabled ? "enable_biometric_unlock" : "disable_biometric_unlock"),
  changePassword: (oldPassword: string, newPassword: string) =>
    call<void>("change_password", { oldPassword, newPassword }),
  list: () => call<RawEntry[]>("list_entries"),
  listProperties: () => call<JournalProperty[]>("list_properties"),
  saveProperties: (properties: JournalProperty[]) => call<void>("save_properties", { properties }),
  create: (title: string, body: string, id?: string, createdAt?: number, values?: Record<string, string>) =>
    call<RawEntry>("create_entry", { title, body, id, createdAt, values }),
  update: (id: string, title: string, body: string, values: Record<string, string>) =>
    call<RawEntry>("update_entry", { id, title, body, values }),
  remove: (id: string) => call<void>("delete_entry", { id }),
  driveConnect: (clientId: string, clientSecret: string) =>
    call<void>("drive_connect", { clientId, clientSecret }),
  driveDisconnect: () => call<void>("drive_disconnect"),
  sync: () => call<SyncResult>("sync_now"),
  restore: (clientId: string, clientSecret: string, password: string) =>
    call<number>("restore_from_drive", { clientId, clientSecret, password }),
};
