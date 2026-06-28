<script lang="ts">
  import { chooseLibraryFolder, scanLibrary } from "$lib/api/library";
  import {
    getPlaybackStatus,
    pausePlayback,
    playTrack,
    resumePlayback,
    seekPlayback,
    setPlaybackVolume,
  } from "$lib/api/playback";
  import LibrarySection from "$lib/components/LibrarySection.svelte";
  import NowPlayingBar from "$lib/components/NowPlayingBar.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import TrackList from "$lib/components/TrackList.svelte";
  import { buildAlbums, buildArtists } from "$lib/data/libraryViews";
  import { albums as mockAlbums, artists as mockArtists, navItems } from "$lib/data/mockLibrary";
  import type { Album, PlaybackStatus, Track } from "$lib/types/library";
  import { onMount } from "svelte";

  let tracks = $state<Track[]>([]);
  let isScanning = $state(false);
  let scanError = $state<string | null>(null);
  let playbackError = $state<string | null>(null);
  let scannedFolder = $state<string | null>(null);
  let scanCount = $state<number | null>(null);
  let currentTrack = $state<Track | null>(null);
  let currentTrackIndex = $state<number | null>(null);
  let isPlaying = $state(false);
  let positionSeconds = $state(0);
  let durationSeconds = $state<number | null>(null);
  let volume = $state(1);
  let displayAlbums = $derived(scanCount === null ? mockAlbums : buildAlbums(tracks));
  let displayArtists = $derived(scanCount === null ? mockArtists : buildArtists(tracks));
  let canPlayPrevious = $derived(currentTrackIndex !== null && currentTrackIndex > 0);
  let canPlayNext = $derived(currentTrackIndex !== null && currentTrackIndex < tracks.length - 1);

  onMount(() => {
    const statusIntervalId = window.setInterval(async () => {
      if (!currentTrack || !isPlaying) {
        return;
      }

      try {
        applyPlaybackStatus(await getPlaybackStatus());
      } catch (error) {
        playbackError = error instanceof Error ? error.message : String(error);
      }
    }, 1000);

    const progressIntervalId = window.setInterval(() => {
      if (!currentTrack || !isPlaying) {
        return;
      }

      const duration = durationSeconds ?? currentTrack.durationSeconds;
      const nextPosition = positionSeconds + 0.25;
      positionSeconds = duration ? Math.min(nextPosition, duration) : nextPosition;
    }, 250);

    function handleKeydown(event: KeyboardEvent) {
      if (shouldIgnoreShortcut(event.target)) {
        return;
      }

      if (event.code === "Space") {
        event.preventDefault();
        void handleTogglePlayback();
      } else if (event.key === "ArrowRight") {
        event.preventDefault();
        void handleNextTrack();
      } else if (event.key === "ArrowLeft") {
        event.preventDefault();
        void handlePreviousTrack();
      }
    }

    window.addEventListener("keydown", handleKeydown);

    return () => {
      window.clearInterval(statusIntervalId);
      window.clearInterval(progressIntervalId);
      window.removeEventListener("keydown", handleKeydown);
    };
  });

  async function handleScanLibrary() {
    scanError = null;

    try {
      const folder = await chooseLibraryFolder();

      if (!folder) {
        return;
      }

      isScanning = true;
      scannedFolder = folder;
      scanCount = null;
      tracks = [];
      currentTrackIndex = null;

      const scannedTracks = await scanLibrary(folder);
      tracks = scannedTracks;
      scanCount = scannedTracks.length;
    } catch (error) {
      scanError = error instanceof Error ? error.message : String(error);
      scanCount = null;
    } finally {
      isScanning = false;
    }
  }

  async function handleTrackSelect(track: Track) {
    const trackIndex = tracks.findIndex((candidate) => candidate.id === track.id);
    await playTrackAtIndex(trackIndex);
  }

  async function playTrackAtIndex(trackIndex: number) {
    if (trackIndex < 0 || trackIndex >= tracks.length) {
      return;
    }

    const track = tracks[trackIndex];
    playbackError = null;

    try {
      const status = await playTrack(track.filePath);
      currentTrack = track;
      currentTrackIndex = trackIndex;
      applyPlaybackStatus(status);
      durationSeconds = status.durationSeconds ?? track.durationSeconds;
    } catch (error) {
      playbackError = error instanceof Error ? error.message : String(error);
    }
  }

  async function handlePreviousTrack() {
    if (!canPlayPrevious || currentTrackIndex === null) {
      return;
    }

    await playTrackAtIndex(currentTrackIndex - 1);
  }

  async function handleNextTrack() {
    if (!canPlayNext || currentTrackIndex === null) {
      return;
    }

    await playTrackAtIndex(currentTrackIndex + 1);
  }

  async function handleTogglePlayback() {
    if (!currentTrack) {
      return;
    }

    playbackError = null;

    try {
      const status = isPlaying ? await pausePlayback() : await resumePlayback();
      applyPlaybackStatus(status);
    } catch (error) {
      playbackError = error instanceof Error ? error.message : String(error);
    }
  }

  async function handleSeek(nextPositionSeconds: number) {
    playbackError = null;

    try {
      applyPlaybackStatus(await seekPlayback(nextPositionSeconds));
    } catch (error) {
      playbackError = error instanceof Error ? error.message : String(error);
    }
  }

  async function handleVolumeChange(nextVolume: number) {
    playbackError = null;
    volume = nextVolume;

    try {
      applyPlaybackStatus(await setPlaybackVolume(nextVolume));
    } catch (error) {
      playbackError = error instanceof Error ? error.message : String(error);
    }
  }

  function applyPlaybackStatus(status: PlaybackStatus) {
    isPlaying = status.isPlaying;
    positionSeconds = status.positionSeconds;
    durationSeconds = status.durationSeconds ?? currentTrack?.durationSeconds ?? null;
    volume = status.volume;
  }

  function shouldIgnoreShortcut(target: EventTarget | null) {
    if (!(target instanceof HTMLElement)) {
      return false;
    }

    if (target.isContentEditable) {
      return true;
    }

    return ["INPUT", "TEXTAREA", "SELECT", "BUTTON"].includes(target.tagName);
  }

  function albumDetail(album: Album) {
    const year = album.year ? ` · ${album.year}` : "";
    const trackCount = `${album.trackCount} ${album.trackCount === 1 ? "song" : "songs"}`;

    return `${album.artist}${year} · ${trackCount}`;
  }
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
          {#if scanCount !== null && !isScanning}
            <p class="scan-status">
              Found {scanCount} {scanCount === 1 ? "track" : "tracks"} in {scannedFolder}
            </p>
          {:else}
            <p class="scan-status">Pick a folder to scan your local music files.</p>
          {/if}
        </div>
        <button type="button" disabled={isScanning} onclick={handleScanLibrary}>
          {isScanning ? "Scanning..." : "Scan Library"}
        </button>
      </header>

      <LibrarySection title="Recently Added">
        {#if scanError}
          <div class="scan-error" role="alert">{scanError}</div>
        {/if}
        {#if playbackError}
          <div class="scan-error" role="alert">{playbackError}</div>
        {/if}
        <TrackList
          {tracks}
          {isScanning}
          selectedTrackId={currentTrack?.id}
          onTrackSelect={handleTrackSelect}
        />
      </LibrarySection>

      <LibrarySection title="Albums">
        {#if displayAlbums.length === 0}
          <div class="group-empty">
            <h3>No albums found</h3>
            <p>Album tags were not found in the scanned tracks.</p>
          </div>
        {:else}
          <div class="album-grid">
            {#each displayAlbums as album}
              <article class="album-card">
                <div class="album-art" style={`--item-color: ${album.color}`} aria-hidden="true">
                  <span></span>
                </div>
                <h3>{album.title}</h3>
                <p>{albumDetail(album)}</p>
              </article>
            {/each}
          </div>
        {/if}
      </LibrarySection>

      <LibrarySection title="Artists">
        {#if displayArtists.length === 0}
          <div class="group-empty">
            <h3>No artists found</h3>
            <p>Artist tags were not found in the scanned tracks.</p>
          </div>
        {:else}
          <div class="artist-grid">
            {#each displayArtists as artist}
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
        {/if}
      </LibrarySection>
    </main>
  </div>

  <NowPlayingBar
    track={currentTrack}
    {isPlaying}
    {positionSeconds}
    {durationSeconds}
    {volume}
    {canPlayPrevious}
    {canPlayNext}
    onTogglePlayback={handleTogglePlayback}
    onPrevious={handlePreviousTrack}
    onNext={handleNextTrack}
    onSeek={handleSeek}
    onVolumeChange={handleVolumeChange}
  />
</div>

<style>
  :global(*) {
    box-sizing: border-box;
  }

  :global(html) {
    height: 100%;
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
    height: 100vh;
    margin: 0;
    overflow: hidden;
    background: #0d0f13;
  }

  :global(button) {
    font-family: inherit;
  }

  .app-shell {
    display: grid;
    grid-template-rows: minmax(0, 1fr) auto;
    width: 100vw;
    height: 100vh;
    overflow: hidden;
    background:
      radial-gradient(circle at top right, rgba(47, 143, 131, 0.16), transparent 30rem),
      #0d0f13;
  }

  .workspace {
    display: flex;
    min-height: 0;
    overflow: hidden;
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

  .home-header button:disabled {
    border-color: #303844;
    background: #1a2028;
    color: #8d96a3;
  }

  .scan-status {
    max-width: 720px;
    margin: 14px 0 0;
    overflow-wrap: anywhere;
    color: #9aa4b1;
    font-size: 0.95rem;
    font-weight: 650;
  }

  .scan-error {
    border: 1px solid #6e3333;
    border-radius: 8px;
    background: #241719;
    color: #ffcbc8;
    font-size: 0.9rem;
    font-weight: 650;
    padding: 12px 14px;
  }

  .home :global(.library-section + .library-section) {
    margin-top: 30px;
  }

  .group-empty {
    display: grid;
    min-height: 110px;
    place-content: center;
    border: 1px dashed #303844;
    border-radius: 8px;
    background: rgba(18, 22, 28, 0.74);
    padding: 20px;
    text-align: center;
  }

  .group-empty h3 {
    margin: 0 0 6px;
  }

  .group-empty p {
    margin: 0;
    color: #929daa;
    font-size: 0.9rem;
    font-weight: 650;
  }

  .album-card p,
  .artist-card p {
    margin: 0;
    color: #8f9aa8;
    font-size: 0.9rem;
    font-weight: 620;
  }

  .album-card,
  .artist-card > div:last-child {
    min-width: 0;
  }

  .album-card h3,
  .album-card p,
  .artist-card h3,
  .artist-card p {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  h3 {
    margin: 0;
    color: #f4f7fb;
    font-size: 0.98rem;
    line-height: 1.25;
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
      height: 100dvh;
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
  }

  @media (max-width: 520px) {
    .album-grid,
    .artist-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
