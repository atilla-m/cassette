<script lang="ts">
  import type { Track } from "$lib/types/library";
  import { localImageSource } from "$lib/utils/localImage";

  type DropdownOption = {
    value: string;
    label: string;
  };

  type Props = {
    tracks: Track[];
    libraryTrackCount: number;
    isScanning: boolean;
    selectedTrackId: string | null;
    hasSearchQuery: boolean;
    sort: string;
    sortOptions: DropdownOption[];
    sortDirectionLabel: string;
    formatFilter: string;
    formatOptions: DropdownOption[];
    artistFilter: string;
    artistOptions: DropdownOption[];
    albumFilter: string;
    albumOptions: DropdownOption[];
    onSortChange: (value: string) => void;
    onToggleSortDirection: () => void;
    onFormatFilterChange: (value: string) => void;
    onArtistFilterChange: (value: string) => void;
    onAlbumFilterChange: (value: string) => void;
    onTrackSelect: (track: Track, queue: Track[]) => void;
    onTrackContextMenu: (track: Track, queue: Track[], x: number, y: number) => void;
    onArtistSelect: (track: Track) => void;
    onAlbumSelect: (track: Track) => void;
    onToggleFavorite: (track: Track) => void;
  };

  let {
    tracks,
    libraryTrackCount,
    isScanning,
    selectedTrackId,
    hasSearchQuery,
    sort,
    sortOptions,
    sortDirectionLabel,
    formatFilter,
    formatOptions,
    artistFilter,
    artistOptions,
    albumFilter,
    albumOptions,
    onSortChange,
    onToggleSortDirection,
    onFormatFilterChange,
    onArtistFilterChange,
    onAlbumFilterChange,
    onTrackSelect,
    onTrackContextMenu,
    onArtistSelect,
    onAlbumSelect,
    onToggleFavorite,
  }: Props = $props();

  function inputValue(event: Event) {
    return event.currentTarget instanceof HTMLSelectElement ? event.currentTarget.value : "All";
  }

  function displayArtist(track: Track) {
    return track.artist ?? track.albumArtist ?? "Unknown Artist";
  }

  function displayAlbum(track: Track) {
    return track.album ?? "Unknown Album";
  }

  function formatDuration(seconds: number | null | undefined) {
    if (seconds === null || seconds === undefined) {
      return "--:--";
    }

    const wholeSeconds = Math.max(0, Math.floor(seconds));
    const minutes = Math.floor(wholeSeconds / 60);
    const remainingSeconds = wholeSeconds % 60;
    return `${minutes}:${remainingSeconds.toString().padStart(2, "0")}`;
  }

  function selectArtist(event: MouseEvent, track: Track) {
    event.stopPropagation();
    onArtistSelect(track);
  }

  function selectAlbum(event: MouseEvent, track: Track) {
    event.stopPropagation();
    onAlbumSelect(track);
  }

  function toggleFavorite(event: MouseEvent, track: Track) {
    event.stopPropagation();
    onToggleFavorite(track);
  }

  function openContextMenu(event: MouseEvent, track: Track) {
    event.preventDefault();
    event.stopPropagation();
    onTrackContextMenu(track, tracks, event.clientX, event.clientY);
  }

  function openContextMenuFromButton(event: MouseEvent, track: Track) {
    event.stopPropagation();
    const rect = event.currentTarget instanceof HTMLElement ? event.currentTarget.getBoundingClientRect() : null;
    onTrackContextMenu(track, tracks, rect?.right ?? event.clientX, rect?.bottom ?? event.clientY);
  }

  function handleRowKeydown(event: KeyboardEvent, track: Track) {
    if (event.target !== event.currentTarget || (event.key !== "Enter" && event.key !== " ")) {
      return;
    }

    event.preventDefault();
    onTrackSelect(track, tracks);
  }

  function showImage(event: Event) {
    if (event.currentTarget instanceof HTMLImageElement) {
      event.currentTarget.hidden = false;
    }
  }

  function hideImage(event: Event) {
    if (event.currentTarget instanceof HTMLImageElement) {
      event.currentTarget.hidden = true;
    }
  }
</script>

<section class="modern-songs" aria-labelledby="modern-songs-title">
  <header class="modern-songs-command">
    <div>
      <p>Full library</p>
      <h3 id="modern-songs-title">Songs</h3>
    </div>

    <div class="modern-song-totals">
      <strong>{tracks.length}</strong>
      <span>{tracks.length === 1 ? "song" : "songs"}</span>
    </div>

    <div class="modern-song-controls" aria-label="Song sorting and filters">
      <label>
        <span>Sort</span>
        <select value={sort} onchange={(event) => onSortChange(inputValue(event))}>
          {#each sortOptions as option}
            <option value={option.value}>{option.label}</option>
          {/each}
        </select>
      </label>
      <button class="direction" type="button" aria-label={`Song sort direction: ${sortDirectionLabel}`} onclick={onToggleSortDirection}>
        {sortDirectionLabel}
      </button>
      <label>
        <span>Artist</span>
        <select value={artistFilter} onchange={(event) => onArtistFilterChange(inputValue(event))}>
          {#each artistOptions as option}
            <option value={option.value}>{option.label}</option>
          {/each}
        </select>
      </label>
      <label>
        <span>Album</span>
        <select value={albumFilter} onchange={(event) => onAlbumFilterChange(inputValue(event))}>
          {#each albumOptions as option}
            <option value={option.value}>{option.label}</option>
          {/each}
        </select>
      </label>
      <label class="format-control">
        <span>Format</span>
        <select value={formatFilter} onchange={(event) => onFormatFilterChange(inputValue(event))}>
          {#each formatOptions as option}
            <option value={option.value}>{option.label}</option>
          {/each}
        </select>
      </label>
    </div>
  </header>

  {#if isScanning}
    <div class="modern-songs-empty" role="status">
      <h3>Scanning library...</h3>
      <p>Looking for FLAC, MP3, OGG, OPUS, WAV, and M4A files.</p>
    </div>
  {:else if libraryTrackCount === 0}
    <div class="modern-songs-empty">
      <h3>No songs in your library</h3>
      <p>Scan a music folder to build a full-library track browser.</p>
    </div>
  {:else if tracks.length === 0}
    <div class="modern-songs-empty">
      <h3>{hasSearchQuery ? "No songs matched" : "No songs match these filters"}</h3>
      <p>{hasSearchQuery ? "Try another title, artist, album, or file name." : "Change the artist, album, or format filter."}</p>
    </div>
  {:else}
    <div class="modern-song-table" role="table" aria-label="Songs">
      <div class="modern-song-header" role="row">
        <span class="title-heading" role="columnheader">Title</span>
        <span class="artist-heading" role="columnheader">Artist</span>
        <span class="album-heading" role="columnheader">Album</span>
        <span class="format-heading" role="columnheader">Format</span>
        <span class="duration-heading" role="columnheader">Time</span>
        <span role="columnheader"><span class="visually-hidden">Favorite</span></span>
        <span role="columnheader"><span class="visually-hidden">Actions</span></span>
      </div>

      <div class="modern-song-body" role="rowgroup">
        {#each tracks as track (track.id)}
          {@const isPlaying = track.id === selectedTrackId}
          <div
            class:playing={isPlaying}
            class="modern-song-row"
            role="row"
            tabindex="0"
            aria-current={isPlaying ? "true" : undefined}
            title={track.filePath}
            ondblclick={() => onTrackSelect(track, tracks)}
            oncontextmenu={(event) => openContextMenu(event, track)}
            onkeydown={(event) => handleRowKeydown(event, track)}
          >
            <div class="modern-song-title" role="cell">
              <span class="modern-song-cover" aria-hidden="true">
                <i>{track.extension.toUpperCase()}</i>
                {#if track.coverArtPath}
                  <img
                    src={localImageSource(track.coverArtPath) ?? ""}
                    alt=""
                    loading="lazy"
                    onload={showImage}
                    onerror={hideImage}
                  />
                {/if}
              </span>
              <span class="modern-song-title-copy">
                <button type="button" onclick={() => onTrackSelect(track, tracks)}>{track.title}</button>
                {#if isPlaying}
                  <small><i aria-hidden="true"><b></b><b></b><b></b></i>Playing</small>
                {:else}
                  <small>{track.fileName}</small>
                {/if}
              </span>
            </div>
            <button class="modern-song-link modern-song-artist" type="button" role="cell" onclick={(event) => selectArtist(event, track)}>
              {displayArtist(track)}
            </button>
            <button class="modern-song-link modern-song-album" type="button" role="cell" onclick={(event) => selectAlbum(event, track)}>
              {displayAlbum(track)}
            </button>
            <span class="modern-song-format" role="cell">{track.extension.toUpperCase()}</span>
            <span class="modern-song-duration" role="cell">{formatDuration(track.durationSeconds)}</span>
            <button
              class:active={track.isFavorite}
              class="modern-song-favorite"
              type="button"
              role="cell"
              aria-label={track.isFavorite ? `Remove ${track.title} from liked songs` : `Add ${track.title} to liked songs`}
              onclick={(event) => toggleFavorite(event, track)}
            >
              {track.isFavorite ? "★" : "☆"}
            </button>
            <button
              class="modern-song-menu"
              type="button"
              role="cell"
              aria-label={`Open actions for ${track.title}`}
              onclick={(event) => openContextMenuFromButton(event, track)}
            >
              •••
            </button>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</section>

<style>
  .modern-songs {
    min-width: 0;
    padding-bottom: 10px;
  }

  .modern-songs-command {
    display: grid;
    grid-template-columns: auto auto minmax(0, 1fr);
    align-items: end;
    gap: 24px;
    margin-bottom: 18px;
  }

  .modern-songs-command p,
  .modern-songs-command h3 {
    margin: 0;
  }

  .modern-songs-command p {
    color: var(--text-dim);
    font-size: 0.64rem;
    font-weight: 650;
    letter-spacing: 0.13em;
    text-transform: uppercase;
  }

  .modern-songs-command h3 {
    margin-top: 4px;
    color: var(--text-muted);
    font-size: 0.9rem;
    font-weight: 590;
  }

  .modern-song-totals {
    display: flex;
    align-items: baseline;
    gap: 5px;
    padding-bottom: 3px;
  }

  .modern-song-totals strong {
    color: var(--text);
    font-size: 1.05rem;
    font-variant-numeric: tabular-nums;
    font-weight: 620;
  }

  .modern-song-totals span {
    color: var(--text-soft);
    font-size: 0.7rem;
  }

  .modern-song-controls {
    display: flex;
    min-width: 0;
    align-items: center;
    justify-content: flex-end;
    gap: 10px 14px;
  }

  .modern-song-controls label {
    display: flex;
    min-width: 0;
    min-height: 32px;
    align-items: center;
    gap: 6px;
    border-bottom: 1px solid color-mix(in srgb, var(--border-strong) 72%, transparent);
    padding: 0 2px;
  }

  .modern-song-controls label > span {
    color: var(--text-dim);
    font-size: 0.66rem;
    font-weight: 600;
  }

  .modern-song-controls select {
    min-width: 0;
    max-width: 150px;
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: 0.72rem;
    font-weight: 610;
  }

  .modern-song-controls button.direction {
    min-height: 32px;
    border: 0;
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: 0.72rem;
    font-weight: 650;
    padding: 0 4px;
  }

  .modern-song-controls button.direction:hover,
  .modern-song-controls button.direction:focus-visible {
    color: var(--accent-text);
    outline: 2px solid var(--focus-ring);
    outline-offset: 3px;
  }

  .modern-song-table {
    min-width: 0;
    border-top: 1px solid color-mix(in srgb, var(--border) 56%, transparent);
  }

  .modern-song-header,
  .modern-song-row {
    display: grid;
    grid-template-columns:
      minmax(260px, 1.55fr)
      minmax(150px, 0.82fr)
      minmax(170px, 1fr)
      58px
      52px
      34px
      34px;
    align-items: center;
    gap: 14px;
  }

  .modern-song-header {
    position: sticky;
    z-index: 4;
    top: -24px;
    min-height: 38px;
    background: color-mix(in srgb, var(--bg) 96%, transparent);
    color: var(--text-dim);
    font-size: 0.62rem;
    font-weight: 620;
    letter-spacing: 0.08em;
    padding: 0 8px;
    text-transform: uppercase;
    backdrop-filter: blur(12px);
  }

  .modern-song-header .duration-heading,
  .modern-song-header .format-heading {
    text-align: right;
  }

  .modern-song-row {
    min-height: 62px;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 48%, transparent);
    color: inherit;
    padding: 7px 8px;
    transition: background 110ms ease, box-shadow 110ms ease;
  }

  .modern-song-row:hover,
  .modern-song-row:focus-visible {
    background: linear-gradient(90deg, color-mix(in srgb, var(--panel-hover) 78%, transparent), transparent 94%);
    box-shadow: inset 2px 0 0 color-mix(in srgb, var(--focus-ring) 72%, transparent);
    outline: none;
  }

  .modern-song-row.playing {
    background: linear-gradient(90deg, color-mix(in srgb, var(--accent) 10%, var(--panel)), transparent 92%);
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .modern-song-title {
    display: grid;
    min-width: 0;
    grid-template-columns: 42px minmax(0, 1fr);
    align-items: center;
    gap: 11px;
  }

  .modern-song-cover {
    position: relative;
    display: grid;
    width: 42px;
    height: 42px;
    overflow: hidden;
    place-items: center;
    border-radius: 3px;
    background: color-mix(in srgb, var(--accent) 14%, var(--panel-strong));
    color: var(--text-dim);
  }

  .modern-song-cover i {
    font-size: 0.52rem;
    font-style: normal;
    font-weight: 680;
  }

  .modern-song-cover img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .modern-song-title-copy {
    display: grid;
    min-width: 0;
    gap: 3px;
  }

  .modern-song-title-copy > button,
  .modern-song-title-copy > small,
  .modern-song-link {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .modern-song-title-copy > button,
  .modern-song-link {
    min-width: 0;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    padding: 0;
    text-align: left;
  }

  .modern-song-title-copy > button {
    color: var(--text);
    font-size: 0.9rem;
    font-weight: 590;
  }

  .modern-song-title-copy > button:hover,
  .modern-song-title-copy > button:focus-visible,
  .modern-song-link:hover,
  .modern-song-link:focus-visible {
    color: var(--accent-text);
    outline: none;
    text-decoration: underline;
    text-decoration-color: var(--accent);
    text-underline-offset: 3px;
  }

  .modern-song-title-copy > small {
    color: var(--text-dim);
    font-size: 0.64rem;
    font-weight: 510;
  }

  .modern-song-title-copy > small i {
    display: inline-flex;
    height: 9px;
    align-items: flex-end;
    gap: 1px;
    margin-right: 5px;
  }

  .modern-song-title-copy > small b {
    display: block;
    width: 2px;
    height: 5px;
    background: var(--accent);
  }

  .modern-song-title-copy > small b:nth-child(2) {
    height: 9px;
  }

  .modern-song-title-copy > small b:nth-child(3) {
    height: 7px;
  }

  .modern-song-row.playing .modern-song-title-copy > small {
    color: var(--accent-text);
    font-weight: 620;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .modern-song-link {
    color: var(--text-soft);
    font-size: 0.76rem;
    font-weight: 520;
  }

  .modern-song-album {
    color: var(--text-dim);
  }

  .modern-song-format,
  .modern-song-duration {
    justify-self: end;
    color: var(--text-dim);
    font-size: 0.67rem;
    font-variant-numeric: tabular-nums;
    font-weight: 540;
  }

  .modern-song-favorite,
  .modern-song-menu {
    display: grid;
    width: 32px;
    height: 32px;
    place-items: center;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--text-soft);
    font: inherit;
  }

  .modern-song-menu {
    font-size: 0.65rem;
    letter-spacing: -0.12em;
  }

  .modern-song-favorite:hover,
  .modern-song-favorite:focus-visible,
  .modern-song-favorite.active,
  .modern-song-menu:hover,
  .modern-song-menu:focus-visible {
    background: var(--panel-hover);
    color: var(--accent-text);
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }

  .modern-song-favorite.active {
    color: var(--warning);
  }

  .modern-songs-empty {
    display: grid;
    min-height: 260px;
    place-content: center;
    border-top: 1px solid color-mix(in srgb, var(--border) 54%, transparent);
    text-align: center;
  }

  .modern-songs-empty h3 {
    margin: 0 0 6px;
    color: var(--text);
    font-size: 1.15rem;
    font-weight: 620;
  }

  .modern-songs-empty p {
    margin: 0;
    color: var(--text-soft);
    font-size: 0.82rem;
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    clip-path: inset(50%);
    white-space: nowrap;
  }

  @media (max-width: 1380px) {
    .modern-songs-command {
      grid-template-columns: auto auto minmax(0, 1fr);
      gap: 18px;
    }

    .modern-song-controls {
      flex-wrap: wrap;
    }

    .modern-song-header,
    .modern-song-row {
      grid-template-columns:
        minmax(240px, 1.45fr)
        minmax(140px, 0.8fr)
        minmax(160px, 0.9fr)
        52px
        34px
        34px;
    }

    .format-heading,
    .modern-song-format {
      display: none;
    }
  }

  @media (max-width: 1120px) {
    .modern-songs-command {
      grid-template-columns: auto auto;
    }

    .modern-song-controls {
      grid-column: 1 / -1;
      justify-content: flex-start;
    }

    .modern-song-header,
    .modern-song-row {
      grid-template-columns:
        minmax(220px, 1.35fr)
        minmax(130px, 0.75fr)
        52px
        34px
        34px;
      gap: 10px;
    }

    .album-heading,
    .modern-song-album {
      display: none;
    }
  }

  @media (max-width: 760px) {
    .modern-song-controls label {
      flex: 1 1 150px;
    }

    .modern-song-controls .format-control {
      display: none;
    }

    .modern-song-header,
    .modern-song-row {
      grid-template-columns: minmax(190px, 1.25fr) minmax(110px, 0.7fr) 48px 32px 32px;
    }

    .modern-song-cover {
      width: 38px;
      height: 38px;
    }

    .modern-song-title {
      grid-template-columns: 38px minmax(0, 1fr);
      gap: 9px;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .modern-song-row {
      transition: none;
    }
  }
</style>
