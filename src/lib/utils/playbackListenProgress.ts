import type { ListeningMinute } from "$lib/types/listening";

// The media clock supplies listening time. The monotonic clock only rejects
// implausible jumps between samples, such as a seek that is still settling.
const POSITION_JUMP_TOLERANCE_SECONDS = 0.25;
const MINIMUM_LISTEN_SECONDS = 5;

type PositionSample = { position: number; at: number };

export class PlaybackListenProgress {
  private sample: PositionSample | null = null;
  private possibleDiscontinuity: PositionSample | null = null;
  private listenedSeconds = 0;

  reset(now: number) {
    this.listenedSeconds = 0;
    this.sample = { position: 0, at: now };
    this.possibleDiscontinuity = null;
  }

  forgetPosition() {
    this.sample = null;
    this.possibleDiscontinuity = null;
  }

  observe(position: number, now: number, wasPlaying: boolean, requestedAt = now) {
    if (!Number.isFinite(position) || position < 0) return 0;

    const sample = { position, at: requestedAt };
    if (!this.sample || !wasPlaying) {
      this.sample = sample;
      this.possibleDiscontinuity = null;
      return 0;
    }

    // A clock may report the same position several times or briefly report
    // zero. Keep the last measured boundary until the position advances.
    if (position <= this.sample.position) {
      this.possibleDiscontinuity = null;
      return 0;
    }

    const advanceFrom = (previous: PositionSample) => {
      const advance = position - previous.position;
      const elapsed = Math.max(0, now - previous.at) / 1000;
      return advance > 0 && advance <= elapsed + POSITION_JUMP_TOLERANCE_SECONDS ? advance : 0;
    };
    const advance = advanceFrom(this.sample)
      || (this.possibleDiscontinuity ? advanceFrom(this.possibleDiscontinuity) : 0);
    if (!advance) {
      if (position !== this.possibleDiscontinuity?.position) this.possibleDiscontinuity = sample;
      return 0;
    }

    this.sample = sample;
    this.possibleDiscontinuity = null;
    this.listenedSeconds += advance;
    return advance;
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

  batch(sessionId?: string) {
    const pending = [...this.pending.values()];
    return (sessionId ? pending.filter((sample) => sample.sessionId === sessionId) : pending).slice(0, 512);
  }

  saved(batch: ListeningMinute[]) {
    for (const sample of batch) {
      const key = `${sample.sessionId}:${sample.minuteUtc}`;
      if (this.pending.get(key) === sample) this.pending.delete(key);
    }
  }
}
