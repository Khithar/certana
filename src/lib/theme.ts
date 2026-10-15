// SPDX-License-Identifier: GPL-3.0-or-later
// Colour theme: "system" follows prefers-color-scheme, "light"/"dark" force it. The choice is a
// per-device preference and therefore lives in localStorage, not in the database.
export type Theme = "system" | "light" | "dark";
export const THEMES: Theme[] = ["system", "light", "dark"];
const KEY = "certana.theme";
const mq = window.matchMedia("(prefers-color-scheme: dark)");

export function getTheme(): Theme {
  try {
    const v = localStorage.getItem(KEY);
    if (v === "light" || v === "dark") return v;
  } catch {
    /* storage unavailable */
  }
  return "system";
}

export function applyTheme() {
  const t = getTheme();
  const dark = t === "dark" || (t === "system" && mq.matches);
  document.documentElement.classList.toggle("dark", dark);
}

export function setTheme(t: Theme) {
  try {
    if (t === "system") localStorage.removeItem(KEY);
    else localStorage.setItem(KEY, t);
  } catch {
    /* storage unavailable */
  }
  applyTheme();
}

export function initTheme() {
  applyTheme();
  mq.addEventListener("change", applyTheme);
}
