<script lang="ts">
  import type { Track } from "$lib/types/library";
  import { localImageSource } from "$lib/utils/localImage";
  import { onDestroy } from "svelte";

  type RepeatMode = "off" | "all" | "one";

  type Props = {
    track: Track | null;
    isPlaying: boolean;
    positionSeconds: number;
    durationSeconds: number | null;
    volume: number;
    canPlayPrevious?: boolean;
    canPlayNext?: boolean;
    queueCount?: number;
    isQueueOpen?: boolean;
    isShuffleEnabled?: boolean;
    repeatMode?: RepeatMode;
    compact?: boolean;
    interfaceMode?: "modern" | "legacy";
    onTogglePlayback?: () => void;
    onPrevious?: () => void;
    onNext?: () => void;
    onSeek?: (positionSeconds: number) => void | Promise<void>;
    onVolumeChange?: (volume: number) => void;
    onToggleFavorite?: (track: Track) => void;
    onToggleQueue?: () => void;
    onToggleShuffle?: () => void;
    onToggleRepeat?: () => void;
    onOpenNowPlaying?: () => void;
    onOpenLyrics?: () => void;
  };

  let {
    track,
    isPlaying,
    positionSeconds,
    durationSeconds,
    volume,
    canPlayPrevious = false,
    canPlayNext = false,
    queueCount = 0,
    isQueueOpen = false,
    isShuffleEnabled = false,
    repeatMode = "off",
    compact = false,
    interfaceMode = "legacy",
    onTogglePlayback,
    onPrevious,
    onNext,
    onSeek,
    onVolumeChange,
    onToggleFavorite,
    onToggleQueue,
    onToggleShuffle,
    onToggleRepeat,
    onOpenNowPlaying,
    onOpenLyrics,
  }: Props = $props();

  let localVolume = $state(1);
  let localPosition = $state(0);
  let isPointerSeeking = $state(false);
  let isSeekPending = $state(false);
  let requestedSeekPosition = $state<number | null>(null);
  let seekStartedFromPosition = $state<number | null>(null);
  let seekHoldUntil = $state(0);
  let seekHoldTimeoutId: number | null = null;
  let lastTrackId: string | null = null;
  let coverArtSrc = $derived(localImageSource(track?.coverArtPath));
  let effectiveDuration = $derived(durationSeconds ?? track?.durationSeconds ?? null);
  let progressFillPercent = $derived(rangeFillPercent(localPosition, effectiveDuration ?? 0));
  let volumeFillPercent = $derived(rangeFillPercent(localVolume, 1));

  $effect(() => {
    localVolume = volume;
  });

  $effect(() => {
    const trackId = track?.id ?? null;

    if (trackId !== lastTrackId) {
      lastTrackId = trackId;
      isPointerSeeking = false;
      isSeekPending = false;
      requestedSeekPosition = null;
      seekStartedFromPosition = null;
      localPosition = positionSeconds;
      return;
    }

    if (canSyncBackendPosition()) {
      localPosition = positionSeconds;
    }
  });

  onDestroy(() => {
    if (seekHoldTimeoutId !== null) {
      window.clearTimeout(seekHoldTimeoutId);
    }
  });

  function displayArtist(track: Track | null) {
    return track?.artist ?? "Unknown Artist";
  }

  function displayAlbum(track: Track | null) {
    return track?.album ? ` · ${track.album}` : "";
  }

  function formatDuration(seconds: number | null | undefined) {
    if (!seconds) {
      return "--:--";
    }

    const wholeSeconds = Math.floor(seconds);
    const minutes = Math.floor(wholeSeconds / 60);
    const remainingSeconds = wholeSeconds % 60;

    return `${minutes}:${remainingSeconds.toString().padStart(2, "0")}`;
  }

  function rangeFillPercent(value: number, max: number) {
    if (!max || max <= 0 || !Number.isFinite(value)) {
      return 0;
    }

    return Math.min(100, Math.max(0, (value / max) * 100));
  }

  function canSyncBackendPosition() {
    if (isPointerSeeking || isSeekPending) {
      return false;
    }

    if (requestedSeekPosition === null) {
      return true;
    }

    const hasCaughtUp = Math.abs(positionSeconds - requestedSeekPosition) <= 1.25;
    const isStalePreSeekPosition = seekStartedFromPosition !== null
      && Math.abs(positionSeconds - seekStartedFromPosition) <= 2
      && Math.abs(positionSeconds - requestedSeekPosition) > 1.25;

    if (hasCaughtUp) {
      requestedSeekPosition = null;
      seekStartedFromPosition = null;
      return true;
    }

    if (isStalePreSeekPosition) {
      return false;
    }

    if (Date.now() >= seekHoldUntil) {
      requestedSeekPosition = null;
      seekStartedFromPosition = null;
      return true;
    }

    return false;
  }

  function seekPositionFromInput(event: Event) {
    const duration = durationSeconds ?? track?.durationSeconds ?? null;

    if (!(event.currentTarget instanceof HTMLInputElement) || !duration || duration <= 0) {
      return null;
    }

    const value = Number(event.currentTarget.value);

    if (!Number.isFinite(value)) {
      return null;
    }

    return Math.min(Math.max(value, 0), duration);
  }

  async function commitSeek() {
    const duration = durationSeconds ?? track?.durationSeconds ?? null;

    if (!track || !duration || duration <= 0 || !Number.isFinite(localPosition)) {
      isPointerSeeking = false;
      return;
    }

    const nextPosition = Math.min(Math.max(localPosition, 0), duration);

    if (isSeekPending && requestedSeekPosition !== null && Math.abs(requestedSeekPosition - nextPosition) < 0.05) {
      return;
    }

    localPosition = nextPosition;
    requestedSeekPosition = nextPosition;
    seekStartedFromPosition = positionSeconds;
    isSeekPending = true;

    try {
      await onSeek?.(nextPosition);
    } finally {
      isSeekPending = false;
      holdSeekPreview(nextPosition);
    }
  }

  function handleSeekStart() {
    isPointerSeeking = true;
  }

  function handleSeekInput(event: Event) {
    const nextPosition = seekPositionFromInput(event);

    isPointerSeeking = true;

    if (nextPosition !== null) {
      localPosition = nextPosition;
    }
  }

  function handleSeekEnd() {
    isPointerSeeking = false;
    void commitSeek();
  }

  function handleSeekChange() {
    isPointerSeeking = false;
    void commitSeek();
  }

  function handleSeekCancel() {
    isPointerSeeking = false;
  }

  function holdSeekPreview(position: number) {
    requestedSeekPosition = position;
    seekHoldUntil = Date.now() + 900;

    if (seekHoldTimeoutId !== null) {
      window.clearTimeout(seekHoldTimeoutId);
    }

    seekHoldTimeoutId = window.setTimeout(() => {
      seekHoldTimeoutId = null;

      if (!isPointerSeeking && !isSeekPending) {
        const isStalePreSeekPosition = seekStartedFromPosition !== null
          && Math.abs(positionSeconds - seekStartedFromPosition) <= 2
          && Math.abs(positionSeconds - position) > 1.25;

        if (!isStalePreSeekPosition) {
          requestedSeekPosition = null;
          seekStartedFromPosition = null;
          localPosition = positionSeconds;
        }
      }
    }, 900);
  }

  function handleVolumeInput() {
    onVolumeChange?.(localVolume);
  }

  function handleFavoriteClick() {
    if (track) {
      onToggleFavorite?.(track);
    }
  }

  function queueLabel() {
    return queueCount > 0 ? `Queue · ${queueCount}` : "Queue";
  }

  function repeatLabel() {
    if (repeatMode === "all") {
      return "Repeat All";
    }

    if (repeatMode === "one") {
      return "Repeat 1";
    }

    return "Repeat";
  }

  function repeatAriaLabel() {
    if (repeatMode === "all") {
      return "Repeat all is on";
    }

    if (repeatMode === "one") {
      return "Repeat one is on";
    }

    return "Repeat is off";
  }

  function hideBrokenImage(event: Event) {
    if (event.currentTarget instanceof HTMLImageElement) {
      event.currentTarget.hidden = true;
    }
  }

  function showLoadedImage(event: Event) {
    if (event.currentTarget instanceof HTMLImageElement) {
      event.currentTarget.hidden = false;
    }
  }
</script>

<footer class:compact class:modern={interfaceMode === "modern"} class="player" aria-label="Now playing">
  <div class="track">
    {#if !compact}
      <button class="track-open" type="button" aria-label="Open current track" disabled={!track} onclick={onOpenNowPlaying}>
        <span class="cover" aria-hidden="true">
          {#if coverArtSrc}
            <img src={coverArtSrc} alt="" onload={showLoadedImage} onerror={hideBrokenImage} />
          {/if}
        </span>
        <span class="track-copy">
          <span>{track?.title ?? "No track selected"}</span>
          <small>{displayArtist(track)}{displayAlbum(track)}</small>
        </span>
      </button>
    {/if}
    <button
      class:active={track?.isFavorite}
      class="favorite"
      type="button"
      aria-label={track?.isFavorite ? "Remove from liked songs" : "Add to liked songs"}
      disabled={!track}
      onclick={handleFavoriteClick}
    >
      {track?.isFavorite ? "★" : "☆"}
    </button>
  </div>

  <div class="transport" aria-label="Playback controls">
    <button
      class:active={isShuffleEnabled}
      class="mode-button"
      type="button"
      aria-label={isShuffleEnabled ? "Shuffle is on" : "Shuffle is off"}
      disabled={!track}
      onclick={onToggleShuffle}
    >
      Shuffle
    </button>
    <button type="button" aria-label="Previous track" disabled={!canPlayPrevious} onclick={onPrevious}>&lt;&lt;</button>
    <button
      class="play"
      type="button"
      aria-label={isPlaying ? "Pause" : "Play"}
      disabled={!track}
      onclick={onTogglePlayback}
    >
      {isPlaying ? "||" : ">"}
    </button>
    <button type="button" aria-label="Next track" disabled={!canPlayNext} onclick={onNext}>&gt;&gt;</button>
    <button
      class:active={repeatMode !== "off"}
      class="mode-button"
      type="button"
      aria-label={repeatAriaLabel()}
      disabled={!track}
      onclick={onToggleRepeat}
    >
      {repeatLabel()}
    </button>
  </div>

  <div class="progress-area">
    <span>{formatDuration(localPosition)}</span>
    <input
      class="progress"
      type="range"
      min="0"
      max={effectiveDuration ?? 0}
      step="0.1"
      bind:value={localPosition}
      style={`--range-fill: ${progressFillPercent}%`}
      disabled={!track || !effectiveDuration}
      aria-label="Playback progress"
      onpointerdown={handleSeekStart}
      onpointerup={handleSeekEnd}
      onpointercancel={handleSeekCancel}
      onblur={handleSeekCancel}
      oninput={handleSeekInput}
      onchange={handleSeekChange}
    />
    <span>{formatDuration(effectiveDuration)}</span>
  </div>

  <div class="volume" aria-label="Volume">
    {#if !compact}
      <button
        class="lyrics-button"
        type="button"
        aria-label="Open lyrics"
        disabled={!track}
        onclick={onOpenLyrics}
      >
        Lyrics
      </button>
    {/if}
    <button
      class:active={isQueueOpen}
      class="queue-button"
      type="button"
      aria-label="Show Up Next"
      disabled={!track && queueCount === 0}
      onclick={onToggleQueue}
    >
      {queueLabel()}
    </button>
    <span>Vol</span>
    <input
      class="volume-bar"
      type="range"
      min="0"
      max="1"
      step="0.01"
      bind:value={localVolume}
      style={`--range-fill: ${volumeFillPercent}%`}
      disabled={!track}
      aria-label="Volume"
      oninput={handleVolumeInput}
    />
  </div>
</footer>

<style>
  .player {
    display: grid;
    grid-template-columns: minmax(180px, 1.1fr) auto minmax(220px, 1.6fr) minmax(120px, 0.7fr);
    align-items: center;
    gap: 22px;
    min-height: 86px;
    padding: 14px 24px;
    border-top: 1px solid rgba(255, 255, 255, 0.08);
    background: var(--panel);
  }

  .player.compact {
    grid-template-columns: auto auto minmax(260px, min(42vw, 620px)) auto;
    justify-content: center;
    gap: 14px;
    min-height: 64px;
    padding: 10px 22px;
    border-top-color: rgba(255, 255, 255, 0.06);
    background: color-mix(in srgb, var(--bg) 90%, transparent);
  }

  .player.modern:not(.compact) {
    grid-template-areas:
      "track transport volume"
      "track progress volume";
    grid-template-columns: minmax(230px, 1fr) minmax(390px, 1.5fr) minmax(250px, 0.95fr);
    grid-template-rows: auto auto;
    gap: 7px clamp(20px, 2.2vw, 38px);
    min-height: 108px;
    border-top-color: color-mix(in srgb, var(--border-strong) 72%, transparent);
    background: var(--modern-player, var(--panel));
    box-shadow: 0 -14px 36px color-mix(in srgb, var(--modern-shadow, var(--shadow)) 54%, transparent);
    padding: 13px clamp(18px, 2vw, 30px);
  }

  .player.modern:not(.compact) .track {
    grid-area: track;
  }

  .player.modern:not(.compact) .transport {
    grid-area: transport;
    align-self: end;
  }

  .player.modern:not(.compact) .progress-area {
    grid-area: progress;
    align-self: start;
  }

  .player.modern:not(.compact) .volume {
    grid-area: volume;
    justify-content: flex-end;
  }

  .player.modern:not(.compact) .cover {
    width: 68px;
    height: 68px;
    border-radius: 9px;
    box-shadow: 0 8px 24px var(--modern-shadow, var(--shadow));
  }

  .player.modern:not(.compact) .track-copy > span {
    font-size: 0.94rem;
    font-weight: 790;
  }

  .player.modern:not(.compact) .track-copy small {
    font-size: 0.76rem;
  }

  .player.modern:not(.compact) button.play {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    box-shadow: 0 6px 20px color-mix(in srgb, var(--accent) 22%, transparent);
  }

  .player.modern:not(.compact) .progress,
  .player.modern:not(.compact) .volume-bar {
    height: 6px;
  }

  .track {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }

  .player.compact .track {
    width: 34px;
  }

  .track-open {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    width: 100%;
    height: auto;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    padding: 0;
    text-align: left;
  }

  .track-open:hover .track-copy > span,
  .track-open:focus-visible .track-copy > span {
    color: var(--text);
  }

  .track-open:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 4px;
  }

  .cover {
    position: relative;
    width: 54px;
    height: 54px;
    flex: 0 0 auto;
    overflow: hidden;
    border-radius: 8px;
    background:
      linear-gradient(135deg, rgba(255, 255, 255, 0.18), transparent 58%),
      var(--accent);
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.12);
  }

  .cover img {
    position: absolute;
    inset: 0;
    z-index: 1;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .track-copy {
    min-width: 0;
  }

  .track-copy > span,
  .track-copy small {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .track-copy > span {
    margin: 0 0 4px;
    color: var(--text);
    font-weight: 750;
  }

  .track-copy small,
  .progress-area span,
  .volume span {
    color: var(--text-soft);
    font-size: 0.84rem;
    font-weight: 650;
  }

  .transport {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
  }

  .player.compact .transport {
    gap: 7px;
  }

  button.favorite {
    width: 34px;
    height: 34px;
    flex: 0 0 auto;
    border-color: var(--border-strong);
    background: var(--panel-strong);
    color: var(--text-soft);
    font-size: 0.95rem;
  }

  .player.compact button.favorite {
    width: 34px;
    height: 34px;
  }

  button.favorite:hover,
  button.favorite:focus-visible,
  button.favorite.active {
    border-color: color-mix(in srgb, var(--warning) 48%, var(--border));
    background: color-mix(in srgb, var(--warning) 16%, var(--panel));
    color: var(--warning);
  }

  button.favorite:disabled {
    border-color: var(--border-strong);
    background: var(--panel-soft);
    color: var(--text-dim);
  }

  button.queue-button,
  button.lyrics-button {
    width: auto;
    min-width: 74px;
    padding: 0 10px;
    white-space: nowrap;
  }

  button.lyrics-button:hover,
  button.lyrics-button:focus-visible,
  button.queue-button.active,
  button.queue-button:hover,
  button.queue-button:focus-visible {
    border-color: var(--accent-strong);
    background: var(--accent-soft);
    color: var(--accent-text);
    outline: none;
  }

  button.mode-button {
    width: auto;
    min-width: 64px;
    padding: 0 10px;
    white-space: nowrap;
  }

  button.mode-button.active,
  button.mode-button:hover,
  button.mode-button:focus-visible {
    border-color: var(--accent-strong);
    background: var(--accent-soft);
    color: var(--accent-text);
    outline: none;
  }

  button {
    display: grid;
    width: 38px;
    height: 38px;
    place-items: center;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel-strong);
    color: var(--text);
    font: inherit;
    font-size: 0.78rem;
    font-weight: 900;
    cursor: default;
  }

  .player.compact button {
    height: 34px;
    border-color: rgba(58, 68, 82, 0.76);
    background: color-mix(in srgb, var(--panel-strong) 78%, transparent);
    font-size: 0.74rem;
  }

  button:disabled {
    color: var(--text-dim);
    background: var(--panel-soft);
  }

  button.track-open {
    display: flex;
    width: 100%;
    height: auto;
    justify-content: flex-start;
    border: 0;
    background: transparent;
    padding: 0;
  }

  button.track-open:disabled {
    color: inherit;
    background: transparent;
  }

  button.play {
    width: 44px;
    height: 44px;
    border-color: var(--accent);
    background: var(--accent);
    color: var(--accent-contrast);
  }

  .player.compact button.play {
    width: 40px;
    height: 40px;
    border-color: var(--accent);
    background: var(--accent);
    color: var(--accent-contrast);
    box-shadow: 0 0 24px color-mix(in srgb, var(--accent) 18%, transparent);
  }

  button.play:disabled {
    border-color: var(--border-strong);
    background: var(--panel-soft);
    color: var(--text-dim);
  }

  .progress-area,
  .volume {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

  .player.compact .progress-area {
    gap: 8px;
    width: min(40vw, 560px);
  }

  .player.compact .volume {
    justify-content: flex-end;
    width: min(24vw, 240px);
  }

  .player.compact .progress,
  .player.compact .volume-bar {
    height: 6px;
    background:
      linear-gradient(
        to right,
        var(--accent) 0%,
        var(--accent) var(--range-fill, 0%),
        color-mix(in srgb, var(--range-empty) 72%, transparent) var(--range-fill, 0%),
        color-mix(in srgb, var(--range-empty) 72%, transparent) 100%
      );
  }

  .progress,
  .volume-bar {
    appearance: none;
    height: 7px;
    min-width: 0;
    flex: 1;
    overflow: hidden;
    border: 0;
    border-radius: 999px;
    background:
      linear-gradient(
        to right,
        var(--accent) 0%,
        var(--accent) var(--range-fill, 0%),
        var(--border) var(--range-fill, 0%),
        var(--border) 100%
      );
    accent-color: var(--accent);
  }

  .progress:disabled,
  .volume-bar:disabled {
    opacity: 0.55;
  }

  .progress::-webkit-slider-thumb,
  .volume-bar::-webkit-slider-thumb {
    appearance: none;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--text-muted);
  }

  .progress::-moz-range-thumb,
  .volume-bar::-moz-range-thumb {
    width: 12px;
    height: 12px;
    border: 0;
    border-radius: 50%;
    background: var(--text-muted);
  }

  .progress::-moz-range-progress,
  .volume-bar::-moz-range-progress {
    height: 7px;
    background: var(--accent);
  }

  @media (max-width: 920px) {
    .player,
    .player.compact {
      grid-template-columns: minmax(170px, 1fr) auto;
    }

    .progress-area,
    .volume {
      display: none;
    }

    .player.modern:not(.compact) {
      grid-template-areas:
        "track transport"
        "progress progress"
        "volume volume";
      grid-template-columns: minmax(200px, 1fr) auto;
      min-height: 154px;
      gap: 8px 18px;
    }

    .player.modern:not(.compact) .progress-area,
    .player.modern:not(.compact) .volume {
      display: flex;
    }

    .player.modern:not(.compact) .volume {
      justify-content: flex-start;
    }
  }

  @media (max-width: 560px) {
    .player,
    .player.compact {
      grid-template-columns: 1fr;
      gap: 12px;
      padding: 14px 16px;
    }

    .transport {
      justify-content: flex-start;
    }

    .player.modern:not(.compact) {
      grid-template-areas:
        "track"
        "transport"
        "progress"
        "volume";
      grid-template-columns: minmax(0, 1fr);
      min-height: 220px;
    }
  }
</style>
