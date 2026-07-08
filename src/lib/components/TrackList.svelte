<script lang="ts">
  import type { Track } from "$lib/types/library";
  import { localImageSource } from "$lib/utils/localImage";

  type Props = {
    tracks: Track[];
    isScanning: boolean;
    variant?: "default" | "library";
    selectedTrackId?: string | null;
    onTrackSelect?: (track: Track, queue: Track[]) => void;
    onTrackContextMenu?: (track: Track, queue: Track[], x: number, y: number) => void;
    onArtistSelect?: (track: Track) => void;
    onAlbumSelect?: (track: Track) => void;
    onToggleFavorite?: (track: Track) => void;
    onRemoveTrack?: (track: Track) => void;
    onMoveTrackUp?: (track: Track) => void;
    onMoveTrackDown?: (track: Track) => void;
    canMoveTrackUp?: (track: Track) => boolean;
    canMoveTrackDown?: (track: Track) => boolean;
  };

  let {
    tracks,
    isScanning,
    variant = "default",
    selectedTrackId = null,
    onTrackSelect,
    onTrackContextMenu,
    onArtistSelect,
    onAlbumSelect,
    onToggleFavorite,
    onRemoveTrack,
    onMoveTrackUp,
    onMoveTrackDown,
    canMoveTrackUp,
    canMoveTrackDown,
  }: Props = $props();

  let hasMoveControls = $derived(Boolean(onMoveTrackUp || onMoveTrackDown));
  let isLibraryVariant = $derived(variant === "library");

  function displayArtist(track: Track) {
    return track.artist ?? "Unknown Artist";
  }

  function displayAlbum(track: Track) {
    return track.album ?? track.fileName;
  }

  function selectTrack(track: Track) {
    onTrackSelect?.(track, tracks);
  }

  function openTrackContextMenu(event: MouseEvent, track: Track) {
    event.preventDefault();
    event.stopPropagation();
    onTrackContextMenu?.(track, tracks, event.clientX, event.clientY);
  }

  function handleRowKeydown(event: KeyboardEvent, track: Track) {
    if (event.target !== event.currentTarget) {
      return;
    }

    if (event.key === "Enter") {
      event.preventDefault();
      event.stopPropagation();
      selectTrack(track);
    }
  }

  function selectArtist(event: MouseEvent, track: Track) {
    event.stopPropagation();
    onArtistSelect?.(track);
  }

  function selectAlbum(event: MouseEvent, track: Track) {
    event.stopPropagation();
    onAlbumSelect?.(track);
  }

  function toggleFavorite(event: MouseEvent, track: Track) {
    event.stopPropagation();
    onToggleFavorite?.(track);
  }

  function removeTrack(event: MouseEvent, track: Track) {
    event.stopPropagation();
    onRemoveTrack?.(track);
  }

  function moveTrackUp(event: MouseEvent, track: Track) {
    event.stopPropagation();
    onMoveTrackUp?.(track);
  }

  function moveTrackDown(event: MouseEvent, track: Track) {
    event.stopPropagation();
    onMoveTrackDown?.(track);
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

  function playCountLabel(track: Track) {
    return `${track.playCount} ${track.playCount === 1 ? "play" : "plays"}`;
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
</script>

{#if isScanning}
  <div class="empty-state" role="status">
    <h3>Scanning library...</h3>
    <p>Looking for FLAC, MP3, OGG, OPUS, WAV, and M4A files.</p>
  </div>
{:else if tracks.length === 0}
  <div class="empty-state">
    <h3>No songs scanned yet</h3>
    <p>Choose a folder to build the local library.</p>
  </div>
{:else}
  <div class:library-list={isLibraryVariant} class="track-list">
    {#each tracks as track (track.id)}
      <div
        class:active={track.id === selectedTrackId}
        class:library={isLibraryVariant}
        class:withMove={hasMoveControls}
        class:withRemove={Boolean(onRemoveTrack)}
        class="track-row"
        role="button"
        tabindex="0"
        title={track.filePath}
        onclick={() => selectTrack(track)}
        oncontextmenu={(event) => openTrackContextMenu(event, track)}
        onkeydown={(event) => handleRowKeydown(event, track)}
      >
        <div class="mini-cover" aria-hidden="true">
          <span>{track.extension.toUpperCase()}</span>
          {#if track.coverArtPath}
            <img
              src={localImageSource(track.coverArtPath) ?? ""}
              alt=""
              loading="lazy"
              onload={showLoadedImage}
              onerror={hideBrokenImage}
            />
          {/if}
        </div>
        <div class="track-title">
          <span class="track-name">{track.title}</span>
          <button class="track-link" type="button" onclick={(event) => selectArtist(event, track)}>
            {displayArtist(track)}
          </button>
        </div>
        <button class="track-link album-link" type="button" onclick={(event) => selectAlbum(event, track)}>
          {displayAlbum(track)}
        </button>
        {#if isLibraryVariant}
          <span class="track-duration">{formatDuration(track.durationSeconds)}</span>
        {/if}
        {#if track.playCount > 0}
          <span class="play-count">{playCountLabel(track)}</span>
        {:else}
          <span class="play-count empty" aria-hidden="true"></span>
        {/if}
        <button
          class:active={track.isFavorite}
          class="favorite-button"
          type="button"
          aria-label={track.isFavorite ? "Remove from liked songs" : "Add to liked songs"}
          onclick={(event) => toggleFavorite(event, track)}
        >
          {track.isFavorite ? "★" : "☆"}
        </button>
        {#if hasMoveControls}
          <div class="move-buttons" aria-label="Playlist track order">
            <button
              type="button"
              aria-label={`Move ${track.title} up`}
              disabled={canMoveTrackUp ? !canMoveTrackUp(track) : !onMoveTrackUp}
              onclick={(event) => moveTrackUp(event, track)}
            >
              Up
            </button>
            <button
              type="button"
              aria-label={`Move ${track.title} down`}
              disabled={canMoveTrackDown ? !canMoveTrackDown(track) : !onMoveTrackDown}
              onclick={(event) => moveTrackDown(event, track)}
            >
              Down
            </button>
          </div>
        {/if}
        {#if onRemoveTrack}
          <button
            class="remove-button"
            type="button"
            aria-label="Remove from playlist"
            onclick={(event) => removeTrack(event, track)}
          >
            Remove
          </button>
        {/if}
        <span class="format-badge">{track.extension.toUpperCase()}</span>
      </div>
    {/each}
  </div>
{/if}

<style>
  .empty-state {
    display: grid;
    min-height: 150px;
    place-content: center;
    border: 1px dashed var(--border-strong);
    border-radius: 8px;
    background: color-mix(in srgb, var(--panel) 74%, transparent);
    padding: 24px;
    text-align: center;
  }

  .empty-state h3 {
    margin: 0 0 6px;
    color: var(--text);
    font-size: 1rem;
  }

  .empty-state p {
    max-width: 360px;
    margin: 0;
    color: var(--text-soft);
    font-size: 0.9rem;
    font-weight: 650;
  }

  .track-list {
    display: grid;
    gap: 8px;
  }

  .track-list.library-list {
    gap: 5px;
  }

  .track-row {
    display: grid;
    grid-template-columns: auto minmax(160px, 1.2fr) minmax(140px, 0.9fr) auto auto auto;
    align-items: center;
    gap: 14px;
    min-height: 64px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: color-mix(in srgb, var(--panel-soft) 86%, transparent);
    color: inherit;
    font: inherit;
    text-align: left;
    padding: 10px 14px;
    cursor: default;
    outline: none;
    transition:
      border-color 140ms ease,
      background 140ms ease,
      box-shadow 140ms ease;
  }

  .track-row.library {
    grid-template-columns: 46px minmax(190px, 1.35fr) minmax(150px, 0.85fr) minmax(54px, auto) minmax(66px, auto) 34px minmax(48px, auto);
    gap: 12px;
    min-height: 58px;
    border-color: color-mix(in srgb, var(--border) 82%, transparent);
    background: color-mix(in srgb, var(--panel-soft) 72%, transparent);
    padding: 7px 10px;
  }

  .track-row.withRemove {
    grid-template-columns: auto minmax(160px, 1.2fr) minmax(140px, 0.9fr) auto auto auto auto;
  }

  .track-row.withMove {
    grid-template-columns: auto minmax(140px, 1.2fr) minmax(120px, 0.9fr) auto auto auto auto;
  }

  .track-row.withMove.withRemove {
    grid-template-columns: auto minmax(130px, 1.2fr) minmax(110px, 0.8fr) auto auto auto auto auto;
  }

  .track-row:hover,
  .track-row.active,
  .track-row:focus-visible {
    border-color: var(--accent-strong);
    background: var(--panel-hover);
  }

  .track-row.library:hover,
  .track-row.library:focus-visible {
    background: color-mix(in srgb, var(--panel-hover) 88%, transparent);
  }

  .track-row.active {
    background: linear-gradient(
      90deg,
      color-mix(in srgb, var(--accent-soft) 50%, transparent),
      var(--panel-hover)
    );
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .track-row.library.active {
    border-color: color-mix(in srgb, var(--accent) 56%, transparent);
    background: linear-gradient(
      90deg,
      color-mix(in srgb, var(--accent-soft) 58%, transparent),
      color-mix(in srgb, var(--panel-hover) 72%, transparent)
    );
  }

  .track-row > span {
    margin: 0;
    color: var(--text-soft);
    font-size: 0.9rem;
    font-weight: 620;
  }

  .track-row.library > span {
    justify-self: end;
  }

  .favorite-button {
    display: grid;
    width: 32px;
    height: 32px;
    place-items: center;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel-strong);
    color: var(--text-soft);
    cursor: default;
    font: inherit;
    font-size: 0.95rem;
    font-weight: 900;
    line-height: 1;
  }

  .track-row.library .favorite-button {
    justify-self: end;
    border-color: color-mix(in srgb, var(--border-strong) 82%, transparent);
    background: color-mix(in srgb, var(--panel-strong) 80%, transparent);
  }

  .remove-button {
    min-height: 32px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel-strong);
    color: var(--text-muted);
    cursor: default;
    font: inherit;
    font-size: 0.78rem;
    font-weight: 850;
    padding: 0 9px;
  }

  .move-buttons {
    display: flex;
    gap: 6px;
  }

  .move-buttons button {
    min-height: 32px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel-strong);
    color: var(--text-muted);
    cursor: default;
    font: inherit;
    font-size: 0.76rem;
    font-weight: 850;
    padding: 0 8px;
  }

  .move-buttons button:hover,
  .move-buttons button:focus-visible {
    border-color: var(--accent-strong);
    background: var(--panel-hover);
    color: var(--text);
    outline: none;
  }

  .move-buttons button:disabled {
    border-color: var(--border);
    background: var(--panel-soft);
    color: var(--text-dim);
  }

  .remove-button:hover,
  .remove-button:focus-visible {
    border-color: color-mix(in srgb, var(--danger) 34%, var(--border));
    background: var(--danger-soft);
    color: var(--danger);
    outline: none;
  }

  .favorite-button:hover,
  .favorite-button:focus-visible,
  .favorite-button.active {
    border-color: color-mix(in srgb, var(--warning) 48%, var(--border));
    background: color-mix(in srgb, var(--warning) 16%, var(--panel));
    color: var(--warning);
    outline: none;
  }

  .track-title {
    min-width: 0;
  }

  .track-name,
  .track-link {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mini-cover {
    position: relative;
    display: grid;
    width: 42px;
    height: 42px;
    overflow: hidden;
    place-items: center;
    border-radius: 7px;
    background: var(--accent);
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.16);
    color: var(--accent-contrast);
    font-size: 0.7rem;
    font-weight: 900;
  }

  .track-row.library .mini-cover {
    width: 46px;
    height: 46px;
    border-radius: 8px;
    background:
      radial-gradient(circle at 30% 18%, rgba(255, 255, 255, 0.24), transparent 30%),
      linear-gradient(145deg, color-mix(in srgb, var(--accent) 82%, var(--panel)), var(--panel-strong));
  }

  .mini-cover img {
    position: absolute;
    inset: 0;
    z-index: 1;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .track-name {
    display: block;
    color: var(--text);
    font-size: 0.98rem;
    font-weight: 700;
    line-height: 1.25;
  }

  .track-row.library .track-name {
    font-weight: 760;
  }

  .track-link {
    display: block;
    width: fit-content;
    max-width: 100%;
    border: 0;
    background: transparent;
    color: var(--text-soft);
    font-size: 0.86rem;
    font-weight: 650;
    line-height: 1.3;
    margin: 3px 0 0;
    padding: 0;
    text-align: left;
    text-decoration: none;
    cursor: default;
  }

  .album-link {
    width: 100%;
    margin: 0;
    color: var(--text-soft);
    font-size: 0.9rem;
    font-weight: 620;
  }

  .track-row.library .album-link {
    justify-self: stretch;
  }

  .track-link:hover,
  .track-link:focus-visible {
    color: var(--accent-text);
    outline: none;
    text-decoration: underline;
    text-decoration-color: var(--accent);
    text-underline-offset: 3px;
  }

  .play-count {
    min-width: 58px;
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    background: var(--panel-strong);
    color: var(--text-muted);
    font-size: 0.75rem;
    font-weight: 850;
    line-height: 1;
    padding: 6px 8px;
    text-align: center;
    white-space: nowrap;
  }

  .track-row.library .play-count {
    min-width: 66px;
    border-color: color-mix(in srgb, var(--border-strong) 74%, transparent);
    background: color-mix(in srgb, var(--panel-strong) 70%, transparent);
    color: var(--text-soft);
  }

  .play-count.empty {
    visibility: hidden;
  }

  .track-duration {
    min-width: 54px;
    color: var(--text-muted);
    font-size: 0.84rem;
    font-variant-numeric: tabular-nums;
    font-weight: 760;
    text-align: right;
    white-space: nowrap;
  }

  .format-badge {
    min-width: 48px;
    border: 1px solid color-mix(in srgb, var(--border-strong) 74%, transparent);
    border-radius: 999px;
    background: color-mix(in srgb, var(--bg-soft) 64%, transparent);
    color: var(--text-soft) !important;
    font-size: 0.72rem !important;
    font-weight: 860 !important;
    line-height: 1;
    padding: 6px 8px;
    text-align: center;
    white-space: nowrap;
  }

  .track-row:not(.library) .format-badge {
    min-width: 0;
    border: 0;
    border-radius: 0;
    background: transparent;
    color: var(--text-soft) !important;
    font-size: 0.9rem !important;
    font-weight: 620 !important;
    line-height: normal;
    padding: 0;
    text-align: left;
  }

  .track-row.library.active .track-duration,
  .track-row.library.active .album-link,
  .track-row.library.active .track-link,
  .track-row.library.active .play-count,
  .track-row.library.active .format-badge {
    color: var(--accent-text) !important;
  }

  .track-row.library.active .format-badge,
  .track-row.library.active .play-count {
    border-color: var(--accent-strong);
    background: color-mix(in srgb, var(--accent-soft) 72%, transparent);
  }

  @media (max-width: 760px) {
    .track-row {
      grid-template-columns: auto minmax(0, 1fr) auto auto;
    }

    .track-row.library {
      grid-template-columns: 44px minmax(0, 1fr) minmax(48px, auto) 34px minmax(48px, auto);
      gap: 10px;
      min-height: 58px;
      padding: 7px 8px;
    }

    .track-row.withRemove {
      grid-template-columns: auto minmax(0, 1fr) auto auto auto;
    }

    .track-row.withMove,
    .track-row.withMove.withRemove {
      grid-template-columns: auto minmax(0, 1fr) auto auto auto auto;
      gap: 10px;
    }

    .album-link,
    .play-count {
      display: none;
    }

    .track-row.library .track-duration,
    .track-row.library .favorite-button,
    .track-row.library .format-badge {
      display: grid;
    }
  }

  @media (max-width: 520px) {
    .track-row.library {
      grid-template-columns: 42px minmax(0, 1fr) 34px minmax(46px, auto);
    }

    .track-row.library .track-duration {
      display: none;
    }
  }
</style>
