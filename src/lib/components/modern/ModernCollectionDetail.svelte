<script lang="ts">
  import ModernAlbumsPage from "$lib/components/modern/ModernAlbumsPage.svelte";
  import type { Album, Artist } from "$lib/types/library";
  import { localImageSource } from "$lib/utils/localImage";

  type AlbumSortKey = "title" | "artist" | "year" | "trackCount";
  type DetailKind = "Artist" | "Genre";

  type Props = {
    kind: DetailKind;
    title: string;
    backLabel: string;
    albums: Album[];
    atmosphereAlbums?: Album[];
    currentAlbumId: string | null;
    hasSearchQuery: boolean;
    stats: string[];
    secondaryLabels?: string[];
    relatedArtists?: Artist[];
    searchValue: string;
    searchPlaceholder: string;
    albumSort: AlbumSortKey;
    albumSortDirectionLabel: string;
    onBack: () => void;
    onSearchInput: (value: string) => void;
    onSearchKeydown: (event: KeyboardEvent) => void;
    onClearSearch: () => void;
    onPlay: () => void;
    onShuffle: () => void;
    onQueue: () => void;
    onAlbumSortChange: (value: AlbumSortKey) => void;
    onToggleAlbumSortDirection: () => void;
    onOpenAlbum: (album: Album) => void;
    onPlayAlbum: (album: Album, shuffle: boolean) => void;
    onQueueAlbum: (album: Album) => void;
    onOpenAlbumContextMenu: (event: MouseEvent, album: Album) => void;
    onOpenArtist?: (artist: Artist) => void;
    hasTracks: boolean;
  };

  let {
    kind,
    title,
    backLabel,
    albums,
    atmosphereAlbums = albums,
    currentAlbumId,
    hasSearchQuery,
    stats,
    secondaryLabels = [],
    relatedArtists = [],
    searchValue,
    searchPlaceholder,
    albumSort,
    albumSortDirectionLabel,
    onBack,
    onSearchInput,
    onSearchKeydown,
    onClearSearch,
    onPlay,
    onShuffle,
    onQueue,
    onAlbumSortChange,
    onToggleAlbumSortDirection,
    onOpenAlbum,
    onPlayAlbum,
    onQueueAlbum,
    onOpenAlbumContextMenu,
    onOpenArtist,
    hasTracks,
  }: Props = $props();

  let heroAlbums = $derived(atmosphereAlbums.slice(0, 3));
  let titleId = $derived(`modern-${kind.toLowerCase()}-detail-title`);

  function inputValue(event: Event) {
    return event.currentTarget instanceof HTMLInputElement ? event.currentTarget.value : "";
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

<section class="modern-collection-detail" aria-labelledby={titleId}>
  <button class="modern-detail-back" type="button" onclick={onBack}>{backLabel}</button>

  <header class="modern-detail-hero">
    {#if heroAlbums[0]?.coverArtPath}
      <img
        class="modern-detail-atmosphere"
        src={localImageSource(heroAlbums[0].coverArtPath) ?? ""}
        alt=""
        aria-hidden="true"
        onload={showImage}
        onerror={hideImage}
      />
    {/if}

    <div class="modern-detail-mosaic" aria-hidden="true">
      {#if heroAlbums.length === 0}
        <span class="modern-detail-fallback">
          <i></i><i></i><i></i>
          <strong>{title.slice(0, 2).toUpperCase()}</strong>
        </span>
      {:else}
        {#each heroAlbums as album}
          <span class="modern-detail-cover">
            <strong>{albumInitials(album)}</strong>
            {#if album.coverArtPath}
              <img
                src={localImageSource(album.coverArtPath) ?? ""}
                alt=""
                onload={showImage}
                onerror={hideImage}
              />
            {/if}
          </span>
        {/each}
      {/if}
    </div>

    <div class="modern-detail-copy">
      <p>{kind}</p>
      <h2 id={titleId}>{title}</h2>
      <div class="modern-detail-stats" aria-label={`${title} local library totals`}>
        {#each stats as stat}
          <span>{stat}</span>
        {/each}
      </div>
      {#if secondaryLabels.length > 0}
        <div class="modern-detail-secondary" aria-label={`${title} genres`}>
          {#each secondaryLabels as label}
            <span>{label}</span>
          {/each}
        </div>
      {/if}
      <div class="modern-detail-actions">
        <button class="primary" type="button" disabled={!hasTracks} onclick={onPlay}>Play {kind}</button>
        <button type="button" disabled={!hasTracks} onclick={onShuffle}>Shuffle</button>
        <button type="button" disabled={!hasTracks} onclick={onQueue}>Add to Queue</button>
      </div>
    </div>
  </header>

  <div class="modern-detail-command">
    <label class="modern-detail-search">
      <span aria-hidden="true">⌕</span>
      <input
        type="search"
        value={searchValue}
        placeholder={searchPlaceholder}
        aria-label={searchPlaceholder}
        oninput={(event) => onSearchInput(inputValue(event))}
        onkeydown={onSearchKeydown}
      />
      {#if searchValue}
        <button type="button" aria-label={`Clear ${kind.toLowerCase()} search`} onclick={onClearSearch}>×</button>
      {/if}
    </label>
  </div>

  <ModernAlbumsPage
    {albums}
    {currentAlbumId}
    {hasSearchQuery}
    eyebrow={kind === "Artist" ? "Discography" : "Albums in this genre"}
    title="Albums"
    countLabel={hasSearchQuery
      ? `${albums.length} ${albums.length === 1 ? "match" : "matches"}`
      : `${albums.length} ${albums.length === 1 ? "album" : "albums"}`}
    showControls={false}
    emptyTitle={hasSearchQuery ? "No albums matched" : `No albums found for this ${kind.toLowerCase()}`}
    emptyMessage={hasSearchQuery
      ? `Search is limited to this ${kind.toLowerCase()}'s albums.`
      : `No album tags were found for this ${kind.toLowerCase()}.`}
    sort={albumSort}
    sortDirectionLabel={albumSortDirectionLabel}
    onSortChange={onAlbumSortChange}
    onToggleSortDirection={onToggleAlbumSortDirection}
    {onOpenAlbum}
    {onPlayAlbum}
    {onQueueAlbum}
    onOpenContextMenu={onOpenAlbumContextMenu}
  />

  {#if kind === "Genre" && relatedArtists.length > 0 && onOpenArtist}
    <section class="modern-related-artists" aria-labelledby="modern-related-artists-title">
      <header>
        <div>
          <p>Artists in this genre</p>
          <h3 id="modern-related-artists-title">Artists</h3>
        </div>
        <span>{relatedArtists.length} {relatedArtists.length === 1 ? "artist" : "artists"}</span>
      </header>
      <div class="modern-related-artist-list">
        {#each relatedArtists as artist (artist.name)}
          <button type="button" onclick={() => onOpenArtist?.(artist)}>
            <strong>{artist.name}</strong>
            <span>{artist.detail}</span>
            <i aria-hidden="true">↗</i>
          </button>
        {/each}
      </div>
    </section>
  {/if}
</section>

<style>
  .modern-collection-detail {
    display: grid;
    gap: 28px;
    min-width: 0;
  }

  .modern-detail-back {
    width: fit-content;
    min-height: 32px;
    border: 0;
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: 0.78rem;
    font-weight: 560;
    padding: 0;
  }

  .modern-detail-back:hover,
  .modern-detail-back:focus-visible {
    color: var(--accent-text);
    outline: 2px solid var(--focus-ring);
    outline-offset: 4px;
  }

  .modern-detail-hero {
    position: relative;
    display: grid;
    grid-template-columns: minmax(230px, 300px) minmax(0, 1fr);
    align-items: center;
    gap: clamp(32px, 4.5vw, 72px);
    min-height: 310px;
    overflow: hidden;
    border-bottom: 1px solid color-mix(in srgb, var(--accent) 20%, var(--border));
    background:
      linear-gradient(90deg, rgba(10, 9, 8, 0.58), rgba(10, 9, 8, 0.9) 52%, var(--bg) 92%),
      linear-gradient(180deg, color-mix(in srgb, var(--accent) 9%, var(--panel)), var(--bg));
    padding: clamp(24px, 3vw, 42px) clamp(8px, 3vw, 48px);
    isolation: isolate;
  }

  .modern-detail-hero::before {
    position: absolute;
    z-index: -1;
    inset: 0;
    background:
      radial-gradient(circle at 20% 45%, color-mix(in srgb, var(--accent) 18%, transparent), transparent 38%),
      linear-gradient(90deg, color-mix(in srgb, var(--text) 2%, transparent), transparent 46%);
    content: "";
    pointer-events: none;
  }

  .modern-detail-atmosphere {
    position: absolute;
    z-index: -2;
    top: -44%;
    bottom: -44%;
    left: -12%;
    width: 58%;
    height: 188%;
    object-fit: cover;
    filter: blur(48px) saturate(0.42) brightness(0.52);
    opacity: 0.2;
    transform: scale(1.12);
  }

  .modern-detail-mosaic {
    position: relative;
    display: grid;
    width: min(100%, 300px);
    aspect-ratio: 1.04;
    grid-template-columns: 1.12fr 0.88fr;
    grid-template-rows: 1fr 1fr;
    gap: 3px;
    overflow: hidden;
    border-radius: 4px;
    background: var(--panel-strong);
    box-shadow:
      0 24px 48px rgba(0, 0, 0, 0.38),
      0 0 0 1px color-mix(in srgb, var(--text) 10%, transparent);
  }

  .modern-detail-cover {
    position: relative;
    display: grid;
    overflow: hidden;
    place-items: center;
    background: color-mix(in srgb, var(--accent) 12%, var(--panel-strong));
    color: var(--text-muted);
    font-size: 1rem;
  }

  .modern-detail-cover:first-child {
    grid-row: 1 / 3;
  }

  .modern-detail-cover:only-child {
    grid-column: 1 / 3;
  }

  .modern-detail-cover img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .modern-detail-fallback {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    gap: 9px;
    background: linear-gradient(145deg, color-mix(in srgb, var(--accent) 14%, var(--panel-strong)), var(--panel));
    padding-bottom: 44px;
  }

  .modern-detail-fallback i {
    width: 22%;
    height: 38%;
    background: color-mix(in srgb, var(--text) 5%, var(--panel-strong));
  }

  .modern-detail-fallback i:nth-child(2) {
    height: 60%;
    background: color-mix(in srgb, var(--accent) 20%, var(--panel-strong));
  }

  .modern-detail-fallback i:nth-child(3) {
    height: 48%;
  }

  .modern-detail-fallback strong {
    position: absolute;
    color: var(--text-soft);
    font-size: 1.2rem;
    font-weight: 660;
    letter-spacing: 0.08em;
  }

  .modern-detail-copy {
    display: grid;
    min-width: 0;
    gap: 13px;
  }

  .modern-detail-copy > p,
  .modern-detail-copy h2 {
    margin: 0;
  }

  .modern-detail-copy > p {
    color: var(--text-dim);
    font-size: 0.64rem;
    font-weight: 650;
    letter-spacing: 0.16em;
    text-transform: uppercase;
  }

  .modern-detail-copy h2 {
    overflow: hidden;
    max-width: 980px;
    color: var(--text);
    font-size: clamp(2.7rem, 5vw, 5.2rem);
    font-weight: 680;
    letter-spacing: -0.055em;
    line-height: 0.94;
    overflow-wrap: anywhere;
  }

  .modern-detail-stats,
  .modern-detail-secondary {
    display: flex;
    flex-wrap: wrap;
    gap: 5px 0;
    color: var(--text-soft);
    font-size: 0.75rem;
    font-weight: 540;
  }

  .modern-detail-secondary {
    color: var(--text-muted);
  }

  .modern-detail-stats span + span::before,
  .modern-detail-secondary span + span::before {
    margin: 0 10px;
    color: var(--text-dim);
    content: "·";
  }

  .modern-detail-actions {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 18px;
    margin-top: 4px;
  }

  .modern-detail-actions button {
    min-height: 38px;
    border: 0;
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: 0.8rem;
    font-weight: 620;
    padding: 0 2px;
  }

  .modern-detail-actions button.primary {
    min-height: 46px;
    border-radius: 999px;
    background: var(--text);
    color: var(--accent-contrast);
    font-weight: 740;
    padding: 0 22px;
  }

  .modern-detail-actions button:hover,
  .modern-detail-actions button:focus-visible {
    color: var(--accent-text);
    outline: 2px solid var(--focus-ring);
    outline-offset: 4px;
  }

  .modern-detail-actions button.primary:hover,
  .modern-detail-actions button.primary:focus-visible {
    background: color-mix(in srgb, var(--accent) 42%, var(--text));
    color: var(--accent-contrast);
  }

  .modern-detail-actions button:disabled {
    color: var(--text-dim);
    opacity: 0.55;
  }

  .modern-detail-actions button.primary:disabled {
    background: var(--panel-strong);
  }

  .modern-detail-command {
    display: flex;
    justify-content: flex-end;
  }

  .modern-detail-search {
    display: grid;
    width: min(100%, 360px);
    min-height: 38px;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 8px;
    border-bottom: 1px solid var(--modern-input-border, var(--border-strong));
    color: var(--text-dim);
    padding: 0 2px;
  }

  .modern-detail-search input {
    min-width: 0;
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 0.78rem;
  }

  .modern-detail-search input::placeholder {
    color: var(--text-dim);
  }

  .modern-detail-search button {
    width: 28px;
    height: 28px;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--text-soft);
    font: inherit;
  }

  .modern-detail-search:focus-within {
    border-bottom-color: var(--accent);
    box-shadow: 0 1px 0 color-mix(in srgb, var(--focus-ring) 58%, transparent);
  }

  .modern-related-artists {
    min-width: 0;
    border-top: 1px solid color-mix(in srgb, var(--border) 54%, transparent);
    padding-top: 20px;
  }

  .modern-related-artists > header {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 20px;
    margin-bottom: 8px;
  }

  .modern-related-artists p,
  .modern-related-artists h3 {
    margin: 0;
  }

  .modern-related-artists p {
    color: var(--text-dim);
    font-size: 0.64rem;
    font-weight: 650;
    letter-spacing: 0.13em;
    text-transform: uppercase;
  }

  .modern-related-artists h3 {
    margin-top: 4px;
    color: var(--text-muted);
    font-size: 0.9rem;
    font-weight: 590;
  }

  .modern-related-artists > header > span {
    color: var(--text-soft);
    font-size: 0.72rem;
  }

  .modern-related-artist-list {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0 28px;
  }

  .modern-related-artist-list button {
    display: grid;
    min-width: 0;
    min-height: 58px;
    grid-template-columns: minmax(0, 1fr) auto;
    grid-template-rows: auto auto;
    align-items: center;
    gap: 1px 12px;
    border: 0;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 52%, transparent);
    background: transparent;
    color: inherit;
    font: inherit;
    padding: 9px 4px;
    text-align: left;
  }

  .modern-related-artist-list strong,
  .modern-related-artist-list span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .modern-related-artist-list strong {
    color: var(--text);
    font-size: 0.84rem;
    font-weight: 590;
  }

  .modern-related-artist-list span {
    color: var(--text-dim);
    font-size: 0.68rem;
  }

  .modern-related-artist-list i {
    grid-column: 2;
    grid-row: 1 / 3;
    color: var(--text-dim);
    font-style: normal;
  }

  .modern-related-artist-list button:hover,
  .modern-related-artist-list button:focus-visible {
    background: linear-gradient(90deg, color-mix(in srgb, var(--panel-hover) 74%, transparent), transparent);
    box-shadow: inset 2px 0 0 var(--focus-ring);
    outline: none;
    padding-inline: 10px 4px;
  }

  @media (max-width: 1180px) {
    .modern-detail-hero {
      grid-template-columns: minmax(210px, 250px) minmax(0, 1fr);
      gap: 36px;
      min-height: 280px;
      padding: 22px 24px;
    }

    .modern-detail-mosaic {
      max-width: 250px;
    }

    .modern-detail-copy h2 {
      font-size: clamp(2.4rem, 5vw, 4.2rem);
    }

    .modern-related-artist-list {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: 760px) {
    .modern-detail-hero {
      grid-template-columns: minmax(150px, 200px) minmax(0, 1fr);
      gap: 22px;
      min-height: 260px;
      padding: 18px 4px;
    }

    .modern-detail-copy h2 {
      font-size: clamp(2rem, 6vw, 3.2rem);
    }

    .modern-detail-actions {
      gap: 10px;
    }

    .modern-detail-actions button.primary {
      min-height: 42px;
      padding-inline: 16px;
    }

    .modern-detail-command {
      justify-content: stretch;
    }

    .modern-detail-search {
      width: 100%;
    }

    .modern-related-artist-list {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 560px) {
    .modern-detail-hero {
      grid-template-columns: 1fr;
    }

    .modern-detail-mosaic {
      width: 190px;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .modern-related-artist-list button {
      transition: none;
    }
  }
</style>
