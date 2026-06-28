<script lang="ts">
  import type { Track } from "$lib/types/library";
  import { localImageSource } from "$lib/utils/localImage";

  type Props = {
    tracks: Track[];
    isScanning: boolean;
    selectedTrackId?: string | null;
    onTrackSelect?: (track: Track, queue: Track[]) => void;
    onToggleFavorite?: (track: Track) => void;
  };

  let {
    tracks,
    isScanning,
    selectedTrackId = null,
    onTrackSelect,
    onToggleFavorite,
  }: Props = $props();

  function displayArtist(track: Track) {
    return track.artist ?? "Unknown Artist";
  }

  function displayAlbum(track: Track) {
    return track.album ?? track.fileName;
  }

  function selectTrack(track: Track) {
    onTrackSelect?.(track, tracks);
  }

  function handleRowKeydown(event: KeyboardEvent, track: Track) {
    if (event.target !== event.currentTarget) {
      return;
    }

    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      event.stopPropagation();
      selectTrack(track);
    }
  }

  function toggleFavorite(event: MouseEvent, track: Track) {
    event.stopPropagation();
    onToggleFavorite?.(track);
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
  <div class="track-list">
    {#each tracks as track}
      <div
        class:active={track.id === selectedTrackId}
        class="track-row"
        role="button"
        tabindex="0"
        title={track.filePath}
        onclick={() => selectTrack(track)}
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
          <p>{displayArtist(track)}</p>
        </div>
        <p>{displayAlbum(track)}</p>
        <button
          class:active={track.isFavorite}
          class="favorite-button"
          type="button"
          aria-label={track.isFavorite ? "Remove from liked songs" : "Add to liked songs"}
          onclick={(event) => toggleFavorite(event, track)}
        >
          {track.isFavorite ? "★" : "☆"}
        </button>
        <span>{track.extension.toUpperCase()}</span>
      </div>
    {/each}
  </div>
{/if}

<style>
  .empty-state {
    display: grid;
    min-height: 150px;
    place-content: center;
    border: 1px dashed #303844;
    border-radius: 8px;
    background: rgba(18, 22, 28, 0.74);
    padding: 24px;
    text-align: center;
  }

  .empty-state h3 {
    margin: 0 0 6px;
    color: #f4f7fb;
    font-size: 1rem;
  }

  .empty-state p {
    max-width: 360px;
    margin: 0;
    color: #929daa;
    font-size: 0.9rem;
    font-weight: 650;
  }

  .track-list {
    display: grid;
    gap: 8px;
  }

  .track-row {
    display: grid;
    grid-template-columns: auto minmax(160px, 1.2fr) minmax(140px, 0.9fr) auto auto;
    align-items: center;
    gap: 14px;
    min-height: 64px;
    border: 1px solid #242b35;
    border-radius: 8px;
    background: rgba(22, 26, 32, 0.86);
    color: inherit;
    font: inherit;
    text-align: left;
    padding: 10px 14px;
    cursor: default;
    outline: none;
  }

  .track-row:hover,
  .track-row.active,
  .track-row:focus-visible {
    border-color: #35544f;
    background: #1b2027;
  }

  .track-row > p,
  .track-row > span {
    margin: 0;
    color: #8f9aa8;
    font-size: 0.9rem;
    font-weight: 620;
  }

  .favorite-button {
    display: grid;
    width: 32px;
    height: 32px;
    place-items: center;
    border: 1px solid #303844;
    border-radius: 8px;
    background: #171c23;
    color: #8f9aa8;
    cursor: default;
    font: inherit;
    font-size: 0.95rem;
    font-weight: 900;
    line-height: 1;
  }

  .favorite-button:hover,
  .favorite-button:focus-visible,
  .favorite-button.active {
    border-color: #6d5b2a;
    background: #262214;
    color: #f0c85a;
    outline: none;
  }

  .track-row > p,
  .track-title {
    min-width: 0;
  }

  .track-row > p,
  .track-name,
  .track-title p {
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
    background: #2f8f83;
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.16);
    color: #07110f;
    font-size: 0.7rem;
    font-weight: 900;
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
    color: #f4f7fb;
    font-size: 0.98rem;
    font-weight: 700;
    line-height: 1.25;
  }

  .track-title p {
    margin: 3px 0 0;
    color: #929daa;
    font-size: 0.86rem;
    font-weight: 650;
  }

  @media (max-width: 760px) {
    .track-row {
      grid-template-columns: auto minmax(0, 1fr) auto auto;
    }

    .track-row > p {
      display: none;
    }
  }
</style>
