<script lang="ts">
  import {
    chooseLibraryFolder,
    getLibraryCache,
    scanLibrary,
    setAlbumGenres,
    setArtistGenres,
    toggleTrackFavorite,
  } from "$lib/api/library";
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
  import { buildAlbums, buildArtists, buildGenres } from "$lib/data/libraryViews";
  import { albums as mockAlbums, artists as mockArtists, genres as mockGenres, navItems } from "$lib/data/mockLibrary";
  import type { Album, Artist, Genre, PlaybackStatus, Track } from "$lib/types/library";
  import { localImageSource } from "$lib/utils/localImage";
  import { onMount } from "svelte";

  type SongSortKey = "title" | "artist" | "album" | "duration";
  type AlbumSortKey = "title" | "artist" | "year" | "trackCount";
  type ArtistSortKey = "name" | "songCount" | "albumCount";
  type GenreSortKey = "name" | "songCount" | "artistCount" | "albumCount";
  type SortDirection = "asc" | "desc";
  type RepeatMode = "off" | "all" | "one";
  type QueuePanelEntry = {
    track: Track;
    queueIndex: number;
    offset: number;
  };
  type MixCategory = "genre" | "artist" | "album";

  const mixFormatOptions = ["All", "FLAC", "MP3", "OGG", "OPUS", "WAV", "M4A"];

  let tracks = $state<Track[]>([]);
  let isScanning = $state(false);
  let scanError = $state<string | null>(null);
  let playbackError = $state<string | null>(null);
  let genreEditError = $state<string | null>(null);
  let genreEditMessage = $state<string | null>(null);
  let scannedFolder = $state<string | null>(null);
  let scanCount = $state<number | null>(null);
  let hasLoadedCache = $state(false);
  let currentTrack = $state<Track | null>(null);
  let currentTrackIndex = $state<number | null>(null);
  let playbackQueue = $state<Track[]>([]);
  let currentQueueIndex = $state<number | null>(null);
  let isQueueOpen = $state(false);
  let isShuffleEnabled = $state(false);
  let shuffledQueueOrder = $state<number[]>([]);
  let repeatMode = $state<RepeatMode>("off");
  let mainElement: HTMLElement | undefined = $state();
  let activeView = $state("Home");
  let selectedAlbumId = $state<string | null>(null);
  let selectedArtistName = $state<string | null>(null);
  let selectedGenreName = $state<string | null>(null);
  let isLikedSongsOpen = $state(false);
  let isMixBuilderOpen = $state(false);
  let searchQuery = $state("");
  let songSort = $state<SongSortKey>("title");
  let songSortDirection = $state<SortDirection>("asc");
  let songFormatFilter = $state("All");
  let albumSort = $state<AlbumSortKey>("title");
  let albumSortDirection = $state<SortDirection>("asc");
  let artistSort = $state<ArtistSortKey>("name");
  let artistSortDirection = $state<SortDirection>("asc");
  let genreSort = $state<GenreSortKey>("name");
  let genreSortDirection = $state<SortDirection>("asc");
  let albumGenreDraft = $state("");
  let artistGenreDraft = $state("");
  let isSavingGenreAssignment = $state(false);
  let mixSelectedGenres = $state<string[]>([]);
  let mixSelectedArtists = $state<string[]>([]);
  let mixSelectedAlbums = $state<string[]>([]);
  let mixLikedOnly = $state(false);
  let mixFormatFilter = $state("All");
  let mixMessage = $state<string | null>(null);
  let isPlaying = $state(false);
  let hasCurrentTrackEnded = $state(false);
  let positionSeconds = $state(0);
  let durationSeconds = $state<number | null>(null);
  let volume = $state(1);
  let displayAlbums = $derived(!hasLoadedCache ? mockAlbums : buildAlbums(tracks));
  let displayArtists = $derived(!hasLoadedCache ? mockArtists : buildArtists(tracks));
  let displayGenres = $derived(!hasLoadedCache ? mockGenres : buildGenres(tracks));
  let favoriteTracks = $derived(tracks.filter((track) => track.isFavorite));
  let availableFormats = $derived(availableTrackFormats(tracks));
  let sortedSongTracks = $derived(sortTracks(tracks, songSort, songSortDirection));
  let filteredSongTracks = $derived(filterTracksByFormat(sortedSongTracks, songFormatFilter));
  let sortedAlbums = $derived(sortAlbums(displayAlbums, albumSort, albumSortDirection));
  let sortedArtists = $derived(sortArtists(displayArtists, artistSort, artistSortDirection));
  let sortedGenres = $derived(sortGenres(displayGenres, genreSort, genreSortDirection));
  let homeTracks = $derived(tracks.slice(0, 8));
  let homeAlbums = $derived(sortedAlbums.slice(0, 4));
  let homeArtists = $derived(sortedArtists.slice(0, 4));
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
  let searchGenres = $derived(
    normalizedSearchQuery ? displayGenres.filter((genre) => genreMatchesSearch(genre, normalizedSearchQuery)) : [],
  );
  let hasSearchResults = $derived(
    searchTracks.length > 0 || searchAlbums.length > 0 || searchArtists.length > 0 || searchGenres.length > 0,
  );
  let selectedAlbum = $derived(displayAlbums.find((album) => album.id === selectedAlbumId) ?? null);
  let selectedArtist = $derived(displayArtists.find((artist) => artist.name === selectedArtistName) ?? null);
  let selectedGenre = $derived(displayGenres.find((genre) => genre.name === selectedGenreName) ?? null);
  let selectedAlbumTracks = $derived(
    selectedAlbum ? tracks.filter((track) => albumIdForTrack(track) === selectedAlbum.id) : [],
  );
  let selectedArtistTracks = $derived(
    selectedArtist ? tracks.filter((track) => artistNameForTrack(track) === selectedArtist.name) : [],
  );
  let selectedArtistAlbums = $derived(
    selectedArtist ? sortedAlbums.filter((album) => album.artist === selectedArtist.name) : [],
  );
  let selectedGenreTracks = $derived(
    selectedGenre ? tracks.filter((track) => trackGenres(track).includes(selectedGenre.name)) : [],
  );
  let selectedGenreAlbums = $derived(selectedGenre ? albumsForTracks(selectedGenreTracks, sortedAlbums) : []);
  let selectedGenreArtists = $derived(selectedGenre ? buildArtists(selectedGenreTracks) : []);
  let selectedAlbumGenreText = $derived(genreDisplayForTracks(selectedAlbumTracks));
  let selectedArtistGenreText = $derived(genreDisplayForTracks(selectedArtistTracks));
  let mixSelectedGenreSet = $derived(new Set(mixSelectedGenres));
  let mixSelectedArtistSet = $derived(new Set(mixSelectedArtists));
  let mixSelectedAlbumSet = $derived(new Set(mixSelectedAlbums));
  let mixHasSelection = $derived(
    mixSelectedGenres.length > 0 || mixSelectedArtists.length > 0 || mixSelectedAlbums.length > 0,
  );
  let mixTracks = $derived(buildMixTracks(tracks));
  let mixSelectedSourceCount = $derived(mixSelectedGenres.length + mixSelectedArtists.length + mixSelectedAlbums.length);
  let playbackOrder = $derived(
    isShuffleEnabled
      ? normalizedQueueOrder(shuffledQueueOrder, playbackQueue.length)
      : playbackQueue.map((_, index) => index),
  );
  let currentOrderIndex = $derived(queueOrderIndex(playbackOrder, currentQueueIndex));
  let queuePanelEntries = $derived(buildQueuePanelEntries(playbackQueue, playbackOrder, currentQueueIndex));
  let canPlayPrevious = $derived(
    currentQueueIndex !== null
      && playbackQueue.length > 1
      && (currentOrderIndex > 0 || repeatMode === "all"),
  );
  let canPlayNext = $derived(
    currentQueueIndex !== null
      && playbackQueue.length > 1
      && (currentOrderIndex < playbackOrder.length - 1 || repeatMode === "all"),
  );
  let hadSearchQuery = false;

  $effect(() => {
    const hasSearchQuery = normalizedSearchQuery.length > 0;

    if (hasSearchQuery && !hadSearchQuery) {
      mainElement?.scrollTo({ top: 0 });
    }

    hadSearchQuery = hasSearchQuery;
  });

  $effect(() => {
    if (!availableFormats.includes(songFormatFilter)) {
      songFormatFilter = "All";
    }
  });

  onMount(() => {
    void loadLibraryCache();

    const statusIntervalId = window.setInterval(async () => {
      if (!currentTrack || !isPlaying) {
        return;
      }

      try {
        const status = await getPlaybackStatus();
        applyPlaybackStatus(status);
        await handlePlaybackStatusUpdate(status);
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
      selectedGenreName = null;
      clearGenreEditState();
      isLikedSongsOpen = false;
      isMixBuilderOpen = false;
      clearMixSelection();
      searchQuery = "";
      currentTrackIndex = null;
      hasCurrentTrackEnded = false;
      playbackQueue = [];
      currentQueueIndex = null;
      shuffledQueueOrder = [];
      isQueueOpen = false;
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

  async function handleTrackSelect(track: Track, queue: Track[] = tracks) {
    let nextQueue = queue.length > 0 ? [...queue] : [track];
    let queueIndex = nextQueue.findIndex((candidate) => candidate.id === track.id);

    if (queueIndex === -1) {
      nextQueue = [track, ...nextQueue];
      queueIndex = 0;
    }

    playbackQueue = nextQueue;
    currentQueueIndex = queueIndex;
    shuffledQueueOrder = isShuffleEnabled ? buildShuffledQueueOrder(nextQueue.length, queueIndex) : [];
    await playQueuedTrackAtIndex(queueIndex);
  }

  function handleNavigate(label: string) {
    activeView = label;
    selectedAlbumId = null;
    selectedArtistName = null;
    selectedGenreName = null;
    clearGenreEditState();
    isLikedSongsOpen = false;
    isMixBuilderOpen = false;
    mainElement?.scrollTo({ top: 0 });
  }

  function handleAlbumSelect(album: Album) {
    searchQuery = "";
    activeView = "Albums";
    selectedAlbumId = album.id;
    selectedArtistName = null;
    selectedGenreName = null;
    clearGenreEditState();
    albumGenreDraft = genreDraftForTracks(tracks.filter((track) => albumIdForTrack(track) === album.id));
    isLikedSongsOpen = false;
    isMixBuilderOpen = false;
    mainElement?.scrollTo({ top: 0 });
  }

  function handleArtistSelect(artist: Artist) {
    searchQuery = "";
    activeView = "Artists";
    selectedArtistName = artist.name;
    selectedAlbumId = null;
    selectedGenreName = null;
    clearGenreEditState();
    artistGenreDraft = genreDraftForTracks(tracks.filter((track) => artistNameForTrack(track) === artist.name));
    isLikedSongsOpen = false;
    isMixBuilderOpen = false;
    mainElement?.scrollTo({ top: 0 });
  }

  function handleGenreSelect(genre: Genre) {
    searchQuery = "";
    activeView = "Genres";
    selectedGenreName = genre.name;
    selectedAlbumId = null;
    selectedArtistName = null;
    clearGenreEditState();
    isLikedSongsOpen = false;
    isMixBuilderOpen = false;
    mainElement?.scrollTo({ top: 0 });
  }

  function handleLikedSongsSelect() {
    activeView = "Playlists";
    selectedAlbumId = null;
    selectedArtistName = null;
    selectedGenreName = null;
    clearGenreEditState();
    isLikedSongsOpen = true;
    isMixBuilderOpen = false;
    searchQuery = "";
    mainElement?.scrollTo({ top: 0 });
  }

  function handleMixBuilderSelect() {
    activeView = "Playlists";
    selectedAlbumId = null;
    selectedArtistName = null;
    selectedGenreName = null;
    clearGenreEditState();
    isLikedSongsOpen = false;
    isMixBuilderOpen = true;
    mixMessage = null;
    searchQuery = "";
    mainElement?.scrollTo({ top: 0 });
  }

  function handleBackToAlbums() {
    selectedAlbumId = null;
    clearGenreEditState();
    mainElement?.scrollTo({ top: 0 });
  }

  function handleBackToArtists() {
    selectedArtistName = null;
    clearGenreEditState();
    mainElement?.scrollTo({ top: 0 });
  }

  function handleBackToGenres() {
    selectedGenreName = null;
    mainElement?.scrollTo({ top: 0 });
  }

  function handleBackToPlaylists() {
    isLikedSongsOpen = false;
    isMixBuilderOpen = false;
    mixMessage = null;
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

  async function playQueuedTrackAtIndex(queueIndex: number) {
    if (queueIndex < 0 || queueIndex >= playbackQueue.length) {
      return;
    }

    const track = playbackQueue[queueIndex];
    const trackIndex = tracks.findIndex((candidate) => candidate.id === track.id);
    playbackError = null;
    currentTrack = track;
    currentTrackIndex = trackIndex >= 0 ? trackIndex : null;
    currentQueueIndex = queueIndex;
    durationSeconds = track.durationSeconds;
    positionSeconds = 0;
    isPlaying = false;
    hasCurrentTrackEnded = false;

    try {
      const status = await playTrack(track.filePath);
      applyPlaybackStatus(status);
      durationSeconds = status.durationSeconds ?? track.durationSeconds;
    } catch (error) {
      playbackError = error instanceof Error ? error.message : String(error);
    }
  }

  async function handlePlaybackStatusUpdate(status: PlaybackStatus) {
    if (!status.hasEnded || !currentTrack || status.filePath !== currentTrack.filePath) {
      return;
    }

    hasCurrentTrackEnded = true;
    const nextQueueIndex = getNextQueueIndex(true);

    if (nextQueueIndex === null) {
      isPlaying = false;
      positionSeconds = durationSeconds ?? positionSeconds;
      return;
    }

    await playQueuedTrackAtIndex(nextQueueIndex);
  }

  async function handleToggleFavorite(track: Track) {
    playbackError = null;

    try {
      const isFavorite = await toggleTrackFavorite(track.id);
      applyTrackFavorite(track.id, isFavorite);
    } catch (error) {
      playbackError = error instanceof Error ? error.message : String(error);
    }
  }

  function applyTrackFavorite(trackId: string, isFavorite: boolean) {
    tracks = tracks.map((track) => (track.id === trackId ? { ...track, isFavorite } : track));
    playbackQueue = playbackQueue.map((track) => (track.id === trackId ? { ...track, isFavorite } : track));

    if (currentTrack?.id === trackId) {
      currentTrack = { ...currentTrack, isFavorite };
    }
  }

  async function handleSaveAlbumGenres() {
    const album = selectedAlbum;

    if (!album) {
      return;
    }

    genreEditError = null;
    genreEditMessage = null;
    isSavingGenreAssignment = true;

    try {
      const submittedGenres = parseGenreDraft(albumGenreDraft);
      const updatedTracks = await setAlbumGenres(album.id, submittedGenres);
      applyUpdatedTracks(updatedTracks);
      albumGenreDraft = genreDraftForTracks(updatedTracks.filter((track) => albumIdForTrack(track) === album.id));
      genreEditMessage = submittedGenres.length > 0 ? "Album genres saved." : "Album genre assignment cleared.";
    } catch (error) {
      genreEditError = error instanceof Error ? error.message : String(error);
    } finally {
      isSavingGenreAssignment = false;
    }
  }

  async function handleSaveArtistGenres() {
    const artist = selectedArtist;

    if (!artist) {
      return;
    }

    genreEditError = null;
    genreEditMessage = null;
    isSavingGenreAssignment = true;

    try {
      const submittedGenres = parseGenreDraft(artistGenreDraft);
      const updatedTracks = await setArtistGenres(artist.name, submittedGenres);
      applyUpdatedTracks(updatedTracks);
      artistGenreDraft = genreDraftForTracks(updatedTracks.filter((track) => artistNameForTrack(track) === artist.name));
      genreEditMessage = submittedGenres.length > 0 ? "Artist genres saved." : "Artist genre assignment cleared.";
    } catch (error) {
      genreEditError = error instanceof Error ? error.message : String(error);
    } finally {
      isSavingGenreAssignment = false;
    }
  }

  function applyUpdatedTracks(updatedTracks: Track[]) {
    const tracksById = new Map(updatedTracks.map((track) => [track.id, track]));

    tracks = updatedTracks;
    playbackQueue = playbackQueue.map((track) => tracksById.get(track.id) ?? track);

    if (currentTrack) {
      currentTrack = tracksById.get(currentTrack.id) ?? currentTrack;
      currentTrackIndex = tracks.findIndex((track) => track.id === currentTrack?.id);
    }
  }

  function clearGenreEditState() {
    genreEditError = null;
    genreEditMessage = null;
    isSavingGenreAssignment = false;
    albumGenreDraft = "";
    artistGenreDraft = "";
  }

  async function handlePreviousTrack() {
    const previousQueueIndex = getPreviousQueueIndex();

    if (!canPlayPrevious || previousQueueIndex === null) {
      return;
    }

    await playQueuedTrackAtIndex(previousQueueIndex);
  }

  async function handleNextTrack() {
    const nextQueueIndex = getNextQueueIndex(false);

    if (!canPlayNext || nextQueueIndex === null) {
      return;
    }

    await playQueuedTrackAtIndex(nextQueueIndex);
  }

  async function handleShuffleGenre() {
    if (selectedGenreTracks.length === 0) {
      return;
    }

    const startIndex = Math.floor(Math.random() * selectedGenreTracks.length);
    playbackQueue = [...selectedGenreTracks];
    currentQueueIndex = startIndex;
    isShuffleEnabled = true;
    shuffledQueueOrder = buildShuffledQueueOrder(selectedGenreTracks.length, startIndex);
    await playQueuedTrackAtIndex(startIndex);
  }

  async function handleStartMix() {
    mixMessage = null;

    if (!mixHasSelection) {
      mixMessage = "Select at least one genre, artist, or album to start a mix.";
      return;
    }

    if (mixTracks.length === 0) {
      mixMessage = "No tracks match this mix.";
      return;
    }

    const shuffledMix = shuffleTracks(mixTracks);
    playbackQueue = shuffledMix;
    currentQueueIndex = 0;
    isShuffleEnabled = true;
    shuffledQueueOrder = shuffledMix.map((_, index) => index);
    isQueueOpen = true;
    await playQueuedTrackAtIndex(0);
  }

  function handleToggleMixItem(category: MixCategory, value: string) {
    mixMessage = null;

    if (category === "genre") {
      mixSelectedGenres = toggleSelection(mixSelectedGenres, value);
    } else if (category === "artist") {
      mixSelectedArtists = toggleSelection(mixSelectedArtists, value);
    } else {
      mixSelectedAlbums = toggleSelection(mixSelectedAlbums, value);
    }
  }

  function handleClearMixSelection() {
    clearMixSelection();
  }

  function clearMixSelection() {
    mixSelectedGenres = [];
    mixSelectedArtists = [];
    mixSelectedAlbums = [];
    mixLikedOnly = false;
    mixFormatFilter = "All";
    mixMessage = null;
  }

  function handleToggleQueue() {
    isQueueOpen = !isQueueOpen;
  }

  function handleClearQueue() {
    playbackQueue = [];
    currentQueueIndex = null;
    shuffledQueueOrder = [];
    isQueueOpen = false;
  }

  function handleToggleShuffle() {
    isShuffleEnabled = !isShuffleEnabled;
    shuffledQueueOrder = !isShuffleEnabled
      ? []
      : buildShuffledQueueOrder(playbackQueue.length, currentQueueIndex);
  }

  function handleToggleRepeat() {
    repeatMode = nextRepeatMode(repeatMode);
  }

  async function handleTogglePlayback() {
    if (!currentTrack) {
      return;
    }

    playbackError = null;

    if (!isPlaying && hasCurrentTrackEnded) {
      if (currentQueueIndex === null) {
        playbackQueue = [currentTrack];
        currentQueueIndex = 0;
        shuffledQueueOrder = isShuffleEnabled ? [0] : [];
      }

      await playQueuedTrackAtIndex(currentQueueIndex ?? 0);
      return;
    }

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
      hasCurrentTrackEnded = false;
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

    if (status.isPlaying) {
      hasCurrentTrackEnded = false;
    }
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

  function artistTrackCount(artist: Artist) {
    if (tracks.length > 0) {
      return tracks.filter((track) => artistNameForTrack(track) === artist.name).length;
    }

    return Number.parseInt(artist.detail, 10) || 0;
  }

  function artistAlbumCount(artist: Artist) {
    return displayAlbums.filter((album) => album.artist === artist.name).length;
  }

  function genreDetail(genre: Genre) {
    return `${genre.songCount} ${genre.songCount === 1 ? "song" : "songs"} · ${genre.artistCount} ${genre.artistCount === 1 ? "artist" : "artists"} · ${genre.albumCount} ${genre.albumCount === 1 ? "album" : "albums"}`;
  }

  function trackGenres(track: Track) {
    return track.genres.length > 0 ? track.genres : ["Unknown Genre"];
  }

  function toggleSelection(values: string[], value: string) {
    return values.includes(value)
      ? values.filter((candidate) => candidate !== value)
      : [...values, value];
  }

  function buildMixTracks(libraryTracks: Track[]) {
    if (!mixHasSelection) {
      return [];
    }

    const seen = new Set<string>();
    const mixTracks: Track[] = [];

    for (const track of libraryTracks) {
      if (!trackMatchesMixSelection(track) || !trackMatchesMixFilters(track)) {
        continue;
      }

      if (!seen.has(track.id)) {
        seen.add(track.id);
        mixTracks.push(track);
      }
    }

    return mixTracks;
  }

  function trackMatchesMixSelection(track: Track) {
    return (
      trackGenres(track).some((genre) => mixSelectedGenreSet.has(genre))
      || mixSelectedArtistSet.has(artistNameForTrack(track))
      || mixSelectedAlbumSet.has(albumIdForTrack(track))
    );
  }

  function trackMatchesMixFilters(track: Track) {
    if (mixLikedOnly && !track.isFavorite) {
      return false;
    }

    if (mixFormatFilter !== "All" && track.extension.toUpperCase() !== mixFormatFilter) {
      return false;
    }

    return true;
  }

  function mixTrackCountForGenre(genre: Genre) {
    return tracks.filter((track) => trackGenres(track).includes(genre.name) && trackMatchesMixFilters(track)).length;
  }

  function mixTrackCountForArtist(artist: Artist) {
    return tracks.filter((track) => artistNameForTrack(track) === artist.name && trackMatchesMixFilters(track)).length;
  }

  function mixTrackCountForAlbum(album: Album) {
    return tracks.filter((track) => albumIdForTrack(track) === album.id && trackMatchesMixFilters(track)).length;
  }

  function shuffleTracks(libraryTracks: Track[]) {
    const shuffled = [...libraryTracks];

    for (let index = shuffled.length - 1; index > 0; index -= 1) {
      const swapIndex = Math.floor(Math.random() * (index + 1));
      [shuffled[index], shuffled[swapIndex]] = [shuffled[swapIndex], shuffled[index]];
    }

    return shuffled;
  }

  function parseGenreDraft(value: string) {
    const seen = new Set<string>();
    const genres: string[] = [];

    for (const part of value.split(",")) {
      const genre = part.trim();
      const key = genre.toLocaleLowerCase();

      if (genre && !seen.has(key)) {
        seen.add(key);
        genres.push(genre);
      }
    }

    return genres;
  }

  function uniqueGenresForTracks(libraryTracks: Track[]) {
    const seen = new Set<string>();
    const genres: string[] = [];

    for (const track of libraryTracks) {
      for (const genre of trackGenres(track)) {
        const key = genre.toLocaleLowerCase();

        if (!seen.has(key)) {
          seen.add(key);
          genres.push(genre);
        }
      }
    }

    return genres;
  }

  function genreDisplayForTracks(libraryTracks: Track[]) {
    return uniqueGenresForTracks(libraryTracks).join(", ") || "Unknown Genre";
  }

  function genreDraftForTracks(libraryTracks: Track[]) {
    return uniqueGenresForTracks(libraryTracks)
      .filter((genre) => genre !== "Unknown Genre")
      .join(", ");
  }

  function albumIdsForTracks(libraryTracks: Track[]) {
    return new Set(libraryTracks.map(albumIdForTrack));
  }

  function albumsForTracks(libraryTracks: Track[], albums: Album[]) {
    const albumIds = albumIdsForTracks(libraryTracks);

    return albums.filter((album) => albumIds.has(album.id));
  }

  function availableTrackFormats(libraryTracks: Track[]) {
    const formats = new Set(libraryTracks.map((track) => track.extension.toUpperCase()));
    const order = ["FLAC", "MP3", "OGG", "OPUS", "WAV", "M4A"];

    return ["All", ...order.filter((format) => formats.has(format))];
  }

  function filterTracksByFormat(libraryTracks: Track[], format: string) {
    if (format === "All") {
      return libraryTracks;
    }

    return libraryTracks.filter((track) => track.extension.toUpperCase() === format);
  }

  function compareText(left: string | null | undefined, right: string | null | undefined) {
    return (left ?? "").localeCompare(right ?? "", undefined, { sensitivity: "base" });
  }

  function applySortDirection(value: number, direction: SortDirection) {
    return direction === "asc" ? value : -value;
  }

  function nextSortDirection(direction: SortDirection) {
    return direction === "asc" ? "desc" : "asc";
  }

  function sortDirectionLabel(direction: SortDirection) {
    return direction === "asc" ? "Asc" : "Desc";
  }

  function nextRepeatMode(mode: RepeatMode): RepeatMode {
    if (mode === "off") {
      return "all";
    }

    if (mode === "all") {
      return "one";
    }

    return "off";
  }

  function getNextQueueIndex(isAutoAdvance: boolean) {
    if (currentQueueIndex === null || playbackQueue.length === 0) {
      return null;
    }

    if (isAutoAdvance && repeatMode === "one") {
      return currentQueueIndex;
    }

    const nextOrderIndex = currentOrderIndex + 1;

    if (nextOrderIndex < playbackOrder.length) {
      return playbackOrder[nextOrderIndex];
    }

    if (repeatMode === "all") {
      return playbackOrder[0] ?? null;
    }

    return null;
  }

  function getPreviousQueueIndex() {
    if (currentQueueIndex === null || playbackQueue.length === 0) {
      return null;
    }

    const previousOrderIndex = currentOrderIndex - 1;

    if (previousOrderIndex >= 0) {
      return playbackOrder[previousOrderIndex];
    }

    if (repeatMode === "all") {
      return playbackOrder.at(-1) ?? null;
    }

    return null;
  }

  function normalizedQueueOrder(order: number[], queueLength: number) {
    const seen = new Set<number>();
    const normalized = order.filter((index) => {
      const isValid = Number.isInteger(index) && index >= 0 && index < queueLength && !seen.has(index);
      seen.add(index);
      return isValid;
    });

    for (let index = 0; index < queueLength; index += 1) {
      if (!seen.has(index)) {
        normalized.push(index);
      }
    }

    return normalized;
  }

  function queueOrderIndex(order: number[], queueIndex: number | null) {
    return queueIndex === null ? -1 : order.indexOf(queueIndex);
  }

  function buildQueuePanelEntries(queue: Track[], order: number[], queueIndex: number | null): QueuePanelEntry[] {
    const startOrderIndex = queueIndex === null ? 0 : Math.max(0, order.indexOf(queueIndex));

    return order.slice(startOrderIndex).flatMap((trackIndex, offset) => {
      const track = queue[trackIndex];

      return track ? [{ track, queueIndex: trackIndex, offset }] : [];
    });
  }

  function buildShuffledQueueOrder(queueLength: number, queueIndex: number | null) {
    if (queueLength === 0) {
      return [];
    }

    const currentIndex = queueIndex ?? 0;
    const remainingIndices = Array.from({ length: queueLength }, (_, index) => index)
      .filter((index) => index !== currentIndex);

    for (let index = remainingIndices.length - 1; index > 0; index -= 1) {
      const swapIndex = Math.floor(Math.random() * (index + 1));
      [remainingIndices[index], remainingIndices[swapIndex]] = [remainingIndices[swapIndex], remainingIndices[index]];
    }

    return [currentIndex, ...remainingIndices];
  }

  function queuePositionLabel(offset: number) {
    if (currentQueueIndex !== null && offset === 0) {
      return "Now";
    }

    return currentQueueIndex === null ? `${offset + 1}` : `+${offset}`;
  }

  function sortTracks(libraryTracks: Track[], sortKey: SongSortKey, direction: SortDirection) {
    return [...libraryTracks].sort((left, right) => {
      let result = 0;

      if (sortKey === "artist") {
        result = compareText(left.artist ?? left.albumArtist, right.artist ?? right.albumArtist)
          || compareText(left.title, right.title);
        return applySortDirection(result, direction);
      }

      if (sortKey === "album") {
        result = compareText(left.album, right.album)
          || compareText(left.discNumber?.toString(), right.discNumber?.toString())
          || (left.trackNumber ?? 0) - (right.trackNumber ?? 0)
          || compareText(left.title, right.title);
        return applySortDirection(result, direction);
      }

      if (sortKey === "duration") {
        result = (left.durationSeconds ?? Number.MAX_SAFE_INTEGER)
          - (right.durationSeconds ?? Number.MAX_SAFE_INTEGER)
          || compareText(left.title, right.title);
        return applySortDirection(result, direction);
      }

      result = compareText(left.title, right.title);
      return applySortDirection(result, direction);
    });
  }

  function sortAlbums(albums: Album[], sortKey: AlbumSortKey, direction: SortDirection) {
    return [...albums].sort((left, right) => {
      let result = 0;

      if (sortKey === "artist") {
        result = compareText(left.artist, right.artist) || compareText(left.title, right.title);
        return applySortDirection(result, direction);
      }

      if (sortKey === "year") {
        result = (left.year ?? Number.MAX_SAFE_INTEGER) - (right.year ?? Number.MAX_SAFE_INTEGER)
          || compareText(left.title, right.title);
        return applySortDirection(result, direction);
      }

      if (sortKey === "trackCount") {
        result = left.trackCount - right.trackCount || compareText(left.title, right.title);
        return applySortDirection(result, direction);
      }

      result = compareText(left.title, right.title);
      return applySortDirection(result, direction);
    });
  }

  function sortArtists(artists: Artist[], sortKey: ArtistSortKey, direction: SortDirection) {
    return [...artists].sort((left, right) => {
      let result = 0;

      if (sortKey === "songCount") {
        result = artistTrackCount(left) - artistTrackCount(right) || compareText(left.name, right.name);
        return applySortDirection(result, direction);
      }

      if (sortKey === "albumCount") {
        result = artistAlbumCount(left) - artistAlbumCount(right) || compareText(left.name, right.name);
        return applySortDirection(result, direction);
      }

      result = compareText(left.name, right.name);
      return applySortDirection(result, direction);
    });
  }

  function sortGenres(genres: Genre[], sortKey: GenreSortKey, direction: SortDirection) {
    return [...genres].sort((left, right) => {
      let result = 0;

      if (sortKey === "songCount") {
        result = left.songCount - right.songCount || compareText(left.name, right.name);
        return applySortDirection(result, direction);
      }

      if (sortKey === "artistCount") {
        result = left.artistCount - right.artistCount || compareText(left.name, right.name);
        return applySortDirection(result, direction);
      }

      if (sortKey === "albumCount") {
        result = left.albumCount - right.albumCount || compareText(left.name, right.name);
        return applySortDirection(result, direction);
      }

      result = compareText(left.name, right.name);
      return applySortDirection(result, direction);
    });
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
      ...trackGenres(track),
    ]);
  }

  function albumMatchesSearch(album: Album, query: string) {
    return matchesSearch(query, [album.title, album.artist]);
  }

  function artistMatchesSearch(artist: Artist, query: string) {
    return matchesSearch(query, [artist.name]);
  }

  function genreMatchesSearch(genre: Genre, query: string) {
    return matchesSearch(query, [genre.name]);
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
      const total = searchTracks.length + searchAlbums.length + searchArtists.length + searchGenres.length;
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
          placeholder="Search songs, albums, artists, genres..."
          aria-label="Search songs, albums, artists, genres"
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
                onToggleFavorite={handleToggleFavorite}
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

          <LibrarySection title="Genres" viewAllLabel={`${searchGenres.length} ${searchGenres.length === 1 ? "match" : "matches"}`}>
            {#if searchGenres.length === 0}
              <div class="group-empty">
                <h3>No genres matched</h3>
                <p>Try a different genre name.</p>
              </div>
            {:else}
              <div class="genre-grid">
                {#each searchGenres as genre}
                  <button class="genre-card" type="button" onclick={() => handleGenreSelect(genre)}>
                    <div class="genre-mark" style={`--item-color: ${genre.color}`} aria-hidden="true">
                      {genre.name.slice(0, 1)}
                    </div>
                    <div>
                      <h3>{genre.name}</h3>
                      <p>{genreDetail(genre)}</p>
                    </div>
                  </button>
                {/each}
              </div>
            {/if}
          </LibrarySection>
        {:else}
          <div class="group-empty">
            <h3>No matches found</h3>
            <p>Search looks at song titles, artists, albums, album artists, genres, and file names.</p>
          </div>
        {/if}
      {:else if activeView === "Home"}
        <LibrarySection title="Recently Added">
          <TrackList
            tracks={homeTracks}
            {isScanning}
            selectedTrackId={currentTrack?.id}
            onTrackSelect={handleTrackSelect}
            onToggleFavorite={handleToggleFavorite}
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

            <div class="genre-editor" aria-label="Album genre editor">
              <div class="genre-editor-copy">
                <p class="eyebrow">Genres</p>
                <p>{selectedAlbumGenreText}</p>
              </div>
              <form onsubmit={(event) => { event.preventDefault(); void handleSaveAlbumGenres(); }}>
                <input
                  type="text"
                  bind:value={albumGenreDraft}
                  placeholder="Technical Death Metal, Jazz"
                  aria-label="Album genres"
                  disabled={isSavingGenreAssignment}
                />
                <button type="submit" disabled={isSavingGenreAssignment}>
                  {isSavingGenreAssignment ? "Saving..." : "Save"}
                </button>
              </form>
              {#if genreEditError}
                <p class="genre-editor-error" role="alert">{genreEditError}</p>
              {:else if genreEditMessage}
                <p class="genre-editor-message">{genreEditMessage}</p>
              {/if}
            </div>

            <LibrarySection title="Album Songs" viewAllLabel={`${selectedAlbumTracks.length} total`}>
              <TrackList
                tracks={selectedAlbumTracks}
                isScanning={false}
                selectedTrackId={currentTrack?.id}
                onTrackSelect={handleTrackSelect}
                onToggleFavorite={handleToggleFavorite}
              />
            </LibrarySection>
          </section>
        {:else}
          <LibrarySection title="All Albums" viewAllLabel={`${sortedAlbums.length} total`}>
            <div class="control-bar">
              <label>
                <span>Sort</span>
                <select bind:value={albumSort}>
                  <option value="title">Album title</option>
                  <option value="artist">Artist</option>
                  <option value="year">Year</option>
                  <option value="trackCount">Song count</option>
                </select>
              </label>
              <button
                class="direction-toggle"
                type="button"
                aria-label={`Album sort direction: ${sortDirectionLabel(albumSortDirection)}`}
                onclick={() => albumSortDirection = nextSortDirection(albumSortDirection)}
              >
                {sortDirectionLabel(albumSortDirection)}
              </button>
            </div>
            {#if sortedAlbums.length === 0}
              <div class="group-empty">
                <h3>No albums found</h3>
                <p>Scan a music folder to build your local album library.</p>
              </div>
            {:else}
              <div class="album-grid">
                {#each sortedAlbums as album}
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

            <div class="genre-editor" aria-label="Artist genre editor">
              <div class="genre-editor-copy">
                <p class="eyebrow">Genres</p>
                <p>{selectedArtistGenreText}</p>
              </div>
              <form onsubmit={(event) => { event.preventDefault(); void handleSaveArtistGenres(); }}>
                <input
                  type="text"
                  bind:value={artistGenreDraft}
                  placeholder="Technical Death Metal, Jazz"
                  aria-label="Artist genres"
                  disabled={isSavingGenreAssignment}
                />
                <button type="submit" disabled={isSavingGenreAssignment}>
                  {isSavingGenreAssignment ? "Saving..." : "Save"}
                </button>
              </form>
              {#if genreEditError}
                <p class="genre-editor-error" role="alert">{genreEditError}</p>
              {:else if genreEditMessage}
                <p class="genre-editor-message">{genreEditMessage}</p>
              {/if}
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
                onToggleFavorite={handleToggleFavorite}
              />
            </LibrarySection>
          </section>
        {:else}
          <LibrarySection title="All Artists" viewAllLabel={`${sortedArtists.length} total`}>
            <div class="control-bar">
              <label>
                <span>Sort</span>
                <select bind:value={artistSort}>
                  <option value="name">Artist name</option>
                  <option value="songCount">Song count</option>
                  <option value="albumCount">Album count</option>
                </select>
              </label>
              <button
                class="direction-toggle"
                type="button"
                aria-label={`Artist sort direction: ${sortDirectionLabel(artistSortDirection)}`}
                onclick={() => artistSortDirection = nextSortDirection(artistSortDirection)}
              >
                {sortDirectionLabel(artistSortDirection)}
              </button>
            </div>
            {#if sortedArtists.length === 0}
              <div class="group-empty">
                <h3>No artists found</h3>
                <p>Scan a music folder to build your local artist library.</p>
              </div>
            {:else}
              <div class="artist-grid">
                {#each sortedArtists as artist}
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
      {:else if activeView === "Genres"}
        {#if selectedGenre}
          <section class="detail-view" aria-labelledby="genre-detail-title">
            <div class="detail-actions">
              <button class="back-button" type="button" onclick={handleBackToGenres}>Back to Genres</button>
              <button
                class="back-button accent"
                type="button"
                disabled={selectedGenreTracks.length === 0}
                onclick={handleShuffleGenre}
              >
                Shuffle Genre
              </button>
            </div>
            <div class="genre-detail-header">
              <div class="genre-mark detail-avatar" style={`--item-color: ${selectedGenre.color}`} aria-hidden="true">
                {selectedGenre.name.slice(0, 1)}
              </div>
              <div class="detail-copy">
                <p class="eyebrow">Genre</p>
                <h3 id="genre-detail-title">{selectedGenre.name}</h3>
                <p>{genreDetail(selectedGenre)}</p>
              </div>
            </div>

            <LibrarySection title="Albums" viewAllLabel={`${selectedGenreAlbums.length} total`}>
              {#if selectedGenreAlbums.length === 0}
                <div class="group-empty">
                  <h3>No albums found</h3>
                  <p>No album tags were found for this genre.</p>
                </div>
              {:else}
                <div class="album-grid">
                  {#each selectedGenreAlbums as album}
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

            <LibrarySection title="Artists" viewAllLabel={`${selectedGenreArtists.length} total`}>
              {#if selectedGenreArtists.length === 0}
                <div class="group-empty">
                  <h3>No artists found</h3>
                  <p>No artist tags were found for this genre.</p>
                </div>
              {:else}
                <div class="artist-grid">
                  {#each selectedGenreArtists as artist}
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

            <LibrarySection title="Songs" viewAllLabel={`${selectedGenreTracks.length} total`}>
              <TrackList
                tracks={selectedGenreTracks}
                isScanning={false}
                selectedTrackId={currentTrack?.id}
                onTrackSelect={handleTrackSelect}
                onToggleFavorite={handleToggleFavorite}
              />
            </LibrarySection>
          </section>
        {:else}
          <LibrarySection title="All Genres" viewAllLabel={`${sortedGenres.length} total`}>
            <div class="control-bar">
              <label>
                <span>Sort</span>
                <select bind:value={genreSort}>
                  <option value="name">Genre name</option>
                  <option value="songCount">Song count</option>
                  <option value="artistCount">Artist count</option>
                  <option value="albumCount">Album count</option>
                </select>
              </label>
              <button
                class="direction-toggle"
                type="button"
                aria-label={`Genre sort direction: ${sortDirectionLabel(genreSortDirection)}`}
                onclick={() => genreSortDirection = nextSortDirection(genreSortDirection)}
              >
                {sortDirectionLabel(genreSortDirection)}
              </button>
            </div>
            {#if sortedGenres.length === 0}
              <div class="group-empty">
                <h3>No genres found</h3>
                <p>Scan a music folder to build your local genre library.</p>
              </div>
            {:else}
              <div class="genre-grid">
                {#each sortedGenres as genre}
                  <button class="genre-card" type="button" onclick={() => handleGenreSelect(genre)}>
                    <div class="genre-mark" style={`--item-color: ${genre.color}`} aria-hidden="true">
                      {genre.name.slice(0, 1)}
                    </div>
                    <div>
                      <h3>{genre.name}</h3>
                      <p>{genreDetail(genre)}</p>
                    </div>
                  </button>
                {/each}
              </div>
            {/if}
          </LibrarySection>
        {/if}
      {:else if activeView === "Songs"}
        <LibrarySection title="All Songs" viewAllLabel={`${filteredSongTracks.length} shown`}>
          <div class="control-bar">
            <label>
              <span>Sort</span>
              <select bind:value={songSort}>
                <option value="title">Title</option>
                <option value="artist">Artist</option>
                <option value="album">Album</option>
                <option value="duration">Duration</option>
              </select>
            </label>
            <button
              class="direction-toggle"
              type="button"
              aria-label={`Song sort direction: ${sortDirectionLabel(songSortDirection)}`}
              onclick={() => songSortDirection = nextSortDirection(songSortDirection)}
            >
              {sortDirectionLabel(songSortDirection)}
            </button>
            <label>
              <span>Format</span>
              <select bind:value={songFormatFilter}>
                {#each availableFormats as format}
                  <option value={format}>{format}</option>
                {/each}
              </select>
            </label>
          </div>
          <TrackList
            tracks={filteredSongTracks}
            {isScanning}
            selectedTrackId={currentTrack?.id}
            onTrackSelect={handleTrackSelect}
            onToggleFavorite={handleToggleFavorite}
          />
        </LibrarySection>
      {:else if activeView === "Playlists"}
        {#if isLikedSongsOpen}
          <section class="detail-view" aria-labelledby="liked-songs-title">
            <button class="back-button" type="button" onclick={handleBackToPlaylists}>Back to Playlists</button>
            <div class="playlist-detail-header">
              <div class="liked-mark" aria-hidden="true">★</div>
              <div class="detail-copy">
                <p class="eyebrow">Smart Playlist</p>
                <h3 id="liked-songs-title">Liked Songs</h3>
                <p>{favoriteTracks.length} {favoriteTracks.length === 1 ? "song" : "songs"}</p>
              </div>
            </div>

            <LibrarySection title="Songs" viewAllLabel={`${favoriteTracks.length} total`}>
              {#if favoriteTracks.length === 0}
                <div class="group-empty">
                  <h3>No liked songs yet</h3>
                  <p>Use the star button on any song row or in the player to add it here.</p>
                </div>
              {:else}
                <TrackList
                  tracks={favoriteTracks}
                  isScanning={false}
                  selectedTrackId={currentTrack?.id}
                  onTrackSelect={handleTrackSelect}
                  onToggleFavorite={handleToggleFavorite}
                />
              {/if}
            </LibrarySection>
          </section>
        {:else if isMixBuilderOpen}
          <section class="detail-view" aria-labelledby="mix-builder-title">
            <button class="back-button" type="button" onclick={handleBackToPlaylists}>Back to Playlists</button>
            <div class="playlist-detail-header">
              <div class="mix-mark" aria-hidden="true">M</div>
              <div class="detail-copy">
                <p class="eyebrow">Temporary Mix</p>
                <h3 id="mix-builder-title">Mix Builder</h3>
                <p>{mixSelectedSourceCount} {mixSelectedSourceCount === 1 ? "source" : "sources"} · {mixTracks.length} {mixTracks.length === 1 ? "track" : "tracks"} selected</p>
              </div>
            </div>

            <div class="mix-builder-panel">
              <div class="mix-builder-controls">
                <label class="mix-toggle">
                  <input type="checkbox" bind:checked={mixLikedOnly} onchange={() => mixMessage = null} />
                  <span>Liked songs only</span>
                </label>
                <label>
                  <span>Format</span>
                  <select bind:value={mixFormatFilter} onchange={() => mixMessage = null}>
                    {#each mixFormatOptions as format}
                      <option value={format}>{format}</option>
                    {/each}
                  </select>
                </label>
              </div>
              <div class="mix-builder-actions">
                <p>{mixTracks.length} {mixTracks.length === 1 ? "track" : "tracks"} selected</p>
                <button type="button" onclick={handleClearMixSelection} disabled={!mixHasSelection && !mixLikedOnly && mixFormatFilter === "All"}>
                  Clear Selection
                </button>
                <button class="accent" type="button" onclick={() => void handleStartMix()}>
                  Start Mix
                </button>
              </div>
              {#if mixMessage}
                <p class="mix-message" role="status">{mixMessage}</p>
              {:else if mixHasSelection && mixTracks.length === 0}
                <p class="mix-message" role="status">No tracks match this mix.</p>
              {:else if !mixHasSelection}
                <p class="mix-message" role="status">Select at least one source.</p>
              {/if}
            </div>

            <LibrarySection title="Genres" viewAllLabel={`${mixSelectedGenres.length} selected`}>
              {#if sortedGenres.length === 0}
                <div class="group-empty">
                  <h3>No genres found</h3>
                  <p>Scan a music folder to build your local genre library.</p>
                </div>
              {:else}
                <div class="mix-option-grid">
                  {#each sortedGenres as genre}
                    <label class:selected={mixSelectedGenreSet.has(genre.name)} class="mix-option-card">
                      <input
                        type="checkbox"
                        checked={mixSelectedGenreSet.has(genre.name)}
                        onchange={() => handleToggleMixItem("genre", genre.name)}
                      />
                      <span class="mix-option-mark" style={`--item-color: ${genre.color}`} aria-hidden="true">
                        {genre.name.slice(0, 1)}
                      </span>
                      <span>
                        <strong>{genre.name}</strong>
                        <small>{mixTrackCountForGenre(genre)} {mixTrackCountForGenre(genre) === 1 ? "track" : "tracks"}</small>
                      </span>
                    </label>
                  {/each}
                </div>
              {/if}
            </LibrarySection>

            <LibrarySection title="Artists" viewAllLabel={`${mixSelectedArtists.length} selected`}>
              {#if sortedArtists.length === 0}
                <div class="group-empty">
                  <h3>No artists found</h3>
                  <p>Scan a music folder to build your local artist library.</p>
                </div>
              {:else}
                <div class="mix-option-grid">
                  {#each sortedArtists as artist}
                    <label class:selected={mixSelectedArtistSet.has(artist.name)} class="mix-option-card">
                      <input
                        type="checkbox"
                        checked={mixSelectedArtistSet.has(artist.name)}
                        onchange={() => handleToggleMixItem("artist", artist.name)}
                      />
                      <span class="mix-option-mark round" style={`--item-color: ${artist.color}`} aria-hidden="true">
                        {artist.name.slice(0, 1)}
                      </span>
                      <span>
                        <strong>{artist.name}</strong>
                        <small>{mixTrackCountForArtist(artist)} {mixTrackCountForArtist(artist) === 1 ? "track" : "tracks"}</small>
                      </span>
                    </label>
                  {/each}
                </div>
              {/if}
            </LibrarySection>

            <LibrarySection title="Albums" viewAllLabel={`${mixSelectedAlbums.length} selected`}>
              {#if sortedAlbums.length === 0}
                <div class="group-empty">
                  <h3>No albums found</h3>
                  <p>Scan a music folder to build your local album library.</p>
                </div>
              {:else}
                <div class="mix-option-grid">
                  {#each sortedAlbums as album}
                    <label class:selected={mixSelectedAlbumSet.has(album.id)} class="mix-option-card">
                      <input
                        type="checkbox"
                        checked={mixSelectedAlbumSet.has(album.id)}
                        onchange={() => handleToggleMixItem("album", album.id)}
                      />
                      <span class="mix-option-mark" style={`--item-color: ${album.color}`} aria-hidden="true">
                        {album.title.slice(0, 1)}
                      </span>
                      <span>
                        <strong>{album.title}</strong>
                        <small>{album.artist} · {mixTrackCountForAlbum(album)} {mixTrackCountForAlbum(album) === 1 ? "track" : "tracks"}</small>
                      </span>
                    </label>
                  {/each}
                </div>
              {/if}
            </LibrarySection>
          </section>
        {:else}
          <section class="playlist-grid" aria-labelledby="playlists-title">
            <button class="playlist-card" type="button" onclick={handleLikedSongsSelect}>
              <div class="liked-mark" aria-hidden="true">★</div>
              <div>
                <p class="eyebrow">Smart Playlist</p>
                <h3 id="playlists-title">Liked Songs</h3>
                <p>{favoriteTracks.length} {favoriteTracks.length === 1 ? "song" : "songs"}</p>
              </div>
            </button>
            <button class="playlist-card" type="button" onclick={handleMixBuilderSelect}>
              <div class="mix-mark" aria-hidden="true">M</div>
              <div>
                <p class="eyebrow">Temporary Mix</p>
                <h3>Mix Builder</h3>
                <p>{tracks.length} {tracks.length === 1 ? "track" : "tracks"} available</p>
              </div>
            </button>
          </section>
        {/if}
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

  {#if isQueueOpen}
    <aside class="queue-panel" aria-label="Up Next">
      <div class="queue-panel-header">
        <div>
          <p class="eyebrow">Playback</p>
          <h3>Up Next</h3>
        </div>
        <button type="button" disabled={playbackQueue.length === 0} onclick={handleClearQueue}>
          Clear
        </button>
      </div>

      {#if queuePanelEntries.length === 0}
        <div class="group-empty">
          <h3>No queued songs</h3>
          <p>Play a song from any list to build an Up Next queue.</p>
        </div>
      {:else}
        <div class="queue-list">
          {#each queuePanelEntries as entry (entry.track.id)}
            <button
              class:active={entry.queueIndex === currentQueueIndex}
              class="queue-row"
              type="button"
              title={entry.track.filePath}
              onclick={() => playQueuedTrackAtIndex(entry.queueIndex)}
            >
              <span>{queuePositionLabel(entry.offset)}</span>
              <div>
                <p>{entry.track.title}</p>
                <small>{entry.track.artist ?? "Unknown Artist"}</small>
              </div>
            </button>
          {/each}
        </div>
      {/if}
    </aside>
  {/if}

  <NowPlayingBar
    track={currentTrack}
    {isPlaying}
    {positionSeconds}
    {durationSeconds}
    {volume}
    {canPlayPrevious}
    {canPlayNext}
    queueCount={playbackQueue.length}
    {isQueueOpen}
    {isShuffleEnabled}
    {repeatMode}
    onTogglePlayback={handleTogglePlayback}
    onPrevious={handlePreviousTrack}
    onNext={handleNextTrack}
    onSeek={handleSeek}
    onVolumeChange={handleVolumeChange}
    onToggleFavorite={handleToggleFavorite}
    onToggleQueue={handleToggleQueue}
    onToggleShuffle={handleToggleShuffle}
    onToggleRepeat={handleToggleRepeat}
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

  .control-bar {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    align-items: center;
  }

  .control-bar label {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 36px;
    color: #8f9aa8;
    font-size: 0.82rem;
    font-weight: 800;
  }

  .control-bar select {
    min-height: 36px;
    border: 1px solid #303844;
    border-radius: 8px;
    background: #12161c;
    color: #f4f7fb;
    font: inherit;
    font-size: 0.86rem;
    font-weight: 750;
    outline: none;
    padding: 0 10px;
  }

  .control-bar select:focus {
    border-color: #2f8f83;
    box-shadow: 0 0 0 2px rgba(47, 143, 131, 0.18);
  }

  .direction-toggle {
    min-height: 36px;
    min-width: 58px;
    border: 1px solid #35544f;
    border-radius: 8px;
    background: #17332f;
    color: #d8fffa;
    cursor: default;
    font: inherit;
    font-size: 0.82rem;
    font-weight: 900;
    padding: 0 10px;
  }

  .direction-toggle:hover,
  .direction-toggle:focus-visible {
    border-color: #2f8f83;
    background: #1b403a;
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
  .artist-card p,
  .genre-card p {
    margin: 0;
    color: #8f9aa8;
    font-size: 0.9rem;
    font-weight: 620;
  }

  .album-card,
  .artist-card > div:last-child,
  .genre-card > div:last-child,
  .playlist-card > div:last-child {
    min-width: 0;
  }

  .album-card h3,
  .album-card p,
  .artist-card h3,
  .artist-card p,
  .genre-card h3,
  .genre-card p,
  .playlist-card h3,
  .playlist-card p:not(.eyebrow) {
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
  .artist-grid,
  .genre-grid,
  .playlist-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 14px;
  }

  .album-card,
  .artist-card,
  .genre-card {
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
  .artist-card:focus-visible,
  .genre-card:hover,
  .genre-card:focus-visible {
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

  .artist-card,
  .genre-card {
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

  .genre-mark {
    display: grid;
    width: 52px;
    height: 52px;
    flex: 0 0 auto;
    place-items: center;
    border-radius: 8px;
    background: var(--item-color);
    color: #0d0f13;
    font-size: 1.15rem;
    font-weight: 900;
  }

  .artist-card p,
  .genre-card p {
    margin-top: 4px;
  }

  .playlist-card {
    display: flex;
    align-items: center;
    gap: 14px;
    min-height: 112px;
    border: 1px solid #242b35;
    border-radius: 8px;
    background: #151a21;
    color: inherit;
    cursor: default;
    font: inherit;
    padding: 18px;
    text-align: left;
  }

  .playlist-card:hover,
  .playlist-card:focus-visible {
    border-color: #35544f;
    background: #1b2027;
    outline: none;
  }

  .playlist-card h3 {
    margin-bottom: 5px;
  }

  .playlist-card p:not(.eyebrow) {
    margin: 0;
    color: #8f9aa8;
    font-weight: 650;
  }

  .liked-mark,
  .mix-mark {
    display: grid;
    width: 58px;
    height: 58px;
    flex: 0 0 auto;
    place-items: center;
    border-radius: 8px;
    font-size: 1.45rem;
    font-weight: 900;
  }

  .liked-mark {
    background: #262214;
    color: #f0c85a;
  }

  .mix-mark {
    background: #17332f;
    color: #9ee3d9;
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

  .back-button.accent {
    border-color: #35544f;
    background: #17332f;
    color: #d8fffa;
  }

  .back-button:disabled {
    border-color: #303844;
    background: #151a21;
    color: #626c79;
  }

  .detail-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }

  .album-detail-header,
  .artist-detail-header,
  .genre-detail-header,
  .playlist-detail-header {
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

  .genre-editor {
    display: grid;
    grid-template-columns: minmax(160px, 0.45fr) minmax(220px, 1fr);
    gap: 12px;
    align-items: center;
    border: 1px solid #242b35;
    border-radius: 8px;
    background: #12161c;
    padding: 14px;
  }

  .genre-editor-copy {
    min-width: 0;
  }

  .genre-editor-copy p:not(.eyebrow) {
    overflow: hidden;
    margin: 0;
    color: #d5dce5;
    font-weight: 750;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .genre-editor form {
    display: flex;
    min-width: 0;
    gap: 10px;
  }

  .genre-editor input {
    width: 100%;
    min-width: 0;
    min-height: 40px;
    border: 1px solid #303844;
    border-radius: 8px;
    background: #0f1318;
    color: #f4f7fb;
    font: inherit;
    font-size: 0.92rem;
    font-weight: 650;
    outline: none;
    padding: 0 12px;
  }

  .genre-editor input:focus {
    border-color: #2f8f83;
    box-shadow: 0 0 0 2px rgba(47, 143, 131, 0.18);
  }

  .genre-editor button {
    min-width: 76px;
    min-height: 40px;
    border: 1px solid #35544f;
    border-radius: 8px;
    background: #17332f;
    color: #d8fffa;
    cursor: default;
    font: inherit;
    font-size: 0.88rem;
    font-weight: 850;
    padding: 0 13px;
  }

  .genre-editor button:disabled {
    border-color: #303844;
    background: #151a21;
    color: #626c79;
  }

  .genre-editor-error,
  .genre-editor-message {
    grid-column: 1 / -1;
    margin: 0;
    font-size: 0.86rem;
    font-weight: 700;
  }

  .genre-editor-error {
    color: #ffcbc8;
  }

  .genre-editor-message {
    color: #9ee3d9;
  }

  .mix-builder-panel {
    display: grid;
    grid-template-columns: minmax(240px, 1fr) auto;
    gap: 14px;
    align-items: center;
    border: 1px solid #242b35;
    border-radius: 8px;
    background: #12161c;
    padding: 14px;
  }

  .mix-builder-controls,
  .mix-builder-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    align-items: center;
  }

  .mix-builder-controls label {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 36px;
    color: #8f9aa8;
    font-size: 0.82rem;
    font-weight: 800;
  }

  .mix-toggle input,
  .mix-option-card input {
    accent-color: #2f8f83;
  }

  .mix-builder-controls select {
    min-height: 36px;
    border: 1px solid #303844;
    border-radius: 8px;
    background: #0f1318;
    color: #f4f7fb;
    font: inherit;
    font-size: 0.86rem;
    font-weight: 750;
    outline: none;
    padding: 0 10px;
  }

  .mix-builder-controls select:focus {
    border-color: #2f8f83;
    box-shadow: 0 0 0 2px rgba(47, 143, 131, 0.18);
  }

  .mix-builder-actions {
    justify-content: flex-end;
  }

  .mix-builder-actions p {
    margin: 0;
    color: #d5dce5;
    font-size: 0.9rem;
    font-weight: 800;
  }

  .mix-builder-actions button {
    min-height: 36px;
    border: 1px solid #303844;
    border-radius: 8px;
    background: #161a20;
    color: #d5dce5;
    cursor: default;
    font: inherit;
    font-size: 0.86rem;
    font-weight: 850;
    padding: 0 12px;
  }

  .mix-builder-actions button.accent {
    border-color: #35544f;
    background: #17332f;
    color: #d8fffa;
  }

  .mix-builder-actions button:disabled {
    border-color: #303844;
    background: #151a21;
    color: #626c79;
  }

  .mix-message {
    grid-column: 1 / -1;
    margin: 0;
    color: #9aa4b1;
    font-size: 0.9rem;
    font-weight: 700;
  }

  .mix-option-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 10px;
  }

  .mix-option-card {
    display: grid;
    grid-template-columns: auto 42px minmax(0, 1fr);
    align-items: center;
    gap: 10px;
    min-height: 68px;
    border: 1px solid #242b35;
    border-radius: 8px;
    background: #151a21;
    cursor: default;
    padding: 10px;
  }

  .mix-option-card:hover,
  .mix-option-card:focus-within {
    border-color: #35544f;
    background: #1b2027;
  }

  .mix-option-card.selected {
    border-color: #2f8f83;
    background: #14241f;
    box-shadow: inset 0 0 0 1px rgba(47, 143, 131, 0.28);
  }

  .mix-option-card input {
    width: 16px;
    height: 16px;
    margin: 0;
  }

  .mix-option-mark {
    display: grid;
    width: 42px;
    height: 42px;
    place-items: center;
    border-radius: 8px;
    background: var(--item-color);
    color: #0d0f13;
    font-size: 0.98rem;
    font-weight: 900;
  }

  .mix-option-mark.round {
    border-radius: 50%;
  }

  .mix-option-card > span:last-child {
    min-width: 0;
  }

  .mix-option-card strong,
  .mix-option-card small {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mix-option-card strong {
    color: #f4f7fb;
    font-size: 0.9rem;
    line-height: 1.25;
  }

  .mix-option-card small {
    margin-top: 3px;
    color: #8f9aa8;
    font-size: 0.78rem;
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

  .queue-panel {
    position: fixed;
    right: 24px;
    bottom: 104px;
    z-index: 4;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: 14px;
    width: min(420px, calc(100vw - 32px));
    max-height: min(520px, calc(100dvh - 136px));
    overflow: hidden;
    border: 1px solid #2a313c;
    border-radius: 8px;
    background: #12161c;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.34);
    padding: 16px;
  }

  .queue-panel-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 14px;
  }

  .queue-panel-header h3 {
    font-size: 1.1rem;
  }

  .queue-panel-header button {
    min-height: 34px;
    border: 1px solid #303844;
    border-radius: 8px;
    background: #171c23;
    color: #d5dce5;
    cursor: default;
    font: inherit;
    font-size: 0.82rem;
    font-weight: 850;
    padding: 0 11px;
  }

  .queue-panel-header button:hover,
  .queue-panel-header button:focus-visible {
    border-color: #35544f;
    background: #1b2027;
    outline: none;
  }

  .queue-panel-header button:disabled {
    color: #626c79;
    background: #151a21;
  }

  .queue-list {
    display: grid;
    align-content: start;
    gap: 8px;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding-right: 2px;
  }

  .queue-row {
    display: grid;
    grid-template-columns: 44px minmax(0, 1fr);
    align-items: center;
    gap: 10px;
    min-height: 54px;
    border: 1px solid #242b35;
    border-radius: 8px;
    background: #151a21;
    color: inherit;
    cursor: default;
    font: inherit;
    padding: 9px 10px;
    text-align: left;
  }

  .queue-row:hover,
  .queue-row:focus-visible,
  .queue-row.active {
    border-color: #35544f;
    background: #1b2027;
    outline: none;
  }

  .queue-row > span {
    display: grid;
    min-height: 30px;
    place-items: center;
    border-radius: 7px;
    background: #1d252e;
    color: #8f9aa8;
    font-size: 0.76rem;
    font-weight: 900;
  }

  .queue-row.active > span {
    background: #17332f;
    color: #d8fffa;
  }

  .queue-row div {
    min-width: 0;
  }

  .queue-row p,
  .queue-row small {
    display: block;
    overflow: hidden;
    margin: 0;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .queue-row p {
    color: #f4f7fb;
    font-size: 0.92rem;
    font-weight: 750;
  }

  .queue-row small {
    margin-top: 2px;
    color: #929daa;
    font-size: 0.8rem;
    font-weight: 650;
  }

  @media (max-width: 1020px) {
    .album-grid,
    .artist-grid,
    .genre-grid,
    .playlist-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .mix-option-grid {
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
    .artist-detail-header,
    .genre-detail-header,
    .playlist-detail-header {
      align-items: flex-start;
      flex-direction: column;
    }

    .detail-cover {
      width: min(100%, 220px);
    }

    .genre-editor {
      grid-template-columns: 1fr;
    }

    .mix-builder-panel {
      grid-template-columns: 1fr;
    }

    .mix-builder-actions {
      justify-content: flex-start;
    }

    .queue-panel {
      left: 16px;
      right: 16px;
      bottom: 150px;
      width: auto;
      max-height: calc(100dvh - 178px);
    }
  }

  @media (max-width: 520px) {
    .album-grid,
    .artist-grid,
    .genre-grid,
    .playlist-grid,
    .mix-option-grid,
    .settings-grid {
      grid-template-columns: 1fr;
    }

    .genre-editor form {
      flex-direction: column;
    }
  }
</style>
