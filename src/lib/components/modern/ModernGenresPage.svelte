<script lang="ts">
  import type { Album, Genre, Track } from "$lib/types/library";
  import { localImageSource } from "$lib/utils/localImage";

  type GenreSortKey = "name" | "songCount" | "artistCount" | "albumCount";

  type Props = {
    genres: Genre[];
    albums: Album[];
    tracks: Track[];
    currentAlbumId: string | null;
    hasSearchQuery: boolean;
    sort: GenreSortKey;
    sortDirectionLabel: string;
    onSortChange: (value: GenreSortKey) => void;
    onToggleSortDirection: () => void;
    onOpenGenre: (genre: Genre) => void;
    onOpenContextMenu: (event: MouseEvent, genre: Genre) => void;
  };

  let {
    genres,
    albums,
    tracks,
    currentAlbumId,
    hasSearchQuery,
    sort,
    sortDirectionLabel,
    onSortChange,
    onToggleSortDirection,
    onOpenGenre,
    onOpenContextMenu,
  }: Props = $props();

  function inputValue(event: Event): GenreSortKey {
    return event.currentTarget instanceof HTMLSelectElement
      ? event.currentTarget.value as GenreSortKey
      : "name";
  }

  function albumKeysForGenre(genre: Genre) {
    return new Set(
      tracks
        .filter((track) => (track.genres.length > 0 ? track.genres : ["Unknown Genre"]).includes(genre.name))
        .map((track) => `${(track.albumArtist ?? track.artist ?? "Unknown Artist").toLowerCase()}\u0000${(track.album ?? "Unknown Album").toLowerCase()}`),
    );
  }

  function genreAlbums(genre: Genre) {
    const albumKeys = albumKeysForGenre(genre);
    return albums.filter((album) => albumKeys.has(album.id)).slice(0, 3);
  }

  function genreIsPlaying(genre: Genre) {
    return currentAlbumId !== null && albumKeysForGenre(genre).has(currentAlbumId);
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

<section class="modern-genres" aria-labelledby="modern-genres-title">
  <header class="modern-genres-header">
    <div>
      <p>Browse by sound</p>
      <h3 id="modern-genres-title">Genres</h3>
    </div>
    <div class="modern-genre-controls">
      <span>{genres.length} {genres.length === 1 ? "genre" : "genres"}</span>
      <label>
        <span>Sort by</span>
        <select value={sort} onchange={(event) => onSortChange(inputValue(event))}>
          <option value="name">Genre name</option>
          <option value="songCount">Song count</option>
          <option value="artistCount">Artist count</option>
          <option value="albumCount">Album count</option>
        </select>
      </label>
      <button type="button" aria-label={`Genre sort direction: ${sortDirectionLabel}`} onclick={onToggleSortDirection}>
        {sortDirectionLabel}
      </button>
    </div>
  </header>

  {#if genres.length === 0}
    <div class="modern-genres-empty">
      <h3>{hasSearchQuery ? "No genres matched" : "No genres found"}</h3>
      <p>{hasSearchQuery ? "Try another genre name." : "Scan a music folder to build your local genre library."}</p>
    </div>
  {:else}
    <div class="modern-genre-grid">
      {#each genres as genre (genre.name)}
        {@const covers = genreAlbums(genre)}
        {@const isPlaying = genreIsPlaying(genre)}
        <button
          class:playing={isPlaying}
          class="modern-genre-card"
          type="button"
          aria-label={`Open ${genre.name}: ${genre.songCount} songs, ${genre.artistCount} artists, ${genre.albumCount} albums`}
          onclick={() => onOpenGenre(genre)}
          oncontextmenu={(event) => onOpenContextMenu(event, genre)}
        >
          <span class="modern-genre-mosaic" aria-hidden="true">
            {#if covers.length === 0}
              <span class="modern-genre-empty-art"><i></i><i></i><i></i></span>
            {:else}
              {#each covers as album}
                <span class="modern-genre-cover" style={`--album-color: ${album.color}`}>
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
            {/if}
          </span>
          <span class="modern-genre-copy">
            <span class="modern-genre-title">
              <strong>{genre.name}</strong>
              {#if isPlaying}<i>Playing</i>{/if}
            </span>
            <span class="modern-genre-counts">
              <span>{genre.songCount} {genre.songCount === 1 ? "song" : "songs"}</span>
              <span>{genre.artistCount} {genre.artistCount === 1 ? "artist" : "artists"}</span>
              <span>{genre.albumCount} {genre.albumCount === 1 ? "album" : "albums"}</span>
            </span>
          </span>
          <span class="modern-genre-arrow" aria-hidden="true">↗</span>
        </button>
      {/each}
    </div>
  {/if}
</section>

<style>
  .modern-genres {
    min-width: 0;
  }

  .modern-genres-header {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 20px;
    margin-bottom: 22px;
  }

  .modern-genres-header p,
  .modern-genres-header h3 {
    margin: 0;
  }

  .modern-genres-header p {
    color: var(--text-dim);
    font-size: 0.64rem;
    font-weight: 650;
    letter-spacing: 0.13em;
    text-transform: uppercase;
  }

  .modern-genres-header h3 {
    margin-top: 4px;
    color: var(--text-muted);
    font-size: 0.9rem;
    font-weight: 590;
  }

  .modern-genre-controls {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .modern-genre-controls > span {
    margin-right: 8px;
    color: var(--text-soft);
    font-size: 0.75rem;
    font-weight: 540;
  }

  .modern-genre-controls label {
    display: flex;
    min-height: 32px;
    align-items: center;
    gap: 6px;
    border-bottom: 1px solid color-mix(in srgb, var(--border-strong) 72%, transparent);
    padding: 0 2px;
  }

  .modern-genre-controls label > span {
    color: var(--text-dim);
    font-size: 0.68rem;
    font-weight: 600;
  }

  .modern-genre-controls select {
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: 0.74rem;
    font-weight: 620;
  }

  .modern-genre-controls button {
    min-height: 32px;
    border: 0;
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: 0.72rem;
    font-weight: 650;
    padding: 0 4px;
  }

  .modern-genre-controls button:hover,
  .modern-genre-controls button:focus-visible {
    color: var(--accent-text);
    outline: 2px solid var(--focus-ring);
    outline-offset: 3px;
  }

  .modern-genre-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 1px 30px;
  }

  .modern-genre-card {
    position: relative;
    display: grid;
    grid-template-columns: 116px minmax(0, 1fr) auto;
    align-items: center;
    gap: 20px;
    min-width: 0;
    min-height: 142px;
    border: 0;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 58%, transparent);
    background: transparent;
    color: inherit;
    cursor: default;
    font: inherit;
    padding: 18px 4px;
    text-align: left;
    transition: background 120ms ease, padding 120ms ease;
  }

  .modern-genre-card:hover,
  .modern-genre-card:focus-visible {
    background: linear-gradient(90deg, color-mix(in srgb, var(--panel-hover) 74%, transparent), transparent 92%);
    outline: none;
    padding-inline: 10px 4px;
  }

  .modern-genre-card:focus-visible {
    box-shadow: inset 3px 0 0 var(--focus-ring);
  }

  .modern-genre-mosaic {
    position: relative;
    display: block;
    width: 112px;
    height: 94px;
  }

  .modern-genre-cover {
    --local-cover-accent: color-mix(in srgb, var(--album-color) 18%, var(--accent));
    position: absolute;
    top: 7px;
    left: 0;
    display: grid;
    width: 76px;
    height: 76px;
    place-items: center;
    overflow: hidden;
    border: 2px solid var(--bg);
    border-radius: 3px;
    background: color-mix(in srgb, var(--local-cover-accent) 28%, var(--panel-strong));
    box-shadow: 0 8px 18px rgba(0, 0, 0, 0.28);
    color: var(--text-muted);
    font-size: 0.84rem;
  }

  .modern-genre-cover:nth-child(2) {
    top: 0;
    left: 18px;
    transform: rotate(2deg);
  }

  .modern-genre-cover:nth-child(3) {
    top: 11px;
    left: 36px;
    transform: rotate(4deg);
  }

  .modern-genre-cover img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .modern-genre-empty-art {
    position: absolute;
    inset: 9px 7px 5px 5px;
    display: flex;
    align-items: flex-end;
    gap: 5px;
    border-bottom: 1px solid var(--border-strong);
    padding: 0 12px 14px;
  }

  .modern-genre-empty-art i {
    width: 18px;
    height: 34px;
    background: var(--panel-strong);
  }

  .modern-genre-empty-art i:nth-child(2) {
    height: 54px;
    background: color-mix(in srgb, var(--accent) 24%, var(--panel-strong));
  }

  .modern-genre-empty-art i:nth-child(3) {
    height: 43px;
  }

  .modern-genre-copy,
  .modern-genre-title,
  .modern-genre-counts {
    min-width: 0;
  }

  .modern-genre-copy,
  .modern-genre-title {
    display: flex;
  }

  .modern-genre-copy {
    flex-direction: column;
    gap: 10px;
  }

  .modern-genre-title {
    align-items: center;
    gap: 9px;
  }

  .modern-genre-title strong {
    overflow: hidden;
    color: var(--text);
    font-size: 1.04rem;
    font-weight: 650;
    letter-spacing: -0.02em;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .modern-genre-title i {
    color: var(--accent-text);
    font-size: 0.58rem;
    font-style: normal;
    font-weight: 650;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .modern-genre-title i::before {
    display: inline-block;
    width: 6px;
    height: 6px;
    margin-right: 5px;
    border-radius: 50%;
    background: var(--accent);
    content: "";
  }

  .modern-genre-counts {
    display: flex;
    flex-wrap: wrap;
    gap: 5px 12px;
    color: var(--text-soft);
    font-size: 0.72rem;
    font-weight: 520;
  }

  .modern-genre-counts span + span::before {
    margin-right: 12px;
    color: var(--text-dim);
    content: "·";
  }

  .modern-genre-arrow {
    color: var(--text-dim);
    font-size: 0.9rem;
    transition: color 120ms ease, transform 120ms ease;
  }

  .modern-genre-card:hover .modern-genre-arrow,
  .modern-genre-card:focus-visible .modern-genre-arrow {
    color: var(--accent-text);
    transform: translate(2px, -2px);
  }

  .modern-genres-empty {
    display: grid;
    min-height: 260px;
    place-content: center;
    text-align: center;
  }

  .modern-genres-empty h3 {
    margin: 0 0 6px;
    color: var(--text);
  }

  .modern-genres-empty p {
    margin: 0;
    color: var(--text-soft);
    font-size: 0.82rem;
  }

  @media (max-width: 1480px) {
    .modern-genre-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: 820px) {
    .modern-genres-header {
      align-items: flex-start;
      flex-direction: column;
    }

    .modern-genre-controls {
      width: 100%;
      flex-wrap: wrap;
    }

    .modern-genre-grid {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .modern-genre-card,
    .modern-genre-arrow {
      transition: none;
    }
  }
</style>
