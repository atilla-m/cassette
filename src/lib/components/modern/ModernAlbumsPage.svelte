<script lang="ts">
  import type { Album } from "$lib/types/library";
  import { localImageSource } from "$lib/utils/localImage";

  type AlbumSortKey = "title" | "artist" | "year" | "trackCount";

  type Props = {
    albums: Album[];
    currentAlbumId: string | null;
    hasSearchQuery: boolean;
    eyebrow?: string;
    title?: string;
    countLabel?: string | null;
    showControls?: boolean;
    emptyTitle?: string | null;
    emptyMessage?: string | null;
    sort: AlbumSortKey;
    sortDirectionLabel: string;
    onSortChange: (value: AlbumSortKey) => void;
    onToggleSortDirection: () => void;
    onOpenAlbum: (album: Album) => void;
    onPlayAlbum: (album: Album, shuffle: boolean) => void;
    onQueueAlbum: (album: Album) => void;
    onOpenContextMenu: (event: MouseEvent, album: Album) => void;
  };

  let {
    albums,
    currentAlbumId,
    hasSearchQuery,
    eyebrow = "Browse the shelf",
    title = "Albums",
    countLabel = null,
    showControls = true,
    emptyTitle = null,
    emptyMessage = null,
    sort,
    sortDirectionLabel,
    onSortChange,
    onToggleSortDirection,
    onOpenAlbum,
    onPlayAlbum,
    onQueueAlbum,
    onOpenContextMenu,
  }: Props = $props();

  function albumInitials(album: Album) {
    const words = (album.title.trim() || album.artist.trim() || "Album").split(/\s+/).filter(Boolean);
    return (words.length === 1 ? words[0].slice(0, 2) : words.slice(0, 2).map((word) => word[0]).join("")).toUpperCase();
  }

  function inputValue(event: Event): AlbumSortKey {
    return event.currentTarget instanceof HTMLSelectElement
      ? event.currentTarget.value as AlbumSortKey
      : "title";
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

<section class="modern-albums" aria-labelledby="modern-albums-title">
  <header class="modern-collection-header">
    <div>
      <p>{eyebrow}</p>
      <h3 id="modern-albums-title">{title}</h3>
    </div>
    <div class="modern-album-controls">
      <span>{countLabel ?? `${albums.length} ${albums.length === 1 ? "album" : "albums"}`}</span>
      {#if showControls}
        <label>
          <span>Sort by</span>
          <select value={sort} onchange={(event) => onSortChange(inputValue(event))}>
            <option value="title">Album title</option>
            <option value="artist">Artist</option>
            <option value="year">Year</option>
            <option value="trackCount">Song count</option>
          </select>
        </label>
        <button type="button" aria-label={`Album sort direction: ${sortDirectionLabel}`} onclick={onToggleSortDirection}>
          {sortDirectionLabel}
        </button>
      {/if}
    </div>
  </header>

  {#if albums.length === 0}
    <div class="modern-empty-state">
      <span aria-hidden="true">C</span>
      <h3>{emptyTitle ?? (hasSearchQuery ? "No albums matched" : "Your album shelf is empty")}</h3>
      <p>{emptyMessage ?? (hasSearchQuery ? "Try another album title, artist, or year." : "Scan a music folder to build your local collection.")}</p>
    </div>
  {:else}
    <div class="modern-album-grid">
      {#each albums as album (album.id)}
        <article
          class:playing={album.id === currentAlbumId}
          class="modern-album-tile"
          oncontextmenu={(event) => onOpenContextMenu(event, album)}
        >
          <div class="modern-album-art" style={`--album-color: ${album.color}`}>
            {#if album.coverArtPath}
              <img
                src={localImageSource(album.coverArtPath) ?? ""}
                alt={`${album.title} by ${album.artist} cover`}
                loading="lazy"
                onload={showImage}
                onerror={hideImage}
              />
            {:else}
              <span class="modern-album-placeholder" aria-hidden="true">
                <strong>{albumInitials(album)}</strong>
                <small>Cassette</small>
              </span>
            {/if}

            <button class="modern-album-open" type="button" aria-label={`Open ${album.title} by ${album.artist}`} onclick={() => onOpenAlbum(album)}></button>

            <div class="modern-album-overlay" role="group" aria-label={`${album.title} actions`}>
              <button class="play" type="button" aria-label={`Play ${album.title}`} onclick={() => onPlayAlbum(album, false)}>
                <span aria-hidden="true">▶</span>
              </button>
              <button type="button" aria-label={`Shuffle ${album.title}`} onclick={() => onPlayAlbum(album, true)}>Shuffle</button>
              <button type="button" aria-label={`Add ${album.title} to queue`} onclick={() => onQueueAlbum(album)}>Queue</button>
            </div>

            {#if album.id === currentAlbumId}
              <span class="modern-playing-badge">Now playing</span>
            {/if}
          </div>

          <button class="modern-album-copy" type="button" onclick={() => onOpenAlbum(album)}>
            <strong>{album.title}</strong>
            <span>{album.artist}</span>
            <small>{[album.year, `${album.trackCount} ${album.trackCount === 1 ? "song" : "songs"}`].filter(Boolean).join(" · ")}</small>
          </button>
        </article>
      {/each}
    </div>
  {/if}
</section>

<style>
  .modern-albums {
    min-width: 0;
  }

  .modern-collection-header {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 20px;
    margin-bottom: 22px;
  }

  .modern-collection-header p,
  .modern-collection-header h3 {
    margin: 0;
  }

  .modern-collection-header p {
    color: var(--text-dim);
    font-size: 0.64rem;
    font-weight: 650;
    letter-spacing: 0.13em;
    text-transform: uppercase;
  }

  .modern-collection-header h3 {
    margin-top: 4px;
    color: var(--text-secondary, var(--text-muted));
    font-size: 0.9rem;
    font-weight: 590;
  }

  .modern-album-controls {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .modern-album-controls > span {
    margin-right: 8px;
    color: var(--text-soft);
    font-size: 0.75rem;
    font-weight: 540;
  }

  .modern-album-controls label {
    display: flex;
    min-height: 32px;
    align-items: center;
    gap: 6px;
    border: 0;
    border-bottom: 1px solid color-mix(in srgb, var(--border-strong) 72%, transparent);
    border-radius: 0;
    background: transparent;
    padding: 0 2px;
  }

  .modern-album-controls label > span {
    color: var(--text-dim);
    font-size: 0.68rem;
    font-weight: 600;
  }

  .modern-album-controls select {
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: 0.74rem;
    font-weight: 620;
  }

  .modern-album-controls button {
    min-height: 32px;
    border: 0;
    border-radius: 0;
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: 0.72rem;
    font-weight: 650;
    padding: 0 4px;
  }

  .modern-album-controls button:hover,
  .modern-album-controls button:focus-visible {
    color: var(--accent-text);
    outline: 2px solid var(--focus-ring);
    outline-offset: 3px;
  }

  .modern-album-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(205px, 1fr));
    gap: clamp(30px, 3.2vw, 48px) clamp(20px, 2.1vw, 32px);
  }

  .modern-album-tile {
    min-width: 0;
  }

  .modern-album-art {
    --local-album-accent: color-mix(in srgb, var(--album-color) 22%, var(--accent));
    position: relative;
    aspect-ratio: 1;
    overflow: hidden;
    border-radius: 4px;
    background:
      linear-gradient(145deg, color-mix(in srgb, var(--local-album-accent) 36%, var(--panel-strong)), color-mix(in srgb, var(--local-album-accent) 10%, var(--bg))),
      var(--panel-strong);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--border-strong) 48%, transparent);
    isolation: isolate;
  }

  .modern-album-art::after {
    position: absolute;
    z-index: 1;
    inset: 0;
    background: linear-gradient(to top, rgba(7, 5, 4, 0.9), rgba(7, 5, 4, 0.12) 66%, transparent);
    content: "";
    opacity: 0;
    pointer-events: none;
    transition: opacity 120ms ease;
  }

  .modern-album-art img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    transition: transform 160ms ease;
  }

  .modern-album-placeholder {
    position: absolute;
    inset: 0;
    display: grid;
    align-content: center;
    justify-items: center;
    color: color-mix(in srgb, var(--text) 90%, var(--local-album-accent));
    text-shadow: 0 2px 18px rgba(0, 0, 0, 0.28);
  }

  .modern-album-placeholder::before,
  .modern-album-placeholder::after {
    position: absolute;
    width: 46%;
    height: 19%;
    border: 1px solid rgba(255, 255, 255, 0.24);
    border-radius: 2px;
    content: "";
  }

  .modern-album-placeholder::before {
    top: 17%;
  }

  .modern-album-placeholder::after {
    bottom: 17%;
  }

  .modern-album-placeholder strong {
    font-size: clamp(1.6rem, 3vw, 2.8rem);
    font-weight: 900;
    letter-spacing: -0.06em;
  }

  .modern-album-placeholder small {
    margin-top: 2px;
    font-size: 0.63rem;
    font-weight: 850;
    letter-spacing: 0.16em;
    text-transform: uppercase;
  }

  .modern-album-open {
    position: absolute;
    z-index: 2;
    inset: 0;
    width: 100%;
    border: 0;
    background: transparent;
  }

  .modern-album-open:focus-visible {
    border-radius: 4px;
    box-shadow: inset 0 0 0 3px color-mix(in srgb, var(--focus-ring) 86%, transparent);
    outline: none;
  }

  .modern-album-overlay {
    position: absolute;
    z-index: 3;
    right: 13px;
    bottom: 13px;
    left: 13px;
    display: flex;
    align-items: center;
    gap: 12px;
    opacity: 0;
    pointer-events: none;
    transform: translateY(4px);
    transition: opacity 120ms ease, transform 120ms ease;
  }

  .modern-album-overlay button {
    min-height: 34px;
    flex: 0 0 auto;
    border: 0;
    border-radius: 0;
    background: transparent;
    color: var(--modern-text-secondary, #c5bbae);
    font: inherit;
    font-size: 0.68rem;
    font-weight: 650;
    padding: 0;
  }

  .modern-album-overlay button.play {
    display: grid;
    width: 46px;
    min-height: 46px;
    place-items: center;
    margin-right: auto;
    border-radius: 50%;
    background: var(--text);
    color: var(--accent-contrast);
  }

  .modern-album-overlay button.play span {
    margin-left: 2px;
    font-size: 0.8rem;
  }

  .modern-album-overlay button:hover,
  .modern-album-overlay button:focus-visible {
    color: var(--text);
    outline: 2px solid color-mix(in srgb, var(--focus-ring) 82%, transparent);
    outline-offset: 4px;
  }

  .modern-album-overlay button.play:hover,
  .modern-album-overlay button.play:focus-visible {
    background: color-mix(in srgb, var(--local-album-accent) 54%, var(--text));
    color: var(--accent-contrast);
  }

  .modern-album-tile:hover .modern-album-art::after,
  .modern-album-tile:focus-within .modern-album-art::after,
  .modern-album-tile:hover .modern-album-overlay,
  .modern-album-tile:focus-within .modern-album-overlay {
    opacity: 1;
  }

  .modern-album-tile:hover .modern-album-overlay,
  .modern-album-tile:focus-within .modern-album-overlay {
    pointer-events: auto;
    transform: none;
  }

  .modern-album-tile:hover .modern-album-art img,
  .modern-album-tile:focus-within .modern-album-art img {
    transform: scale(1.018);
  }

  .modern-playing-badge {
    position: absolute;
    z-index: 4;
    top: 12px;
    left: 12px;
    display: flex;
    align-items: center;
    gap: 6px;
    background: transparent;
    color: var(--text);
    font-size: 0.6rem;
    font-weight: 720;
    letter-spacing: 0.08em;
    padding: 0;
    text-transform: uppercase;
    text-shadow: 0 1px 8px #000;
  }

  .modern-playing-badge::before {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--local-album-accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--local-album-accent) 20%, transparent);
    content: "";
  }

  .modern-album-tile.playing .modern-album-art {
    box-shadow: 0 0 0 1px var(--local-album-accent), inset 0 0 0 1px color-mix(in srgb, var(--text) 18%, transparent);
  }

  .modern-album-copy {
    display: block;
    width: 100%;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    padding: 13px 1px 0;
    text-align: left;
  }

  .modern-album-copy strong,
  .modern-album-copy span,
  .modern-album-copy small {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .modern-album-copy strong {
    color: var(--text);
    font-size: 0.96rem;
    font-weight: 650;
    letter-spacing: -0.012em;
  }

  .modern-album-copy span {
    margin-top: 4px;
    color: var(--text-muted);
    font-size: 0.78rem;
    font-weight: 540;
  }

  .modern-album-copy small {
    margin-top: 5px;
    color: var(--text-dim);
    font-size: 0.69rem;
    font-weight: 520;
  }

  .modern-album-copy:hover strong,
  .modern-album-copy:focus-visible strong {
    color: var(--text);
  }

  .modern-album-copy:focus-visible {
    border-radius: 2px;
    outline: 2px solid var(--focus-ring);
    outline-offset: 4px;
  }

  .modern-empty-state {
    display: grid;
    min-height: 310px;
    place-items: center;
    align-content: center;
    color: var(--text-soft);
    text-align: center;
  }

  .modern-empty-state > span {
    display: grid;
    width: 58px;
    height: 58px;
    place-items: center;
    border-radius: 12px;
    background: var(--panel-strong);
    color: var(--accent);
    font-size: 1.3rem;
    font-weight: 900;
  }

  .modern-empty-state h3 {
    margin: 14px 0 3px;
    color: var(--text);
    font-size: 1rem;
  }

  .modern-empty-state p {
    margin: 0;
    font-size: 0.82rem;
  }

  @media (max-width: 1366px) {
    .modern-album-grid {
      grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
      gap: 30px 22px;
    }
  }

  @media (max-width: 820px) {
    .modern-collection-header {
      align-items: flex-start;
      flex-direction: column;
    }

    .modern-album-controls {
      width: 100%;
      flex-wrap: wrap;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .modern-album-art::after,
    .modern-album-art img,
    .modern-album-overlay {
      transition: none;
    }
  }
</style>
