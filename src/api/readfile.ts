// SPDX-License-Identifier: GPL-3.0-or-later
import { invoke } from "@tauri-apps/api/core";
/** Read a small text file through the Rust side (no fs plugin in the webview). */
export const readTextFile = (path: string) => invoke<string>("read_text_file", { path });
