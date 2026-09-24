export type SyncedLyricCue = {
  timeSeconds: number;
  text: string;
  kind: "lyric" | "break";
  breakSource: "blank" | "instrumental" | null;
};

export type SyncedLyricsPlaybackState =
  | { kind: "none"; activeCueIndex: -1 }
  | { kind: "intro"; activeCueIndex: -1 }
  | { kind: "lyric" | "break"; activeCueIndex: number };

const ACTIVE_CUE_EARLY_TOLERANCE_SECONDS = 0.15;

export function parseLrcLyrics(text: string): SyncedLyricCue[] {
  const cues = text
    .split(/\r?\n/)
    .flatMap((line) => parseLrcLine(line));

  return cues
    .map((cue, sourceOrder) => ({ cue, sourceOrder }))
    .sort((left, right) => left.cue.timeSeconds - right.cue.timeSeconds || left.sourceOrder - right.sourceOrder)
    .map(({ cue }) => cue);
}

export function parseLrcLine(line: string): SyncedLyricCue[] {
  const timestampPattern = /\[(\d{1,2}):(\d{2})(?:\.(\d{1,3}))?\]/g;
  const timestamps: number[] = [];
  let match: RegExpExecArray | null;

  while ((match = timestampPattern.exec(line)) !== null) {
    const minutes = Number.parseInt(match[1], 10);
    const seconds = Number.parseInt(match[2], 10);
    const fraction = match[3] ? Number.parseFloat(`0.${match[3].padEnd(3, "0")}`) : 0;

    if (Number.isFinite(minutes) && Number.isFinite(seconds)) {
      timestamps.push(minutes * 60 + seconds + fraction);
    }
  }

  if (timestamps.length === 0) {
    return [];
  }

  const cueText = line.replace(timestampPattern, "").trim();
  const breakSource = cueText.length === 0
    ? "blank"
    : isExplicitInstrumentalMarker(cueText)
      ? "instrumental"
      : null;

  return timestamps.map((timeSeconds) => ({
    timeSeconds,
    text: breakSource ? "" : cueText,
    kind: breakSource ? "break" : "lyric",
    breakSource,
  }));
}

export function resolveSyncedLyricsState(
  cues: SyncedLyricCue[],
  playbackPositionSeconds: number,
  offsetSeconds = 0,
): SyncedLyricsPlaybackState {
  if (cues.length === 0 || !Number.isFinite(playbackPositionSeconds) || !Number.isFinite(offsetSeconds)) {
    return { kind: "none", activeCueIndex: -1 };
  }

  const adjustedPositionSeconds = playbackPositionSeconds - offsetSeconds;
  const cueThresholdSeconds = adjustedPositionSeconds + ACTIVE_CUE_EARLY_TOLERANCE_SECONDS;
  let activeCueIndex = -1;

  for (let index = 0; index < cues.length; index += 1) {
    if (cues[index].timeSeconds <= cueThresholdSeconds) {
      activeCueIndex = index;
    } else {
      break;
    }
  }

  if (activeCueIndex < 0) {
    return { kind: "intro", activeCueIndex: -1 };
  }

  if (cues[activeCueIndex].kind === "break") {
    while (activeCueIndex > 0 && cues[activeCueIndex - 1].kind === "break") {
      activeCueIndex -= 1;
    }
  }

  return {
    kind: cues[activeCueIndex].kind,
    activeCueIndex,
  };
}

export function startsSyncedLyricBreak(cues: SyncedLyricCue[], cueIndex: number) {
  return cues[cueIndex]?.kind === "break" && (cueIndex === 0 || cues[cueIndex - 1]?.kind !== "break");
}

// Keep the intro cue in the timeline after it stops being active. Removing it
// changes the rendered grid and makes the opening note vanish on the first line.
export function hasSyncedLyricsIntro(cues: SyncedLyricCue[], offsetSeconds = 0) {
  return resolveSyncedLyricsState(cues, 0, offsetSeconds).kind === "intro";
}

function isExplicitInstrumentalMarker(text: string) {
  const normalized = unwrapMarker(text.trim().toLocaleLowerCase()).replace(/\s+/g, " ");

  return /^(?:instrumental(?: break)?|music(?:al)? break|music|interlude|[♪♫♬]+)$/.test(normalized);
}

function unwrapMarker(text: string) {
  const markerPairs: ReadonlyArray<readonly [string, string]> = [
    ["[", "]"],
    ["(", ")"],
    ["{", "}"],
    ["【", "】"],
    ["（", "）"],
  ];
  let unwrapped = text;

  for (const [start, end] of markerPairs) {
    if (unwrapped.startsWith(start) && unwrapped.endsWith(end)) {
      unwrapped = unwrapped.slice(start.length, -end.length).trim();
      break;
    }
  }

  return unwrapped;
}
