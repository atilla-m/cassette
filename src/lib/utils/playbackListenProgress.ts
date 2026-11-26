import type { ListeningMinute } from "$lib/types/listening";

// The media clock supplies listening time. The monotonic clock only rejects
// implausible jumps between samples, such as a seek that is still settling.
const POSITION_JUMP_TOLERANCE_SECONDS = 0.25;
const MINIMUM_LISTEN_SECONDS = 5;

export class PlaybackListenProgress {
  private sample: { position: number; at: number } | null = null;
  private listenedSeconds = 0;

  reset(now: number) {
    this.listenedSeconds = 0;
    this.sample = { position: 0, at: now };
  }

  forgetPosition() {
    this.sample = null;
  }

  observe(position: number, now: number, wasPlaying: boolean) {
    if (!Number.isFinite(position) || position < 0) {
      this.forgetPosition();
      return 0;
    }

    const previous = this.sample;
    this.sample = { position, at: now };
    if (!previous || !wasPlaying) return 0;

    const advance = position - previous.position;
    const elapsed = Math.max(0, now - previous.at) / 1000;
    if (advance > 0 && advance <= elapsed + POSITION_JUMP_TOLERANCE_SECONDS) {
      this.listenedSeconds += advance;
      return advance;
    }
    return 0;
  }

  qualifies(duration: number | null, percentage: number, naturalEnd: boolean) {
    if (this.listenedSeconds < MINIMUM_LISTEN_SECONDS) return false;
    if (duration === null || !Number.isFinite(duration) || duration <= 0) return naturalEnd;

    const tolerance = naturalEnd && percentage === 100 ? POSITION_JUMP_TOLERANCE_SECONDS : 0;
    return this.listenedSeconds + tolerance >= duration * percentage / 100;
  }
}

export class ListeningTimeBuffer {
  private session: { trackId: string; sessionId: string } | null = null;
  private minutes = new Map<number, number>();
  private pending = new Map<string, ListeningMinute>();

  start(trackId: string | null, sessionId: string | null) {
    this.session = trackId && sessionId ? { trackId, sessionId } : null;
    this.minutes.clear();
  }

  observe(seconds: number, wallClockMs: number) {
    if (!this.session || seconds <= 0) return;
    const end = Math.round(wallClockMs);
    let start = Math.round(wallClockMs - seconds * 1000);

    while (start < end) {
      const minuteStart = Math.floor(start / 60_000) * 60_000;
      const segmentEnd = Math.min(end, minuteStart + 60_000);
      const minuteUtc = minuteStart / 1000;
      const listenedMs = Math.min(60_000, (this.minutes.get(minuteUtc) ?? 0) + segmentEnd - start);
      this.minutes.set(minuteUtc, listenedMs);
      this.pending.set(`${this.session.sessionId}:${minuteUtc}`, {
        ...this.session, minuteUtc, listenedMs, lastSampleUtcMs: segmentEnd,
      });
      start = segmentEnd;
    }
  }

  batch() {
    return [...this.pending.values()].slice(0, 512);
  }

  saved(batch: ListeningMinute[]) {
    for (const sample of batch) {
      const key = `${sample.sessionId}:${sample.minuteUtc}`;
      if (this.pending.get(key) === sample) this.pending.delete(key);
    }
  }
}
