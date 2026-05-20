import { afterEach, describe, expect, it, vi } from "vitest";
import { logToConsole } from "../../consoleLog";
import {
  getNotificationSoundEnabled,
  playNotificationSound,
  setNotificationSoundEnabled,
  subscribeNotificationSoundEnabled,
} from "../notificationSound";

vi.mock("../../consoleLog", async () => {
  const actual = await vi.importActual<typeof import("../../consoleLog")>("../../consoleLog");
  return { ...actual, logToConsole: vi.fn() };
});

afterEach(() => {
  // reset module-level state to default
  setNotificationSoundEnabled(true);
});

describe("services/notification/notificationSound", () => {
  it("getNotificationSoundEnabled returns current state", () => {
    expect(getNotificationSoundEnabled()).toBe(true);
    setNotificationSoundEnabled(false);
    expect(getNotificationSoundEnabled()).toBe(false);
  });

  it("setNotificationSoundEnabled is idempotent when value unchanged", () => {
    setNotificationSoundEnabled(true);
    setNotificationSoundEnabled(true); // should not emit
    expect(getNotificationSoundEnabled()).toBe(true);
  });

  it("isolates notification sound subscribers when one throws", () => {
    const throwingListener = vi.fn(() => {
      throw new Error("listener boom");
    });
    const healthyListener = vi.fn();

    const unsubscribeThrowing = subscribeNotificationSoundEnabled(throwingListener);
    const unsubscribeHealthy = subscribeNotificationSoundEnabled(healthyListener);

    try {
      setNotificationSoundEnabled(false);

      expect(throwingListener).toHaveBeenCalledTimes(1);
      expect(healthyListener).toHaveBeenCalledTimes(1);
      expect(logToConsole).toHaveBeenCalledWith("warn", "通知音效状态订阅处理失败", {
        error: "Error: listener boom",
      });
    } finally {
      unsubscribeThrowing();
      unsubscribeHealthy();
    }
  });

  it("playNotificationSound handles Audio errors gracefully", () => {
    // JSDOM doesn't implement Audio — stub it to throw
    const origAudio = globalThis.Audio;
    globalThis.Audio = class {
      play() {
        return Promise.reject(new Error("play rejected"));
      }
      set currentTime(_v: number) {}
    } as unknown as typeof Audio;

    expect(() => playNotificationSound()).not.toThrow();

    globalThis.Audio = origAudio;
  });

  it("playNotificationSound handles constructor errors gracefully", () => {
    const origAudio = globalThis.Audio;
    globalThis.Audio = class {
      constructor() {
        throw new Error("Audio unavailable");
      }
    } as unknown as typeof Audio;

    expect(() => playNotificationSound()).not.toThrow();

    globalThis.Audio = origAudio;
  });
});
