// SPDX-License-Identifier: GPL-3.0-or-later
import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

/** shadcn/ui class helper */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
