/**
 * Notification Sound module - custom notification sound control
 *
 * Usage:
 * - `setNotificationSoundEnabled(true/false)` to toggle
 * - `useNotificationSoundEnabled()` for React state
 * - `playNotificationSound()` to play ding.mp3
 */

import { useSyncExternalStore } from "react";

import { logToConsole } from "../consoleLog";

let enabled = true;
type NotificationSoundListener = () => void;

const listeners = new Set<NotificationSoundListener>();

function emitChange() {
  for (const listener of Array.from(listeners)) {
    if (!listeners.has(listener)) continue;
    try {
      listener();
    } catch (err) {
      logToConsole("warn", "通知音效状态订阅处理失败", { error: String(err) });
    }
  }
}

export function setNotificationSoundEnabled(value: boolean) {
  if (enabled === value) return;
  enabled = value;
  emitChange();
}

export function getNotificationSoundEnabled(): boolean {
  return enabled;
}

export function subscribeNotificationSoundEnabled(listener: NotificationSoundListener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function useNotificationSoundEnabled(): boolean {
  return useSyncExternalStore(subscribeNotificationSoundEnabled, () => enabled);
}

let cachedAudio: HTMLAudioElement | null = null;

export function playNotificationSound(): void {
  try {
    // Create a fresh Audio instance each time to avoid stale state issues in Tauri WebView.
    // Reusing a cached instance can fail silently after the first play on some platforms.
    const audio = cachedAudio ?? new Audio("/ding.mp3");
    cachedAudio = audio;
    audio.currentTime = 0;
    audio.play()?.catch((err) => {
      logToConsole("warn", "通知音效播放失败", { error: String(err) });
    });
  } catch (err) {
    logToConsole("warn", "通知音效创建失败", { error: String(err) });
  }
}
