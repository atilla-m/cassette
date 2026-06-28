<script lang="ts">
  import { chooseLibraryFolder, getLibraryCache, scanLibrary } from "$lib/api/library";
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
  import type { Album, Artist, PlaybackStatus, Track } from "$lib/types/library";
  import { localImageSource } from "$lib/utils/localImage";
  import { onMount } from "svelte";

  let tracks = $state<Track[]>([]);
  let isScanning = $state(false);
  let scanError = $state<string | null>(null);
  let playbackError = $state<string | null>(null);
  let scannedFolder = $state<string | null>(null);
  let scanCount = $state<number | null>(null);
  let hasLoadedCache = $state(false);
  let currentTrack = $state<Track | null>(null);
  let currentTrackIndex = $state<number | null>(null);
  let mainElement: HTMLElement | undefined = $state();
  let activeView = $state("Home");
  let selectedAlbumId = $state<string | null>(null);
  let selectedArtistName = $state<string | null>(null);
  let searchQuery = $state("");
  let isPlaying = $state(false);
  let positionSeconds = $state(0);
  let durationSeconds = $state<number | null>(null);
  let volume = $state(1);
  let displayAlbums = $derived(!hasLoadedCache ? mockAlbums : buildAlbums(tracks));
  let displayArtists = $derived(!hasLoadedCache ? mockArtists : buildArtists(tracks));
  let homeTracks = $derived(tracks.slice(0, 8));
  let homeAlbums = $derived(displayAlbums.slice(0, 4));
  let homeArtists = $derived(displayArtists.slice(0, 4));
  let normalizedSearchQuery = $derived(normalizeSearch(searchQuery));
  let searchTracks = $derived(
    normalizedSearchQuery ? tracks.filter((track) => trackMatchesSearch(track, normalizedSearchQuery)) : [],
  );
  let searchAlbums = $derived(
    normalizedSearchQuery ? displayAlbums.filter((album) => albumMatchesSearch(album, normalizedSearchQuery)) : [],
  );
  let searchArtists = $derived(
    normalizedSearchQuery ? displayArtists.filter((artist) => artistMatchesSearch(artist, normalizedSearchQuery)) : [],
  );
  let hasSearchResults = $derived(
    searchTracks.length > 0 || searchAlbums.length > 0 || searchArtists.length > 0,
  );
  let selectedAlbum = $derived(displayAlbums.find((album) => album.id === selectedAlbumId) ?? null);
  let selectedArtist = $derived(displayArtists.find((artist) => artist.name === selectedArtistName) ?? null);
  let selectedAlbumTracks = $derived(
    selectedAlbum ? tracks.filter((track) => albumIdForTrack(track) === selectedAlbum.id) : [],
  );
  let selectedArtistTracks = $derived(
    selectedArtist ? tracks.filter((track) => artistNameForTrack(track) === selectedArtist.name) : [],
  );
  let selectedArtistAlbums = $derived(
    selectedArtist ? displayAlbums.filter((album) => album.artist === selectedArtist.name) : [],
  );
  let canPlayPrevious = $derived(currentTrackIndex !== null && currentTrackIndex > 0);
  let canPlayNext = $derived(currentTrackIndex !== null && currentTrackIndex < tracks.length - 1);
  let hadSearchQuery = false;

  $effect(() => {
    const hasSearchQuery = normalizedSearchQuery.length > 0;

    if (hasSearchQuery && !hadSearchQuery) {
      mainElement?.scrollTo({ top: 0 });
    }

    hadSearchQuery = hasSearchQuery;
  });

  onMount(() => {
    void loadLibraryCache();

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
      if (event.key === "Escape" && normalizedSearchQuery) {
        event.preventDefault();
        clearSearch();
        return;
      }

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

  async function loadLibraryCache() {
    try {
      const cache = await getLibraryCache();
      tracks = cache.tracks;
      scannedFolder = cache.lastScannedFolder;
      scanCount = cache.tracks.length;
      hasLoadedCache = true;
    } catch (error) {
      scanError = error instanceof Error ? error.message : String(error);
      hasLoadedCache = true;
      scanCount = 0;
    }
  }

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
      selectedAlbumId = null;
      selectedArtistName = null;
      searchQuery = "";
      currentTrackIndex = null;
      hasLoadedCache = true;

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

  function handleNavigate(label: string) {
    activeView = label;
    selectedAlbumId = null;
    selectedArtistName = null;
    mainElement?.scrollTo({ top: 0 });
  }

  function handleAlbumSelect(album: Album) {
    searchQuery = "";
    activeView = "Albums";
    selectedAlbumId = album.id;
    selectedArtistName = null;
    mainElement?.scrollTo({ top: 0 });
  }

  function handleArtistSelect(artist: Artist) {
    searchQuery = "";
    activeView = "Artists";
    selectedArtistName = artist.name;
    selectedAlbumId = null;
    mainElement?.scrollTo({ top: 0 });
  }

  function handleBackToAlbums() {
    selectedAlbumId = null;
    mainElement?.scrollTo({ top: 0 });
  }

  function handleBackToArtists() {
    selectedArtistName = null;
    mainElement?.scrollTo({ top: 0 });
  }

  function clearSearch() {
    searchQuery = "";
  }

  function handleSearchKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      clearSearch();
    }
  }

  async function playTrackAtIndex(trackIndex: number) {
    if (trackIndex < 0 || trackIndex >= tracks.length) {
      return;
    }

    const track = tracks[trackIndex];
    playbackError = null;
    currentTrack = track;
    currentTrackIndex = trackIndex;
    durationSeconds = track.durationSeconds;
    positionSeconds = 0;
    isPlaying = false;

    try {
      const status = await playTrack(track.filePath);
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

  function albumIdForTrack(track: Track) {
    const title = track.album ?? "Unknown Album";
    const artist = track.albumArtist ?? track.artist ?? "Unknown Artist";

    return `${artist.toLowerCase()}\u0000${title.toLowerCase()}`;
  }

  function artistNameForTrack(track: Track) {
    return track.artist ?? track.albumArtist ?? "Unknown Artist";
  }

  function artistSongCount(artist: Artist) {
    return artist.detail;
  }

  function normalizeSearch(value: string) {
    return value.trim().normalize("NFKC").toLocaleLowerCase();
  }

  function searchableValue(value: string | null | undefined) {
    return normalizeSearch(value ?? "");
  }

  function matchesSearch(query: string, values: Array<string | null | undefined>) {
    return values.some((value) => searchableValue(value).includes(query));
  }

  function trackMatchesSearch(track: Track, query: string) {
    return matchesSearch(query, [
      track.title,
      track.artist,
      track.album,
      track.albumArtist,
      track.fileName,
    ]);
  }

  function albumMatchesSearch(album: Album, query: string) {
    return matchesSearch(query, [album.title, album.artist]);
  }

  function artistMatchesSearch(artist: Artist, query: string) {
    return matchesSearch(query, [artist.name]);
  }

  function viewTitle() {
    if (normalizedSearchQuery) {
      return "Search Results";
    }

    if (activeView === "Home") {
      return "Your music, on this machine.";
    }

    return activeView;
  }

  function viewEyebrow() {
    if (normalizedSearchQuery) {
      return "Search";
    }

    return activeView;
  }

  function viewStatus() {
    if (normalizedSearchQuery) {
      const total = searchTracks.length + searchAlbums.length + searchArtists.length;
      return `${total} ${total === 1 ? "match" : "matches"} for "${searchQuery.trim()}"`;
    }

    if (isScanning) {
      return "Scanning your local music files...";
    }

    if (!hasLoadedCache) {
      return "Loading cached library...";
    }

    if (activeView === "Settings") {
      return scannedFolder
        ? `Library folder: ${scannedFolder}`
        : "No library folder is cached yet.";
    }

    if (scanCount !== null && scannedFolder) {
      return `Found ${scanCount} ${scanCount === 1 ? "track" : "tracks"} in ${scannedFolder}`;
    }

    return "No cached tracks yet. Pick a folder to scan your local music files.";
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

<svelte:head>
  <title>Cassette</title>
</svelte:head>

<div class="app-shell">
  <div class="workspace">
    <Sidebar items={navItems} active={activeView} onNavigate={handleNavigate} />

    <main class="home" bind:this={mainElement}>
      <header class="home-header">
        <div>
          <p class="eyebrow">{viewEyebrow()}</p>
          <h2>{viewTitle()}</h2>
          <p class="scan-status">{viewStatus()}</p>
        </div>
        <button type="button" disabled={isScanning} onclick={handleScanLibrary}>
          {isScanning ? "Scanning..." : "Scan Library"}
        </button>
      </header>

      <div class="search-bar">
        <input
          type="search"
          bind:value={searchQuery}
          placeholder="Search songs, albums, artists..."
          aria-label="Search songs, albums, artists"
          onkeydown={handleSearchKeydown}
        />
        {#if searchQuery}
          <button type="button" aria-label="Clear search" onclick={clearSearch}>Clear</button>
        {/if}
      </div>

      {#if scanError}
        <div class="scan-error" role="alert">{scanError}</div>
      {/if}
      {#if playbackError}
        <div class="scan-error" role="alert">{playbackError}</div>
      {/if}

      {#if normalizedSearchQuery}
        {#if hasSearchResults}
          <LibrarySection title="Songs" viewAllLabel={`${searchTracks.length} ${searchTracks.length === 1 ? "match" : "matches"}`}>
            {#if searchTracks.length === 0}
              <div class="group-empty">
                <h3>No songs matched</h3>
                <p>Try a track title, artist, album, or file name.</p>
              </div>
            {:else}
              <TrackList
                tracks={searchTracks}
                isScanning={false}
                selectedTrackId={currentTrack?.id}
                onTrackSelect={handleTrackSelect}
              />
            {/if}
          </LibrarySection>

          <LibrarySection title="Albums" viewAllLabel={`${searchAlbums.length} ${searchAlbums.length === 1 ? "match" : "matches"}`}>
            {#if searchAlbums.length === 0}
              <div class="group-empty">
                <h3>No albums matched</h3>
                <p>Try an album title or artist name.</p>
              </div>
            {:else}
              <div class="album-grid">
                {#each searchAlbums as album}
                  <button class="album-card" type="button" onclick={() => handleAlbumSelect(album)}>
                    <div class="album-art" style={`--item-color: ${album.color}`} aria-hidden="true">
                      {#if album.coverArtPath}
                        <img
                          src={localImageSource(album.coverArtPath) ?? ""}
                          alt=""
                          loading="lazy"
                          onload={showLoadedImage}
                          onerror={hideBrokenImage}
                        />
                      {/if}
                      <span></span>
                    </div>
                    <h3>{album.title}</h3>
                    <p>{albumDetail(album)}</p>
                  </button>
                {/each}
              </div>
            {/if}
          </LibrarySection>

          <LibrarySection title="Artists" viewAllLabel={`${searchArtists.length} ${searchArtists.length === 1 ? "match" : "matches"}`}>
            {#if searchArtists.length === 0}
              <div class="group-empty">
                <h3>No artists matched</h3>
                <p>Try a different artist name.</p>
              </div>
            {:else}
              <div class="artist-grid">
                {#each searchArtists as artist}
                  <button class="artist-card" type="button" onclick={() => handleArtistSelect(artist)}>
                    <div class="artist-avatar" style={`--item-color: ${artist.color}`} aria-hidden="true">
                      {artist.name.slice(0, 1)}
                    </div>
                    <div>
                      <h3>{artist.name}</h3>
                      <p>{artistSongCount(artist)}</p>
                    </div>
                  </button>
                {/each}
              </div>
            {/if}
          </LibrarySection>
        {:else}
          <div class="group-empty">
            <h3>No matches found</h3>
            <p>Search looks at song titles, artists, albums, album artists, and file names.</p>
          </div>
        {/if}
      {:else if activeView === "Home"}
        <LibrarySection title="Recently Added">
          <TrackList
            tracks={homeTracks}
            {isScanning}
            selectedTrackId={currentTrack?.id}
            onTrackSelect={handleTrackSelect}
          />
        </LibrarySection>

        <LibrarySection title="Albums" viewAllLabel="Preview">
          {#if homeAlbums.length === 0}
            <div class="group-empty">
              <h3>No albums found</h3>
              <p>Album tags were not found in the scanned tracks.</p>
            </div>
          {:else}
            <div class="album-grid">
              {#each homeAlbums as album}
                <button class="album-card" type="button" onclick={() => handleAlbumSelect(album)}>
                  <div class="album-art" style={`--item-color: ${album.color}`} aria-hidden="true">
                    {#if album.coverArtPath}
                      <img
                        src={localImageSource(album.coverArtPath) ?? ""}
                        alt=""
                        loading="lazy"
                        onload={showLoadedImage}
                        onerror={hideBrokenImage}
                      />
                    {/if}
                    <span></span>
                  </div>
                  <h3>{album.title}</h3>
                  <p>{albumDetail(album)}</p>
                </button>
              {/each}
            </div>
          {/if}
        </LibrarySection>

        <LibrarySection title="Artists" viewAllLabel="Preview">
          {#if homeArtists.length === 0}
            <div class="group-empty">
              <h3>No artists found</h3>
              <p>Artist tags were not found in the scanned tracks.</p>
            </div>
          {:else}
            <div class="artist-grid">
              {#each homeArtists as artist}
                <button class="artist-card" type="button" onclick={() => handleArtistSelect(artist)}>
                  <div class="artist-avatar" style={`--item-color: ${artist.color}`} aria-hidden="true">
                    {artist.name.slice(0, 1)}
                  </div>
                  <div>
                    <h3>{artist.name}</h3>
                    <p>{artistSongCount(artist)}</p>
                  </div>
                </button>
              {/each}
            </div>
          {/if}
        </LibrarySection>
      {:else if activeView === "Albums"}
        {#if selectedAlbum}
          <section class="detail-view" aria-labelledby="album-detail-title">
            <button class="back-button" type="button" onclick={handleBackToAlbums}>Back to Albums</button>
            <div class="album-detail-header">
              <div class="album-art detail-cover" style={`--item-color: ${selectedAlbum.color}`} aria-hidden="true">
                {#if selectedAlbum.coverArtPath}
                  <img
                    src={localImageSource(selectedAlbum.coverArtPath) ?? ""}
                    alt=""
                    onload={showLoadedImage}
                    onerror={hideBrokenImage}
                  />
                {/if}
                <span></span>
              </div>
              <div class="detail-copy">
                <p class="eyebrow">Album</p>
                <h3 id="album-detail-title">{selectedAlbum.title}</h3>
                <p>{albumDetail(selectedAlbum)}</p>
              </div>
            </div>

            <LibrarySection title="Album Songs" viewAllLabel={`${selectedAlbumTracks.length} total`}>
              <TrackList
                tracks={selectedAlbumTracks}
                isScanning={false}
                selectedTrackId={currentTrack?.id}
                onTrackSelect={handleTrackSelect}
              />
            </LibrarySection>
          </section>
        {:else}
          <LibrarySection title="All Albums" viewAllLabel={`${displayAlbums.length} total`}>
            {#if displayAlbums.length === 0}
              <div class="group-empty">
                <h3>No albums found</h3>
                <p>Scan a music folder to build your local album library.</p>
              </div>
            {:else}
              <div class="album-grid">
                {#each displayAlbums as album}
                  <button class="album-card" type="button" onclick={() => handleAlbumSelect(album)}>
                    <div class="album-art" style={`--item-color: ${album.color}`} aria-hidden="true">
                      {#if album.coverArtPath}
                        <img
                          src={localImageSource(album.coverArtPath) ?? ""}
                          alt=""
                          loading="lazy"
                          onload={showLoadedImage}
                          onerror={hideBrokenImage}
                        />
                      {/if}
                      <span></span>
                    </div>
                    <h3>{album.title}</h3>
                    <p>{albumDetail(album)}</p>
                  </button>
                {/each}
              </div>
            {/if}
          </LibrarySection>
        {/if}
      {:else if activeView === "Artists"}
        {#if selectedArtist}
          <section class="detail-view" aria-labelledby="artist-detail-title">
            <button class="back-button" type="button" onclick={handleBackToArtists}>Back to Artists</button>
            <div class="artist-detail-header">
              <div class="artist-avatar detail-avatar" style={`--item-color: ${selectedArtist.color}`} aria-hidden="true">
                {selectedArtist.name.slice(0, 1)}
              </div>
              <div class="detail-copy">
                <p class="eyebrow">Artist</p>
                <h3 id="artist-detail-title">{selectedArtist.name}</h3>
                <p>{artistSongCount(selectedArtist)} · {selectedArtistAlbums.length} {selectedArtistAlbums.length === 1 ? "album" : "albums"}</p>
              </div>
            </div>

            <LibrarySection title="Albums" viewAllLabel={`${selectedArtistAlbums.length} total`}>
              {#if selectedArtistAlbums.length === 0}
                <div class="group-empty">
                  <h3>No albums found</h3>
                  <p>No album tags were found for this artist.</p>
                </div>
              {:else}
                <div class="album-grid">
                  {#each selectedArtistAlbums as album}
                    <button class="album-card" type="button" onclick={() => handleAlbumSelect(album)}>
                      <div class="album-art" style={`--item-color: ${album.color}`} aria-hidden="true">
                        {#if album.coverArtPath}
                          <img
                            src={localImageSource(album.coverArtPath) ?? ""}
                            alt=""
                            loading="lazy"
                            onload={showLoadedImage}
                            onerror={hideBrokenImage}
                          />
                        {/if}
                        <span></span>
                      </div>
                      <h3>{album.title}</h3>
                      <p>{albumDetail(album)}</p>
                    </button>
                  {/each}
                </div>
              {/if}
            </LibrarySection>

            <LibrarySection title="Songs" viewAllLabel={`${selectedArtistTracks.length} total`}>
              <TrackList
                tracks={selectedArtistTracks}
                isScanning={false}
                selectedTrackId={currentTrack?.id}
                onTrackSelect={handleTrackSelect}
              />
            </LibrarySection>
          </section>
        {:else}
          <LibrarySection title="All Artists" viewAllLabel={`${displayArtists.length} total`}>
            {#if displayArtists.length === 0}
              <div class="group-empty">
                <h3>No artists found</h3>
                <p>Scan a music folder to build your local artist library.</p>
              </div>
            {:else}
              <div class="artist-grid">
                {#each displayArtists as artist}
                  <button class="artist-card" type="button" onclick={() => handleArtistSelect(artist)}>
                    <div class="artist-avatar" style={`--item-color: ${artist.color}`} aria-hidden="true">
                      {artist.name.slice(0, 1)}
                    </div>
                    <div>
                      <h3>{artist.name}</h3>
                      <p>{artistSongCount(artist)}</p>
                    </div>
                  </button>
                {/each}
              </div>
            {/if}
          </LibrarySection>
        {/if}
      {:else if activeView === "Songs"}
        <LibrarySection title="All Songs" viewAllLabel={`${tracks.length} total`}>
          <TrackList
            {tracks}
            {isScanning}
            selectedTrackId={currentTrack?.id}
            onTrackSelect={handleTrackSelect}
          />
        </LibrarySection>
      {:else if activeView === "Playlists"}
        <section class="placeholder-panel" aria-labelledby="playlists-title">
          <p class="eyebrow">Coming Later</p>
          <h3 id="playlists-title">Playlists</h3>
          <p>Playlist management is not wired up yet. Your scanned library and playback controls are ready here when that feature lands.</p>
        </section>
      {:else if activeView === "Settings"}
        <section class="placeholder-panel" aria-labelledby="settings-title">
          <p class="eyebrow">Library</p>
          <h3 id="settings-title">Settings</h3>
          <div class="settings-grid">
            <div>
              <span>Library folder</span>
              <p>{scannedFolder ?? "No folder scanned"}</p>
            </div>
            <div>
              <span>Tracks</span>
              <p>{scanCount ?? tracks.length}</p>
            </div>
          </div>
          <p>More playback, library, and appearance settings will live here later.</p>
        </section>
      {/if}
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

  .search-bar {
    display: flex;
    align-items: center;
    gap: 10px;
    max-width: 720px;
    margin: -12px 0 24px;
  }

  .search-bar input {
    width: 100%;
    min-width: 0;
    min-height: 42px;
    border: 1px solid #303844;
    border-radius: 8px;
    background: #12161c;
    color: #f4f7fb;
    font: inherit;
    font-weight: 650;
    outline: none;
    padding: 0 14px;
  }

  .search-bar input::placeholder {
    color: #727d8a;
  }

  .search-bar input:focus {
    border-color: #2f8f83;
    box-shadow: 0 0 0 2px rgba(47, 143, 131, 0.18);
  }

  .search-bar button {
    min-height: 42px;
    border: 1px solid #303844;
    border-radius: 8px;
    background: #161a20;
    color: #d5dce5;
    cursor: default;
    font: inherit;
    font-size: 0.86rem;
    font-weight: 800;
    padding: 0 13px;
  }

  .search-bar button:hover,
  .search-bar button:focus-visible {
    border-color: #35544f;
    background: #1b2027;
    outline: none;
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

  .scan-error + :global(.library-section),
  .scan-error + .placeholder-panel {
    margin-top: 16px;
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
    width: 100%;
    border: 1px solid #242b35;
    border-radius: 8px;
    background: #151a21;
    color: inherit;
    cursor: default;
    font: inherit;
    text-align: left;
  }

  .album-card:hover,
  .album-card:focus-visible,
  .artist-card:hover,
  .artist-card:focus-visible {
    border-color: #35544f;
    background: #1b2027;
    outline: none;
  }

  .album-card {
    padding: 14px;
  }

  .album-art {
    position: relative;
    display: grid;
    aspect-ratio: 1;
    overflow: hidden;
    place-items: center;
    margin-bottom: 12px;
    border-radius: 8px;
    background:
      linear-gradient(135deg, rgba(255, 255, 255, 0.18), transparent 42%),
      var(--item-color);
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.14);
  }

  .album-art img {
    position: absolute;
    inset: 0;
    z-index: 1;
    width: 100%;
    height: 100%;
    object-fit: cover;
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

  .detail-view {
    display: grid;
    gap: 22px;
  }

  .back-button {
    justify-self: start;
    min-height: 36px;
    border: 1px solid #303844;
    border-radius: 8px;
    background: #161a20;
    color: #d5dce5;
    cursor: default;
    font: inherit;
    font-size: 0.88rem;
    font-weight: 800;
    padding: 0 13px;
  }

  .back-button:hover,
  .back-button:focus-visible {
    border-color: #35544f;
    background: #1b2027;
    outline: none;
  }

  .album-detail-header,
  .artist-detail-header {
    display: flex;
    align-items: center;
    gap: 22px;
    min-width: 0;
    border: 1px solid #242b35;
    border-radius: 8px;
    background: #151a21;
    padding: 18px;
  }

  .detail-cover {
    width: min(28vw, 190px);
    min-width: 136px;
    margin: 0;
  }

  .detail-avatar {
    width: 104px;
    height: 104px;
    font-size: 2.1rem;
  }

  .detail-copy {
    min-width: 0;
  }

  .detail-copy h3 {
    margin: 0 0 8px;
    overflow: hidden;
    color: #f7f9fc;
    font-size: clamp(1.7rem, 4vw, 3.2rem);
    line-height: 1.03;
    text-overflow: ellipsis;
  }

  .detail-copy p:not(.eyebrow) {
    margin: 0;
    color: #9aa4b1;
    font-weight: 700;
  }

  .detail-view :global(.library-section + .library-section) {
    margin-top: 8px;
  }

  .placeholder-panel {
    max-width: 760px;
    border: 1px solid #242b35;
    border-radius: 8px;
    background: #151a21;
    padding: 22px;
  }

  .placeholder-panel h3 {
    margin: 0 0 8px;
    font-size: 1.3rem;
  }

  .placeholder-panel p {
    max-width: 620px;
    margin: 0;
    color: #98a3b0;
    font-weight: 620;
  }

  .placeholder-panel .eyebrow {
    margin-bottom: 8px;
  }

  .settings-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
    margin: 16px 0;
  }

  .settings-grid div {
    min-width: 0;
    border: 1px solid #2a313c;
    border-radius: 8px;
    background: #12161c;
    padding: 14px;
  }

  .settings-grid span {
    display: block;
    margin-bottom: 5px;
    color: #8f9aa8;
    font-size: 0.78rem;
    font-weight: 800;
    text-transform: uppercase;
  }

  .settings-grid p {
    overflow: hidden;
    margin: 0;
    color: #f4f7fb;
    text-overflow: ellipsis;
    white-space: nowrap;
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

    .album-detail-header,
    .artist-detail-header {
      align-items: flex-start;
      flex-direction: column;
    }

    .detail-cover {
      width: min(100%, 220px);
    }
  }

  @media (max-width: 520px) {
    .album-grid,
    .artist-grid,
    .settings-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
