<script lang="ts">
  import type { Track } from "$lib/types/library";
  import { localImageSource } from "$lib/utils/localImage";

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
    onTogglePlayback?: () => void;
    onPrevious?: () => void;
    onNext?: () => void;
    onSeek?: (positionSeconds: number) => void;
    onVolumeChange?: (volume: number) => void;
    onToggleFavorite?: (track: Track) => void;
    onToggleQueue?: () => void;
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
    onTogglePlayback,
    onPrevious,
    onNext,
    onSeek,
    onVolumeChange,
    onToggleFavorite,
    onToggleQueue,
  }: Props = $props();

  let localVolume = $state(1);
  let localPosition = $state(0);
  let isSeeking = $state(false);
  let coverArtSrc = $derived(localImageSource(track?.coverArtPath));

  $effect(() => {
    localVolume = volume;
  });

  $effect(() => {
    if (!isSeeking) {
      localPosition = positionSeconds;
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

  function handleSeek() {
    isSeeking = false;
    onSeek?.(localPosition);
  }

  function handleSeekInput() {
    isSeeking = true;
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
    return queueCount > 0 ? `Queue ${queueCount}` : "Queue";
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

<footer class="player" aria-label="Now playing">
  <div class="track">
    <div class="cover" aria-hidden="true">
      {#if coverArtSrc}
        <img src={coverArtSrc} alt="" onload={showLoadedImage} onerror={hideBrokenImage} />
      {/if}
    </div>
    <div class="track-copy">
      <p>{track?.title ?? "No track selected"}</p>
      <span>{displayArtist(track)}{displayAlbum(track)}</span>
    </div>
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
  </div>

  <div class="progress-area">
    <span>{formatDuration(localPosition)}</span>
    <input
      class="progress"
      type="range"
      min="0"
      max={durationSeconds ?? 0}
      step="0.1"
      bind:value={localPosition}
      disabled={!track || !durationSeconds}
      aria-label="Playback progress"
      oninput={handleSeekInput}
      onchange={handleSeek}
    />
    <span>{formatDuration(durationSeconds ?? track?.durationSeconds)}</span>
  </div>

  <div class="volume" aria-label="Volume">
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
    background: #12161c;
  }

  .track {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
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
      #2f8f83;
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

  .track-copy p,
  .track-copy span {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .track-copy p {
    margin: 0 0 4px;
    color: #f5f7fb;
    font-weight: 750;
  }

  .track-copy span,
  .progress-area span,
  .volume span {
    color: #919ba9;
    font-size: 0.84rem;
    font-weight: 650;
  }

  .transport {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
  }

  button.favorite {
    width: 34px;
    height: 34px;
    flex: 0 0 auto;
    border-color: #303844;
    background: #171c23;
    color: #8f9aa8;
    font-size: 0.95rem;
  }

  button.favorite:hover,
  button.favorite:focus-visible,
  button.favorite.active {
    border-color: #6d5b2a;
    background: #262214;
    color: #f0c85a;
  }

  button.favorite:disabled {
    border-color: #303844;
    background: #151a21;
    color: #626c79;
  }

  button.queue-button {
    width: auto;
    min-width: 74px;
    padding: 0 10px;
    white-space: nowrap;
  }

  button.queue-button.active,
  button.queue-button:hover,
  button.queue-button:focus-visible {
    border-color: #35544f;
    background: #17332f;
    color: #d8fffa;
    outline: none;
  }

  button {
    display: grid;
    width: 38px;
    height: 38px;
    place-items: center;
    border: 1px solid #303844;
    border-radius: 8px;
    background: #1a2028;
    color: #eef3f8;
    font: inherit;
    font-size: 0.78rem;
    font-weight: 900;
    cursor: default;
  }

  button:disabled {
    color: #626c79;
    background: #151a21;
  }

  button.play {
    width: 44px;
    height: 44px;
    border-color: #2f8f83;
    background: #2f8f83;
    color: #07110f;
  }

  button.play:disabled {
    border-color: #303844;
    background: #151a21;
    color: #626c79;
  }

  .progress-area,
  .volume {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
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
    background: #2a313c;
    accent-color: #2f8f83;
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
    background: #d8dde4;
    box-shadow: -220px 0 0 214px #d8dde4;
  }

  .progress::-moz-range-thumb,
  .volume-bar::-moz-range-thumb {
    width: 12px;
    height: 12px;
    border: 0;
    border-radius: 50%;
    background: #d8dde4;
  }

  .progress::-moz-range-progress,
  .volume-bar::-moz-range-progress {
    height: 7px;
    background: #2f8f83;
  }

  @media (max-width: 920px) {
    .player {
      grid-template-columns: minmax(170px, 1fr) auto;
    }

    .progress-area,
    .volume {
      display: none;
    }
  }

  @media (max-width: 560px) {
    .player {
      grid-template-columns: 1fr;
      gap: 12px;
      padding: 14px 16px;
    }

    .transport {
      justify-content: flex-start;
    }
  }
</style>
