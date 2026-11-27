<script lang="ts">
  import type { Track } from "$lib/types/library";
  import { listeningDurationLabel, listeningTotalLabel } from "$lib/utils/listeningStats";
  import { localImageSource } from "$lib/utils/localImage";

  type Props = {
    tracks: Track[];
    isScanning: boolean;
    variant?: "default" | "library" | "songs";
    showOrder?: boolean;
    showPlayCounts?: boolean;
    showListeningTime?: boolean;
    activityLabel?: (track: Track) => string;
    selectedTrackId?: string | null;
    unavailableTrackIds?: Set<string>;
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
    showOrder = false,
    showPlayCounts = true,
    showListeningTime = false,
    activityLabel = playCountLabel,
    selectedTrackId = null,
    unavailableTrackIds = new Set<string>(),
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
  let isLibraryVariant = $derived(variant !== "default");
  let isSongsVariant = $derived(variant === "songs");

  function displayArtist(track: Track) {
    return track.artist ?? "Unknown Artist";
  }

  function displayAlbum(track: Track) {
    return track.album ?? track.fileName;
  }

  function selectTrack(track: Track) {
    if (track.unavailable || unavailableTrackIds.has(track.id)) return;
    onTrackSelect?.(track, tracks);
  }

  function openTrackContextMenu(event: MouseEvent, track: Track) {
    event.preventDefault();
    event.stopPropagation();
    if (unavailableTrackIds.has(track.id)) return;
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
    return track.playCount > 0 ? `${track.playCount} ${track.playCount === 1 ? "play" : "plays"}` : "";
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
  <div
    class:library-list={isLibraryVariant}
    class:songs-list={isSongsVariant}
    class:numbered={showOrder}
    class:with-plays={showPlayCounts}
    class:with-time={showListeningTime}
    class="track-list"
  >
    {#if isSongsVariant}
      <div class="song-headings" aria-hidden="true">
        <span class="song-title-heading">Song</span>
        <span class="song-album-heading">Album</span>
        <span class="song-length-heading">Length</span>
        <div class="song-activity">
          {#if showPlayCounts}<span>Plays</span>{/if}
          {#if showListeningTime}<span>Listening time</span>{/if}
        </div>
        <span class="song-format-heading">Format</span>
      </div>
    {/if}
    {#each tracks as track, index (track.id)}
      {@const listenedMs = track.listenedMs ?? 0}
      <div
        class:active={track.id === selectedTrackId}
        class:library={isLibraryVariant}
        class:song-row={isSongsVariant}
        class:withOrder={showOrder}
        class:withMove={hasMoveControls}
        class:withRemove={Boolean(onRemoveTrack)}
        class="track-row"
        role="button"
        aria-disabled={Boolean(track.unavailable || unavailableTrackIds.has(track.id))}
        tabindex={track.unavailable || unavailableTrackIds.has(track.id) ? -1 : 0}
        title={unavailableTrackIds.has(track.id) ? "Retained history: add and associate this music to enable playback" : track.filePath}
        onclick={() => selectTrack(track)}
        oncontextmenu={(event) => openTrackContextMenu(event, track)}
        onkeydown={(event) => handleRowKeydown(event, track)}
      >
        {#if showOrder}
          <span class="track-order" aria-label={`List position ${index + 1}`}>{String(index + 1).padStart(2, "0")}</span>
        {/if}
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
          <span class="track-name">{track.title}{track.unavailable ? " · Unavailable" : track.ignored ? " · Ignored" : ""}</span>
          <button class="track-link" type="button" disabled={unavailableTrackIds.has(track.id)} onclick={(event) => selectArtist(event, track)}>
            {displayArtist(track)}
          </button>
        </div>
        <button class="track-link album-link" type="button" disabled={unavailableTrackIds.has(track.id)} onclick={(event) => selectAlbum(event, track)}>
          {displayAlbum(track)}
        </button>
        {#if isLibraryVariant}
          <span class="track-duration">{formatDuration(track.durationSeconds)}</span>
        {/if}
        {#if isSongsVariant}
          <div class="song-activity" class:no-activity={track.ignored || (!showPlayCounts && !listenedMs && !track.playCount)}>
            {#if showPlayCounts}
              {#if track.ignored}
                <span class="song-metric-empty" aria-hidden="true">—</span>
              {:else}
                <span class="song-metric" title={`${track.playCount.toLocaleString()} ${track.playCount === 1 ? "play" : "plays"}`}>
                  {track.playCount.toLocaleString()}<span class="song-metric-label"> {track.playCount === 1 ? "play" : "plays"}</span>
                </span>
              {/if}
            {/if}
            {#if showListeningTime}
              {#if track.ignored || (!listenedMs && !track.playCount)}
                <span class="song-metric-empty" aria-hidden="true">—</span>
              {:else if !listenedMs}
                <span class="song-metric" title="No listening time was saved for these plays">{listeningTotalLabel(listenedMs, track.playCount)}</span>
              {:else}
                <span class="song-metric" title={`${listeningDurationLabel(listenedMs)} listened`}>
                  {listeningDurationLabel(listenedMs)}<span class="song-metric-label"> listened</span>
                </span>
              {/if}
            {/if}
          </div>
        {:else if !track.ignored && activityLabel(track)}
          <span class="play-count">{activityLabel(track)}</span>
        {:else}
          <span class="play-count empty" aria-hidden="true"></span>
        {/if}
        <button
          class:active={track.isFavorite}
          class="favorite-button"
          type="button"
          disabled={unavailableTrackIds.has(track.id)}
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
    grid-template-columns: minmax(0, 1fr);
    container: track-list / inline-size;
    min-width: 0;
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

  .track-row.library.withOrder {
    grid-template-columns: 34px 46px minmax(180px, 1.35fr) minmax(140px, 0.85fr) minmax(54px, auto) minmax(66px, auto) 34px minmax(48px, auto);
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

  .track-row.library.withMove {
    grid-template-columns: 46px minmax(150px, 1.25fr) minmax(120px, 0.76fr) minmax(54px, auto) minmax(66px, auto) 34px minmax(78px, auto) minmax(48px, auto);
  }

  .track-row.library.withOrder.withMove {
    grid-template-columns: 34px 46px minmax(150px, 1.25fr) minmax(120px, 0.76fr) minmax(54px, auto) minmax(66px, auto) 34px minmax(78px, auto) minmax(48px, auto);
  }

  .track-row.library.withMove.withRemove {
    grid-template-columns: 46px minmax(140px, 1.16fr) minmax(110px, 0.7fr) minmax(54px, auto) minmax(66px, auto) 34px minmax(78px, auto) minmax(72px, auto) minmax(48px, auto);
  }

  .track-row.library.withOrder.withMove.withRemove {
    grid-template-columns: 34px 46px minmax(140px, 1.16fr) minmax(110px, 0.7fr) minmax(54px, auto) minmax(66px, auto) 34px minmax(78px, auto) minmax(72px, auto) minmax(48px, auto);
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

  .track-order {
    color: var(--text-dim) !important;
    font-size: 0.8rem !important;
    font-variant-numeric: tabular-nums;
    font-weight: 850 !important;
    text-align: right;
  }

  .track-row.library.active .track-order {
    color: var(--accent-text) !important;
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

  .track-row.library .move-buttons {
    justify-self: end;
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
    width: fit-content;
    max-width: 100%;
    margin: 0;
    color: var(--text-soft);
    font-size: 0.9rem;
    font-weight: 620;
  }

  .track-row.library .album-link {
    justify-self: start;
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

  @container track-list (max-width: 760px) {
    .track-row {
      grid-template-columns: auto minmax(0, 1fr) auto auto;
    }

    .track-row.library {
      grid-template-columns: 44px minmax(0, 1fr) minmax(48px, auto) 34px minmax(48px, auto);
      gap: 10px;
      min-height: 58px;
      padding: 7px 8px;
    }

    .track-row.library.withOrder {
      grid-template-columns: 28px 44px minmax(0, 1fr) 34px minmax(48px, auto);
    }

    .track-row.withRemove {
      grid-template-columns: auto minmax(0, 1fr) auto auto auto;
    }

    .track-row.withMove,
    .track-row.withMove.withRemove {
      grid-template-columns: auto minmax(0, 1fr) auto auto auto auto;
      gap: 10px;
    }

    .track-row.library.withMove,
    .track-row.library.withMove.withRemove {
      grid-template-columns: 44px minmax(0, 1fr) 34px minmax(48px, auto);
      gap: 10px;
    }

    .track-row.library.withOrder.withMove,
    .track-row.library.withOrder.withMove.withRemove {
      grid-template-columns: 28px 44px minmax(0, 1fr) 34px minmax(48px, auto);
    }

    .track-row.library .move-buttons,
    .track-row.library .remove-button {
      grid-column: 2 / -1;
      justify-self: start;
    }

    .track-row.library.withMove .track-duration {
      display: none;
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

  @container track-list (max-width: 520px) {
    .track-row.library {
      grid-template-columns: 42px minmax(0, 1fr) 34px minmax(46px, auto);
    }

    .track-row.library.withOrder,
    .track-row.library.withOrder.withMove,
    .track-row.library.withOrder.withMove.withRemove {
      grid-template-columns: 26px 42px minmax(0, 1fr) 34px minmax(46px, auto);
    }

    .track-row.library .track-duration {
      display: none;
    }
  }

  .track-list.songs-list {
    --song-activity-columns: 3.3rem;
    --song-activity-gap: 0px;
    --song-layout: var(--song-order-column,) [cover] 40px [song] minmax(0, 1.25fr)
      [album] minmax(0, 0.95fr) [length] 3.5rem [activity] max-content [favorite] 32px [format] 3.7rem;
    gap: 0;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--panel-soft);
    overflow: hidden;
  }

  .songs-list.numbered {
    --song-order-column: [order] 30px;
  }

  .songs-list.with-time {
    --song-activity-columns: 7rem;
  }

  .songs-list.with-plays.with-time {
    --song-activity-columns: 3.3rem 7rem;
    --song-activity-gap: 14px;
  }

  .song-headings,
  .track-row.library.song-row {
    display: grid;
    grid-template-columns: var(--song-layout);
    column-gap: 14px;
    align-items: center;
    padding-inline: 14px;
  }

  .song-headings {
    min-height: 36px;
    padding-block: 8px;
    background: var(--panel-strong);
    color: var(--text-soft);
    font-size: 0.75rem;
    font-weight: 730;
  }

  .song-title-heading { grid-column: cover / album; }
  .song-album-heading, .song-row .album-link { grid-column: album; }
  .song-length-heading, .song-row .track-duration { grid-column: length; text-align: right; }
  .song-format-heading, .song-row .format-badge { grid-column: format; text-align: center; }
  .song-format-heading { justify-self: stretch; }

  .track-row.library.song-row {
    min-height: 56px;
    border: 0;
    border-top: 1px solid var(--border);
    border-radius: 0;
    background: transparent;
    padding-block: 7px;
  }

  .track-row.library.song-row:hover,
  .track-row.library.song-row:focus-visible {
    background: var(--panel-hover);
  }

  .track-row.library.song-row:focus-visible {
    outline: 1px solid var(--accent-strong);
    outline-offset: -1px;
  }

  .track-row.library.song-row.active {
    background: color-mix(in srgb, var(--accent-soft) 78%, var(--panel-soft));
  }

  .song-row .track-order {
    grid-column: order;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .track-row.library.song-row .mini-cover {
    grid-column: cover;
    width: 40px;
    height: 40px;
  }

  .song-row .track-title { grid-column: song; }

  .song-row .track-title .track-link {
    margin-top: 2px;
    font-weight: 540;
  }

  .song-activity {
    grid-column: activity;
    display: grid;
    grid-template-columns: var(--song-activity-columns);
    gap: var(--song-activity-gap);
    text-align: right;
  }

  .song-row .song-activity {
    min-width: 0;
    color: var(--text-soft);
    font-size: 0.84rem;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .song-metric {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .song-metric-empty {
    color: var(--text-dim);
  }

  .song-metric-label {
    position: absolute;
    clip-path: inset(50%);
  }

  .track-row.library.song-row .favorite-button {
    grid-column: favorite;
    border-color: transparent;
    background: transparent;
  }

  .track-row.library.song-row .favorite-button:hover,
  .track-row.library.song-row .favorite-button:focus-visible {
    border-color: var(--border-strong);
    background: var(--panel-strong);
  }

  .track-row.library.song-row .format-badge {
    justify-self: stretch;
    min-width: 0;
    color: var(--text-soft) !important;
    border: 0;
    border-radius: 0;
    background: transparent;
    font-weight: 720 !important;
    padding: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  @container track-list (max-width: 820px) {
    .song-headings,
    .track-row.library.song-row {
      grid-template-columns: var(--song-order-column,) [cover] 40px [song] minmax(0, 1fr)
        [length] 3.5rem [activity] max-content [favorite] 32px [format] 3.7rem;
      column-gap: 10px;
      padding-inline: 10px;
    }

    .song-title-heading { grid-column: cover / length; }
    .song-album-heading, .song-row .album-link { display: none; }
  }

  @container track-list (max-width: 560px) {
    .song-headings,
    .track-row.library.song-row .track-duration {
      display: none;
    }

    .track-row.library.song-row {
      grid-template-columns: var(--song-order-column,) [cover] 36px [song] minmax(0, 1fr) [favorite] 32px [format] 2.8rem;
      row-gap: 1px;
      column-gap: 8px;
      padding-inline: 8px;
    }

    .track-row.library.song-row .mini-cover {
      width: 36px;
      height: 36px;
    }

    .song-row .track-order,
    .song-row .mini-cover,
    .song-row .favorite-button,
    .song-row .format-badge {
      grid-row: 1 / 3;
    }

    .song-row .track-title { grid-row: 1; }

    .song-row .song-activity {
      display: flex;
      flex-wrap: wrap;
      gap: 2px 10px;
      grid-column: song;
      grid-row: 2;
      font-size: 0.73rem;
      text-align: left;
    }

    .song-row .song-activity.no-activity,
    .song-row .song-metric-empty {
      display: none;
    }

    .song-metric-label {
      position: static;
      clip-path: none;
    }
  }
</style>
