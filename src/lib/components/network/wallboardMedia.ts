export async function toggleFullscreen(): Promise<void> {
  if (typeof document === 'undefined') return;
  try {
    if (document.fullscreenElement) await document.exitFullscreen();
    else await document.documentElement.requestFullscreen();
  } catch {
    // ignore
  }
}

export function createCriticalBeepPlayer() {
  let audioCtx: AudioContext | null = null;

  async function play(enabled: boolean): Promise<void> {
    if (!enabled || typeof window === 'undefined') return;
    const AC = (window as any).AudioContext || (window as any).webkitAudioContext;
    if (!AC) return;
    if (!audioCtx) {
      audioCtx = new AC();
    }
    const ctx = audioCtx;
    if (!ctx) return;
    if (ctx.state === 'suspended') {
      try {
        await ctx.resume();
      } catch {
        return;
      }
    }

    const base = ctx.currentTime;
    const pulse = (start: number, freq: number, gainValue: number, dur = 0.12) => {
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = 'sine';
      osc.frequency.value = freq;
      gain.gain.setValueAtTime(0.0001, start);
      gain.gain.exponentialRampToValueAtTime(gainValue, start + 0.01);
      gain.gain.exponentialRampToValueAtTime(0.0001, start + dur);
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.start(start);
      osc.stop(start + dur + 0.02);
    };

    pulse(base, 740, 0.028, 0.14);
    pulse(base + 0.18, 660, 0.03, 0.16);
  }

  async function close(): Promise<void> {
    try {
      await audioCtx?.close?.();
    } catch {
      // ignore
    } finally {
      audioCtx = null;
    }
  }

  return { play, close };
}

export function createWakeLockController() {
  let wakeLock: any = null;

  async function apply(on: boolean): Promise<void> {
    if (typeof navigator === 'undefined') return;
    const wl = (navigator as any).wakeLock;
    if (!wl) return;
    try {
      if (on) {
        wakeLock = await wl.request('screen');
      } else {
        await wakeLock?.release?.();
        wakeLock = null;
      }
    } catch {
      // ignore
    }
  }

  return { apply };
}
