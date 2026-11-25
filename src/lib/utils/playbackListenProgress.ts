// The media clock supplies listening time. The monotonic clock only rejects
// implausible jumps between samples, such as a seek that is still settling.
const POSITION_JUMP_TOLERANCE_SECONDS = 0.25;

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
      return;
    }

    const previous = this.sample;
    this.sample = { position, at: now };
    if (!previous || !wasPlaying) return;

    const advance = position - previous.position;
    const elapsed = Math.max(0, now - previous.at) / 1000;
    if (advance > 0 && advance <= elapsed + POSITION_JUMP_TOLERANCE_SECONDS) {
      this.listenedSeconds += advance;
    }
  }

  qualifies(duration: number | null, percentage: number, naturalEnd: boolean) {
    if (this.listenedSeconds <= 0) return false;
    if (duration === null || !Number.isFinite(duration) || duration <= 0) return naturalEnd;

    const tolerance = naturalEnd && percentage === 100 ? POSITION_JUMP_TOLERANCE_SECONDS : 0;
    return this.listenedSeconds + tolerance >= duration * percentage / 100;
  }
}
