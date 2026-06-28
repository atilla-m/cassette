<script lang="ts">
  import LibrarySection from "$lib/components/LibrarySection.svelte";
  import NowPlayingBar from "$lib/components/NowPlayingBar.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import { albums, artists, navItems, nowPlaying, recentlyAdded } from "$lib/data/mockLibrary";
</script>

<svelte:head>
  <title>Cassette</title>
</svelte:head>

<div class="app-shell">
  <div class="workspace">
    <Sidebar items={navItems} />

    <main class="home">
      <header class="home-header">
        <div>
          <p class="eyebrow">Home</p>
          <h2>Your music, on this machine.</h2>
        </div>
        <button type="button">Scan Library</button>
      </header>

      <LibrarySection title="Recently Added">
        <div class="track-list">
          {#each recentlyAdded as track}
            <article class="track-row">
              <div class="mini-cover" style={`--item-color: ${track.color}`} aria-hidden="true"></div>
              <div class="track-title">
                <h3>{track.title}</h3>
                <p>{track.artist}</p>
              </div>
              <p>{track.album}</p>
              <span>{track.duration}</span>
            </article>
          {/each}
        </div>
      </LibrarySection>

      <LibrarySection title="Albums">
        <div class="album-grid">
          {#each albums as album}
            <article class="album-card">
              <div class="album-art" style={`--item-color: ${album.color}`} aria-hidden="true">
                <span></span>
              </div>
              <h3>{album.title}</h3>
              <p>{album.artist} · {album.year}</p>
            </article>
          {/each}
        </div>
      </LibrarySection>

      <LibrarySection title="Artists">
        <div class="artist-grid">
          {#each artists as artist}
            <article class="artist-card">
              <div class="artist-avatar" style={`--item-color: ${artist.color}`} aria-hidden="true">
                {artist.name.slice(0, 1)}
              </div>
              <div>
                <h3>{artist.name}</h3>
                <p>{artist.detail}</p>
              </div>
            </article>
          {/each}
        </div>
      </LibrarySection>
    </main>
  </div>

  <NowPlayingBar track={nowPlaying} />
</div>

<style>
  :global(*) {
    box-sizing: border-box;
  }

  :global(html) {
    background: #0d0f13;
    color: #eef3f8;
    font-family:
      Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    font-size: 16px;
    font-synthesis: none;
    line-height: 1.5;
    text-rendering: optimizeLegibility;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
  }

  :global(body) {
    min-width: 320px;
    min-height: 100vh;
    margin: 0;
    background: #0d0f13;
  }

  :global(button) {
    font-family: inherit;
  }

  .app-shell {
    display: grid;
    grid-template-rows: minmax(0, 1fr) auto;
    min-height: 100vh;
    background:
      radial-gradient(circle at top right, rgba(47, 143, 131, 0.16), transparent 30rem),
      #0d0f13;
  }

  .workspace {
    display: flex;
    min-height: 0;
  }

  .home {
    width: 100%;
    min-width: 0;
    overflow: auto;
    padding: 32px;
  }

  .home-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 24px;
    margin-bottom: 32px;
  }

  .eyebrow {
    margin: 0 0 6px;
    color: #2f8f83;
    font-size: 0.78rem;
    font-weight: 800;
    letter-spacing: 0;
    text-transform: uppercase;
  }

  .home-header h2 {
    max-width: 720px;
    margin: 0;
    color: #f7f9fc;
    font-size: clamp(2rem, 4vw, 3.8rem);
    line-height: 1.02;
  }

  .home-header button {
    min-width: 120px;
    min-height: 40px;
    border: 1px solid #35544f;
    border-radius: 8px;
    background: #17332f;
    color: #d8fffa;
    cursor: default;
    font-weight: 800;
    padding: 0 14px;
  }

  .home :global(.library-section + .library-section) {
    margin-top: 30px;
  }

  .track-list {
    display: grid;
    gap: 8px;
  }

  .track-row {
    display: grid;
    grid-template-columns: auto minmax(160px, 1.2fr) minmax(140px, 0.9fr) auto;
    align-items: center;
    gap: 14px;
    min-height: 64px;
    border: 1px solid #242b35;
    border-radius: 8px;
    background: rgba(22, 26, 32, 0.86);
    padding: 10px 14px;
  }

  .track-row > p,
  .track-row > span,
  .album-card p,
  .artist-card p {
    margin: 0;
    color: #8f9aa8;
    font-size: 0.9rem;
    font-weight: 620;
  }

  .track-row > p,
  .track-title,
  .album-card,
  .artist-card > div:last-child {
    min-width: 0;
  }

  .track-row > p,
  .track-title h3,
  .track-title p,
  .album-card h3,
  .album-card p,
  .artist-card h3,
  .artist-card p {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mini-cover {
    width: 42px;
    height: 42px;
    border-radius: 7px;
    background: var(--item-color);
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.16);
  }

  h3 {
    margin: 0;
    color: #f4f7fb;
    font-size: 0.98rem;
    line-height: 1.25;
  }

  .track-title p {
    margin: 3px 0 0;
    color: #929daa;
    font-size: 0.86rem;
    font-weight: 650;
  }

  .album-grid,
  .artist-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 14px;
  }

  .album-card,
  .artist-card {
    border: 1px solid #242b35;
    border-radius: 8px;
    background: #151a21;
  }

  .album-card {
    padding: 14px;
  }

  .album-art {
    display: grid;
    aspect-ratio: 1;
    place-items: center;
    margin-bottom: 12px;
    border-radius: 8px;
    background:
      linear-gradient(135deg, rgba(255, 255, 255, 0.18), transparent 42%),
      var(--item-color);
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.14);
  }

  .album-art span {
    display: block;
    width: 34%;
    aspect-ratio: 1;
    border: 10px solid rgba(13, 15, 19, 0.55);
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.6);
  }

  .album-card p {
    margin-top: 5px;
  }

  .artist-card {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 82px;
    padding: 14px;
  }

  .artist-avatar {
    display: grid;
    width: 52px;
    height: 52px;
    flex: 0 0 auto;
    place-items: center;
    border-radius: 50%;
    background: var(--item-color);
    color: #0d0f13;
    font-size: 1.15rem;
    font-weight: 900;
  }

  .artist-card p {
    margin-top: 4px;
  }

  @media (max-width: 1020px) {
    .album-grid,
    .artist-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: 760px) {
    .app-shell {
      min-height: 100dvh;
    }

    .workspace {
      flex-direction: column;
    }

    .home {
      padding: 22px 16px;
    }

    .home-header {
      align-items: stretch;
      flex-direction: column;
      margin-bottom: 26px;
    }

    .home-header h2 {
      font-size: 2.25rem;
    }

    .home-header button {
      align-self: flex-start;
    }

    .track-row {
      grid-template-columns: auto minmax(0, 1fr) auto;
    }

    .track-row > p {
      display: none;
    }
  }

  @media (max-width: 520px) {
    .album-grid,
    .artist-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
