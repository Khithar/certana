// SPDX-License-Identifier: GPL-3.0-or-later
import { open, save } from "@tauri-apps/plugin-dialog";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { sendNotification, isPermissionGranted, requestPermission } from "@tauri-apps/plugin-notification";

export async function pickFile(title: string, filters?: { name: string; extensions: string[] }[]): Promise<string | null> {
  const r = await open({ title, multiple: false, directory: false, filters });
  return typeof r === "string" ? r : null;
}

export async function pickDirectory(title: string): Promise<string | null> {
  const r = await open({ title, multiple: false, directory: true });
  return typeof r === "string" ? r : null;
}

export async function pickSavePath(title: string, defaultPath?: string, filters?: { name: string; extensions: string[] }[]): Promise<string | null> {
  const r = await save({ title, defaultPath, filters });
  return r ?? null;
}

export const copyToClipboard = (text: string) => writeText(text);

export async function notify(title: string, body: string) {
  try {
    let granted = await isPermissionGranted();
    if (!granted) granted = (await requestPermission()) === "granted";
    if (granted) sendNotification({ title, body });
  } catch {
    /* notifications are best-effort */
  }
}
