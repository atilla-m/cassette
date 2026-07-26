<script lang="ts">
  import type { Album, Artist } from "$lib/types/library";
  import { localImageSource } from "$lib/utils/localImage";

  type ArtistSortKey = "name" | "songCount" | "albumCount";

  type Props = {
    artists: Artist[];
    albums: Album[];
    currentArtistName: string | null;
    hasSearchQuery: boolean;
    sort: ArtistSortKey;
    sortDirectionLabel: string;
    onSortChange: (value: ArtistSortKey) => void;
    onToggleSortDirection: () => void;
    onOpenArtist: (artist: Artist) => void;
    onPlayArtist: (artist: Artist, shuffle: boolean) => void;
    onQueueArtist: (artist: Artist) => void;
    onOpenContextMenu: (event: MouseEvent, artist: Artist) => void;
  };

  let {
    artists,
    albums,
    currentArtistName,
    hasSearchQuery,
    sort,
    sortDirectionLabel,
    onSortChange,
    onToggleSortDirection,
    onOpenArtist,
    onPlayArtist,
    onQueueArtist,
    onOpenContextMenu,
  }: Props = $props();

  function inputValue(event: Event): ArtistSortKey {
    return event.currentTarget instanceof HTMLSelectElement
      ? event.currentTarget.value as ArtistSortKey
      : "name";
  }

  function artistAlbums(artist: Artist) {
    return albums.filter((album) => album.artist === artist.name);
  }

  function albumInitials(album: Album) {
    const words = (album.title.trim() || album.artist.trim() || "Album").split(/\s+/).filter(Boolean);
    return (words.length === 1 ? words[0].slice(0, 2) : words.slice(0, 2).map((word) => word[0]).join("")).toUpperCase();
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

<section class="modern-artists" aria-labelledby="modern-artists-title">
  <header class="modern-artists-header">
    <div>
      <p>Library voices</p>
      <h3 id="modern-artists-title">Artists</h3>
    </div>
    <div class="modern-artist-controls">
      <span>{artists.length} {artists.length === 1 ? "artist" : "artists"}</span>
      <label>
        <span>Sort by</span>
        <select value={sort} onchange={(event) => onSortChange(inputValue(event))}>
          <option value="name">Artist name</option>
          <option value="songCount">Song count</option>
          <option value="albumCount">Album count</option>
        </select>
      </label>
      <button type="button" aria-label={`Artist sort direction: ${sortDirectionLabel}`} onclick={onToggleSortDirection}>
        {sortDirectionLabel}
      </button>
    </div>
  </header>

  {#if artists.length === 0}
    <div class="modern-artists-empty">
      <h3>{hasSearchQuery ? "No artists matched" : "No artists found"}</h3>
      <p>{hasSearchQuery ? "Try another artist name." : "Scan a music folder to build your local artist library."}</p>
    </div>
  {:else}
    <div class="modern-artist-grid">
      {#each artists as artist (artist.name)}
        {@const covers = artistAlbums(artist)}
        {@const isPlaying = artist.name === currentArtistName}
        <article
          class:playing={isPlaying}
          class="modern-artist-tile"
          oncontextmenu={(event) => onOpenContextMenu(event, artist)}
        >
          <div class="modern-artist-art">
            {#if covers.length === 0}
              <span class="modern-artist-fallback" aria-hidden="true">
                <i></i><i></i><i></i>
                <strong>{artist.name.slice(0, 2).toUpperCase()}</strong>
              </span>
            {:else}
              <span class="modern-artist-mosaic" aria-hidden="true">
                {#each covers.slice(0, 3) as album}
                  <span class="modern-artist-cover">
                    <strong>{albumInitials(album)}</strong>
                    {#if album.coverArtPath}
                      <img
                        src={localImageSource(album.coverArtPath) ?? ""}
                        alt=""
                        loading="lazy"
                        onload={showImage}
                        onerror={hideImage}
                      />
                    {/if}
                  </span>
                {/each}
              </span>
            {/if}

            <button class="modern-artist-open" type="button" aria-label={`Open ${artist.name}`} onclick={() => onOpenArtist(artist)}></button>

            <div class="modern-artist-actions" role="group" aria-label={`${artist.name} actions`}>
              <button class="play" type="button" aria-label={`Play ${artist.name}`} onclick={() => onPlayArtist(artist, false)}>
                <span aria-hidden="true">▶</span>
              </button>
              <button type="button" aria-label={`Shuffle ${artist.name}`} onclick={() => onPlayArtist(artist, true)}>Shuffle</button>
              <button type="button" aria-label={`Add ${artist.name} to queue`} onclick={() => onQueueArtist(artist)}>Queue</button>
            </div>

            {#if isPlaying}
              <span class="modern-artist-playing"><i aria-hidden="true"></i>Now playing</span>
            {/if}
          </div>

          <button class="modern-artist-copy" type="button" onclick={() => onOpenArtist(artist)}>
            <strong>{artist.name}</strong>
            <span>{artist.detail}</span>
            <small>{covers.length} {covers.length === 1 ? "album" : "albums"}</small>
          </button>
        </article>
      {/each}
    </div>
  {/if}
</section>

<style>
  .modern-artists {
    min-width: 0;
  }

  .modern-artists-header {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 20px;
    margin-bottom: 24px;
  }

  .modern-artists-header p,
  .modern-artists-header h3 {
    margin: 0;
  }

  .modern-artists-header p {
    color: var(--text-dim);
    font-size: 0.64rem;
    font-weight: 650;
    letter-spacing: 0.13em;
    text-transform: uppercase;
  }

  .modern-artists-header h3 {
    margin-top: 4px;
    color: var(--text-muted);
    font-size: 0.9rem;
    font-weight: 590;
  }

  .modern-artist-controls {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .modern-artist-controls > span {
    margin-right: 8px;
    color: var(--text-soft);
    font-size: 0.75rem;
    font-weight: 540;
  }

  .modern-artist-controls label {
    display: flex;
    min-height: 32px;
    align-items: center;
    gap: 6px;
    border-bottom: 1px solid color-mix(in srgb, var(--border-strong) 72%, transparent);
    padding: 0 2px;
  }

  .modern-artist-controls label > span {
    color: var(--text-dim);
    font-size: 0.68rem;
    font-weight: 600;
  }

  .modern-artist-controls select,
  .modern-artist-controls button {
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: 0.74rem;
    font-weight: 620;
  }

  .modern-artist-controls button {
    min-height: 32px;
    padding: 0 4px;
  }

  .modern-artist-controls button:hover,
  .modern-artist-controls button:focus-visible {
    color: var(--accent-text);
    outline: 2px solid var(--focus-ring);
    outline-offset: 3px;
  }

  .modern-artist-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(270px, 1fr));
    gap: clamp(32px, 3vw, 48px) clamp(22px, 2.4vw, 38px);
  }

  .modern-artist-tile {
    min-width: 0;
  }

  .modern-artist-art {
    position: relative;
    aspect-ratio: 1.48;
    overflow: hidden;
    border-radius: 4px;
    background:
      linear-gradient(145deg, color-mix(in srgb, var(--accent) 12%, var(--panel-strong)), var(--panel));
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--border-strong) 48%, transparent);
    isolation: isolate;
  }

  .modern-artist-art::after {
    position: absolute;
    z-index: 2;
    inset: 0;
    background: linear-gradient(to top, rgba(7, 5, 4, 0.92), rgba(7, 5, 4, 0.1) 70%, transparent);
    content: "";
    opacity: 0;
    pointer-events: none;
    transition: opacity 120ms ease;
  }

  .modern-artist-mosaic {
    position: absolute;
    inset: 0;
    display: grid;
    grid-template-columns: 1.25fr 0.75fr;
    grid-template-rows: 1fr 1fr;
    gap: 2px;
    background: var(--bg);
  }

  .modern-artist-cover {
    position: relative;
    display: grid;
    overflow: hidden;
    place-items: center;
    background: color-mix(in srgb, var(--accent) 10%, var(--panel-strong));
    color: var(--text-muted);
    font-size: 1rem;
  }

  .modern-artist-cover:first-child {
    grid-row: 1 / 3;
  }

  .modern-artist-cover:only-child {
    grid-column: 1 / 3;
  }

  .modern-artist-cover img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    transition: filter 140ms ease, transform 160ms ease;
  }

  .modern-artist-fallback {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    gap: 8px;
    padding-bottom: 34px;
  }

  .modern-artist-fallback i {
    width: 22%;
    height: 38%;
    background: color-mix(in srgb, var(--text) 5%, var(--panel-strong));
  }

  .modern-artist-fallback i:nth-child(2) {
    height: 58%;
    background: color-mix(in srgb, var(--accent) 18%, var(--panel-strong));
  }

  .modern-artist-fallback i:nth-child(3) {
    height: 47%;
  }

  .modern-artist-fallback strong {
    position: absolute;
    color: var(--text-soft);
    font-size: 1.05rem;
    font-weight: 660;
    letter-spacing: 0.08em;
  }

  .modern-artist-open {
    position: absolute;
    z-index: 3;
    inset: 0;
    width: 100%;
    border: 0;
    background: transparent;
  }

  .modern-artist-open:focus-visible {
    border-radius: 4px;
    box-shadow: inset 0 0 0 3px color-mix(in srgb, var(--focus-ring) 86%, transparent);
    outline: none;
  }

  .modern-artist-actions {
    position: absolute;
    z-index: 4;
    right: 14px;
    bottom: 13px;
    left: 14px;
    display: flex;
    align-items: center;
    gap: 14px;
    opacity: 0;
    pointer-events: none;
    transform: translateY(4px);
    transition: opacity 120ms ease, transform 120ms ease;
  }

  .modern-artist-actions button {
    min-height: 34px;
    border: 0;
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: 0.68rem;
    font-weight: 650;
    padding: 0;
  }

  .modern-artist-actions button.play {
    display: grid;
    width: 46px;
    min-height: 46px;
    place-items: center;
    margin-right: auto;
    border-radius: 50%;
    background: var(--text);
    color: var(--accent-contrast);
  }

  .modern-artist-actions button:hover,
  .modern-artist-actions button:focus-visible {
    color: var(--accent-text);
    outline: 2px solid var(--focus-ring);
    outline-offset: 4px;
  }

  .modern-artist-actions button.play:hover,
  .modern-artist-actions button.play:focus-visible {
    background: color-mix(in srgb, var(--accent) 42%, var(--text));
    color: var(--accent-contrast);
  }

  .modern-artist-tile:hover .modern-artist-art::after,
  .modern-artist-tile:focus-within .modern-artist-art::after,
  .modern-artist-tile:hover .modern-artist-actions,
  .modern-artist-tile:focus-within .modern-artist-actions {
    opacity: 1;
  }

  .modern-artist-tile:hover .modern-artist-actions,
  .modern-artist-tile:focus-within .modern-artist-actions {
    pointer-events: auto;
    transform: none;
  }

  .modern-artist-tile:hover .modern-artist-cover img,
  .modern-artist-tile:focus-within .modern-artist-cover img {
    filter: saturate(0.9) brightness(0.86);
    transform: scale(1.018);
  }

  .modern-artist-playing {
    position: absolute;
    z-index: 5;
    top: 12px;
    left: 12px;
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text);
    font-size: 0.59rem;
    font-weight: 720;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .modern-artist-playing i {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 3px rgba(7, 5, 4, 0.72);
  }

  .modern-artist-copy {
    display: grid;
    width: 100%;
    min-width: 0;
    gap: 3px;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    padding: 12px 1px 0;
    text-align: left;
  }

  .modern-artist-copy strong,
  .modern-artist-copy span,
  .modern-artist-copy small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .modern-artist-copy strong {
    color: var(--text);
    font-size: 1rem;
    font-weight: 640;
    letter-spacing: -0.02em;
  }

  .modern-artist-copy span,
  .modern-artist-copy small {
    color: var(--text-soft);
    font-size: 0.72rem;
    font-weight: 520;
  }

  .modern-artist-copy small {
    color: var(--text-dim);
  }

  .modern-artist-copy:hover strong,
  .modern-artist-copy:focus-visible strong {
    color: var(--accent-text);
  }

  .modern-artist-copy:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 4px;
  }

  .modern-artists-empty {
    display: grid;
    min-height: 260px;
    place-content: center;
    border-top: 1px solid color-mix(in srgb, var(--border) 54%, transparent);
    text-align: center;
  }

  .modern-artists-empty h3 {
    margin: 0 0 6px;
    color: var(--text);
    font-size: 1.15rem;
    font-weight: 620;
  }

  .modern-artists-empty p {
    margin: 0;
    color: var(--text-soft);
    font-size: 0.82rem;
  }

  @media (max-width: 1180px) {
    .modern-artist-grid {
      grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
    }
  }

  @media (max-width: 720px) {
    .modern-artists-header {
      align-items: flex-start;
      flex-direction: column;
    }

    .modern-artist-controls {
      width: 100%;
      flex-wrap: wrap;
    }

    .modern-artist-controls > span {
      margin-right: auto;
    }
  }

  @media (hover: none) and (pointer: coarse) {
    .modern-artist-art::after,
    .modern-artist-actions {
      opacity: 1;
    }

    .modern-artist-actions {
      pointer-events: auto;
      transform: none;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .modern-artist-art::after,
    .modern-artist-actions,
    .modern-artist-cover img {
      transition: none;
    }
  }
</style>
