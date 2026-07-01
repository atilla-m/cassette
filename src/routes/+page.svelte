<script lang="ts">
  import {
    addTrackToPlaylist,
    autoFindTrackLyrics,
    chooseCoverImage,
    chooseOutputFolder,
    chooseLibraryFolder,
    chooseVideoFolder,
    createPlaylist,
    deletePlaylist,
    detectAudioCd,
    getLibraryCache,
    getVideoLibrary,
    lookupCdMetadata,
    lookupCdCover,
    movePlaylistTrack,
    recordTrackPlay,
    readTrackLyrics,
    removeTrackFromPlaylist,
    renamePlaylist,
    ripCdToFlac,
    inspectCoverImage,
    scanLibrary,
    scanVideoFolder,
    setAlbumGenres,
    setArtistGenres,
    toggleTrackFavorite,
    updateVideoInfo,
    updateVideoProgress,
  } from "$lib/api/library";
  import {
    getPlaybackStatus,
    pausePlayback,
    playTrack,
    resumePlayback,
    seekPlayback,
    setPlaybackVolume,
  } from "$lib/api/playback";
  import ContextMenu from "$lib/components/ContextMenu.svelte";
  import LibrarySection from "$lib/components/LibrarySection.svelte";
  import NowPlayingBar from "$lib/components/NowPlayingBar.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import TrackList from "$lib/components/TrackList.svelte";
  import { buildAlbums, buildArtists, buildGenres } from "$lib/data/libraryViews";
  import { albums as mockAlbums, artists as mockArtists, genres as mockGenres, navItems } from "$lib/data/mockLibrary";
  import type {
    Album,
    Artist,
    CdCoverLookupResult,
    CdDetectResult,
    CdMetadataLookupResult,
    CdMetadataRelease,
    CdRipEvent,
    CdRipCover,
    CdRipMetadata,
    CdRipMetadataTrack,
    CdRipTrack,
    Genre,
    PlaybackStatus,
    Playlist,
    Track,
    TrackLyrics,
    VideoEntry,
    VideoInfoUpdate,
    VideoLibrary,
  } from "$lib/types/library";
  import { localImageSource } from "$lib/utils/localImage";
  import { listen } from "@tauri-apps/api/event";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { onMount } from "svelte";
  import packageInfo from "../../package.json";

  type SongSortKey = "title" | "artist" | "album" | "duration" | "recentlyAdded" | "recentlyPlayed" | "playCount";
  type AlbumSortKey = "title" | "artist" | "year" | "trackCount";
  type ArtistSortKey = "name" | "songCount" | "albumCount";
  type GenreSortKey = "name" | "songCount" | "artistCount" | "albumCount";
  type VideoSortKey = "title" | "artist" | "year" | "recentlyPlayed" | "duration";
  type SortDirection = "asc" | "desc";
  type RepeatMode = "off" | "all" | "one";
  type QueuePanelEntry = {
    track: Track;
    queueIndex: number;
    offset: number;
  };
  type AlbumDiscGroup = {
    discNumber: number | null;
    tracks: Track[];
  };
  type ContextMenuItem = {
    label: string;
    disabled?: boolean;
    action: () => void | Promise<void>;
  };
  type ContextMenuState = {
    x: number;
    y: number;
    items: ContextMenuItem[];
  };
  type MixCategory = "genre" | "artist" | "album";
  type DuplicateAlbumGroup = {
    title: string;
    albums: Album[];
    folders: string[];
    trackCount: number;
  };
  type LibraryDiagnostics = {
    totalTracks: number;
    totalAlbums: number;
    totalArtists: number;
    missingGenreTracks: Track[];
    missingCoverTracks: Track[];
    missingCoverAlbums: Album[];
    unknownArtistTracks: Track[];
    unknownAlbumTracks: Track[];
    missingTrackNumberTracks: Track[];
    missingYearTracks: Track[];
    duplicateAlbumGroups: DuplicateAlbumGroup[];
  };
  type TopArtistStat = {
    name: string;
    color: string;
    totalPlays: number;
    songCount: number;
  };
  type TopAlbumStat = {
    album: Album;
    totalPlays: number;
    songCount: number;
  };
  type TopGenreStat = {
    genre: Genre;
    totalPlays: number;
    songCount: number;
  };
  type SyncedLyricLine = {
    timeSeconds: number;
    text: string;
  };
  type ShortcutItem = {
    keys: string[];
    description: string;
  };
  type ShortcutGroup = {
    title: string;
    shortcuts: ShortcutItem[];
  };
  type VideoEditDraft = {
    title: string;
    artist: string;
    showTitle: string;
    albumOrRelease: string;
    year: string;
    venue: string;
    city: string;
    country: string;
  };

  const mixFormatOptions = ["All", "FLAC", "MP3", "OGG", "OPUS", "WAV", "M4A"];
  const appVersion = packageInfo.version?.trim() || "Development build";
  const shortcutGroups: ShortcutGroup[] = [
    {
      title: "Playback",
      shortcuts: [
        { keys: ["Space"], description: "Play or pause the current track" },
        { keys: ["Arrow Right"], description: "Play the next track" },
        { keys: ["Arrow Left"], description: "Play the previous track" },
      ],
    },
    {
      title: "Navigation",
      shortcuts: [
        { keys: ["?"], description: "Open keyboard shortcuts" },
        { keys: ["Ctrl", "/"], description: "Open keyboard shortcuts" },
        { keys: ["Escape"], description: "Close the shortcuts overlay first" },
      ],
    },
    {
      title: "Library / Search",
      shortcuts: [
        { keys: ["Escape"], description: "Clear the active search" },
      ],
    },
    {
      title: "Queue / Panels",
      shortcuts: [
        { keys: ["Escape"], description: "Close context menus and overlays" },
      ],
    },
  ];

  let tracks = $state<Track[]>([]);
  let videos = $state<VideoEntry[]>([]);
  let isScanning = $state(false);
  let isScanningVideos = $state(false);
  let scanError = $state<string | null>(null);
  let videoError = $state<string | null>(null);
  let videoMessage = $state<string | null>(null);
  let videoFolder = $state<string | null>(null);
  let lastVideoScannedAt = $state<number | null>(null);
  let selectedVideoId = $state<string | null>(null);
  let videoSearchQuery = $state("");
  let videoSort = $state<VideoSortKey>("title");
  let videoSortDirection = $state<SortDirection>("asc");
  let isEditingVideo = $state(false);
  let isSavingVideoInfo = $state(false);
  let videoEditDraft = $state<VideoEditDraft>(emptyVideoEditDraft());
  let videoElement: HTMLVideoElement | undefined = $state();
  let videoPlaybackError = $state<string | null>(null);
  let videoPlaybackMessage = $state<string | null>(null);
  let videoSessionVideoId = $state<string | null>(null);
  let videoSessionWatchedSeconds = 0;
  let videoObservedPositionSeconds = 0;
  let videoLastProgressSaveMs = 0;
  let videoPlayCountRecorded = false;
  let cdRipError = $state<string | null>(null);
  let cdRipMessage = $state<string | null>(null);
  let isDetectingCd = $state(false);
  let isRippingCd = $state(false);
  let cdDriveFound = $state<boolean | null>(null);
  let audioCdFound = $state<boolean | null>(null);
  let cdTracks = $state<CdRipTrack[]>([]);
  let cdRawOutput = $state("");
  let cdOutputFolder = $state<string | null>(null);
  let lastRippedFolder = $state<string | null>(null);
  let isLookingUpCdMetadata = $state(false);
  let cdMetadataError = $state<string | null>(null);
  let cdMetadataMessage = $state<string | null>(null);
  let cdMetadataResults = $state<CdMetadataRelease[]>([]);
  let selectedCdReleaseId = $state<string | null>(null);
  let cdRipMetadata = $state<CdRipMetadata | null>(null);
  let cdRipMetadataSnapshot = $state<CdRipMetadata | null>(null);
  let cdDiscId = $state<string | null>(null);
  let isLookingUpCdCover = $state(false);
  let cdCoverError = $state<string | null>(null);
  let cdCoverMessage = $state<string | null>(null);
  let playbackError = $state<string | null>(null);
  let genreEditError = $state<string | null>(null);
  let genreEditMessage = $state<string | null>(null);
  let scannedFolder = $state<string | null>(null);
  let lastScannedAt = $state<number | null>(null);
  let scanCount = $state<number | null>(null);
  let hasLoadedCache = $state(false);
  let playlists = $state<Playlist[]>([]);
  let currentTrack = $state<Track | null>(null);
  let currentTrackIndex = $state<number | null>(null);
  let playbackQueue = $state<Track[]>([]);
  let currentQueueIndex = $state<number | null>(null);
  let isQueueOpen = $state(false);
  let isShuffleEnabled = $state(false);
  let shuffledQueueOrder = $state<number[]>([]);
  let repeatMode = $state<RepeatMode>("off");
  let isHandlingTrackEnd = $state(false);
  let mainElement: HTMLElement | undefined = $state();
  let activeView = $state("Home");
  let selectedAlbumId = $state<string | null>(null);
  let selectedArtistName = $state<string | null>(null);
  let selectedGenreName = $state<string | null>(null);
  let isLikedSongsOpen = $state(false);
  let isMixBuilderOpen = $state(false);
  let isLibraryHealthOpen = $state(false);
  let selectedPlaylistId = $state<string | null>(null);
  let isShortcutHelpOpen = $state(false);
  let searchQuery = $state("");
  let playlistPendingDelete = $state<Playlist | null>(null);
  let isDeletingPlaylist = $state(false);
  let playlistNameDraft = $state("");
  let playlistMessage = $state<string | null>(null);
  let playlistError = $state<string | null>(null);
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
  let countedPlaybackTrackId = $state<string | null>(null);
  let playbackSessionTrackId: string | null = null;
  let playbackSessionListenedSeconds = 0;
  let playbackSessionStartedAtMs: number | null = null;
  let positionSeconds = $state(0);
  let durationSeconds = $state<number | null>(null);
  let volume = $state(1);
  let contextMenu = $state<ContextMenuState | null>(null);
  let currentLyrics = $state<TrackLyrics | null>(null);
  let isLoadingLyrics = $state(false);
  let isAutoFindingLyrics = $state(false);
  let lyricsLookupMessage = $state<string | null>(null);
  let lyricsLookupError = $state<string | null>(null);
  let shortcutModalElement: HTMLElement | undefined = $state();
  let deletePlaylistModalElement: HTMLElement | undefined = $state();
  let lyricsPanelElement: HTMLElement | undefined = $state();
  let albumGenreInput: HTMLInputElement | undefined = $state();
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
  let recentlyPlayedTracks = $derived(recentlyPlayed(tracks).slice(0, 8));
  let mostPlayedTracks = $derived(mostPlayed(tracks).slice(0, 8));
  let statsTopTracks = $derived(mostPlayed(tracks).slice(0, 10));
  let statsRecentlyPlayedTracks = $derived(recentlyPlayed(tracks).slice(0, 10));
  let statsTopArtists = $derived(buildTopArtistStats(tracks, displayArtists).slice(0, 8));
  let statsTopAlbums = $derived(buildTopAlbumStats(tracks, displayAlbums).slice(0, 8));
  let statsTopGenres = $derived(buildTopGenreStats(tracks, displayGenres).slice(0, 8));
  let statsTotalPlays = $derived(tracks.reduce((total, track) => total + track.playCount, 0));
  let statsRecentlyPlayedCount = $derived(tracks.filter((track) => track.lastPlayedAt !== null).length);
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
  let libraryTracksById = $derived(new Map(tracks.map((track) => [track.id, track])));
  let isHomeSearchActive = $derived(activeView === "Home" && Boolean(normalizedSearchQuery));
  let hasSearchResults = $derived(
    searchTracks.length > 0 || searchAlbums.length > 0 || searchArtists.length > 0 || searchGenres.length > 0,
  );
  let selectedAlbum = $derived(displayAlbums.find((album) => album.id === selectedAlbumId) ?? null);
  let selectedArtist = $derived(displayArtists.find((artist) => artist.name === selectedArtistName) ?? null);
  let selectedGenre = $derived(displayGenres.find((genre) => genre.name === selectedGenreName) ?? null);
  let selectedPlaylist = $derived(playlists.find((playlist) => playlist.id === selectedPlaylistId) ?? null);
  let selectedVideo = $derived(videos.find((video) => video.id === selectedVideoId) ?? null);
  let selectedVideoSrc = $derived(localImageSource(selectedVideo?.filePath));
  let selectedVideoThumbnailSrc = $derived(localImageSource(selectedVideo?.thumbnailPath));
  let selectedCdRelease = $derived(cdMetadataResults.find((release) => release.id === selectedCdReleaseId) ?? null);
  let selectedCdCover = $derived(activeCdRipMetadata()?.cover ?? null);
  let selectedCdCoverSrc = $derived(localImageSource(selectedCdCover?.path));
  let selectedPlaylistTracks = $derived(selectedPlaylist ? tracksForPlaylist(selectedPlaylist) : []);
  let filteredLikedTracks = $derived(searchFilterTracks(favoriteTracks, normalizedSearchQuery));
  let selectedPlaylistSearchTracks = $derived(
    selectedPlaylist && normalizedSearchQuery
      ? selectedPlaylistTracks.filter((track) => playlistTrackMatchesSearch(track, normalizedSearchQuery))
      : selectedPlaylistTracks,
  );
  let selectedAlbumTracks = $derived(
    selectedAlbum ? tracksForAlbum(selectedAlbum) : [],
  );
  let selectedAlbumSearchTracks = $derived(searchFilterAlbumTracks(selectedAlbumTracks, normalizedSearchQuery));
  let selectedAlbumDiscGroups = $derived(albumDiscGroups(selectedAlbumSearchTracks));
  let selectedAlbumIsMultiDisc = $derived(albumHasMultipleDiscs(selectedAlbumSearchTracks));
  let selectedAlbumDurationLabel = $derived(albumTotalDurationLabel(selectedAlbumTracks));
  let selectedAlbumFormatSummary = $derived(albumFormatSummary(selectedAlbumTracks));
  let selectedArtistTracks = $derived(
    selectedArtist ? tracks.filter((track) => artistNameForTrack(track) === selectedArtist.name) : [],
  );
  let selectedArtistAlbums = $derived(
    selectedArtist ? sortedAlbums.filter((album) => album.artist === selectedArtist.name) : [],
  );
  let selectedArtistSearchTracks = $derived(searchFilterArtistTracks(selectedArtistTracks, normalizedSearchQuery));
  let selectedArtistSearchAlbums = $derived(searchFilterAlbums(selectedArtistAlbums, normalizedSearchQuery));
  let selectedGenreTracks = $derived(
    selectedGenre ? tracks.filter((track) => trackGenres(track).includes(selectedGenre.name)) : [],
  );
  let selectedGenreAlbums = $derived(selectedGenre ? albumsForTracks(selectedGenreTracks, sortedAlbums) : []);
  let selectedGenreArtists = $derived(selectedGenre ? buildArtists(selectedGenreTracks) : []);
  let selectedGenreSearchTracks = $derived(searchFilterGenreTracks(selectedGenreTracks, normalizedSearchQuery));
  let selectedGenreSearchAlbums = $derived(searchFilterGenreAlbums(selectedGenreAlbums, normalizedSearchQuery));
  let selectedGenreSearchArtists = $derived(searchFilterArtists(selectedGenreArtists, normalizedSearchQuery));
  let visibleSongTracks = $derived(searchFilterTracks(filteredSongTracks, normalizedSearchQuery));
  let visibleAlbums = $derived(searchFilterAlbums(sortedAlbums, normalizedSearchQuery));
  let visibleArtists = $derived(searchFilterArtists(sortedArtists, normalizedSearchQuery));
  let visibleGenres = $derived(searchFilterGenres(sortedGenres, normalizedSearchQuery));
  let normalizedVideoSearchQuery = $derived(normalizeSearch(videoSearchQuery));
  let visibleVideos = $derived(searchFilterVideos(sortVideos(videos, videoSort, videoSortDirection), normalizedVideoSearchQuery));
  let videoLibraryStats = $derived([
    { label: "Videos", value: String(videos.length) },
    { label: "Partially watched", value: String(videos.filter((video) => video.lastPositionSeconds > 0).length) },
    { label: "Played", value: String(videos.filter((video) => video.playCount > 0).length) },
    { label: "Last scan", value: lastVideoScannedAt ? formatDateTime(lastVideoScannedAt) : "Not available" },
  ]);
  let selectedAlbumGenreText = $derived(genreDisplayForTracks(selectedAlbumTracks));
  let selectedArtistGenreText = $derived(genreDisplayForTracks(selectedArtistTracks));
  let selectedPlaylistMissingTrackCount = $derived(selectedPlaylist ? missingTrackCountForPlaylist(selectedPlaylist) : 0);
  let canCreatePlaylist = $derived(normalizePlaylistName(playlistNameDraft).length > 0);
  let libraryDiagnostics = $derived(buildLibraryDiagnostics(tracks, displayAlbums, displayArtists));
  let libraryHealthIssueCount = $derived(libraryHealthTotalIssueCount(libraryDiagnostics));
  let librarySettingsStats = $derived([
    { label: "Tracks", value: String(hasLoadedCache ? tracks.length : 0) },
    { label: "Albums", value: String(hasLoadedCache ? displayAlbums.length : 0) },
    { label: "Artists", value: String(hasLoadedCache ? displayArtists.length : 0) },
    { label: "Genres", value: String(hasLoadedCache ? displayGenres.length : 0) },
  ]);
  let queueLengthLabel = $derived(`${playbackQueue.length} ${playbackQueue.length === 1 ? "track" : "tracks"}`);
  let volumePercentLabel = $derived(`${Math.round(volume * 100)}%`);
  let lastScanLabel = $derived(lastScannedAt ? formatDateTime(lastScannedAt) : "Not available");
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
  let currentTrackGenres = $derived(currentTrack ? trackGenres(currentTrack).filter((genre) => genre !== "Unknown Genre") : []);
  let currentTrackCoverArtSrc = $derived(localImageSource(currentTrack?.coverArtPath));
  let currentTrackDuration = $derived(durationSeconds ?? currentTrack?.durationSeconds ?? null);
  let syncedLyricLines = $derived(currentLyrics?.kind === "synced" ? parseLrcLyrics(currentLyrics.text) : []);
  let activeLyricIndex = $derived(activeSyncedLyricIndex(syncedLyricLines, positionSeconds));
  let lyricsBadgeLabel = $derived(currentLyrics ? lyricsKindLabel(currentLyrics) : null);
  let cachedLyricsLabel = $derived(currentLyrics?.source === "lrclib" ? cachedLyricsStatus(currentLyrics) : null);
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
  let lyricsRequestId = 0;

  $effect(() => {
    const hasSearchQuery = normalizedSearchQuery.length > 0;

    if (hasSearchQuery && !hadSearchQuery) {
      mainElement?.scrollTo({ top: 0 });
    }

    hadSearchQuery = hasSearchQuery;
  });

  $effect(() => {
    if (!isShortcutHelpOpen) {
      return;
    }

    window.requestAnimationFrame(() => {
      shortcutModalElement?.focus();
    });
  });

  $effect(() => {
    if (!playlistPendingDelete) {
      return;
    }

    window.requestAnimationFrame(() => {
      deletePlaylistModalElement?.focus();
    });
  });

  $effect(() => {
    void loadCurrentTrackLyrics(currentTrack?.filePath ?? null);
  });

  $effect(() => {
    if (activeLyricIndex < 0 || !lyricsPanelElement) {
      return;
    }

    const activeLine = lyricsPanelElement.querySelector<HTMLElement>("[data-active='true']");
    activeLine?.scrollIntoView({ block: "center", behavior: "smooth" });
  });

  $effect(() => {
    if (!availableFormats.includes(songFormatFilter)) {
      songFormatFilter = "All";
    }
  });

  onMount(() => {
    void loadLibraryCache();
    void loadVideoLibrary();
    const mprisUnlisteners = [
      listen("mpris-play-pause", () => void handleTogglePlayback()),
      listen("mpris-play", () => void handleMprisPlay()),
      listen("mpris-pause", () => void handleMprisPause()),
      listen("mpris-stop", () => void handleMprisStop()),
      listen("mpris-next", () => void handleNextTrack()),
      listen("mpris-previous", () => void handlePreviousTrack()),
      listen<number>("mpris-seek", (event) => void handleSeek(event.payload)),
      listen<CdRipEvent>("cd-rip-started", (event) => handleCdRipStarted(event.payload)),
      listen<CdRipEvent>("cd-rip-track-started", (event) => handleCdRipTrackStarted(event.payload)),
      listen<CdRipEvent>("cd-rip-track-finished", (event) => handleCdRipTrackFinished(event.payload)),
      listen<CdRipEvent>("cd-rip-track-error", (event) => handleCdRipTrackError(event.payload)),
      listen<CdRipEvent>("cd-rip-finished", (event) => handleCdRipFinished(event.payload)),
    ];

    const statusIntervalId = window.setInterval(async () => {
      if (!currentTrack || (!isPlaying && !hasCurrentTrackEnded)) {
        return;
      }

      try {
        const status = await getPlaybackStatus();
        applyPlaybackStatus(status);
        await handlePlaybackStatusUpdate(status, "status");
      } catch (error) {
        playbackError = error instanceof Error ? error.message : String(error);
      }
    }, 1000);

    const progressIntervalId = window.setInterval(() => {
      if (!currentTrack || !isPlaying) {
        return;
      }

      updatePlaybackListenClock(true);
      void maybeRecordTrackPlay();

      const duration = durationSeconds ?? currentTrack.durationSeconds;
      const nextPosition = positionSeconds + 0.25;
      positionSeconds = duration ? Math.min(nextPosition, duration) : nextPosition;

      if (duration && nextPosition >= duration) {
        void handleTrackEnd("duration");
      }
    }, 250);

    const videoProgressIntervalId = window.setInterval(() => {
      void saveActiveVideoProgress(false);
    }, 5000);

    function handleKeydown(event: KeyboardEvent) {
      if (playlistPendingDelete) {
        if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          closeDeletePlaylistConfirmation();
        }

        return;
      }

      if (isShortcutHelpOpen) {
        if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          closeShortcutHelp();
        }

        return;
      }

      if (event.key === "Escape" && normalizedSearchQuery) {
        event.preventDefault();
        clearSearch();
        return;
      }

      if (shouldIgnoreShortcut(event.target)) {
        return;
      }

      if (isShortcutHelpEvent(event)) {
        event.preventDefault();
        event.stopPropagation();
        openShortcutHelp();
      } else if (event.code === "Space") {
        event.preventDefault();
        event.stopPropagation();
        void handleTogglePlayback();
      } else if (event.key === "ArrowRight") {
        event.preventDefault();
        event.stopPropagation();
        void handleNextTrack();
      } else if (event.key === "ArrowLeft") {
        event.preventDefault();
        event.stopPropagation();
        void handlePreviousTrack();
      }
    }

    window.addEventListener("keydown", handleKeydown, true);

    return () => {
      window.clearInterval(statusIntervalId);
      window.clearInterval(progressIntervalId);
      window.clearInterval(videoProgressIntervalId);
      window.removeEventListener("keydown", handleKeydown, true);
      for (const unlisten of mprisUnlisteners) {
        void unlisten.then((cleanup) => cleanup());
      }
    };
  });

  async function loadLibraryCache() {
    try {
      const cache = await getLibraryCache();
      tracks = cache.tracks;
      playlists = cache.playlists;
      scannedFolder = cache.lastScannedFolder;
      lastScannedAt = cache.lastScannedAt;
      scanCount = cache.tracks.length;
      hasLoadedCache = true;
    } catch (error) {
      scanError = error instanceof Error ? error.message : String(error);
      hasLoadedCache = true;
      scanCount = 0;
      lastScannedAt = null;
    }
  }

  async function loadVideoLibrary() {
    try {
      applyVideoLibrary(await getVideoLibrary());
    } catch (error) {
      videoError = error instanceof Error ? error.message : String(error);
    }
  }

  function applyVideoLibrary(library: VideoLibrary) {
    videos = library.videos;
    videoFolder = library.lastVideoFolder;
    lastVideoScannedAt = library.lastVideoScannedAt;

    if (selectedVideoId && !videos.some((video) => video.id === selectedVideoId)) {
      selectedVideoId = null;
      isEditingVideo = false;
    }
  }

  async function loadCurrentTrackLyrics(trackPath: string | null) {
    const requestId = ++lyricsRequestId;

    if (!trackPath) {
      currentLyrics = null;
      isLoadingLyrics = false;
      isAutoFindingLyrics = false;
      lyricsLookupMessage = null;
      lyricsLookupError = null;
      return;
    }

    isLoadingLyrics = true;
    isAutoFindingLyrics = false;
    lyricsLookupMessage = null;
    lyricsLookupError = null;

    try {
      const lyrics = await readTrackLyrics(trackPath);

      if (requestId === lyricsRequestId) {
        currentLyrics = lyrics;
      }
    } catch {
      if (requestId === lyricsRequestId) {
        currentLyrics = null;
      }
    } finally {
      if (requestId === lyricsRequestId) {
        isLoadingLyrics = false;
      }
    }
  }

  async function handleAutoFindLyrics(replaceCached = false) {
    if (!currentTrack || isAutoFindingLyrics || (currentLyrics && !replaceCached)) {
      return;
    }

    isAutoFindingLyrics = true;
    lyricsLookupMessage = replaceCached ? "Searching LRCLIB for replacement lyrics..." : "Searching LRCLIB...";
    lyricsLookupError = null;

    try {
      const result = await autoFindTrackLyrics(currentTrack.filePath, replaceCached);

      if (result.lyrics) {
        currentLyrics = result.lyrics;
      }

      if (result.status === "synced") {
        lyricsLookupMessage = "Synced lyrics found.";
      } else if (result.status === "plain") {
        lyricsLookupMessage = "Plain lyrics found.";
      } else if (result.status === "existing") {
        lyricsLookupMessage = "Lyrics are already available for this track.";
      } else {
        lyricsLookupMessage = "No lyrics found on LRCLIB.";
      }
    } catch (error) {
      lyricsLookupMessage = null;
      lyricsLookupError = error instanceof Error ? error.message : "Could not search LRCLIB. Check your connection.";
    } finally {
      isAutoFindingLyrics = false;
    }
  }

  async function handleScanLibrary() {
    scanError = null;

    try {
      const folder = await chooseLibraryFolder();

      if (!folder) {
        return;
      }

      await scanFolderIntoLibrary(folder);
    } catch (error) {
      scanError = error instanceof Error ? error.message : String(error);
      scanCount = null;
    } finally {
      isScanning = false;
    }
  }

  async function scanFolderIntoLibrary(folder: string) {
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
    isLibraryHealthOpen = false;
    selectedPlaylistId = null;
    clearMixSelection();
    searchQuery = "";
    currentTrackIndex = null;
    hasCurrentTrackEnded = false;
    resetPlaybackListenSession(null);
    playbackQueue = [];
    currentQueueIndex = null;
    shuffledQueueOrder = [];
    isQueueOpen = false;
    hasLoadedCache = true;

    const scannedTracks = await scanLibrary(folder);
    tracks = scannedTracks;
    scanCount = scannedTracks.length;
    lastScannedAt = Math.floor(Date.now() / 1000);
  }

  async function handleScanRippedFolder() {
    if (!lastRippedFolder || isScanning) {
      return;
    }

    scanError = null;
    cdRipError = null;
    cdRipMessage = "Scanning ripped folder...";

    try {
      await scanFolderIntoLibrary(lastRippedFolder);
      cdRipMessage = "Ripped folder scanned into Cassette.";
    } catch (error) {
      cdRipMessage = null;
      cdRipError = error instanceof Error ? error.message : String(error);
      scanCount = null;
    } finally {
      isScanning = false;
    }
  }

  async function handleAddVideoFolder() {
    if (isScanningVideos) {
      return;
    }

    videoError = null;
    videoMessage = null;

    try {
      const folder = await chooseVideoFolder();

      if (!folder) {
        return;
      }

      await scanFolderIntoVideos(folder);
    } catch (error) {
      videoMessage = null;
      videoError = error instanceof Error ? error.message : String(error);
    } finally {
      isScanningVideos = false;
    }
  }

  async function handleRescanVideos() {
    if (isScanningVideos) {
      return;
    }

    if (!videoFolder) {
      await handleAddVideoFolder();
      return;
    }

    videoError = null;
    videoMessage = null;

    try {
      await scanFolderIntoVideos(videoFolder);
    } catch (error) {
      videoMessage = null;
      videoError = error instanceof Error ? error.message : String(error);
    } finally {
      isScanningVideos = false;
    }
  }

  async function scanFolderIntoVideos(folder: string) {
    isScanningVideos = true;
    videoFolder = folder;
    videoMessage = "Scanning videos...";
    const library = await scanVideoFolder(folder);
    applyVideoLibrary(library);
    videoMessage = `${library.videos.length} ${library.videos.length === 1 ? "video" : "videos"} available.`;
  }

  function handleVideoSelect(video: VideoEntry) {
    selectedVideoId = video.id;
    isEditingVideo = false;
    videoEditDraft = draftFromVideo(video);
    videoPlaybackError = null;
    videoPlaybackMessage = null;
    mainElement?.scrollTo({ top: 0 });
  }

  function handleBackToVideos() {
    void saveActiveVideoProgress(true);
    selectedVideoId = null;
    isEditingVideo = false;
    videoPlaybackError = null;
    videoPlaybackMessage = null;
    mainElement?.scrollTo({ top: 0 });
  }

  function handleEditVideoInfo() {
    if (!selectedVideo) {
      return;
    }

    videoEditDraft = draftFromVideo(selectedVideo);
    isEditingVideo = true;
    videoError = null;
    videoMessage = null;
  }

  function handleCancelVideoEdit() {
    isEditingVideo = false;
    videoEditDraft = selectedVideo ? draftFromVideo(selectedVideo) : emptyVideoEditDraft();
  }

  async function handleSaveVideoInfo() {
    const video = selectedVideo;

    if (!video || isSavingVideoInfo) {
      return;
    }

    const info = videoInfoUpdateFromDraft(videoEditDraft);

    if (!info.title.trim()) {
      videoError = "Video title is required.";
      return;
    }

    isSavingVideoInfo = true;
    videoError = null;
    videoMessage = null;

    try {
      applyUpdatedVideo(await updateVideoInfo(video.id, info));
      isEditingVideo = false;
      videoMessage = "Video info saved.";
    } catch (error) {
      videoError = error instanceof Error ? error.message : String(error);
    } finally {
      isSavingVideoInfo = false;
    }
  }

  async function handleResetVideoProgress() {
    const video = selectedVideo;

    if (!video) {
      return;
    }

    videoPlaybackError = null;
    videoPlaybackMessage = null;

    if (videoElement) {
      videoElement.currentTime = 0;
    }

    try {
      applyUpdatedVideo(await updateVideoProgress(video.id, 0, false));
      resetVideoSession(video.id);
      videoPlaybackMessage = "Progress reset.";
    } catch (error) {
      videoPlaybackError = error instanceof Error ? error.message : String(error);
    }
  }

  async function handleVideoPlayFromStart() {
    await playSelectedVideo(0);
  }

  async function handleVideoResume() {
    const video = selectedVideo;

    if (!video) {
      return;
    }

    await playSelectedVideo(video.lastPositionSeconds);
  }

  async function playSelectedVideo(positionSeconds: number) {
    const video = selectedVideo;

    if (!video || !videoElement) {
      return;
    }

    videoPlaybackError = null;
    videoPlaybackMessage = null;

    try {
      if (isPlaying) {
        applyPlaybackStatus(await pausePlayback());
      }

      videoElement.currentTime = Math.max(0, Math.min(positionSeconds, video.durationSeconds ?? positionSeconds));
      await videoElement.play();
    } catch (error) {
      videoPlaybackError = error instanceof Error ? error.message : String(error);
    }
  }

  async function handleOpenVideoExternal() {
    const video = selectedVideo;

    if (!video) {
      return;
    }

    videoPlaybackError = null;
    videoPlaybackMessage = null;

    try {
      if (isPlaying) {
        applyPlaybackStatus(await pausePlayback());
      }

      await openPath(video.filePath);
      videoPlaybackMessage = "Opened in external player.";
    } catch (error) {
      videoPlaybackError = error instanceof Error ? error.message : String(error);
    }
  }

  function handleVideoPlay() {
    const video = selectedVideo;

    if (!video || !videoElement) {
      return;
    }

    if (videoSessionVideoId !== video.id) {
      resetVideoSession(video.id);
    }

    videoObservedPositionSeconds = videoElement.currentTime;
  }

  function handleVideoPause() {
    void saveActiveVideoProgress(true);
  }

  function handleVideoEnded() {
    const video = selectedVideo;

    if (!video || !videoElement) {
      return;
    }

    videoElement.currentTime = 0;
    const shouldIncrementPlayCount = !videoPlayCountRecorded;
    resetVideoSession(video.id);
    void saveVideoProgress(video, 0, shouldIncrementPlayCount);
  }

  function handleVideoTimeUpdate() {
    const video = selectedVideo;

    if (!video || !videoElement) {
      return;
    }

    if (videoSessionVideoId !== video.id) {
      resetVideoSession(video.id);
    }

    const currentPosition = videoElement.currentTime;
    const delta = currentPosition - videoObservedPositionSeconds;

    if (delta > 0 && delta < 10) {
      videoSessionWatchedSeconds += delta;
    }

    videoObservedPositionSeconds = currentPosition;
    void saveActiveVideoProgress(false);
  }

  function handleVideoLoadedMetadata() {
    const video = selectedVideo;

    if (!video || !videoElement || video.lastPositionSeconds <= 0) {
      return;
    }

    const duration = videoElement.duration;
    const safePosition = Number.isFinite(duration)
      ? Math.min(video.lastPositionSeconds, Math.max(0, duration - 1))
      : video.lastPositionSeconds;
    videoElement.currentTime = safePosition;
  }

  async function saveActiveVideoProgress(force: boolean) {
    const video = selectedVideo;

    if (!video || !videoElement || videoElement.ended || (!force && videoElement.paused)) {
      return;
    }

    const now = Date.now();

    if (!force && now - videoLastProgressSaveMs < 5000) {
      return;
    }

    videoLastProgressSaveMs = now;
    const threshold = video.durationSeconds ? Math.min(30, video.durationSeconds * 0.5) : 30;
    const shouldIncrementPlayCount = !videoPlayCountRecorded && videoSessionWatchedSeconds >= threshold;

    await saveVideoProgress(video, Math.floor(videoElement.currentTime), shouldIncrementPlayCount);

    if (shouldIncrementPlayCount) {
      videoPlayCountRecorded = true;
    }
  }

  async function saveVideoProgress(video: VideoEntry, positionSeconds: number, incrementPlayCount: boolean) {
    try {
      applyUpdatedVideo(await updateVideoProgress(video.id, Math.max(0, positionSeconds), incrementPlayCount));
    } catch (error) {
      videoPlaybackError = error instanceof Error ? error.message : String(error);
    }
  }

  function resetVideoSession(videoId: string | null) {
    videoSessionVideoId = videoId;
    videoSessionWatchedSeconds = 0;
    videoObservedPositionSeconds = 0;
    videoLastProgressSaveMs = 0;
    videoPlayCountRecorded = false;
  }

  function applyUpdatedVideo(updatedVideo: VideoEntry) {
    videos = videos.some((video) => video.id === updatedVideo.id)
      ? videos.map((video) => video.id === updatedVideo.id ? updatedVideo : video)
      : [...videos, updatedVideo];

    if (selectedVideoId === updatedVideo.id) {
      selectedVideoId = updatedVideo.id;
    }
  }

  function updateVideoDraftField(key: keyof VideoEditDraft, value: string) {
    videoEditDraft = { ...videoEditDraft, [key]: value };
  }

  function emptyVideoEditDraft(): VideoEditDraft {
    return {
      title: "",
      artist: "",
      showTitle: "",
      albumOrRelease: "",
      year: "",
      venue: "",
      city: "",
      country: "",
    };
  }

  function draftFromVideo(video: VideoEntry): VideoEditDraft {
    return {
      title: video.title,
      artist: video.artist ?? "",
      showTitle: video.showTitle ?? "",
      albumOrRelease: video.albumOrRelease ?? "",
      year: video.year?.toString() ?? "",
      venue: video.venue ?? "",
      city: video.city ?? "",
      country: video.country ?? "",
    };
  }

  function videoInfoUpdateFromDraft(draft: VideoEditDraft): VideoInfoUpdate {
    const parsedYear = Number.parseInt(draft.year, 10);

    return {
      title: draft.title.trim(),
      artist: nullableDraftValue(draft.artist),
      showTitle: nullableDraftValue(draft.showTitle),
      albumOrRelease: nullableDraftValue(draft.albumOrRelease),
      year: Number.isFinite(parsedYear) && parsedYear > 0 ? parsedYear : null,
      venue: nullableDraftValue(draft.venue),
      city: nullableDraftValue(draft.city),
      country: nullableDraftValue(draft.country),
    };
  }

  function nullableDraftValue(value: string) {
    const trimmed = value.trim();
    return trimmed.length > 0 ? trimmed : null;
  }

  function videoDetailLine(video: VideoEntry) {
    return [
      video.artist ?? "Unknown Artist",
      video.showTitle ?? video.albumOrRelease,
      video.year?.toString(),
      videoLocationLine(video),
    ].filter(Boolean).join(" · ");
  }

  function videoLocationLine(video: VideoEntry) {
    return [video.venue, video.city, video.country].filter(Boolean).join(" · ");
  }

  function videoCardDetail(video: VideoEntry) {
    return [
      video.artist ?? "Unknown Artist",
      video.year?.toString(),
      video.showTitle ?? video.albumOrRelease,
    ].filter(Boolean).join(" · ");
  }

  function videoProgressPercent(video: VideoEntry) {
    if (!video.durationSeconds || video.durationSeconds <= 0) {
      return 0;
    }

    return Math.max(0, Math.min(100, (video.lastPositionSeconds / video.durationSeconds) * 100));
  }

  async function handleDetectCd() {
    if (isDetectingCd || isRippingCd) {
      return;
    }

    isDetectingCd = true;
    cdRipError = null;
    cdRipMessage = null;
    resetCdMetadataLookup();
    cdRipMetadataSnapshot = null;
    lastRippedFolder = null;

    try {
      applyCdDetection(await detectAudioCd());
      if (audioCdFound && cdTracks.length > 0) {
        cdRipMessage = `Detected ${cdTracks.length} ${cdTracks.length === 1 ? "track" : "tracks"}.`;
      } else {
        cdRipMessage = null;
      }
    } catch (error) {
      cdRipError = error instanceof Error ? error.message : String(error);
    } finally {
      isDetectingCd = false;
    }
  }

  function applyCdDetection(result: CdDetectResult) {
    cdDriveFound = result.driveFound;
    audioCdFound = result.discFound;
    cdTracks = result.tracks.map((track) => ({
      ...track,
      status: "pending",
      outputFilename: track.outputFilename ?? cdTrackOutputFilename(track.number),
      error: null,
    }));
    cdRawOutput = result.rawOutput;
    cdRipError = result.error;
    cdRipMetadata = defaultCdRipMetadata(cdTracks, cdRipMetadata);
  }

  function resetCdMetadataLookup() {
    cdMetadataError = null;
    cdMetadataMessage = null;
    cdMetadataResults = [];
    selectedCdReleaseId = null;
    cdDiscId = null;
    cdCoverError = null;
    cdCoverMessage = null;
    isLookingUpCdCover = false;
  }

  async function handleLookupCdMetadata() {
    if (isLookingUpCdMetadata || isRippingCd) {
      return;
    }

    if (cdTracks.length === 0 || !audioCdFound) {
      await handleDetectCd();
      if (cdTracks.length === 0 || !audioCdFound) {
        cdMetadataError = cdRipError ?? "Detect an audio CD before looking up metadata.";
        return;
      }
    }

    isLookingUpCdMetadata = true;
    cdMetadataError = null;
    cdMetadataMessage = "Looking up metadata...";
    cdMetadataResults = [];
    selectedCdReleaseId = null;

    try {
      applyCdMetadataLookup(await lookupCdMetadata());
    } catch (error) {
      cdMetadataMessage = null;
      cdMetadataError = error instanceof Error ? error.message : String(error);
    } finally {
      isLookingUpCdMetadata = false;
    }
  }

  function applyCdMetadataLookup(result: CdMetadataLookupResult) {
    cdDiscId = result.discId;
    cdMetadataResults = result.releases;
    cdMetadataError = result.error;

    if (result.releases.length === 0) {
      cdMetadataError = null;
      cdMetadataMessage = "No metadata found for this Disc ID.";
      cdRipMetadata = cdRipMetadata ?? defaultCdRipMetadata(cdTracks, null);
      return;
    }

    cdMetadataMessage = result.releases.length === 1
      ? "One MusicBrainz release found."
      : `${result.releases.length} MusicBrainz releases found. Choose the matching release.`;
    selectCdMetadataRelease(result.releases[0]);
  }

  function selectCdMetadataRelease(release: CdMetadataRelease) {
    if (isRippingCd) {
      return;
    }

    const existingManualCover = cdRipMetadata?.cover?.source === "manual" ? cdRipMetadata.cover : null;
    selectedCdReleaseId = release.id;
    cdRipMetadata = {
      ...metadataFromRelease(release, cdTracks),
      cover: existingManualCover,
    };
    cdMetadataError = null;
    cdCoverError = null;
    cdCoverMessage = existingManualCover ? "Manual cover selected" : null;

    if (!existingManualCover) {
      void handleLookupCdCover(release.id);
    }
  }

  async function handleLookupCdCover(releaseId: string) {
    if (isLookingUpCdCover || isRippingCd) {
      return;
    }

    isLookingUpCdCover = true;
    cdCoverError = null;
    cdCoverMessage = "Loading cover...";

    try {
      applyCdCoverLookup(await lookupCdCover(releaseId), "cover-art-archive");
    } catch (error) {
      cdCoverMessage = null;
      cdCoverError = error instanceof Error ? error.message : String(error);
    } finally {
      isLookingUpCdCover = false;
    }
  }

  async function handleChooseCoverImage() {
    if (isRippingCd) {
      return;
    }

    const path = await chooseCoverImage();
    if (!path) {
      return;
    }

    cdCoverError = null;
    cdCoverMessage = "Loading cover...";

    try {
      applyCdCoverLookup(await inspectCoverImage(path), "manual");
    } catch (error) {
      cdCoverMessage = null;
      cdCoverError = error instanceof Error ? error.message : String(error);
    }
  }

  function applyCdCoverLookup(result: CdCoverLookupResult, source: CdRipCover["source"]) {
    const metadata = cdRipMetadata ?? defaultCdRipMetadata(cdTracks, null);

    if (!result.found || !result.cover) {
      cdRipMetadata = { ...metadata, cover: source === "manual" ? metadata.cover : null };
      cdCoverError = null;
      cdCoverMessage = result.message ?? "No cover art found";
      return;
    }

    cdRipMetadata = {
      ...metadata,
      cover: result.cover,
    };
    cdCoverError = null;
    cdCoverMessage = source === "manual" ? "Manual cover selected" : "Cover found";
  }

  async function handleChooseCdOutputFolder() {
    if (isRippingCd) {
      return;
    }

    const folder = await chooseOutputFolder();
    if (folder) {
      cdOutputFolder = folder;
      cdRipError = null;
    }
  }

  async function handleRipCd() {
    if (isRippingCd || isDetectingCd) {
      return;
    }

    if (!cdOutputFolder) {
      cdRipError = "Choose an output folder before ripping the CD.";
      return;
    }

    if (cdTracks.length === 0 || !audioCdFound) {
      await handleDetectCd();
      if (cdTracks.length === 0 || !audioCdFound) {
        cdRipError = cdRipError ?? "No audio CD tracks are available to rip.";
        return;
      }
    }

    isRippingCd = true;
    const ripMetadataSnapshot = cloneCdRipMetadata(metadataForRip());
    cdRipMetadataSnapshot = ripMetadataSnapshot;
    cdRipError = null;
    cdRipMessage = "Starting CD rip...";
    lastRippedFolder = null;
    cdTracks = cdTracks.map((track) => ({ ...track, status: "pending", error: null }));

    try {
      const result = await ripCdToFlac(cdOutputFolder, ripMetadataSnapshot);
      cdTracks = result.tracks;
      lastRippedFolder = result.outputFolder;
      cdRipMessage = result.tracks.some((track) => track.status === "error")
        ? "Rip finished with one or more track errors."
        : "Rip complete.";
    } catch (error) {
      cdRipMessage = null;
      cdRipError = error instanceof Error ? error.message : String(error);
    } finally {
      isRippingCd = false;
    }
  }

  function handleCdRipStarted(payload: CdRipEvent) {
    if (payload.outputFolder) {
      lastRippedFolder = payload.outputFolder;
    }
    cdRipMessage = payload.message ?? "CD rip started.";
    cdRipError = null;
  }

  function handleCdRipTrackStarted(payload: CdRipEvent) {
    updateCdTrackFromEvent(payload, "ripping");
    cdRipMessage = payload.message ?? "Ripping track...";
  }

  function handleCdRipTrackFinished(payload: CdRipEvent) {
    updateCdTrackFromEvent(payload, "done");
    cdRipMessage = payload.message ?? "Track finished.";
  }

  function handleCdRipTrackError(payload: CdRipEvent) {
    updateCdTrackFromEvent(payload, "error");
    cdRipError = payload.message ?? "A track failed to rip.";
  }

  function handleCdRipFinished(payload: CdRipEvent) {
    if (payload.outputFolder) {
      lastRippedFolder = payload.outputFolder;
    }
    cdRipMessage = payload.message ?? "Rip complete.";
  }

  function updateCdTrackFromEvent(payload: CdRipEvent, status: NonNullable<CdRipTrack["status"]>) {
    if (!payload.trackNumber) {
      return;
    }

    cdTracks = cdTracks.map((track) => {
      if (track.number !== payload.trackNumber) {
        return track;
      }

      return {
        ...track,
        status,
        outputFilename: payload.outputFilename ?? track.outputFilename,
        error: status === "error" ? (payload.message ?? "Track failed.") : null,
      };
    });
  }

  function cdTrackOutputFilename(trackNumber: number) {
    return `${String(trackNumber).padStart(2, "0")} - Track ${String(trackNumber).padStart(2, "0")}.flac`;
  }

  function inputValue(event: Event) {
    return event.currentTarget instanceof HTMLInputElement ? event.currentTarget.value : "";
  }

  function defaultCdRipMetadata(cdTracks: CdRipTrack[], existing: CdRipMetadata | null): CdRipMetadata {
    return {
      albumArtist: existing?.albumArtist ?? "",
      albumTitle: existing?.albumTitle ?? "",
      year: existing?.year ?? "",
      genre: existing?.genre ?? "",
      discNumber: existing?.discNumber ?? null,
      cover: existing?.cover ?? null,
      tracks: cdTracks.map((track) => {
        const existingTrack = existing?.tracks.find((candidate) => candidate.number === track.number);

        return {
          number: track.number,
          title: existingTrack?.title ?? `Track ${String(track.number).padStart(2, "0")}`,
          artist: existingTrack?.artist ?? "",
          discNumber: existingTrack?.discNumber ?? existing?.discNumber ?? null,
        };
      }),
    };
  }

  function metadataFromRelease(release: CdMetadataRelease, cdTracks: CdRipTrack[]): CdRipMetadata {
    return {
      albumArtist: release.artist,
      albumTitle: release.title,
      year: release.year ?? release.date ?? "",
      genre: cdRipMetadata?.genre ?? "",
      discNumber: release.discNumber,
      cover: cdRipMetadata?.cover?.source === "manual" ? cdRipMetadata.cover : null,
      tracks: cdTracks.map((track) => {
        const releaseTrack = release.tracks.find((candidate) => candidate.number === track.number);

        return {
          number: track.number,
          title: releaseTrack?.title ?? `Track ${String(track.number).padStart(2, "0")}`,
          artist: releaseTrack?.artist || release.artist,
          discNumber: releaseTrack?.discNumber ?? release.discNumber,
        };
      }),
    };
  }

  function metadataForRip(): CdRipMetadata | null {
    if (!cdRipMetadata || !metadataHasUserValue(cdRipMetadata)) {
      return null;
    }

    return cdRipMetadata;
  }

  function cloneCdRipMetadata(metadata: CdRipMetadata | null): CdRipMetadata | null {
    if (!metadata) {
      return null;
    }

    return {
      albumArtist: metadata.albumArtist,
      albumTitle: metadata.albumTitle,
      year: metadata.year,
      genre: metadata.genre,
      discNumber: metadata.discNumber,
      cover: metadata.cover ? { ...metadata.cover } : null,
      tracks: metadata.tracks.map((track) => ({ ...track })),
    };
  }

  function metadataHasUserValue(metadata: CdRipMetadata) {
    return Boolean(
      metadata.albumArtist.trim()
      || metadata.albumTitle.trim()
      || metadata.year.trim()
      || metadata.genre.trim()
      || metadata.discNumber
      || metadata.tracks.some((track) => (
        track.artist.trim()
        || track.discNumber
        || track.title.trim() !== `Track ${String(track.number).padStart(2, "0")}`
      )),
    );
  }

  function updateCdAlbumMetadata(key: "albumArtist" | "albumTitle" | "year" | "genre", value: string) {
    if (isRippingCd) {
      return;
    }

    cdRipMetadata = {
      ...(cdRipMetadata ?? defaultCdRipMetadata(cdTracks, null)),
      [key]: value,
    };
  }

  function updateCdDiscNumber(value: string) {
    if (isRippingCd) {
      return;
    }

    const parsed = Number.parseInt(value, 10);
    const discNumber = Number.isFinite(parsed) && parsed > 0 ? parsed : null;
    const metadata = cdRipMetadata ?? defaultCdRipMetadata(cdTracks, null);

    cdRipMetadata = {
      ...metadata,
      discNumber,
      tracks: metadata.tracks.map((track) => ({ ...track, discNumber: track.discNumber ?? discNumber })),
    };
  }

  function updateCdTrackMetadata(trackNumber: number, key: "title" | "artist", value: string) {
    if (isRippingCd) {
      return;
    }

    const metadata = cdRipMetadata ?? defaultCdRipMetadata(cdTracks, null);

    cdRipMetadata = {
      ...metadata,
      tracks: metadata.tracks.map((track) => (
        track.number === trackNumber ? { ...track, [key]: value } : track
      )),
    };
  }

  function cdMetadataTrack(trackNumber: number): CdRipMetadataTrack {
    const metadata = activeCdRipMetadata();

    return metadata?.tracks.find((track) => track.number === trackNumber) ?? {
      number: trackNumber,
      title: `Track ${String(trackNumber).padStart(2, "0")}`,
      artist: "",
      discNumber: metadata?.discNumber ?? null,
    };
  }

  function activeCdRipMetadata() {
    return isRippingCd ? (cdRipMetadataSnapshot ?? cdRipMetadata) : cdRipMetadata;
  }

  function cdTrackOutputFilenamePreview(trackNumber: number) {
    const metadataTrack = cdMetadataTrack(trackNumber);
    const title = metadataTrack.title.trim() || `Track ${String(trackNumber).padStart(2, "0")}`;

    return `${String(trackNumber).padStart(2, "0")} - ${sanitizeFilenamePreview(title, `Track ${String(trackNumber).padStart(2, "0")}`)}.flac`;
  }

  function cdTrackOutputFilenameDisplay(track: CdRipTrack) {
    if (track.status && track.status !== "pending" && track.outputFilename) {
      return track.outputFilename;
    }

    return cdTrackOutputFilenamePreview(track.number);
  }

  function sanitizeFilenamePreview(value: string, fallback: string) {
    let safe = value
      .replace(/[\\/]/g, "-")
      .replace(/[\u0000-\u001f\u007f]/g, "")
      .trim();

    while (safe.includes("  ")) {
      safe = safe.replaceAll("  ", " ");
    }

    safe = safe
      .replace(/^[. ]+|[. ]+$/g, "")
      .split(/\s+/)
      .filter(Boolean)
      .join(" ");

    return safe || fallback;
  }

  function releaseDetail(release: CdMetadataRelease) {
    return [
      release.artist,
      release.year ?? release.date,
      release.country,
      release.format,
      `${release.trackCount} ${release.trackCount === 1 ? "track" : "tracks"}`,
      release.label,
      release.catalogNumber,
    ].filter(Boolean).join(" · ");
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

  function openContextMenu(x: number, y: number, items: ContextMenuItem[]) {
    contextMenu = { x, y, items };
  }

  function closeContextMenu() {
    contextMenu = null;
  }

  function openTrackContextMenu(track: Track, queue: Track[], x: number, y: number) {
    openContextMenu(x, y, [
      { label: "Play", action: () => handleTrackSelect(track, queue) },
      { label: "Play next", action: () => insertTracksNext([track]) },
      { label: "Add to queue", action: () => appendTracksToQueue([track]) },
      ...playlistAddMenuItems(track),
      { label: "Go to artist", action: () => handleTrackArtistSelect(track) },
      { label: "Go to album", action: () => handleTrackAlbumSelect(track) },
      {
        label: track.isFavorite ? "Remove from liked songs" : "Add to liked songs",
        action: () => handleToggleFavorite(track),
      },
    ]);
  }

  function playlistAddMenuItems(track: Track): ContextMenuItem[] {
    return [
      ...playlists.map((playlist) => ({
        label: `Add to ${playlist.name}`,
        action: () => handleAddTrackToPlaylist(playlist.id, track),
      })),
      { label: "Add to new playlist...", action: () => handleCreatePlaylistFromTrack(track) },
    ];
  }

  function openAlbumContextMenu(event: MouseEvent, album: Album) {
    event.preventDefault();
    event.stopPropagation();

    const albumTracks = tracksForAlbum(album);
    const hasArtist = album.artist.trim().length > 0 && album.artist !== "Unknown Artist";

    openContextMenu(event.clientX, event.clientY, [
      { label: "Play album", disabled: albumTracks.length === 0, action: () => playTrackSet(albumTracks) },
      { label: "Shuffle album", disabled: albumTracks.length === 0, action: () => playTrackSet(albumTracks, true) },
      { label: "Add album to queue", disabled: albumTracks.length === 0, action: () => appendTracksToQueue(albumTracks) },
      { label: "Go to artist", disabled: !hasArtist, action: () => selectArtistName(album.artist) },
    ]);
  }

  function openArtistContextMenu(event: MouseEvent, artist: Artist) {
    event.preventDefault();
    event.stopPropagation();

    const artistTracks = tracksForArtist(artist);

    openContextMenu(event.clientX, event.clientY, [
      { label: "Play artist", disabled: artistTracks.length === 0, action: () => playTrackSet(artistTracks) },
      { label: "Shuffle artist", disabled: artistTracks.length === 0, action: () => playTrackSet(artistTracks, true) },
      { label: "Add artist to queue", disabled: artistTracks.length === 0, action: () => appendTracksToQueue(artistTracks) },
    ]);
  }

  function openGenreContextMenu(event: MouseEvent, genre: Genre) {
    event.preventDefault();
    event.stopPropagation();

    const genreTracks = tracksForGenre(genre);

    openContextMenu(event.clientX, event.clientY, [
      { label: "Play genre", disabled: genreTracks.length === 0, action: () => playTrackSet(genreTracks) },
      { label: "Shuffle genre", disabled: genreTracks.length === 0, action: () => playTrackSet(genreTracks, true) },
      { label: "Add genre to queue", disabled: genreTracks.length === 0, action: () => appendTracksToQueue(genreTracks) },
    ]);
  }

  function openPlaylistContextMenu(event: MouseEvent, playlist: Playlist) {
    event.preventDefault();
    event.stopPropagation();

    const playlistTracks = tracksForPlaylist(playlist);

    openContextMenu(event.clientX, event.clientY, [
      { label: "Play playlist", disabled: playlistTracks.length === 0, action: () => playTrackSet(playlistTracks) },
      { label: "Shuffle playlist", disabled: playlistTracks.length === 0, action: () => playTrackSet(playlistTracks, true) },
      { label: "Add playlist to queue", disabled: playlistTracks.length === 0, action: () => appendTracksToQueue(playlistTracks) },
      { label: "Rename playlist", action: () => handleRenamePlaylist(playlist) },
      { label: "Delete playlist", action: () => openDeletePlaylistConfirmation(playlist) },
    ]);
  }

  function handleNavigate(label: string) {
    void saveActiveVideoProgress(true);
    activeView = label;
    selectedAlbumId = null;
    selectedArtistName = null;
    selectedGenreName = null;
    clearGenreEditState();
    isLikedSongsOpen = false;
    isMixBuilderOpen = false;
    isLibraryHealthOpen = false;
    selectedPlaylistId = null;
    selectedVideoId = label === "Live Shows" ? selectedVideoId : null;
    isEditingVideo = false;
    searchQuery = "";
    mainElement?.scrollTo({ top: 0 });
  }

  function handleNowPlayingSelect() {
    handleNavigate("Now Playing");
  }

  function handleHomeSongsViewAll(sortKey: SongSortKey, direction: SortDirection) {
    activeView = "Songs";
    selectedAlbumId = null;
    selectedArtistName = null;
    selectedGenreName = null;
    clearGenreEditState();
    isLikedSongsOpen = false;
    isMixBuilderOpen = false;
    isLibraryHealthOpen = false;
    selectedPlaylistId = null;
    searchQuery = "";
    songSort = sortKey;
    songSortDirection = direction;
    mainElement?.scrollTo({ top: 0 });
  }

  function handleHomeAlbumsViewAll() {
    handleNavigate("Albums");
  }

  function handleHomeArtistsViewAll() {
    handleNavigate("Artists");
  }

  function handleAlbumSelect(album: Album) {
    selectAlbumId(album.id);
  }

  async function handlePlaySelectedAlbum() {
    if (selectedAlbumTracks.length === 0) {
      return;
    }

    await playTrackSet(selectedAlbumTracks);
  }

  async function handleShuffleSelectedAlbum() {
    if (selectedAlbumTracks.length === 0) {
      return;
    }

    await playTrackSet(selectedAlbumTracks, true);
  }

  function handleAddSelectedAlbumToQueue() {
    appendTracksToQueue(selectedAlbumTracks);
  }

  function focusAlbumGenreEditor() {
    albumGenreInput?.focus();
  }

  function handleAlbumTrackSelect(track: Track) {
    void handleTrackSelect(track, selectedAlbumTracks);
  }

  function openAlbumTrackContextMenu(event: MouseEvent, track: Track) {
    event.preventDefault();
    event.stopPropagation();
    openTrackContextMenu(track, selectedAlbumTracks, event.clientX, event.clientY);
  }

  function handleAlbumTrackKeydown(event: KeyboardEvent, track: Track) {
    if (event.target !== event.currentTarget) {
      return;
    }

    if (event.key === "Enter") {
      event.preventDefault();
      event.stopPropagation();
      handleAlbumTrackSelect(track);
    }
  }

  function handleTrackAlbumSelect(track: Track) {
    selectAlbumId(albumIdForTrack(track));
  }

  function handleNowPlayingArtistSelect() {
    if (!currentTrack) {
      return;
    }

    selectArtistName(artistNameForTrack(currentTrack));
  }

  function handleNowPlayingAlbumSelect() {
    if (!currentTrack) {
      return;
    }

    selectAlbumId(albumIdForTrack(currentTrack));
  }

  function handleNowPlayingGenreSelect(genreName: string) {
    const genre = displayGenres.find((candidate) => candidate.name === genreName);

    if (genre) {
      handleGenreSelect(genre);
    }
  }

  function selectAlbumId(albumId: string) {
    searchQuery = "";
    activeView = "Albums";
    selectedAlbumId = albumId;
    selectedArtistName = null;
    selectedGenreName = null;
    clearGenreEditState();
    albumGenreDraft = genreDraftForTracks(tracks.filter((track) => albumIdForTrack(track) === albumId));
    isLikedSongsOpen = false;
    isMixBuilderOpen = false;
    isLibraryHealthOpen = false;
    selectedPlaylistId = null;
    mainElement?.scrollTo({ top: 0 });
  }

  function handleArtistSelect(artist: Artist) {
    selectArtistName(artist.name);
  }

  function handleTrackArtistSelect(track: Track) {
    selectArtistName(artistNameForTrack(track));
  }

  function selectArtistName(artistName: string) {
    searchQuery = "";
    activeView = "Artists";
    selectedArtistName = artistName;
    selectedAlbumId = null;
    selectedGenreName = null;
    clearGenreEditState();
    artistGenreDraft = genreDraftForTracks(tracks.filter((track) => artistNameForTrack(track) === artistName));
    isLikedSongsOpen = false;
    isMixBuilderOpen = false;
    isLibraryHealthOpen = false;
    selectedPlaylistId = null;
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
    isLibraryHealthOpen = false;
    selectedPlaylistId = null;
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
    isLibraryHealthOpen = false;
    selectedPlaylistId = null;
    searchQuery = "";
    playlistMessage = null;
    playlistError = null;
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
    isLibraryHealthOpen = false;
    selectedPlaylistId = null;
    mixMessage = null;
    playlistMessage = null;
    playlistError = null;
    searchQuery = "";
    mainElement?.scrollTo({ top: 0 });
  }

  function handlePlaylistSelect(playlist: Playlist) {
    activeView = "Playlists";
    selectedPlaylistId = playlist.id;
    isLikedSongsOpen = false;
    isMixBuilderOpen = false;
    isLibraryHealthOpen = false;
    searchQuery = "";
    playlistMessage = null;
    playlistError = null;
    mainElement?.scrollTo({ top: 0 });
  }

  async function handleCreatePlaylist() {
    playlistError = null;
    playlistMessage = null;
    const name = playlistNameDraft.trim();

    if (!name) {
      playlistError = "Playlist name is required.";
      return;
    }

    if (playlistNameExists(name)) {
      playlistError = "A playlist with this name already exists.";
      return;
    }

    try {
      const playlist = await createPlaylist(name);
      applyUpdatedPlaylist(playlist);
      playlistNameDraft = "";
      playlistMessage = `${playlist.name} created.`;
    } catch (error) {
      playlistError = error instanceof Error ? error.message : String(error);
    }
  }

  async function handleAddTrackToPlaylist(playlistId: string, track: Track) {
    playlistError = null;
    playlistMessage = null;
    const playlist = playlists.find((candidate) => candidate.id === playlistId);

    if (!playlist) {
      playlistError = "Playlist does not exist.";
      return;
    }

    if (playlist.trackIds.includes(track.id)) {
      playlistMessage = `${track.title} is already in ${playlist.name}.`;
      return;
    }

    try {
      const updatedPlaylist = await addTrackToPlaylist(playlistId, track.id);
      applyUpdatedPlaylist(updatedPlaylist);
      playlistMessage = `${track.title} added to ${updatedPlaylist.name}.`;
    } catch (error) {
      playlistError = error instanceof Error ? error.message : String(error);
    }
  }

  async function handleCreatePlaylistFromTrack(track: Track) {
    const name = window.prompt("Playlist name");

    if (name === null) {
      return;
    }

    const trimmedName = name.trim();
    playlistError = null;
    playlistMessage = null;

    if (!trimmedName) {
      playlistError = "Playlist name is required.";
      return;
    }

    if (playlistNameExists(trimmedName)) {
      playlistError = "A playlist with this name already exists.";
      return;
    }

    try {
      const playlist = await createPlaylist(trimmedName);
      applyUpdatedPlaylist(playlist);
      await handleAddTrackToPlaylist(playlist.id, track);
    } catch (error) {
      playlistError = error instanceof Error ? error.message : String(error);
    }
  }

  async function handleRemoveTrackFromSelectedPlaylist(track: Track) {
    const playlist = selectedPlaylist;

    if (!playlist) {
      return;
    }

    playlistError = null;
    playlistMessage = null;
    const previousPlaylists = playlists;
    playlists = playlists.map((candidate) => candidate.id === playlist.id
      ? { ...candidate, trackIds: candidate.trackIds.filter((trackId) => trackId !== track.id) }
      : candidate);

    try {
      const updatedPlaylist = await removeTrackFromPlaylist(playlist.id, track.id);
      applyUpdatedPlaylist(updatedPlaylist);
      playlistMessage = `${track.title} removed from ${updatedPlaylist.name}.`;
    } catch (error) {
      playlists = previousPlaylists;
      playlistError = error instanceof Error ? error.message : String(error);
    }
  }

  async function handleMoveTrackInSelectedPlaylist(track: Track, direction: "up" | "down") {
    const playlist = selectedPlaylist;

    if (!playlist || !canMovePlaylistTrack(playlist, track, direction)) {
      return;
    }

    playlistError = null;
    playlistMessage = null;
    const previousPlaylists = playlists;
    const optimisticPlaylist = moveTrackIdInPlaylist(playlist, track.id, direction);
    applyUpdatedPlaylist(optimisticPlaylist);

    try {
      const updatedPlaylist = await movePlaylistTrack(playlist.id, track.id, direction);
      applyUpdatedPlaylist(updatedPlaylist);
    } catch (error) {
      playlists = previousPlaylists;
      playlistError = error instanceof Error ? error.message : String(error);
    }
  }

  function canMoveSelectedPlaylistTrack(track: Track, direction: "up" | "down") {
    return selectedPlaylist ? canMovePlaylistTrack(selectedPlaylist, track, direction) : false;
  }

  function canMovePlaylistTrack(playlist: Playlist, track: Track, direction: "up" | "down") {
    const trackIndex = playlist.trackIds.indexOf(track.id);

    if (trackIndex === -1) {
      return false;
    }

    return direction === "up"
      ? trackIndex > 0
      : trackIndex < playlist.trackIds.length - 1;
  }

  function moveTrackIdInPlaylist(playlist: Playlist, trackId: string, direction: "up" | "down"): Playlist {
    const trackIds = [...playlist.trackIds];
    const trackIndex = trackIds.indexOf(trackId);
    const targetIndex = direction === "up" ? trackIndex - 1 : trackIndex + 1;

    if (trackIndex === -1 || targetIndex < 0 || targetIndex >= trackIds.length) {
      return playlist;
    }

    [trackIds[trackIndex], trackIds[targetIndex]] = [trackIds[targetIndex], trackIds[trackIndex]];

    return {
      ...playlist,
      trackIds,
      updatedAt: Math.floor(Date.now() / 1000),
    };
  }

  async function handleRenamePlaylist(playlist: Playlist) {
    const name = window.prompt("Playlist name", playlist.name);

    if (name === null) {
      return;
    }

    const trimmedName = name.trim();
    playlistError = null;
    playlistMessage = null;

    if (!trimmedName) {
      playlistError = "Playlist name is required.";
      return;
    }

    if (playlistNameExists(trimmedName, playlist.id)) {
      playlistError = "A playlist with this name already exists.";
      return;
    }

    try {
      const updatedPlaylist = await renamePlaylist(playlist.id, trimmedName);
      applyUpdatedPlaylist(updatedPlaylist);
      playlistMessage = `${updatedPlaylist.name} renamed.`;
    } catch (error) {
      playlistError = error instanceof Error ? error.message : String(error);
    }
  }

  async function handleDeletePlaylist(playlist: Playlist) {
    if (isDeletingPlaylist) {
      return;
    }

    playlistError = null;
    playlistMessage = null;
    isDeletingPlaylist = true;

    try {
      await deletePlaylist(playlist.id);
      playlists = playlists.filter((candidate) => candidate.id !== playlist.id);
      playlistPendingDelete = null;

      if (selectedPlaylistId === playlist.id) {
        selectedPlaylistId = null;
      }

      activeView = "Playlists";
      isLikedSongsOpen = false;
      isMixBuilderOpen = false;
      searchQuery = "";
      playlistMessage = `${playlist.name} deleted.`;
      mainElement?.scrollTo({ top: 0 });
    } catch (error) {
      playlistError = error instanceof Error ? error.message : String(error);
    } finally {
      isDeletingPlaylist = false;
    }
  }

  function applyUpdatedPlaylist(updatedPlaylist: Playlist) {
    playlists = playlists.some((playlist) => playlist.id === updatedPlaylist.id)
      ? playlists.map((playlist) => playlist.id === updatedPlaylist.id ? updatedPlaylist : playlist)
      : [...playlists, updatedPlaylist];
  }

  function handleLibraryHealthSelect() {
    activeView = "Settings";
    selectedAlbumId = null;
    selectedArtistName = null;
    selectedGenreName = null;
    clearGenreEditState();
    isLikedSongsOpen = false;
    isMixBuilderOpen = false;
    isLibraryHealthOpen = true;
    selectedPlaylistId = null;
    searchQuery = "";
    mainElement?.scrollTo({ top: 0 });
  }

  function handleBackToAlbums() {
    selectedAlbumId = null;
    clearGenreEditState();
    searchQuery = "";
    mainElement?.scrollTo({ top: 0 });
  }

  function handleBackToArtists() {
    selectedArtistName = null;
    clearGenreEditState();
    searchQuery = "";
    mainElement?.scrollTo({ top: 0 });
  }

  function handleBackToGenres() {
    selectedGenreName = null;
    searchQuery = "";
    mainElement?.scrollTo({ top: 0 });
  }

  function handleBackToPlaylists() {
    isLikedSongsOpen = false;
    isMixBuilderOpen = false;
    selectedPlaylistId = null;
    mixMessage = null;
    playlistMessage = null;
    playlistError = null;
    searchQuery = "";
    mainElement?.scrollTo({ top: 0 });
  }

  function handleBackToSettings() {
    isLibraryHealthOpen = false;
    searchQuery = "";
    mainElement?.scrollTo({ top: 0 });
  }

  function openShortcutHelp() {
    contextMenu = null;
    isShortcutHelpOpen = true;
  }

  function closeShortcutHelp() {
    isShortcutHelpOpen = false;
  }

  function handleShortcutBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      closeShortcutHelp();
    }
  }

  function openDeletePlaylistConfirmation(playlist: Playlist) {
    contextMenu = null;
    playlistError = null;
    playlistMessage = null;
    playlistPendingDelete = playlist;
  }

  function closeDeletePlaylistConfirmation() {
    if (isDeletingPlaylist) {
      return;
    }

    playlistPendingDelete = null;
  }

  function handleDeletePlaylistBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      closeDeletePlaylistConfirmation();
    }
  }

  function confirmPendingPlaylistDelete() {
    if (!playlistPendingDelete) {
      return;
    }

    void handleDeletePlaylist(playlistPendingDelete);
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
    await finalizePlaybackListenSession();

    const trackIndex = tracks.findIndex((candidate) => candidate.id === track.id);
    playbackError = null;
    currentTrack = track;
    currentTrackIndex = trackIndex >= 0 ? trackIndex : null;
    currentQueueIndex = queueIndex;
    durationSeconds = track.durationSeconds;
    positionSeconds = 0;
    isPlaying = false;
    hasCurrentTrackEnded = false;
    isHandlingTrackEnd = false;
    resetPlaybackListenSession(track);

    try {
      const status = await playTrack(track.filePath);
      applyPlaybackStatus(status);
      durationSeconds = status.durationSeconds ?? track.durationSeconds;
    } catch (error) {
      playbackError = error instanceof Error ? error.message : String(error);
    }
  }

  async function handlePlaybackStatusUpdate(status: PlaybackStatus, source: "status" | "duration") {
    await maybeRecordTrackPlay();

    if (!status.hasEnded || !currentTrack || status.filePath !== currentTrack.filePath) {
      return;
    }

    await handleTrackEnd(source);
  }

  async function handleTrackEnd(source: "status" | "duration") {
    if (!currentTrack || isHandlingTrackEnd || hasCurrentTrackEnded) {
      return;
    }

    const duration = durationSeconds ?? currentTrack.durationSeconds;

    if (source === "duration" && (!duration || positionSeconds < duration)) {
      return;
    }

    isHandlingTrackEnd = true;
    hasCurrentTrackEnded = true;
    isPlaying = false;
    positionSeconds = duration ?? positionSeconds;
    await maybeRecordTrackPlay();

    const nextQueueIndex = getNextQueueIndex(true);

    if (nextQueueIndex === null) {
      isHandlingTrackEnd = false;
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

  function handleToggleCurrentTrackFavorite() {
    if (currentTrack) {
      void handleToggleFavorite(currentTrack);
    }
  }

  function applyTrackFavorite(trackId: string, isFavorite: boolean) {
    tracks = tracks.map((track) => (track.id === trackId ? { ...track, isFavorite } : track));
    playbackQueue = playbackQueue.map((track) => (track.id === trackId ? { ...track, isFavorite } : track));

    if (currentTrack?.id === trackId) {
      currentTrack = { ...currentTrack, isFavorite };
    }
  }

  async function maybeRecordTrackPlay() {
    if (
      !currentTrack
      || countedPlaybackTrackId === currentTrack.id
      || !hasReachedPlayCountThreshold(currentTrack)
    ) {
      return;
    }

    countedPlaybackTrackId = currentTrack.id;

    try {
      applyUpdatedTrack(await recordTrackPlay(currentTrack.id));
    } catch (error) {
      countedPlaybackTrackId = null;
      playbackError = error instanceof Error ? error.message : String(error);
    }
  }

  async function finalizePlaybackListenSession() {
    updatePlaybackListenClock(false);
    await maybeRecordTrackPlay();
  }

  function resetPlaybackListenSession(track: Track | null) {
    playbackSessionTrackId = track?.id ?? null;
    playbackSessionListenedSeconds = 0;
    playbackSessionStartedAtMs = null;
    countedPlaybackTrackId = null;
  }

  function updatePlaybackListenClock(nextIsPlaying = isPlaying) {
    const now = performance.now();

    if (!currentTrack) {
      playbackSessionTrackId = null;
      playbackSessionListenedSeconds = 0;
      playbackSessionStartedAtMs = null;
      return;
    }

    if (playbackSessionTrackId !== currentTrack.id) {
      playbackSessionTrackId = currentTrack.id;
      playbackSessionListenedSeconds = 0;
      playbackSessionStartedAtMs = null;
      countedPlaybackTrackId = null;
    }

    if (isPlaying && playbackSessionStartedAtMs !== null) {
      playbackSessionListenedSeconds += Math.max(0, (now - playbackSessionStartedAtMs) / 1000);
    }

    playbackSessionStartedAtMs = nextIsPlaying ? now : null;
  }

  function hasReachedPlayCountThreshold(track: Track) {
    const duration = durationSeconds ?? track.durationSeconds;
    const threshold = duration && duration > 0 ? Math.min(30, duration * 0.5) : 30;

    return playbackSessionTrackId === track.id && playbackSessionListenedSeconds >= threshold;
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

  function applyUpdatedTrack(updatedTrack: Track) {
    tracks = tracks.map((track) => (track.id === updatedTrack.id ? updatedTrack : track));
    playbackQueue = playbackQueue.map((track) => (track.id === updatedTrack.id ? updatedTrack : track));

    if (currentTrack?.id === updatedTrack.id) {
      currentTrack = updatedTrack;
      currentTrackIndex = tracks.findIndex((track) => track.id === updatedTrack.id);
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

  function handleSettingsClearQueue() {
    if (playbackQueue.length === 0) {
      return;
    }

    const confirmed = window.confirm("Clear the current Up Next queue? Playback will not delete any files.");

    if (confirmed) {
      handleClearQueue();
    }
  }

  async function playTrackSet(libraryTracks: Track[], shouldShuffle = false) {
    const queue = shouldShuffle ? shuffleTracks(libraryTracks) : orderedContextTracks(libraryTracks);

    if (queue.length === 0) {
      return;
    }

    playbackQueue = queue;
    currentQueueIndex = 0;
    isShuffleEnabled = false;
    shuffledQueueOrder = [];
    await playQueuedTrackAtIndex(0);
  }

  function insertTracksNext(libraryTracks: Track[]) {
    if (libraryTracks.length === 0) {
      return;
    }

    const insertIndex = currentQueueIndex === null ? playbackQueue.length : currentQueueIndex + 1;
    const insertedTracks = [...libraryTracks];
    const previousLength = playbackQueue.length;
    const previousOrder = isShuffleEnabled
      ? normalizedQueueOrder(shuffledQueueOrder, previousLength)
      : [];

    playbackQueue = [
      ...playbackQueue.slice(0, insertIndex),
      ...insertedTracks,
      ...playbackQueue.slice(insertIndex),
    ];

    if (!isShuffleEnabled) {
      return;
    }

    const insertedIndices = insertedTracks.map((_, offset) => insertIndex + offset);
    const shiftedOrder = previousOrder.map((index) => index >= insertIndex ? index + insertedTracks.length : index);
    const currentOrderPosition = currentQueueIndex === null ? -1 : shiftedOrder.indexOf(currentQueueIndex);

    shuffledQueueOrder = currentOrderPosition === -1
      ? [...shiftedOrder, ...insertedIndices]
      : [
          ...shiftedOrder.slice(0, currentOrderPosition + 1),
          ...insertedIndices,
          ...shiftedOrder.slice(currentOrderPosition + 1),
        ];
  }

  function appendTracksToQueue(libraryTracks: Track[]) {
    if (libraryTracks.length === 0) {
      return;
    }

    const startIndex = playbackQueue.length;
    const appendedTracks = [...libraryTracks];
    const appendedIndices = appendedTracks.map((_, offset) => startIndex + offset);

    playbackQueue = [...playbackQueue, ...appendedTracks];

    if (isShuffleEnabled) {
      shuffledQueueOrder = [
        ...normalizedQueueOrder(shuffledQueueOrder, startIndex),
        ...appendedIndices,
      ];
    }
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
      await maybeRecordTrackPlay();
    } catch (error) {
      playbackError = error instanceof Error ? error.message : String(error);
    }
  }

  async function handleMprisPlay() {
    if (!currentTrack || isPlaying) {
      return;
    }

    await handleTogglePlayback();
  }

  async function handleMprisPause() {
    if (!currentTrack || !isPlaying) {
      return;
    }

    await handleTogglePlayback();
  }

  async function handleMprisStop() {
    if (!currentTrack) {
      return;
    }

    playbackError = null;

    try {
      applyPlaybackStatus(await pausePlayback());
      await maybeRecordTrackPlay();
      applyPlaybackStatus(await seekPlayback(0));
      positionSeconds = 0;
      hasCurrentTrackEnded = false;
    } catch (error) {
      playbackError = error instanceof Error ? error.message : String(error);
    }
  }

  async function handleSeek(nextPositionSeconds: number) {
    const duration = durationSeconds ?? currentTrack?.durationSeconds ?? null;

    if (!currentTrack || !duration || duration <= 0 || !Number.isFinite(nextPositionSeconds)) {
      return;
    }

    const clampedPositionSeconds = Math.min(Math.max(nextPositionSeconds, 0), duration);
    playbackError = null;

    try {
      applyPlaybackStatus(await seekPlayback(clampedPositionSeconds));
      await maybeRecordTrackPlay();
      hasCurrentTrackEnded = false;
    } catch (error) {
      playbackError = error instanceof Error ? error.message : String(error);
    }
  }

  function handleLyricLineSeek(timeSeconds: number) {
    void handleSeek(timeSeconds);
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
    updatePlaybackListenClock(status.isPlaying);
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

    return ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName);
  }

  function isShortcutHelpEvent(event: KeyboardEvent) {
    const isModifiedSlash = event.key === "/" && (event.ctrlKey || event.metaKey);
    return event.key === "?" || isModifiedSlash;
  }

  function albumDetail(album: Album) {
    const year = album.year ? ` · ${album.year}` : "";
    const trackCount = `${album.trackCount} ${album.trackCount === 1 ? "song" : "songs"}`;

    return `${album.artist}${year} · ${trackCount}`;
  }

  function albumHeroDetails(album: Album, albumTracks: Track[]) {
    const details = [album.artist];

    if (album.year) {
      details.push(String(album.year));
    }

    details.push(songCountLabel(albumTracks.length));

    if (selectedAlbumDurationLabel) {
      details.push(selectedAlbumDurationLabel);
    }

    if (selectedAlbumFormatSummary) {
      details.push(selectedAlbumFormatSummary);
    }

    return details.join(" · ");
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

  function compareOptionalNumber(left: number | null | undefined, right: number | null | undefined) {
    const leftMissing = left === null || left === undefined;
    const rightMissing = right === null || right === undefined;

    if (leftMissing && rightMissing) {
      return 0;
    }

    if (leftMissing) {
      return 1;
    }

    if (rightMissing) {
      return -1;
    }

    return left - right;
  }

  function compareAlbumTrackOrder(left: Track, right: Track) {
    return compareOptionalNumber(left.discNumber, right.discNumber)
      || compareOptionalNumber(left.trackNumber, right.trackNumber)
      || compareText(left.title, right.title)
      || compareText(left.fileName, right.fileName);
  }

  function orderedAlbumTracks(libraryTracks: Track[]) {
    return [...libraryTracks].sort(compareAlbumTrackOrder);
  }

  function orderedContextTracks(libraryTracks: Track[]) {
    return sortTracks(libraryTracks, "album", "asc");
  }

  function tracksForAlbum(album: Album) {
    return orderedAlbumTracks(tracks.filter((track) => albumIdForTrack(track) === album.id));
  }

  function albumHasMultipleDiscs(albumTracks: Track[]) {
    return new Set(albumTracks
      .map((track) => track.discNumber)
      .filter((discNumber): discNumber is number => discNumber !== null)).size > 1;
  }

  function albumDiscGroups(albumTracks: Track[]): AlbumDiscGroup[] {
    const groups = new Map<number | null, Track[]>();

    for (const track of albumTracks) {
      const discNumber = track.discNumber;
      groups.set(discNumber, [...(groups.get(discNumber) ?? []), track]);
    }

    return Array.from(groups, ([discNumber, tracks]) => ({ discNumber, tracks }));
  }

  function albumDiscLabel(discNumber: number | null) {
    return discNumber === null ? "Other Tracks" : `Disc ${discNumber}`;
  }

  function albumTrackNumberLabel(track: Track) {
    return track.trackNumber === null ? "–" : String(track.trackNumber).padStart(2, "0");
  }

  function albumTotalDurationLabel(albumTracks: Track[]) {
    const knownDurations = albumTracks
      .map((track) => track.durationSeconds)
      .filter((duration): duration is number => duration !== null);

    if (knownDurations.length === 0) {
      return null;
    }

    const totalSeconds = knownDurations.reduce((total, duration) => total + duration, 0);
    return formatDurationSummary(totalSeconds);
  }

  function formatDurationSummary(totalSeconds: number) {
    const hours = Math.floor(totalSeconds / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);

    if (hours > 0) {
      return minutes > 0 ? `${hours} hr ${minutes} min` : `${hours} hr`;
    }

    return `${Math.max(1, minutes)} min`;
  }

  function formatTrackDuration(totalSeconds: number) {
    const minutes = Math.floor(totalSeconds / 60);
    const seconds = Math.floor(totalSeconds % 60);

    return `${minutes}:${seconds.toString().padStart(2, "0")}`;
  }

  function formatPlaybackTime(seconds: number | null | undefined) {
    return seconds === null || seconds === undefined ? "--:--" : formatTrackDuration(seconds);
  }

  function formatVideoDuration(seconds: number | null | undefined) {
    if (seconds === null || seconds === undefined) {
      return "--:--";
    }

    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const remainingSeconds = Math.floor(seconds % 60);

    if (hours > 0) {
      return `${hours}:${minutes.toString().padStart(2, "0")}:${remainingSeconds.toString().padStart(2, "0")}`;
    }

    return `${minutes}:${remainingSeconds.toString().padStart(2, "0")}`;
  }

  function formatDateTime(timestampSeconds: number) {
    return new Intl.DateTimeFormat(undefined, {
      dateStyle: "medium",
      timeStyle: "short",
    }).format(new Date(timestampSeconds * 1000));
  }

  function parseLrcLyrics(text: string): SyncedLyricLine[] {
    return text
      .split(/\r?\n/)
      .flatMap((line) => parseLrcLine(line))
      .sort((left, right) => left.timeSeconds - right.timeSeconds || compareText(left.text, right.text));
  }

  function parseLrcLine(line: string): SyncedLyricLine[] {
    const timestampPattern = /\[(\d{1,2}):(\d{2})(?:\.(\d{1,3}))?\]/g;
    const timestamps: number[] = [];
    let match: RegExpExecArray | null;

    while ((match = timestampPattern.exec(line)) !== null) {
      const minutes = Number.parseInt(match[1], 10);
      const seconds = Number.parseInt(match[2], 10);
      const fraction = match[3] ? Number.parseFloat(`0.${match[3].padEnd(3, "0")}`) : 0;

      if (Number.isFinite(minutes) && Number.isFinite(seconds)) {
        timestamps.push(minutes * 60 + seconds + fraction);
      }
    }

    if (timestamps.length === 0) {
      return [];
    }

    const text = line.replace(timestampPattern, "").trim();

    if (!text) {
      return [];
    }

    return timestamps.map((timeSeconds) => ({ timeSeconds, text }));
  }

  function activeSyncedLyricIndex(lines: SyncedLyricLine[], currentPositionSeconds: number) {
    let activeIndex = -1;

    for (let index = 0; index < lines.length; index += 1) {
      if (lines[index].timeSeconds <= currentPositionSeconds + 0.15) {
        activeIndex = index;
      } else {
        break;
      }
    }

    return activeIndex;
  }

  function lyricsKindLabel(lyrics: TrackLyrics) {
    const kindLabel = lyrics.kind === "synced" ? "LRC" : "TXT";
    return lyrics.source === "lrclib" ? `${kindLabel} · LRCLIB` : kindLabel;
  }

  function cachedLyricsStatus(lyrics: TrackLyrics) {
    if (lyrics.fetchedAt) {
      return `Cached from LRCLIB · ${formatDateTime(lyrics.fetchedAt)}`;
    }

    return "Cached from LRCLIB";
  }

  function playCountLabel(track: Track) {
    return `${track.playCount} ${track.playCount === 1 ? "play" : "plays"}`;
  }

  function playsLabel(totalPlays: number) {
    return `${totalPlays} ${totalPlays === 1 ? "play" : "plays"}`;
  }

  function lastPlayedLabel(track: Track) {
    return track.lastPlayedAt === null ? "Not played yet" : formatDateTime(track.lastPlayedAt);
  }

  function albumFormatSummary(albumTracks: Track[]) {
    const formats = Array.from(new Set(albumTracks.map((track) => track.extension.toUpperCase()))).sort();

    if (formats.length === 0) {
      return null;
    }

    return formats.length === 1 ? formats[0] : "Mixed formats";
  }

  function tracksForArtist(artist: Artist) {
    return orderedContextTracks(tracks.filter((track) => artistNameForTrack(track) === artist.name));
  }

  function tracksForGenre(genre: Genre) {
    return orderedContextTracks(tracks.filter((track) => trackGenres(track).includes(genre.name)));
  }

  function tracksForPlaylist(playlist: Playlist) {
    const seenTrackIds = new Set<string>();

    return playlist.trackIds.flatMap((trackId) => {
      if (seenTrackIds.has(trackId)) {
        return [];
      }

      seenTrackIds.add(trackId);
      const track = libraryTracksById.get(trackId);
      return track ? [track] : [];
    });
  }

  function missingTrackCountForPlaylist(playlist: Playlist) {
    const seenTrackIds = new Set<string>();
    let missingTrackCount = 0;

    for (const trackId of playlist.trackIds) {
      if (seenTrackIds.has(trackId)) {
        continue;
      }

      seenTrackIds.add(trackId);

      if (!libraryTracksById.has(trackId)) {
        missingTrackCount += 1;
      }
    }

    return missingTrackCount;
  }

  function playlistTrackLabel(playlist: Playlist) {
    const availableTrackCount = tracksForPlaylist(playlist).length;
    const missingTrackCount = missingTrackCountForPlaylist(playlist);
    const availableLabel = songCountLabel(availableTrackCount);

    return missingTrackCount > 0
      ? `${availableLabel} · ${missingTrackCount} unavailable`
      : availableLabel;
  }

  function songCountLabel(count: number) {
    return `${count} ${count === 1 ? "song" : "songs"}`;
  }

  function normalizePlaylistName(value: string) {
    return value.trim().toLocaleLowerCase();
  }

  function playlistNameExists(name: string, excludedPlaylistId: string | null = null) {
    const normalizedName = normalizePlaylistName(name);

    return playlists.some((playlist) =>
      playlist.id !== excludedPlaylistId && normalizePlaylistName(playlist.name) === normalizedName);
  }

  function buildLibraryDiagnostics(libraryTracks: Track[], albums: Album[], artists: Artist[]): LibraryDiagnostics {
    const missingGenreTracks = libraryTracks.filter(trackMissingGenre);
    const missingCoverTracks = libraryTracks.filter((track) => !track.coverArtPath);
    const missingCoverAlbums = albums.filter((album) => !album.coverArtPath);

    return {
      totalTracks: libraryTracks.length,
      totalAlbums: albums.length,
      totalArtists: artists.length,
      missingGenreTracks,
      missingCoverTracks,
      missingCoverAlbums,
      unknownArtistTracks: libraryTracks.filter(trackUnknownArtist),
      unknownAlbumTracks: libraryTracks.filter(trackUnknownAlbum),
      missingTrackNumberTracks: libraryTracks.filter((track) => track.trackNumber === null),
      missingYearTracks: libraryTracks.filter((track) => track.year === null),
      duplicateAlbumGroups: buildDuplicateAlbumGroups(libraryTracks, albums),
    };
  }

  function libraryHealthTotalIssueCount(diagnostics: LibraryDiagnostics) {
    return diagnostics.missingGenreTracks.length
      + diagnostics.missingCoverTracks.length
      + diagnostics.unknownArtistTracks.length
      + diagnostics.unknownAlbumTracks.length
      + diagnostics.missingTrackNumberTracks.length
      + diagnostics.missingYearTracks.length
      + diagnostics.duplicateAlbumGroups.length;
  }

  function trackMissingGenre(track: Track) {
    const genres = trackGenres(track);

    return genres.length === 0 || genres.every((genre) => genre === "Unknown Genre");
  }

  function trackUnknownArtist(track: Track) {
    return textMissingOrUnknown(track.artist, "Unknown Artist");
  }

  function trackUnknownAlbum(track: Track) {
    return textMissingOrUnknown(track.album, "Unknown Album");
  }

  function textMissingOrUnknown(value: string | null | undefined, unknownLabel: string) {
    const normalized = value?.trim().toLocaleLowerCase();

    return !normalized || normalized === unknownLabel.toLocaleLowerCase();
  }

  function buildDuplicateAlbumGroups(libraryTracks: Track[], albums: Album[]) {
    const albumsById = new Map(albums.map((album) => [album.id, album]));
    const groupsByTitle = new Map<string, {
      title: string;
      albumIds: Set<string>;
      artists: Set<string>;
      folders: Set<string>;
      trackCount: number;
    }>();

    for (const track of libraryTracks) {
      if (trackUnknownAlbum(track)) {
        continue;
      }

      const title = track.album?.trim() ?? "";
      const key = title.toLocaleLowerCase();
      const group = groupsByTitle.get(key) ?? {
        title,
        albumIds: new Set<string>(),
        artists: new Set<string>(),
        folders: new Set<string>(),
        trackCount: 0,
      };

      group.albumIds.add(albumIdForTrack(track));
      group.artists.add(track.albumArtist ?? track.artist ?? "Unknown Artist");
      group.folders.add(folderForTrack(track));
      group.trackCount += 1;
      groupsByTitle.set(key, group);
    }

    return [...groupsByTitle.values()]
      .filter((group) => group.albumIds.size > 1 && (group.artists.size > 1 || group.folders.size > 1))
      .map((group) => ({
        title: group.title,
        albums: [...group.albumIds]
          .map((albumId) => albumsById.get(albumId))
          .filter((album): album is Album => Boolean(album)),
        folders: [...group.folders].filter((folder) => folder !== "Unknown Folder").sort(),
        trackCount: group.trackCount,
      }))
      .filter((group) => group.albums.length > 1)
      .sort((left, right) => left.title.localeCompare(right.title));
  }

  function folderForTrack(track: Track) {
    const parts = track.filePath.split(/[\\/]/).filter(Boolean);

    if (parts.length <= 1) {
      return "Unknown Folder";
    }

    return parts.slice(0, -1).join("/");
  }

  function folderLabel(path: string) {
    const parts = path.split(/[\\/]/).filter(Boolean);

    return parts.slice(-2).join("/") || path;
  }

  function issueCountLabel(count: number, noun = "track") {
    return `${count} ${count === 1 ? noun : `${noun}s`}`;
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

  function handleSongSortChange(event: Event) {
    const nextSort = event.currentTarget instanceof HTMLSelectElement
      ? event.currentTarget.value as SongSortKey
      : songSort;

    if ((nextSort === "recentlyAdded" || nextSort === "recentlyPlayed" || nextSort === "playCount") && songSortDirection === "asc") {
      songSortDirection = "desc";
    }
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
          || compareAlbumTrackOrder(left, right);
        return applySortDirection(result, direction);
      }

      if (sortKey === "duration") {
        result = (left.durationSeconds ?? Number.MAX_SAFE_INTEGER)
          - (right.durationSeconds ?? Number.MAX_SAFE_INTEGER)
          || compareText(left.title, right.title);
        return applySortDirection(result, direction);
      }

      if (sortKey === "recentlyAdded") {
        result = (left.scannedAt ?? left.modifiedTime ?? 0) - (right.scannedAt ?? right.modifiedTime ?? 0)
          || compareText(left.title, right.title);
        return applySortDirection(result, direction);
      }

      if (sortKey === "recentlyPlayed") {
        result = (left.lastPlayedAt ?? 0) - (right.lastPlayedAt ?? 0)
          || compareText(left.title, right.title);
        return applySortDirection(result, direction);
      }

      if (sortKey === "playCount") {
        result = left.playCount - right.playCount
          || (left.lastPlayedAt ?? 0) - (right.lastPlayedAt ?? 0)
          || compareText(left.title, right.title);
        return applySortDirection(result, direction);
      }

      result = compareText(left.title, right.title);
      return applySortDirection(result, direction);
    });
  }

  function recentlyPlayed(libraryTracks: Track[]) {
    return [...libraryTracks]
      .filter((track) => track.lastPlayedAt !== null)
      .sort((left, right) =>
        (right.lastPlayedAt ?? 0) - (left.lastPlayedAt ?? 0)
        || compareText(left.title, right.title),
      );
  }

  function mostPlayed(libraryTracks: Track[]) {
    return [...libraryTracks]
      .filter((track) => track.playCount > 0)
      .sort((left, right) =>
        right.playCount - left.playCount
        || (right.lastPlayedAt ?? 0) - (left.lastPlayedAt ?? 0)
        || compareText(left.title, right.title),
      );
  }

  function buildTopArtistStats(libraryTracks: Track[], artists: Artist[]): TopArtistStat[] {
    const artistsByName = new Map(artists.map((artist) => [artist.name, artist]));
    const statsByName = new Map<string, TopArtistStat>();

    for (const track of libraryTracks) {
      const name = artistNameForTrack(track);
      const artist = artistsByName.get(name);
      const current = statsByName.get(name) ?? {
        name,
        color: artist?.color ?? "#2f8f83",
        totalPlays: 0,
        songCount: 0,
      };

      current.totalPlays += track.playCount;
      current.songCount += 1;
      statsByName.set(name, current);
    }

    return Array.from(statsByName.values())
      .filter((stat) => stat.totalPlays > 0)
      .sort((left, right) =>
        right.totalPlays - left.totalPlays
        || right.songCount - left.songCount
        || compareText(left.name, right.name),
      );
  }

  function buildTopAlbumStats(libraryTracks: Track[], albums: Album[]): TopAlbumStat[] {
    const albumsById = new Map(albums.map((album) => [album.id, album]));
    const statsByAlbumId = new Map<string, TopAlbumStat>();

    for (const track of libraryTracks) {
      const albumId = albumIdForTrack(track);
      const album = albumsById.get(albumId);

      if (!album) {
        continue;
      }

      const current = statsByAlbumId.get(albumId) ?? {
        album,
        totalPlays: 0,
        songCount: 0,
      };

      current.totalPlays += track.playCount;
      current.songCount += 1;
      statsByAlbumId.set(albumId, current);
    }

    return Array.from(statsByAlbumId.values())
      .filter((stat) => stat.totalPlays > 0)
      .sort((left, right) =>
        right.totalPlays - left.totalPlays
        || right.songCount - left.songCount
        || compareText(left.album.title, right.album.title),
      );
  }

  function buildTopGenreStats(libraryTracks: Track[], genres: Genre[]): TopGenreStat[] {
    const genresByName = new Map(genres.map((genre) => [genre.name, genre]));
    const statsByGenreName = new Map<string, TopGenreStat>();

    for (const track of libraryTracks) {
      for (const genreName of trackGenres(track)) {
        const genre = genresByName.get(genreName);

        if (!genre) {
          continue;
        }

        const current = statsByGenreName.get(genreName) ?? {
          genre,
          totalPlays: 0,
          songCount: 0,
        };

        current.totalPlays += track.playCount;
        current.songCount += 1;
        statsByGenreName.set(genreName, current);
      }
    }

    return Array.from(statsByGenreName.values())
      .filter((stat) => stat.totalPlays > 0)
      .sort((left, right) =>
        right.totalPlays - left.totalPlays
        || right.songCount - left.songCount
        || compareText(left.genre.name, right.genre.name),
      );
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

  function sortVideos(videoEntries: VideoEntry[], sortKey: VideoSortKey, direction: SortDirection) {
    return [...videoEntries].sort((left, right) => {
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

      if (sortKey === "recentlyPlayed") {
        result = (left.lastPlayedAt ?? 0) - (right.lastPlayedAt ?? 0)
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

  function albumTrackMatchesSearch(track: Track, query: string) {
    return matchesSearch(query, [
      track.title,
      track.fileName,
      track.trackNumber?.toString(),
      track.discNumber?.toString(),
    ]);
  }

  function artistTrackMatchesSearch(track: Track, query: string) {
    return matchesSearch(query, [
      track.title,
      track.album,
      track.fileName,
      track.trackNumber?.toString(),
      track.discNumber?.toString(),
    ]);
  }

  function genreTrackMatchesSearch(track: Track, query: string) {
    return matchesSearch(query, [
      track.title,
      track.artist,
      track.albumArtist,
      track.album,
      track.fileName,
    ]);
  }

  function playlistTrackMatchesSearch(track: Track, query: string) {
    return matchesSearch(query, [
      track.title,
      track.artist,
      track.album,
      track.albumArtist,
      track.fileName,
    ]);
  }

  function albumMatchesSearch(album: Album, query: string) {
    return matchesSearch(query, [album.title, album.artist, album.year?.toString()]);
  }

  function genreAlbumMatchesSearch(album: Album, query: string) {
    return matchesSearch(query, [album.title, album.artist, album.year?.toString()]);
  }

  function artistMatchesSearch(artist: Artist, query: string) {
    return matchesSearch(query, [artist.name]);
  }

  function genreMatchesSearch(genre: Genre, query: string) {
    return matchesSearch(query, [genre.name]);
  }

  function videoMatchesSearch(video: VideoEntry, query: string) {
    return matchesSearch(query, [
      video.title,
      video.artist,
      video.showTitle,
      video.albumOrRelease,
      video.venue,
      video.city,
      video.country,
      video.fileName,
      video.year?.toString(),
    ]);
  }

  function searchFilterTracks(libraryTracks: Track[], query: string) {
    return query ? libraryTracks.filter((track) => trackMatchesSearch(track, query)) : libraryTracks;
  }

  function searchFilterVideos(videoEntries: VideoEntry[], query: string) {
    return query ? videoEntries.filter((video) => videoMatchesSearch(video, query)) : videoEntries;
  }

  function searchFilterAlbumTracks(libraryTracks: Track[], query: string) {
    return query ? libraryTracks.filter((track) => albumTrackMatchesSearch(track, query)) : libraryTracks;
  }

  function searchFilterArtistTracks(libraryTracks: Track[], query: string) {
    return query ? libraryTracks.filter((track) => artistTrackMatchesSearch(track, query)) : libraryTracks;
  }

  function searchFilterGenreTracks(libraryTracks: Track[], query: string) {
    return query ? libraryTracks.filter((track) => genreTrackMatchesSearch(track, query)) : libraryTracks;
  }

  function searchFilterAlbums(albums: Album[], query: string) {
    return query ? albums.filter((album) => albumMatchesSearch(album, query)) : albums;
  }

  function searchFilterGenreAlbums(albums: Album[], query: string) {
    return query ? albums.filter((album) => genreAlbumMatchesSearch(album, query)) : albums;
  }

  function searchFilterArtists(artists: Artist[], query: string) {
    return query ? artists.filter((artist) => artistMatchesSearch(artist, query)) : artists;
  }

  function searchFilterGenres(genres: Genre[], query: string) {
    return query ? genres.filter((genre) => genreMatchesSearch(genre, query)) : genres;
  }

  function searchPlaceholder() {
    if (activeView === "Home") {
      return "Search songs, albums, artists...";
    }

    if (activeView === "Songs") {
      return "Search songs...";
    }

    if (activeView === "Albums") {
      return selectedAlbum ? "Search this album..." : "Search albums...";
    }

    if (activeView === "Artists") {
      return selectedArtist ? "Search this artist..." : "Search artists...";
    }

    if (activeView === "Genres") {
      return selectedGenre ? "Search this genre..." : "Search genres...";
    }

    if (activeView === "Playlists") {
      if (selectedPlaylist) {
        return "Search this playlist...";
      }

      if (isLikedSongsOpen) {
        return "Search liked songs...";
      }
    }

    if (activeView === "Live Shows") {
      return "";
    }

    return "";
  }

  function isSearchAvailable() {
    return searchPlaceholder().length > 0;
  }

  function viewTitle() {
    if (isHomeSearchActive) {
      return "Search Results";
    }

    if (activeView === "Home") {
      return "Your music, on this machine.";
    }

    return activeView;
  }

  function viewEyebrow() {
    if (isHomeSearchActive) {
      return "Search";
    }

    return activeView;
  }

  function viewStatus() {
    if (isHomeSearchActive) {
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

    if (activeView === "Stats") {
      return `${playsLabel(statsTotalPlays)} across ${tracks.length} ${tracks.length === 1 ? "track" : "tracks"}.`;
    }

    if (activeView === "CD Rip") {
      if (isRippingCd) {
        return "Ripping audio CD to FLAC files...";
      }

      if (audioCdFound && cdTracks.length > 0) {
        return `${cdTracks.length} ${cdTracks.length === 1 ? "track" : "tracks"} ready to rip.`;
      }

      return "Detect an audio CD, choose an output folder, then rip to FLAC.";
    }

    if (activeView === "Live Shows") {
      if (isScanningVideos) {
        return "Scanning local video files...";
      }

      if (videoFolder) {
        return `${videos.length} ${videos.length === 1 ? "video" : "videos"} from ${videoFolder}`;
      }

      return "Add a folder of local live shows or music videos.";
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
        {#if activeView === "Live Shows"}
          <button type="button" disabled={isScanningVideos} onclick={() => void (videoFolder ? handleRescanVideos() : handleAddVideoFolder())}>
            {isScanningVideos ? "Scanning..." : videoFolder ? "Rescan Videos" : "Add Video Folder"}
          </button>
        {:else}
          <button type="button" disabled={isScanning} onclick={handleScanLibrary}>
            {isScanning ? "Scanning..." : "Scan Library"}
          </button>
        {/if}
      </header>

      {#if isSearchAvailable()}
        <div class="search-bar">
          <input
            type="search"
            bind:value={searchQuery}
            placeholder={searchPlaceholder()}
            aria-label={searchPlaceholder()}
            onkeydown={handleSearchKeydown}
          />
          {#if searchQuery}
            <button type="button" aria-label="Clear search" onclick={clearSearch}>Clear</button>
          {/if}
        </div>
      {/if}

      {#if scanError}
        <div class="scan-error" role="alert">{scanError}</div>
      {/if}
      {#if playbackError}
        <div class="scan-error" role="alert">{playbackError}</div>
      {/if}
      {#if activeView === "Live Shows" && videoError}
        <div class="scan-error" role="alert">{videoError}</div>
      {:else if activeView === "Live Shows" && videoMessage}
        <div class="scan-error status-message" role="status">{videoMessage}</div>
      {/if}
      {#if activeView !== "Playlists" && playlistError}
        <div class="scan-error" role="alert">{playlistError}</div>
      {:else if activeView !== "Playlists" && playlistMessage}
        <div class="scan-error status-message" role="status">{playlistMessage}</div>
      {/if}

      {#if isHomeSearchActive}
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
                onTrackContextMenu={openTrackContextMenu}
                onArtistSelect={handleTrackArtistSelect}
                onAlbumSelect={handleTrackAlbumSelect}
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
                  <button class="album-card" type="button" onclick={() => handleAlbumSelect(album)} oncontextmenu={(event) => openAlbumContextMenu(event, album)}>
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
                  <button class="artist-card" type="button" onclick={() => handleArtistSelect(artist)} oncontextmenu={(event) => openArtistContextMenu(event, artist)}>
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
                  <button class="genre-card" type="button" onclick={() => handleGenreSelect(genre)} oncontextmenu={(event) => openGenreContextMenu(event, genre)}>
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
      {:else if activeView === "Now Playing"}
        <section class="now-playing-page" aria-labelledby="now-playing-title">
          {#if currentTrack}
            <div class="now-playing-hero">
              <div class="now-playing-cover" aria-hidden="true">
                {#if currentTrackCoverArtSrc}
                  <img
                    src={currentTrackCoverArtSrc}
                    alt=""
                    onload={showLoadedImage}
                    onerror={hideBrokenImage}
                  />
                {/if}
                <span></span>
              </div>

              <div class="now-playing-copy">
                <p class="eyebrow">Now Playing</p>
                <h3 id="now-playing-title">{currentTrack.title}</h3>
                <div class="now-playing-links">
                  <button type="button" onclick={handleNowPlayingArtistSelect}>
                    {currentTrack.artist ?? currentTrack.albumArtist ?? "Unknown Artist"}
                  </button>
                  {#if currentTrack.album}
                    <span aria-hidden="true">·</span>
                    <button type="button" onclick={handleNowPlayingAlbumSelect}>{currentTrack.album}</button>
                  {/if}
                  {#if currentTrack.year}
                    <span aria-hidden="true">·</span>
                    <span>{currentTrack.year}</span>
                  {/if}
                </div>

                {#if currentTrackGenres.length > 0}
                  <div class="now-playing-genres" aria-label="Genres">
                    {#each currentTrackGenres as genre}
                      <button type="button" onclick={() => handleNowPlayingGenreSelect(genre)}>{genre}</button>
                    {/each}
                  </div>
                {/if}

                <div class="now-playing-stats">
                  <span>{formatPlaybackTime(positionSeconds)} / {formatPlaybackTime(currentTrackDuration)}</span>
                  {#if currentTrack.playCount > 0}
                    <span>{playCountLabel(currentTrack)}</span>
                  {/if}
                  <span>{currentTrack.extension.toUpperCase()}</span>
                </div>

                <div class="now-playing-actions">
                  <button
                    class:active={currentTrack.isFavorite}
                    type="button"
                    aria-label={currentTrack.isFavorite ? "Remove from liked songs" : "Add to liked songs"}
                    onclick={handleToggleCurrentTrackFavorite}
                  >
                    {currentTrack.isFavorite ? "★ Liked" : "☆ Like"}
                  </button>
                </div>
              </div>
            </div>

            <section class="lyrics-panel" aria-labelledby="lyrics-title">
              <div class="lyrics-header">
                <div>
                  <p class="eyebrow">Local Lyrics</p>
                  <h3 id="lyrics-title">Lyrics</h3>
                </div>
                <div class="lyrics-header-actions">
                  {#if currentLyrics?.source === "lrclib"}
                    <button type="button" disabled={isAutoFindingLyrics} onclick={() => void handleAutoFindLyrics(true)}>
                      {isAutoFindingLyrics ? "Searching..." : "Find Again"}
                    </button>
                  {/if}
                  {#if lyricsBadgeLabel}
                    <span>{lyricsBadgeLabel}</span>
                  {/if}
                </div>
              </div>

              {#if lyricsLookupMessage}
                <p class="lyrics-lookup-message">{lyricsLookupMessage}</p>
              {:else if lyricsLookupError}
                <p class="lyrics-lookup-message error">{lyricsLookupError}</p>
              {:else if cachedLyricsLabel}
                <p class="lyrics-lookup-message cached">{cachedLyricsLabel}</p>
              {/if}

              {#if isLoadingLyrics}
                <div class="group-empty compact" role="status">
                  <h3>Loading lyrics...</h3>
                  <p>Checking for local .lrc and .txt files next to this track.</p>
                </div>
              {:else if currentLyrics?.kind === "synced" && syncedLyricLines.length > 0}
                <div class="synced-lyrics" bind:this={lyricsPanelElement}>
                  {#each syncedLyricLines as line, index}
                    <button
                      class:active={index === activeLyricIndex}
                      data-active={index === activeLyricIndex ? "true" : undefined}
                      type="button"
                      onclick={() => handleLyricLineSeek(line.timeSeconds)}
                    >
                      {line.text}
                    </button>
                  {/each}
                </div>
              {:else if currentLyrics?.kind === "plain"}
                <pre class="plain-lyrics">{currentLyrics.text}</pre>
              {:else if currentLyrics?.kind === "synced"}
                <div class="group-empty compact">
                  <h3>No lyric lines found</h3>
                  <p>This .lrc file was found, but it does not contain timestamped lyric lines.</p>
                </div>
              {:else}
                <div class="group-empty compact">
                  <h3>No local lyrics found</h3>
                  <p>Add a matching .lrc or .txt file next to the song, such as the same filename or lyrics.txt.</p>
                  <button class="auto-lyrics-button" type="button" disabled={isAutoFindingLyrics} onclick={() => void handleAutoFindLyrics()}>
                    {isAutoFindingLyrics ? "Searching..." : "Auto Find Lyrics"}
                  </button>
                </div>
              {/if}
            </section>

            <section class="now-playing-info-panel" aria-label="Playback status">
              <div>
                <p class="eyebrow">Playback</p>
                <h3>{isPlaying ? "Playing" : "Paused"}</h3>
              </div>
              <p>{formatPlaybackTime(positionSeconds)} elapsed of {formatPlaybackTime(currentTrackDuration)}. Use the player bar below for playback controls.</p>
            </section>

            <LibrarySection title="Up Next" viewAllLabel={`${queuePanelEntries.length} ${queuePanelEntries.length === 1 ? "track" : "tracks"}`}>
              {#if queuePanelEntries.length === 0}
                <div class="group-empty">
                  <h3>No queued songs</h3>
                  <p>Play a song from any list to build an Up Next queue.</p>
                </div>
              {:else}
                <div class="now-playing-queue-list">
                  {#each queuePanelEntries as entry (entry.track.id)}
                    <button
                      class:active={entry.queueIndex === currentQueueIndex}
                      type="button"
                      title={entry.track.filePath}
                      onclick={() => void playQueuedTrackAtIndex(entry.queueIndex)}
                    >
                      <span>{queuePositionLabel(entry.offset)}</span>
                      <div>
                        <strong>{entry.track.title}</strong>
                        <small>{entry.track.artist ?? "Unknown Artist"}{entry.track.album ? ` · ${entry.track.album}` : ""}</small>
                      </div>
                      <small>{formatPlaybackTime(entry.track.durationSeconds)}</small>
                    </button>
                  {/each}
                </div>
              {/if}
            </LibrarySection>
          {:else}
            <div class="now-playing-empty">
              <div class="now-playing-empty-mark" aria-hidden="true">N</div>
              <div>
                <p class="eyebrow">Now Playing</p>
                <h3 id="now-playing-title">No track playing</h3>
                <p>Select a song from the library to start playback.</p>
              </div>
            </div>
          {/if}
        </section>
      {:else if activeView === "Home"}
        <LibrarySection title="Recently Played" onViewAll={() => handleHomeSongsViewAll("recentlyPlayed", "desc")}>
          {#if recentlyPlayedTracks.length === 0}
            <div class="group-empty">
              <h3>No playback history yet</h3>
              <p>Played songs will appear here after they pass the listening threshold.</p>
            </div>
          {:else}
            <TrackList
              tracks={recentlyPlayedTracks}
              {isScanning}
              selectedTrackId={currentTrack?.id}
              onTrackSelect={handleTrackSelect}
              onTrackContextMenu={openTrackContextMenu}
              onArtistSelect={handleTrackArtistSelect}
              onAlbumSelect={handleTrackAlbumSelect}
              onToggleFavorite={handleToggleFavorite}
            />
          {/if}
        </LibrarySection>

        <LibrarySection title="Most Played" onViewAll={() => handleHomeSongsViewAll("playCount", "desc")}>
          {#if mostPlayedTracks.length === 0}
            <div class="group-empty">
              <h3>No play counts yet</h3>
              <p>Play counts begin once a song reaches 30 seconds or half its duration.</p>
            </div>
          {:else}
            <TrackList
              tracks={mostPlayedTracks}
              {isScanning}
              selectedTrackId={currentTrack?.id}
              onTrackSelect={handleTrackSelect}
              onTrackContextMenu={openTrackContextMenu}
              onArtistSelect={handleTrackArtistSelect}
              onAlbumSelect={handleTrackAlbumSelect}
              onToggleFavorite={handleToggleFavorite}
            />
          {/if}
        </LibrarySection>

        <LibrarySection title="Recently Added" onViewAll={() => handleHomeSongsViewAll("recentlyAdded", "desc")}>
          <TrackList
            tracks={homeTracks}
            {isScanning}
            selectedTrackId={currentTrack?.id}
            onTrackSelect={handleTrackSelect}
            onTrackContextMenu={openTrackContextMenu}
            onArtistSelect={handleTrackArtistSelect}
            onAlbumSelect={handleTrackAlbumSelect}
            onToggleFavorite={handleToggleFavorite}
          />
        </LibrarySection>

        <LibrarySection title="Albums" onViewAll={handleHomeAlbumsViewAll}>
          {#if homeAlbums.length === 0}
            <div class="group-empty">
              <h3>No albums found</h3>
              <p>Album tags were not found in the scanned tracks.</p>
            </div>
          {:else}
            <div class="album-grid">
              {#each homeAlbums as album}
                <button class="album-card" type="button" onclick={() => handleAlbumSelect(album)} oncontextmenu={(event) => openAlbumContextMenu(event, album)}>
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

        <LibrarySection title="Artists" onViewAll={handleHomeArtistsViewAll}>
          {#if homeArtists.length === 0}
            <div class="group-empty">
              <h3>No artists found</h3>
              <p>Artist tags were not found in the scanned tracks.</p>
            </div>
          {:else}
            <div class="artist-grid">
              {#each homeArtists as artist}
                <button class="artist-card" type="button" onclick={() => handleArtistSelect(artist)} oncontextmenu={(event) => openArtistContextMenu(event, artist)}>
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
      {:else if activeView === "Stats"}
        <section class="stats-page" aria-label="Listening and library statistics">
          <div class="stats-overview-grid" aria-label="Library and listening overview">
            <div class="stats-overview-card">
              <span>Total tracks</span>
              <strong>{tracks.length}</strong>
            </div>
            <div class="stats-overview-card">
              <span>Total albums</span>
              <strong>{hasLoadedCache ? displayAlbums.length : 0}</strong>
            </div>
            <div class="stats-overview-card">
              <span>Total artists</span>
              <strong>{hasLoadedCache ? displayArtists.length : 0}</strong>
            </div>
            <div class="stats-overview-card">
              <span>Total genres</span>
              <strong>{hasLoadedCache ? displayGenres.length : 0}</strong>
            </div>
            <div class="stats-overview-card">
              <span>Total plays</span>
              <strong>{statsTotalPlays}</strong>
            </div>
            <div class="stats-overview-card">
              <span>Liked songs</span>
              <strong>{favoriteTracks.length}</strong>
            </div>
            <div class="stats-overview-card">
              <span>Recently played</span>
              <strong>{statsRecentlyPlayedCount}</strong>
            </div>
            <div class="stats-overview-card muted">
              <span>Listening time</span>
              <strong>Coming later</strong>
            </div>
          </div>

          <LibrarySection title="Top Tracks" viewAllLabel="All time">
            {#if statsTopTracks.length === 0}
              <div class="group-empty">
                <h3>No top tracks yet</h3>
                <p>Tracks appear here after they pass the play-count threshold.</p>
              </div>
            {:else}
              <TrackList
                tracks={statsTopTracks}
                isScanning={false}
                selectedTrackId={currentTrack?.id}
                onTrackSelect={handleTrackSelect}
                onTrackContextMenu={openTrackContextMenu}
                onArtistSelect={handleTrackArtistSelect}
                onAlbumSelect={handleTrackAlbumSelect}
                onToggleFavorite={handleToggleFavorite}
              />
            {/if}
          </LibrarySection>

          <div class="stats-section-grid">
            <LibrarySection title="Top Artists" viewAllLabel="By total plays">
              {#if statsTopArtists.length === 0}
                <div class="group-empty compact">
                  <h3>No artist play data yet</h3>
                  <p>Artist totals will appear after songs are played.</p>
                </div>
              {:else}
                <div class="stats-rank-list">
                  {#each statsTopArtists as stat, index}
                    <button class="stats-rank-card" type="button" onclick={() => selectArtistName(stat.name)}>
                      <span class="stats-rank-number">{index + 1}</span>
                      <span class="artist-avatar stats-avatar" style={`--item-color: ${stat.color}`} aria-hidden="true">
                        {stat.name.slice(0, 1)}
                      </span>
                      <span class="stats-rank-copy">
                        <strong>{stat.name}</strong>
                        <small>{playsLabel(stat.totalPlays)} · {songCountLabel(stat.songCount)}</small>
                      </span>
                    </button>
                  {/each}
                </div>
              {/if}
            </LibrarySection>

            <LibrarySection title="Top Albums" viewAllLabel="By total plays">
              {#if statsTopAlbums.length === 0}
                <div class="group-empty compact">
                  <h3>No album play data yet</h3>
                  <p>Album totals will appear after songs are played.</p>
                </div>
              {:else}
                <div class="stats-rank-list">
                  {#each statsTopAlbums as stat, index}
                    <button class="stats-rank-card" type="button" onclick={() => handleAlbumSelect(stat.album)}>
                      <span class="stats-rank-number">{index + 1}</span>
                      <span class="album-art stats-cover" style={`--item-color: ${stat.album.color}`} aria-hidden="true">
                        {#if stat.album.coverArtPath}
                          <img
                            src={localImageSource(stat.album.coverArtPath) ?? ""}
                            alt=""
                            loading="lazy"
                            onload={showLoadedImage}
                            onerror={hideBrokenImage}
                          />
                        {/if}
                        <span></span>
                      </span>
                      <span class="stats-rank-copy">
                        <strong>{stat.album.title}</strong>
                        <small>{stat.album.artist} · {playsLabel(stat.totalPlays)} · {songCountLabel(stat.songCount)}</small>
                      </span>
                    </button>
                  {/each}
                </div>
              {/if}
            </LibrarySection>

            <LibrarySection title="Top Genres" viewAllLabel="By total plays">
              {#if statsTopGenres.length === 0}
                <div class="group-empty compact">
                  <h3>No genre play data yet</h3>
                  <p>Genre totals will appear after songs with genre data are played.</p>
                </div>
              {:else}
                <div class="stats-rank-list">
                  {#each statsTopGenres as stat, index}
                    <button class="stats-rank-card" type="button" onclick={() => handleGenreSelect(stat.genre)}>
                      <span class="stats-rank-number">{index + 1}</span>
                      <span class="genre-pill stats-genre-mark" style={`--item-color: ${stat.genre.color}`} aria-hidden="true">
                        {stat.genre.name.slice(0, 1)}
                      </span>
                      <span class="stats-rank-copy">
                        <strong>{stat.genre.name}</strong>
                        <small>{playsLabel(stat.totalPlays)} · {songCountLabel(stat.songCount)}</small>
                      </span>
                    </button>
                  {/each}
                </div>
              {/if}
            </LibrarySection>

            <LibrarySection title="Recently Played" viewAllLabel="Most recent">
              {#if statsRecentlyPlayedTracks.length === 0}
                <div class="group-empty compact">
                  <h3>No playback history yet</h3>
                  <p>Recently played tracks will appear after playback is recorded.</p>
                </div>
              {:else}
                <div class="stats-recent-list">
                  {#each statsRecentlyPlayedTracks as track}
                    <button
                      class="stats-recent-card"
                      type="button"
                      title={track.filePath}
                      onclick={() => void handleTrackSelect(track, statsRecentlyPlayedTracks)}
                      oncontextmenu={(event) => {
                        event.preventDefault();
                        event.stopPropagation();
                        openTrackContextMenu(track, statsRecentlyPlayedTracks, event.clientX, event.clientY);
                      }}
                    >
                      <span class="mini-cover stats-mini-cover" aria-hidden="true">
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
                      </span>
                      <span class="stats-rank-copy">
                        <strong>{track.title}</strong>
                        <small>{track.artist ?? "Unknown Artist"}{track.album ? ` · ${track.album}` : ""}</small>
                      </span>
                      <span class="stats-played-at">{lastPlayedLabel(track)}</span>
                    </button>
                  {/each}
                </div>
              {/if}
            </LibrarySection>
          </div>
        </section>
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
                <p>{albumHeroDetails(selectedAlbum, selectedAlbumTracks)}</p>
                <p class="album-genre-line">{selectedAlbumGenreText}</p>
                <div class="album-detail-actions">
                  <button type="button" disabled={selectedAlbumTracks.length === 0} onclick={() => void handlePlaySelectedAlbum()}>
                    Play Album
                  </button>
                  <button type="button" disabled={selectedAlbumTracks.length === 0} onclick={() => void handleShuffleSelectedAlbum()}>
                    Shuffle Album
                  </button>
                  <button type="button" disabled={selectedAlbumTracks.length === 0} onclick={handleAddSelectedAlbumToQueue}>
                    Add to Queue
                  </button>
                  <button type="button" onclick={focusAlbumGenreEditor}>
                    Edit Genres
                  </button>
                </div>
              </div>
            </div>

            <div class="genre-editor" aria-label="Album genre editor">
              <div class="genre-editor-copy">
                <p class="eyebrow">Genres</p>
                <p>{selectedAlbumGenreText}</p>
              </div>
              <form onsubmit={(event) => { event.preventDefault(); void handleSaveAlbumGenres(); }}>
                <input
                  bind:this={albumGenreInput}
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

            <LibrarySection title="Album Songs" viewAllLabel={normalizedSearchQuery ? `${selectedAlbumSearchTracks.length} ${selectedAlbumSearchTracks.length === 1 ? "match" : "matches"}` : `${selectedAlbumTracks.length} total`}>
              {#if selectedAlbumTracks.length === 0}
                <div class="group-empty">
                  <h3>No songs found for this album</h3>
                  <p>Scan the folder that contains the album tracks.</p>
                </div>
              {:else if selectedAlbumSearchTracks.length === 0}
                <div class="group-empty">
                  <h3>No songs matched</h3>
                  <p>Search is limited to tracks in this album.</p>
                </div>
              {:else}
                <div class="album-track-list">
                  {#each selectedAlbumDiscGroups as group (group.discNumber ?? "missing-disc")}
                    {#if selectedAlbumIsMultiDisc}
                      <h4>{albumDiscLabel(group.discNumber)}</h4>
                    {/if}
                    {#each group.tracks as track (track.id)}
                      <div
                        class:active={track.id === currentTrack?.id}
                        class="album-track-row"
                        role="button"
                        tabindex="0"
                        title={track.filePath}
                        onclick={() => handleAlbumTrackSelect(track)}
                        oncontextmenu={(event) => openAlbumTrackContextMenu(event, track)}
                        onkeydown={(event) => handleAlbumTrackKeydown(event, track)}
                      >
                        <span class:missing={track.trackNumber === null} class="album-track-number">
                          {albumTrackNumberLabel(track)}
                        </span>
                        <div class="track-title">
                          <span class="track-name">{track.title}</span>
                          <button class="track-link" type="button" onclick={(event) => { event.stopPropagation(); handleTrackArtistSelect(track); }}>
                            {track.artist ?? "Unknown Artist"}
                          </button>
                        </div>
                        <span class="album-track-duration">{track.durationSeconds === null ? "" : formatTrackDuration(track.durationSeconds)}</span>
                        <button
                          class:active={track.isFavorite}
                          class="favorite-button"
                          type="button"
                          aria-label={track.isFavorite ? "Remove from liked songs" : "Add to liked songs"}
                          onclick={(event) => { event.stopPropagation(); void handleToggleFavorite(track); }}
                        >
                          {track.isFavorite ? "★" : "☆"}
                        </button>
                        <span class="album-track-format">{track.extension.toUpperCase()}</span>
                      </div>
                    {/each}
                  {/each}
                </div>
              {/if}
            </LibrarySection>
          </section>
        {:else}
          <LibrarySection title="All Albums" viewAllLabel={`${visibleAlbums.length} total`}>
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
            {#if visibleAlbums.length === 0}
              <div class="group-empty">
                <h3>{normalizedSearchQuery ? "No albums matched" : "No albums found"}</h3>
                <p>{normalizedSearchQuery ? "Search is limited to album titles, artists, and years." : "Scan a music folder to build your local album library."}</p>
              </div>
            {:else}
              <div class="album-grid">
                {#each visibleAlbums as album}
                  <button class="album-card" type="button" onclick={() => handleAlbumSelect(album)} oncontextmenu={(event) => openAlbumContextMenu(event, album)}>
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

            <LibrarySection title="Albums" viewAllLabel={normalizedSearchQuery ? `${selectedArtistSearchAlbums.length} ${selectedArtistSearchAlbums.length === 1 ? "match" : "matches"}` : `${selectedArtistAlbums.length} total`}>
              {#if selectedArtistSearchAlbums.length === 0}
                <div class="group-empty">
                  <h3>{normalizedSearchQuery ? "No albums matched" : "No albums found"}</h3>
                  <p>{normalizedSearchQuery ? "Search is limited to this artist's albums." : "No album tags were found for this artist."}</p>
                </div>
              {:else}
                <div class="album-grid">
                  {#each selectedArtistSearchAlbums as album}
                    <button class="album-card" type="button" onclick={() => handleAlbumSelect(album)} oncontextmenu={(event) => openAlbumContextMenu(event, album)}>
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

            <LibrarySection title="Songs" viewAllLabel={normalizedSearchQuery ? `${selectedArtistSearchTracks.length} ${selectedArtistSearchTracks.length === 1 ? "match" : "matches"}` : `${selectedArtistTracks.length} total`}>
              {#if selectedArtistSearchTracks.length === 0}
                <div class="group-empty">
                  <h3>{normalizedSearchQuery ? "No songs matched" : "No songs found"}</h3>
                  <p>{normalizedSearchQuery ? "Search is limited to this artist's songs." : "No tracks were found for this artist."}</p>
                </div>
              {:else}
                <TrackList
                  tracks={selectedArtistSearchTracks}
                  isScanning={false}
                  selectedTrackId={currentTrack?.id}
                  onTrackSelect={handleTrackSelect}
                  onTrackContextMenu={openTrackContextMenu}
                  onArtistSelect={handleTrackArtistSelect}
                  onAlbumSelect={handleTrackAlbumSelect}
                  onToggleFavorite={handleToggleFavorite}
                />
              {/if}
            </LibrarySection>
          </section>
        {:else}
          <LibrarySection title="All Artists" viewAllLabel={`${visibleArtists.length} total`}>
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
            {#if visibleArtists.length === 0}
              <div class="group-empty">
                <h3>{normalizedSearchQuery ? "No artists matched" : "No artists found"}</h3>
                <p>{normalizedSearchQuery ? "Search is limited to artist names." : "Scan a music folder to build your local artist library."}</p>
              </div>
            {:else}
              <div class="artist-grid">
                {#each visibleArtists as artist}
                  <button class="artist-card" type="button" onclick={() => handleArtistSelect(artist)} oncontextmenu={(event) => openArtistContextMenu(event, artist)}>
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

            <LibrarySection title="Albums" viewAllLabel={normalizedSearchQuery ? `${selectedGenreSearchAlbums.length} ${selectedGenreSearchAlbums.length === 1 ? "match" : "matches"}` : `${selectedGenreAlbums.length} total`}>
              {#if selectedGenreSearchAlbums.length === 0}
                <div class="group-empty">
                  <h3>{normalizedSearchQuery ? "No albums matched" : "No albums found"}</h3>
                  <p>{normalizedSearchQuery ? "Search is limited to this genre's albums." : "No album tags were found for this genre."}</p>
                </div>
              {:else}
                <div class="album-grid">
                  {#each selectedGenreSearchAlbums as album}
                    <button class="album-card" type="button" onclick={() => handleAlbumSelect(album)} oncontextmenu={(event) => openAlbumContextMenu(event, album)}>
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

            <LibrarySection title="Artists" viewAllLabel={normalizedSearchQuery ? `${selectedGenreSearchArtists.length} ${selectedGenreSearchArtists.length === 1 ? "match" : "matches"}` : `${selectedGenreArtists.length} total`}>
              {#if selectedGenreSearchArtists.length === 0}
                <div class="group-empty">
                  <h3>{normalizedSearchQuery ? "No artists matched" : "No artists found"}</h3>
                  <p>{normalizedSearchQuery ? "Search is limited to this genre's artists." : "No artist tags were found for this genre."}</p>
                </div>
              {:else}
                <div class="artist-grid">
                  {#each selectedGenreSearchArtists as artist}
                    <button class="artist-card" type="button" onclick={() => handleArtistSelect(artist)} oncontextmenu={(event) => openArtistContextMenu(event, artist)}>
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

            <LibrarySection title="Songs" viewAllLabel={normalizedSearchQuery ? `${selectedGenreSearchTracks.length} ${selectedGenreSearchTracks.length === 1 ? "match" : "matches"}` : `${selectedGenreTracks.length} total`}>
              {#if selectedGenreSearchTracks.length === 0}
                <div class="group-empty">
                  <h3>{normalizedSearchQuery ? "No songs matched" : "No songs found"}</h3>
                  <p>{normalizedSearchQuery ? "Search is limited to this genre's songs." : "No tracks were found for this genre."}</p>
                </div>
              {:else}
                <TrackList
                  tracks={selectedGenreSearchTracks}
                  isScanning={false}
                  selectedTrackId={currentTrack?.id}
                  onTrackSelect={handleTrackSelect}
                  onTrackContextMenu={openTrackContextMenu}
                  onArtistSelect={handleTrackArtistSelect}
                  onAlbumSelect={handleTrackAlbumSelect}
                  onToggleFavorite={handleToggleFavorite}
                />
              {/if}
            </LibrarySection>
          </section>
        {:else}
          <LibrarySection title="All Genres" viewAllLabel={`${visibleGenres.length} total`}>
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
            {#if visibleGenres.length === 0}
              <div class="group-empty">
                <h3>{normalizedSearchQuery ? "No genres matched" : "No genres found"}</h3>
                <p>{normalizedSearchQuery ? "Search is limited to genre names." : "Scan a music folder to build your local genre library."}</p>
              </div>
            {:else}
              <div class="genre-grid">
                {#each visibleGenres as genre}
                  <button class="genre-card" type="button" onclick={() => handleGenreSelect(genre)} oncontextmenu={(event) => openGenreContextMenu(event, genre)}>
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
        <LibrarySection title="All Songs" viewAllLabel={`${visibleSongTracks.length} shown`}>
          <div class="control-bar">
            <label>
              <span>Sort</span>
              <select bind:value={songSort} onchange={handleSongSortChange}>
                <option value="title">Title</option>
                <option value="artist">Artist</option>
                <option value="album">Album</option>
                <option value="duration">Duration</option>
                <option value="recentlyAdded">Recently added</option>
                <option value="recentlyPlayed">Recently played</option>
                <option value="playCount">Most played</option>
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
          {#if normalizedSearchQuery && visibleSongTracks.length === 0}
            <div class="group-empty">
              <h3>No songs matched</h3>
              <p>Search is limited to the Songs view.</p>
            </div>
          {:else}
            <TrackList
              tracks={visibleSongTracks}
              {isScanning}
              selectedTrackId={currentTrack?.id}
              onTrackSelect={handleTrackSelect}
              onTrackContextMenu={openTrackContextMenu}
              onArtistSelect={handleTrackArtistSelect}
              onAlbumSelect={handleTrackAlbumSelect}
              onToggleFavorite={handleToggleFavorite}
            />
          {/if}
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

            {#if playlistError}
              <div class="scan-error" role="alert">{playlistError}</div>
            {:else if playlistMessage}
              <div class="scan-error status-message" role="status">{playlistMessage}</div>
            {/if}

            <LibrarySection title="Songs" viewAllLabel={normalizedSearchQuery ? `${filteredLikedTracks.length} ${filteredLikedTracks.length === 1 ? "match" : "matches"}` : `${favoriteTracks.length} total`}>
              {#if favoriteTracks.length === 0}
                <div class="group-empty">
                  <h3>No liked songs yet</h3>
                  <p>Use the star button on any song row or in the player to add it here.</p>
                </div>
              {:else if filteredLikedTracks.length === 0}
                <div class="group-empty">
                  <h3>No songs matched</h3>
                  <p>Search is limited to liked songs.</p>
                </div>
              {:else}
                <TrackList
                  tracks={filteredLikedTracks}
                  isScanning={false}
                  selectedTrackId={currentTrack?.id}
                  onTrackSelect={handleTrackSelect}
                  onTrackContextMenu={openTrackContextMenu}
                  onArtistSelect={handleTrackArtistSelect}
                  onAlbumSelect={handleTrackAlbumSelect}
                  onToggleFavorite={handleToggleFavorite}
                />
              {/if}
            </LibrarySection>
          </section>
        {:else if selectedPlaylist}
          <section class="detail-view" aria-labelledby="custom-playlist-title">
            <div class="detail-actions">
              <button class="back-button" type="button" onclick={handleBackToPlaylists}>Back to Playlists</button>
              <button class="back-button accent" type="button" onclick={() => void handleRenamePlaylist(selectedPlaylist)}>
                Rename
              </button>
              <button class="back-button destructive" type="button" onclick={() => openDeletePlaylistConfirmation(selectedPlaylist)}>
                Delete Playlist
              </button>
            </div>
            <div class="playlist-detail-header">
              <div class="playlist-mark" aria-hidden="true">P</div>
              <div class="detail-copy">
                <p class="eyebrow">Playlist</p>
                <h3 id="custom-playlist-title">{selectedPlaylist.name}</h3>
                <p>{playlistTrackLabel(selectedPlaylist)}</p>
              </div>
            </div>

            {#if playlistError}
              <div class="scan-error" role="alert">{playlistError}</div>
            {:else if playlistMessage}
              <div class="scan-error status-message" role="status">{playlistMessage}</div>
            {/if}
            {#if selectedPlaylistMissingTrackCount > 0}
              <div class="playlist-warning" role="status">
                {selectedPlaylistMissingTrackCount} {selectedPlaylistMissingTrackCount === 1 ? "track is" : "tracks are"} unavailable because the file is no longer in the scanned library.
              </div>
            {/if}

            <LibrarySection title="Songs" viewAllLabel={normalizedSearchQuery ? `${selectedPlaylistSearchTracks.length} ${selectedPlaylistSearchTracks.length === 1 ? "match" : "matches"}` : `${selectedPlaylistTracks.length} playable`}>
              {#if selectedPlaylistTracks.length === 0}
                <div class="group-empty">
                  {#if selectedPlaylist.trackIds.length > 0}
                    <h3>No available songs in this playlist</h3>
                    <p>Rescan the folder that contains these files, or add songs from a track context menu.</p>
                  {:else}
                    <h3>No songs in this playlist</h3>
                    <p>Add songs from a track context menu.</p>
                  {/if}
                </div>
              {:else if selectedPlaylistSearchTracks.length === 0}
                <div class="group-empty">
                  <h3>No songs matched</h3>
                  <p>Search is limited to songs in this playlist.</p>
                </div>
              {:else}
                <TrackList
                  tracks={selectedPlaylistSearchTracks}
                  isScanning={false}
                  selectedTrackId={currentTrack?.id}
                  onTrackSelect={handleTrackSelect}
                  onTrackContextMenu={openTrackContextMenu}
                  onArtistSelect={handleTrackArtistSelect}
                  onAlbumSelect={handleTrackAlbumSelect}
                  onToggleFavorite={handleToggleFavorite}
                  onRemoveTrack={handleRemoveTrackFromSelectedPlaylist}
                  onMoveTrackUp={(track) => void handleMoveTrackInSelectedPlaylist(track, "up")}
                  onMoveTrackDown={(track) => void handleMoveTrackInSelectedPlaylist(track, "down")}
                  canMoveTrackUp={(track) => canMoveSelectedPlaylistTrack(track, "up")}
                  canMoveTrackDown={(track) => canMoveSelectedPlaylistTrack(track, "down")}
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
                    <label class:selected={mixSelectedGenreSet.has(genre.name)} class="mix-option-card" oncontextmenu={(event) => openGenreContextMenu(event, genre)}>
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
                    <label class:selected={mixSelectedArtistSet.has(artist.name)} class="mix-option-card" oncontextmenu={(event) => openArtistContextMenu(event, artist)}>
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
                    <label class:selected={mixSelectedAlbumSet.has(album.id)} class="mix-option-card" oncontextmenu={(event) => openAlbumContextMenu(event, album)}>
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
            <form class="playlist-create-card" onsubmit={(event) => { event.preventDefault(); void handleCreatePlaylist(); }}>
              <div class="playlist-mark" aria-hidden="true">+</div>
              <div>
                <p class="eyebrow">Custom Playlist</p>
                <label for="playlist-name">Create Playlist</label>
                <div>
                  <input
                    id="playlist-name"
                    type="text"
                    bind:value={playlistNameDraft}
                    placeholder="Playlist name"
                    aria-label="Playlist name"
                    aria-invalid={playlistError ? "true" : "false"}
                  />
                  <button type="submit" disabled={!canCreatePlaylist}>Create</button>
                </div>
                {#if playlistError}
                  <p class="form-message error" role="alert">{playlistError}</p>
                {:else if playlistMessage}
                  <p class="form-message" role="status">{playlistMessage}</p>
                {/if}
              </div>
            </form>
            {#each playlists as playlist}
              <button
                class="playlist-card"
                type="button"
                onclick={() => handlePlaylistSelect(playlist)}
                oncontextmenu={(event) => openPlaylistContextMenu(event, playlist)}
              >
                <div class="playlist-mark" aria-hidden="true">P</div>
                <div>
                  <p class="eyebrow">Custom Playlist</p>
                  <h3>{playlist.name}</h3>
                  <p>{playlistTrackLabel(playlist)}</p>
                </div>
              </button>
            {/each}
          </section>
        {/if}
      {:else if activeView === "Live Shows"}
        <section class="videos-page" aria-labelledby="videos-title">
          {#if selectedVideo}
            <button class="back-button" type="button" onclick={handleBackToVideos}>Back to Live Shows</button>

            <section class="video-detail-hero" aria-labelledby="video-detail-title">
              <div class="video-player-panel">
                {#if selectedVideoSrc}
                  <!-- svelte-ignore a11y_media_has_caption -->
                  <video
                    bind:this={videoElement}
                    src={selectedVideoSrc}
                    poster={selectedVideoThumbnailSrc ?? undefined}
                    controls
                    preload="metadata"
                    onplay={handleVideoPlay}
                    onpause={handleVideoPause}
                    onended={handleVideoEnded}
                    ontimeupdate={handleVideoTimeUpdate}
                    onloadedmetadata={handleVideoLoadedMetadata}
                    onerror={() => videoPlaybackError = "Cassette could not play this file in the app. Try opening it in an external player."}
                  ></video>
                {:else if selectedVideoThumbnailSrc}
                  <img src={selectedVideoThumbnailSrc} alt="" onload={showLoadedImage} onerror={hideBrokenImage} />
                {:else}
                  <div class="video-placeholder" aria-hidden="true">V</div>
                {/if}
              </div>

              <div class="video-detail-copy">
                <p class="eyebrow">Live Show</p>
                <h3 id="video-detail-title">{selectedVideo.title}</h3>
                <p>{videoDetailLine(selectedVideo)}</p>
                <div class="video-meta-grid">
                  <div><span>Artist</span><strong>{selectedVideo.artist ?? "Unknown Artist"}</strong></div>
                  <div><span>Show</span><strong>{selectedVideo.showTitle ?? selectedVideo.albumOrRelease ?? "Not set"}</strong></div>
                  <div><span>Year</span><strong>{selectedVideo.year ?? "Not set"}</strong></div>
                  <div><span>Duration</span><strong>{formatVideoDuration(selectedVideo.durationSeconds)}</strong></div>
                  <div><span>Last watched</span><strong>{formatVideoDuration(selectedVideo.lastPositionSeconds)}</strong></div>
                  <div><span>Plays</span><strong>{selectedVideo.playCount}</strong></div>
                </div>
                {#if videoLocationLine(selectedVideo)}
                  <p class="video-location">{videoLocationLine(selectedVideo)}</p>
                {/if}
                <div class="video-actions">
                  <button type="button" onclick={() => void handleVideoPlayFromStart()}>Play</button>
                  <button type="button" disabled={selectedVideo.lastPositionSeconds <= 0} onclick={() => void handleVideoResume()}>Resume</button>
                  <button type="button" onclick={handleEditVideoInfo}>Edit Info</button>
                  <button type="button" onclick={() => void handleResetVideoProgress()}>Reset Progress</button>
                  <button type="button" onclick={() => void handleOpenVideoExternal()}>Open External</button>
                </div>
                {#if videoPlaybackError}
                  <p class="form-message error" role="alert">{videoPlaybackError}</p>
                {:else if videoPlaybackMessage}
                  <p class="form-message" role="status">{videoPlaybackMessage}</p>
                {/if}
              </div>
            </section>

            {#if isEditingVideo}
              <section class="video-edit-panel" aria-labelledby="video-edit-title">
                <div class="settings-section-header">
                  <div>
                    <p class="eyebrow">Manual Metadata</p>
                    <h4 id="video-edit-title">Edit Info</h4>
                  </div>
                  <span class="settings-pill">SQLite only</span>
                </div>
                <form class="video-edit-form" onsubmit={(event) => { event.preventDefault(); void handleSaveVideoInfo(); }}>
                  <label>
                    <span>Title</span>
                    <input type="text" value={videoEditDraft.title} oninput={(event) => updateVideoDraftField("title", inputValue(event))} />
                  </label>
                  <label>
                    <span>Artist</span>
                    <input type="text" value={videoEditDraft.artist} oninput={(event) => updateVideoDraftField("artist", inputValue(event))} />
                  </label>
                  <label>
                    <span>Show title</span>
                    <input type="text" value={videoEditDraft.showTitle} oninput={(event) => updateVideoDraftField("showTitle", inputValue(event))} />
                  </label>
                  <label>
                    <span>Album/release</span>
                    <input type="text" value={videoEditDraft.albumOrRelease} oninput={(event) => updateVideoDraftField("albumOrRelease", inputValue(event))} />
                  </label>
                  <label>
                    <span>Year</span>
                    <input type="number" min="1" max="9999" value={videoEditDraft.year} oninput={(event) => updateVideoDraftField("year", inputValue(event))} />
                  </label>
                  <label>
                    <span>Venue</span>
                    <input type="text" value={videoEditDraft.venue} oninput={(event) => updateVideoDraftField("venue", inputValue(event))} />
                  </label>
                  <label>
                    <span>City</span>
                    <input type="text" value={videoEditDraft.city} oninput={(event) => updateVideoDraftField("city", inputValue(event))} />
                  </label>
                  <label>
                    <span>Country</span>
                    <input type="text" value={videoEditDraft.country} oninput={(event) => updateVideoDraftField("country", inputValue(event))} />
                  </label>
                  <div class="video-edit-actions">
                    <button class="primary" type="submit" disabled={isSavingVideoInfo}>
                      {isSavingVideoInfo ? "Saving..." : "Save Info"}
                    </button>
                    <button type="button" disabled={isSavingVideoInfo} onclick={handleCancelVideoEdit}>Cancel</button>
                  </div>
                </form>
              </section>
            {/if}
          {:else}
            <div class="videos-toolbar">
              <div class="video-search">
                <input
                  type="search"
                  bind:value={videoSearchQuery}
                  placeholder="Search videos..."
                  aria-label="Search videos"
                />
                {#if videoSearchQuery}
                  <button type="button" aria-label="Clear video search" onclick={() => videoSearchQuery = ""}>Clear</button>
                {/if}
              </div>
              <label>
                <span>Sort</span>
                <select bind:value={videoSort}>
                  <option value="title">Title</option>
                  <option value="artist">Artist</option>
                  <option value="year">Year</option>
                  <option value="recentlyPlayed">Recently played</option>
                  <option value="duration">Duration</option>
                </select>
              </label>
              <button
                class="direction-toggle"
                type="button"
                aria-label={`Video sort direction: ${sortDirectionLabel(videoSortDirection)}`}
                onclick={() => videoSortDirection = nextSortDirection(videoSortDirection)}
              >
                {sortDirectionLabel(videoSortDirection)}
              </button>
              <button type="button" disabled={isScanningVideos} onclick={() => void handleAddVideoFolder()}>
                Add Video Folder
              </button>
              <button type="button" disabled={isScanningVideos || !videoFolder} onclick={() => void handleRescanVideos()}>
                Rescan Videos
              </button>
            </div>

            <div class="video-stat-grid" aria-label="Live Shows summary">
              {#each videoLibraryStats as stat}
                <div>
                  <span>{stat.label}</span>
                  <strong>{stat.value}</strong>
                </div>
              {/each}
            </div>

            <section class="dvd-placeholder" aria-labelledby="dvd-placeholder-title">
              <div>
                <p class="eyebrow">Future Import</p>
                <h3 id="dvd-placeholder-title">DVD Import</h3>
                <p>Coming later: import readable live show/music video DVDs. Cassette will not bypass DRM.</p>
              </div>
              <span>Disabled</span>
            </section>

            <p class="settings-note">Video thumbnails/durations use ffmpeg/ffprobe when available.</p>

            {#if visibleVideos.length === 0}
              <div class="group-empty">
                <h3>{videos.length === 0 ? "No videos imported" : "No videos matched"}</h3>
                <p>{videos.length === 0 ? "Add a folder containing MP4, MKV, WebM, MOV, M4V, or AVI files." : "Try a title, artist, venue, city, year, or file name."}</p>
              </div>
            {:else}
              <div class="video-grid">
                {#each visibleVideos as video (video.id)}
                  <button class="video-card" type="button" title={video.filePath} onclick={() => handleVideoSelect(video)}>
                    <span class="video-thumb" aria-hidden="true">
                      {#if video.thumbnailPath}
                        <img
                          src={localImageSource(video.thumbnailPath) ?? ""}
                          alt=""
                          loading="lazy"
                          onload={showLoadedImage}
                          onerror={hideBrokenImage}
                        />
                      {:else}
                        <span>V</span>
                      {/if}
                      {#if video.lastPositionSeconds > 0}
                        <span class="video-progress" style={`--progress: ${videoProgressPercent(video)}%`}></span>
                      {/if}
                    </span>
                    <span class="video-card-copy">
                      <strong>{video.title}</strong>
                      <small>{videoCardDetail(video)}</small>
                      <small>{formatVideoDuration(video.durationSeconds)}{video.lastPositionSeconds > 0 ? ` · ${formatVideoDuration(video.lastPositionSeconds)} watched` : ""}</small>
                    </span>
                  </button>
                {/each}
              </div>
            {/if}
          {/if}
        </section>
      {:else if activeView === "CD Rip"}
        <section class="cd-ripper-page" aria-labelledby="cd-ripper-title">
          <div class="settings-intro">
            <p class="eyebrow">System Tools</p>
            <h3 id="cd-ripper-title">CD Ripper</h3>
            <p>Cassette uses system tools like cdparanoia and flac for CD ripping on Linux.</p>
          </div>

          <section class="cd-status-card" aria-labelledby="cd-status-title">
            <div class="settings-section-header">
              <div>
                <p class="eyebrow">Drive Status</p>
                <h4 id="cd-status-title">Audio CD</h4>
              </div>
              <span class="settings-pill">{isRippingCd ? "Ripping" : isDetectingCd ? "Detecting" : "Idle"}</span>
            </div>

            <div class="cd-status-grid">
              <div>
                <span>CD drive</span>
                <strong>{cdDriveFound === null ? "Not checked" : cdDriveFound ? "Detected" : "Not detected"}</strong>
              </div>
              <div>
                <span>Audio CD</span>
                <strong>{audioCdFound === null ? "Not checked" : audioCdFound ? "Detected" : "Not detected"}</strong>
              </div>
              <div>
                <span>Tracks</span>
                <strong>{cdTracks.length > 0 ? cdTracks.length : "Not available"}</strong>
              </div>
            </div>

            <div class="cd-rip-actions">
              <button type="button" disabled={isDetectingCd || isRippingCd} onclick={() => void handleDetectCd()}>
                {isDetectingCd ? "Detecting..." : "Detect CD"}
              </button>
              <button type="button" disabled={isRippingCd} onclick={() => void handleChooseCdOutputFolder()}>
                Choose Output Folder
              </button>
              <button class="primary" type="button" disabled={isDetectingCd || isRippingCd} onclick={() => void handleRipCd()}>
                {isRippingCd ? "Ripping..." : "Rip CD"}
              </button>
              {#if lastRippedFolder}
                <button type="button" disabled={isScanning || isRippingCd} onclick={() => void handleScanRippedFolder()}>
                  {isScanning ? "Scanning..." : "Scan ripped folder"}
                </button>
              {/if}
            </div>

            <div class="cd-output-folder">
              <span>Output folder</span>
              <strong>{cdOutputFolder ?? "No output folder selected"}</strong>
            </div>

            {#if lastRippedFolder}
              <div class="cd-output-folder">
                <span>Last rip</span>
                <strong>{lastRippedFolder}</strong>
              </div>
            {/if}

            {#if cdRipError}
              <div class="scan-error" role="alert">{cdRipError}</div>
            {:else if cdRipMessage}
              <div class="scan-error status-message" role="status">{cdRipMessage}</div>
            {/if}
          </section>

          <section class="cd-metadata-section" aria-labelledby="cd-metadata-title">
            <div class="settings-section-header">
              <div>
                <p class="eyebrow">MusicBrainz</p>
                <h4 id="cd-metadata-title">Metadata</h4>
              </div>
              <span class="settings-pill">{cdDiscId ? `Disc ID ${cdDiscId}` : "Manual mode"}</span>
            </div>

            <div class="cd-rip-actions">
              <button type="button" disabled={isLookingUpCdMetadata || isDetectingCd || isRippingCd} onclick={() => void handleLookupCdMetadata()}>
                {isLookingUpCdMetadata ? "Looking up metadata..." : "Lookup Metadata"}
              </button>
            </div>

            {#if cdMetadataError}
              <div class="scan-error" role="alert">{cdMetadataError}</div>
            {:else if cdMetadataMessage}
              <div class="scan-error status-message" role="status">{cdMetadataMessage}</div>
            {/if}

            {#if cdMetadataResults.length > 1}
              <div class="cd-release-list" aria-label="MusicBrainz release results">
                {#each cdMetadataResults as release}
                  <button
                    class:active={release.id === selectedCdReleaseId}
                    type="button"
                    disabled={isRippingCd}
                    onclick={() => selectCdMetadataRelease(release)}
                  >
                    <strong>{release.title}</strong>
                    <small>{releaseDetail(release)}</small>
                  </button>
                {/each}
              </div>
            {:else if cdMetadataResults.length === 1 && selectedCdRelease}
              <div class="cd-selected-release">
                <span>Selected release</span>
                <strong>{selectedCdRelease.title}</strong>
                <small>{releaseDetail(selectedCdRelease)}</small>
              </div>
            {:else if cdTracks.length > 0 && !isLookingUpCdMetadata && cdMetadataMessage?.toLowerCase().includes("no metadata")}
              <div class="group-empty compact">
                <h3>No metadata found</h3>
                <p>You can still type album and track metadata manually before ripping.</p>
              </div>
            {/if}

            {#if cdTracks.length > 0 && cdRipMetadata}
              <div class="cd-metadata-form">
                <label>
                  <span>Album artist</span>
                  <input
                    type="text"
                    disabled={isRippingCd}
                    value={cdRipMetadata.albumArtist}
                    oninput={(event) => updateCdAlbumMetadata("albumArtist", inputValue(event))}
                  />
                </label>
                <label>
                  <span>Album title</span>
                  <input
                    type="text"
                    disabled={isRippingCd}
                    value={cdRipMetadata.albumTitle}
                    oninput={(event) => updateCdAlbumMetadata("albumTitle", inputValue(event))}
                  />
                </label>
                <label>
                  <span>Year</span>
                  <input
                    type="text"
                    inputmode="numeric"
                    disabled={isRippingCd}
                    value={cdRipMetadata.year}
                    oninput={(event) => updateCdAlbumMetadata("year", inputValue(event))}
                  />
                </label>
                <label>
                  <span>Disc number</span>
                  <input
                    type="number"
                    min="1"
                    disabled={isRippingCd}
                    value={cdRipMetadata.discNumber ?? ""}
                    oninput={(event) => updateCdDiscNumber(inputValue(event))}
                  />
                </label>
                <label>
                  <span>Genre</span>
                  <input
                    type="text"
                    disabled={isRippingCd}
                    value={cdRipMetadata.genre}
                    oninput={(event) => updateCdAlbumMetadata("genre", inputValue(event))}
                  />
                </label>
              </div>

              <div class="cd-cover-panel">
                <div class="cd-cover-preview">
                  {#if selectedCdCoverSrc}
                    <img src={selectedCdCoverSrc} alt="Selected album cover" onerror={hideBrokenImage} onload={showLoadedImage} />
                  {:else}
                    <span>No cover</span>
                  {/if}
                </div>
                <div class="cd-cover-copy">
                  <span>Album cover</span>
                  <strong>
                    {#if isLookingUpCdCover}
                      Loading cover...
                    {:else if selectedCdCover?.source === "cover-art-archive"}
                      Cover found
                    {:else if selectedCdCover?.source === "manual"}
                      Manual cover selected
                    {:else if cdCoverMessage}
                      {cdCoverMessage}
                    {:else}
                      No cover art found
                    {/if}
                  </strong>
                  <small>{selectedCdCover?.source === "cover-art-archive" ? "Cover Art Archive" : selectedCdCover?.source === "manual" ? "Local image" : "Optional for this rip"}</small>
                  {#if cdCoverError}
                    <small class="error">{cdCoverError}</small>
                  {/if}
                  <button type="button" disabled={isRippingCd} onclick={() => void handleChooseCoverImage()}>
                    Choose Cover Image
                  </button>
                </div>
              </div>
            {/if}
          </section>

          <section class="cd-track-section" aria-labelledby="cd-track-list-title">
            <div class="settings-section-header">
              <div>
                <p class="eyebrow">Rip Queue</p>
                <h4 id="cd-track-list-title">Tracks</h4>
              </div>
              <span class="settings-pill">{cdTracks.length} {cdTracks.length === 1 ? "track" : "tracks"}</span>
            </div>

            {#if cdTracks.length === 0}
              <div class="group-empty compact">
                <h3>No CD tracks detected</h3>
                <p>Insert an audio CD and click Detect CD.</p>
              </div>
            {:else}
              <div class="cd-track-table" role="table" aria-label="CD tracks">
                <div class="cd-track-row cd-track-head" role="row">
                  <span role="columnheader">Track</span>
                  <span role="columnheader">Title</span>
                  <span role="columnheader">Artist</span>
                  <span role="columnheader">Duration</span>
                  <span role="columnheader">Status</span>
                  <span role="columnheader">Output filename</span>
                </div>
                {#each cdTracks as track}
                  <div class:active={track.status === "ripping"} class:error={track.status === "error"} class="cd-track-row" role="row">
                    <span role="cell">{String(track.number).padStart(2, "0")}</span>
                    <span role="cell">
                      <input
                        type="text"
                        aria-label={`Track ${track.number} title`}
                        disabled={isRippingCd}
                        value={cdMetadataTrack(track.number).title}
                        oninput={(event) => updateCdTrackMetadata(track.number, "title", inputValue(event))}
                      />
                    </span>
                    <span role="cell">
                      <input
                        type="text"
                        aria-label={`Track ${track.number} artist`}
                        disabled={isRippingCd}
                        value={cdMetadataTrack(track.number).artist}
                        oninput={(event) => updateCdTrackMetadata(track.number, "artist", inputValue(event))}
                      />
                    </span>
                    <span role="cell">{track.duration ?? "Unknown"}</span>
                    <span role="cell">
                      <strong>{track.status ?? "pending"}</strong>
                      {#if track.error}
                        <small>{track.error}</small>
                      {:else if track.warning}
                        <small>{track.warning}</small>
                      {/if}
                    </span>
                    <span role="cell">{cdTrackOutputFilenameDisplay(track)}</span>
                  </div>
                {/each}
              </div>
            {/if}
          </section>

          {#if cdRawOutput}
            <details class="cd-debug-output">
              <summary>cdparanoia output</summary>
              <pre>{cdRawOutput}</pre>
            </details>
          {/if}
        </section>
      {:else if activeView === "Settings"}
        {#if isLibraryHealthOpen}
          <section class="detail-view health-detail" aria-labelledby="library-health-title">
            <button class="back-button" type="button" onclick={handleBackToSettings}>Back to Settings</button>
            <div class="playlist-detail-header">
              <div class="health-mark" aria-hidden="true">H</div>
              <div class="detail-copy">
                <p class="eyebrow">Library Diagnostics</p>
                <h3 id="library-health-title">Library Health</h3>
                <p>{libraryHealthIssueCount} {libraryHealthIssueCount === 1 ? "issue" : "issues"} found in {libraryDiagnostics.totalTracks} {libraryDiagnostics.totalTracks === 1 ? "track" : "tracks"}</p>
              </div>
            </div>

            <div class="health-summary-grid" aria-label="Library health summary">
              <div><span>Total tracks</span><strong>{libraryDiagnostics.totalTracks}</strong></div>
              <div><span>Total albums</span><strong>{libraryDiagnostics.totalAlbums}</strong></div>
              <div><span>Total artists</span><strong>{libraryDiagnostics.totalArtists}</strong></div>
              <div class:issue={libraryDiagnostics.missingGenreTracks.length > 0}><span>Missing genre</span><strong>{libraryDiagnostics.missingGenreTracks.length}</strong></div>
              <div class:issue={libraryDiagnostics.missingCoverTracks.length > 0}><span>Missing cover art</span><strong>{libraryDiagnostics.missingCoverTracks.length}</strong></div>
              <div class:issue={libraryDiagnostics.unknownArtistTracks.length > 0}><span>Unknown artist</span><strong>{libraryDiagnostics.unknownArtistTracks.length}</strong></div>
              <div class:issue={libraryDiagnostics.unknownAlbumTracks.length > 0}><span>Unknown album</span><strong>{libraryDiagnostics.unknownAlbumTracks.length}</strong></div>
              <div class:issue={libraryDiagnostics.missingTrackNumberTracks.length > 0}><span>Missing track number</span><strong>{libraryDiagnostics.missingTrackNumberTracks.length}</strong></div>
              <div class:issue={libraryDiagnostics.missingYearTracks.length > 0}><span>Missing year</span><strong>{libraryDiagnostics.missingYearTracks.length}</strong></div>
              <div class:issue={libraryDiagnostics.duplicateAlbumGroups.length > 0}><span>Possible duplicate albums</span><strong>{libraryDiagnostics.duplicateAlbumGroups.length}</strong></div>
            </div>

            <LibrarySection title="Missing Genre" viewAllLabel={issueCountLabel(libraryDiagnostics.missingGenreTracks.length)}>
              {#if libraryDiagnostics.missingGenreTracks.length === 0}
                <div class="group-empty compact">
                  <h3>No issues found</h3>
                  <p>All cached tracks have genre data or a Cassette genre assignment.</p>
                </div>
              {:else}
                <div class="diagnostic-album-list">
                  {#each albumsForTracks(libraryDiagnostics.missingGenreTracks, sortedAlbums) as album}
                    <button class="diagnostic-album-card" type="button" onclick={() => handleAlbumSelect(album)} oncontextmenu={(event) => openAlbumContextMenu(event, album)}>
                      <span>{album.title.slice(0, 1)}</span>
                      <div>
                        <strong>{album.title}</strong>
                        <small>{album.artist} · open album to set genre</small>
                      </div>
                    </button>
                  {/each}
                </div>
                <TrackList
                  tracks={libraryDiagnostics.missingGenreTracks}
                  isScanning={false}
                  selectedTrackId={currentTrack?.id}
                  onTrackSelect={handleTrackSelect}
                  onTrackContextMenu={openTrackContextMenu}
                  onArtistSelect={handleTrackArtistSelect}
                  onAlbumSelect={handleTrackAlbumSelect}
                  onToggleFavorite={handleToggleFavorite}
                />
              {/if}
            </LibrarySection>

            <LibrarySection title="Missing Cover Art" viewAllLabel={`${issueCountLabel(libraryDiagnostics.missingCoverTracks.length)} · ${issueCountLabel(libraryDiagnostics.missingCoverAlbums.length, "album")}`}>
              {#if libraryDiagnostics.missingCoverTracks.length === 0 && libraryDiagnostics.missingCoverAlbums.length === 0}
                <div class="group-empty compact">
                  <h3>No issues found</h3>
                  <p>All cached tracks and albums have cover art.</p>
                </div>
              {:else}
                {#if libraryDiagnostics.missingCoverAlbums.length > 0}
                  <div class="diagnostic-album-list">
                    {#each libraryDiagnostics.missingCoverAlbums as album}
                      <button class="diagnostic-album-card" type="button" onclick={() => handleAlbumSelect(album)} oncontextmenu={(event) => openAlbumContextMenu(event, album)}>
                        <span>{album.title.slice(0, 1)}</span>
                        <div>
                          <strong>{album.title}</strong>
                          <small>{album.artist} · {album.trackCount} {album.trackCount === 1 ? "track" : "tracks"}</small>
                        </div>
                      </button>
                    {/each}
                  </div>
                {/if}
                <TrackList
                  tracks={libraryDiagnostics.missingCoverTracks}
                  isScanning={false}
                  selectedTrackId={currentTrack?.id}
                  onTrackSelect={handleTrackSelect}
                  onTrackContextMenu={openTrackContextMenu}
                  onArtistSelect={handleTrackArtistSelect}
                  onAlbumSelect={handleTrackAlbumSelect}
                  onToggleFavorite={handleToggleFavorite}
                />
              {/if}
            </LibrarySection>

            <LibrarySection title="Unknown Artist" viewAllLabel={issueCountLabel(libraryDiagnostics.unknownArtistTracks.length)}>
              {#if libraryDiagnostics.unknownArtistTracks.length === 0}
                <div class="group-empty compact"><h3>No issues found</h3><p>No tracks have an empty or unknown artist.</p></div>
              {:else}
                <TrackList
                  tracks={libraryDiagnostics.unknownArtistTracks}
                  isScanning={false}
                  selectedTrackId={currentTrack?.id}
                  onTrackSelect={handleTrackSelect}
                  onTrackContextMenu={openTrackContextMenu}
                  onArtistSelect={handleTrackArtistSelect}
                  onAlbumSelect={handleTrackAlbumSelect}
                  onToggleFavorite={handleToggleFavorite}
                />
              {/if}
            </LibrarySection>

            <LibrarySection title="Unknown Album" viewAllLabel={issueCountLabel(libraryDiagnostics.unknownAlbumTracks.length)}>
              {#if libraryDiagnostics.unknownAlbumTracks.length === 0}
                <div class="group-empty compact"><h3>No issues found</h3><p>No tracks have an empty or unknown album.</p></div>
              {:else}
                <TrackList
                  tracks={libraryDiagnostics.unknownAlbumTracks}
                  isScanning={false}
                  selectedTrackId={currentTrack?.id}
                  onTrackSelect={handleTrackSelect}
                  onTrackContextMenu={openTrackContextMenu}
                  onArtistSelect={handleTrackArtistSelect}
                  onAlbumSelect={handleTrackAlbumSelect}
                  onToggleFavorite={handleToggleFavorite}
                />
              {/if}
            </LibrarySection>

            <LibrarySection title="Missing Track Number" viewAllLabel={issueCountLabel(libraryDiagnostics.missingTrackNumberTracks.length)}>
              {#if libraryDiagnostics.missingTrackNumberTracks.length === 0}
                <div class="group-empty compact"><h3>No issues found</h3><p>All cached tracks have track numbers.</p></div>
              {:else}
                <TrackList
                  tracks={libraryDiagnostics.missingTrackNumberTracks}
                  isScanning={false}
                  selectedTrackId={currentTrack?.id}
                  onTrackSelect={handleTrackSelect}
                  onTrackContextMenu={openTrackContextMenu}
                  onArtistSelect={handleTrackArtistSelect}
                  onAlbumSelect={handleTrackAlbumSelect}
                  onToggleFavorite={handleToggleFavorite}
                />
              {/if}
            </LibrarySection>

            <LibrarySection title="Missing Year" viewAllLabel={issueCountLabel(libraryDiagnostics.missingYearTracks.length)}>
              {#if libraryDiagnostics.missingYearTracks.length === 0}
                <div class="group-empty compact"><h3>No issues found</h3><p>All cached tracks have year metadata.</p></div>
              {:else}
                <TrackList
                  tracks={libraryDiagnostics.missingYearTracks}
                  isScanning={false}
                  selectedTrackId={currentTrack?.id}
                  onTrackSelect={handleTrackSelect}
                  onTrackContextMenu={openTrackContextMenu}
                  onArtistSelect={handleTrackArtistSelect}
                  onAlbumSelect={handleTrackAlbumSelect}
                  onToggleFavorite={handleToggleFavorite}
                />
              {/if}
            </LibrarySection>

            <LibrarySection title="Possible Duplicate Albums" viewAllLabel={issueCountLabel(libraryDiagnostics.duplicateAlbumGroups.length, "group")}>
              {#if libraryDiagnostics.duplicateAlbumGroups.length === 0}
                <div class="group-empty compact">
                  <h3>No issues found</h3>
                  <p>No conservative duplicate album candidates were found.</p>
                </div>
              {:else}
                <div class="duplicate-album-list">
                  {#each libraryDiagnostics.duplicateAlbumGroups as group}
                    <article class="duplicate-album-card">
                      <div>
                        <p class="eyebrow">Same album title</p>
                        <h3>{group.title}</h3>
                        <p>{group.trackCount} {group.trackCount === 1 ? "track" : "tracks"} · {group.albums.length} album entries</p>
                      </div>
                      <div class="duplicate-album-actions">
                        {#each group.albums as album}
                          <button type="button" onclick={() => handleAlbumSelect(album)} oncontextmenu={(event) => openAlbumContextMenu(event, album)}>
                            {album.artist} · {album.trackCount} {album.trackCount === 1 ? "track" : "tracks"}
                          </button>
                        {/each}
                      </div>
                      {#if group.folders.length > 0}
                        <p class="duplicate-folders">{group.folders.map(folderLabel).join(" · ")}</p>
                      {/if}
                    </article>
                  {/each}
                </div>
              {/if}
            </LibrarySection>
          </section>
        {:else}
          <section class="settings-panel" aria-labelledby="settings-title">
            <div class="settings-intro">
              <p class="eyebrow">Control Center</p>
              <h3 id="settings-title">Settings</h3>
              <p>Manage Cassette's local library, playback state, app tools, and build details.</p>
            </div>

            <section class="settings-section" aria-labelledby="settings-library-title">
              <div class="settings-section-header">
                <div>
                  <p class="eyebrow">Library</p>
                  <h4 id="settings-library-title">Local library</h4>
                </div>
                <span class="settings-pill">{hasLoadedCache ? "Cache loaded" : "Loading cache"}</span>
              </div>

              <div class="library-folder-card">
                <span>Current folder</span>
                <strong>{scannedFolder ?? "No library folder selected"}</strong>
              </div>

              <div class="settings-stat-grid" aria-label="Library summary">
                {#each librarySettingsStats as stat}
                  <div class="settings-stat-tile">
                    <span>{stat.label}</span>
                    <strong>{stat.value}</strong>
                  </div>
                {/each}
                <div class="settings-stat-tile wide">
                  <span>Last scan</span>
                  <strong>{lastScanLabel}</strong>
                </div>
              </div>

              <div class="settings-actions">
                <button class="primary" type="button" disabled={isScanning} onclick={handleScanLibrary}>
                  {isScanning ? "Scanning..." : scannedFolder ? "Rescan Library" : "Scan Library"}
                </button>
                <button type="button" disabled={isScanning} onclick={handleScanLibrary}>
                  Change Library Folder
                </button>
                <button class="primary" type="button" onclick={handleLibraryHealthSelect}>
                  Open Library Health
                </button>
                <button class="danger" type="button" disabled title="Coming later: needs a safe cache-only migration path">
                  Clear Library Cache
                  <span>Coming later</span>
                </button>
              </div>
            </section>

            <section class="settings-section" aria-labelledby="settings-playback-title">
              <div class="settings-section-header">
                <div>
                  <p class="eyebrow">Playback</p>
                  <h4 id="settings-playback-title">Current session</h4>
                </div>
                <span class="settings-pill">{isPlaying ? "Playing" : currentTrack ? "Paused" : "Idle"}</span>
              </div>

              <div class="settings-status-list">
                <div>
                  <span>Shuffle</span>
                  <strong>{isShuffleEnabled ? "On" : "Off"}</strong>
                </div>
                <div>
                  <span>Repeat</span>
                  <strong>{repeatMode}</strong>
                </div>
                <div>
                  <span>Volume</span>
                  <strong>{volumePercentLabel}</strong>
                </div>
                <div>
                  <span>Queue</span>
                  <strong>{queueLengthLabel}</strong>
                </div>
              </div>

              <div class="settings-actions">
                <button type="button" disabled={playbackQueue.length === 0} onclick={handleSettingsClearQueue}>
                  Clear Queue
                </button>
                <button class="primary" type="button" onclick={openShortcutHelp}>
                  Keyboard Shortcut Help
                </button>
                <button type="button" disabled title="Coming later: reset needs explicit playback-engine semantics">
                  Reset Playback State
                  <span>Coming later</span>
                </button>
              </div>
            </section>

            <section class="settings-section" aria-labelledby="settings-interface-title">
              <div class="settings-section-header">
                <div>
                  <p class="eyebrow">Interface</p>
                  <h4 id="settings-interface-title">Display preferences</h4>
                </div>
                <span class="settings-pill">Dark UI</span>
              </div>

              <div class="settings-control-list">
                <div>
                  <span>Theme</span>
                  <strong>Dark only</strong>
                </div>
                <div>
                  <span>Accent color</span>
                  <strong><span class="accent-swatch" aria-hidden="true"></span> Teal</strong>
                  <small>Coming later</small>
                </div>
                <div>
                  <span>Compact mode</span>
                  <strong>Off</strong>
                  <small>Coming later</small>
                </div>
                <div>
                  <span>Album track numbers</span>
                  <strong>Enabled</strong>
                  <small>Always shown in album detail</small>
                </div>
              </div>
            </section>

            <section class="settings-section" aria-labelledby="settings-tools-title">
              <div class="settings-section-header">
                <div>
                  <p class="eyebrow">Tools</p>
                  <h4 id="settings-tools-title">Library utilities</h4>
                </div>
              </div>

              <div class="settings-tool-grid">
                <button type="button" onclick={handleLibraryHealthSelect}>
                  <span class="health-mark" aria-hidden="true">H</span>
                  <strong>Library Health</strong>
                  <small>{libraryHealthIssueCount} {libraryHealthIssueCount === 1 ? "issue" : "issues"} found</small>
                </button>
                <button type="button" onclick={openShortcutHelp}>
                  <span class="shortcut-mark" aria-hidden="true">?</span>
                  <strong>Keyboard Shortcuts</strong>
                  <small>Show the shortcut overlay</small>
                </button>
                <button type="button" onclick={handleMixBuilderSelect}>
                  <span class="mix-tool-mark" aria-hidden="true">M</span>
                  <strong>Mix Builder</strong>
                  <small>Build a local queue from genres, artists, and albums</small>
                </button>
              </div>
            </section>

            <section class="settings-section about-section" aria-labelledby="settings-about-title">
              <div class="settings-section-header">
                <div>
                  <p class="eyebrow">About</p>
                  <h4 id="settings-about-title">Cassette</h4>
                </div>
                <span class="settings-pill">Version {appVersion}</span>
              </div>

              <div class="about-grid">
                <div>
                  <span>Description</span>
                  <strong>Local-first music player</strong>
                </div>
                <div>
                  <span>Platform</span>
                  <strong>Linux-first</strong>
                </div>
                <div>
                  <span>Tech stack</span>
                  <strong>Tauri, Svelte, Rust, GStreamer</strong>
                </div>
              </div>
              <p class="settings-note">Cassette does not modify your audio files unless future tag editing is explicitly used.</p>
            </section>
          </section>
        {/if}
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

  {#if contextMenu}
    <ContextMenu
      x={contextMenu.x}
      y={contextMenu.y}
      items={contextMenu.items}
      onClose={closeContextMenu}
    />
  {/if}

  {#if playlistPendingDelete}
    <div class="confirmation-backdrop" role="presentation" onclick={handleDeletePlaylistBackdropClick}>
      <div
        bind:this={deletePlaylistModalElement}
        class="confirmation-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="delete-playlist-title"
        aria-describedby="delete-playlist-description"
        tabindex="-1"
      >
        <header class="confirmation-header">
          <div>
            <p class="eyebrow">Playlist</p>
            <h3 id="delete-playlist-title">Delete Playlist?</h3>
          </div>
        </header>

        <p id="delete-playlist-description">
          Delete "{playlistPendingDelete.name}" from Cassette? Songs and audio files will stay in your library.
        </p>

        {#if playlistError}
          <div class="scan-error" role="alert">{playlistError}</div>
        {/if}

        <div class="confirmation-actions">
          <button type="button" disabled={isDeletingPlaylist} onclick={closeDeletePlaylistConfirmation}>
            Cancel
          </button>
          <button
            class="destructive"
            type="button"
            disabled={isDeletingPlaylist}
            onclick={confirmPendingPlaylistDelete}
          >
            {isDeletingPlaylist ? "Deleting..." : "Confirm Delete"}
          </button>
        </div>
      </div>
    </div>
  {/if}

  {#if isShortcutHelpOpen}
    <div class="shortcuts-backdrop" role="presentation" onclick={handleShortcutBackdropClick}>
      <div
        bind:this={shortcutModalElement}
        class="shortcuts-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="shortcuts-title"
        tabindex="-1"
      >
        <header class="shortcuts-header">
          <div>
            <p class="eyebrow">Keyboard</p>
            <h3 id="shortcuts-title">Keyboard Shortcuts</h3>
          </div>
          <button type="button" aria-label="Close keyboard shortcuts" onclick={closeShortcutHelp}>Close</button>
        </header>

        <div class="shortcut-group-list">
          {#each shortcutGroups as group}
            <section class="shortcut-group" aria-labelledby={`shortcut-group-${group.title.toLowerCase().replaceAll(" / ", "-").replaceAll(" ", "-")}`}>
              <h4 id={`shortcut-group-${group.title.toLowerCase().replaceAll(" / ", "-").replaceAll(" ", "-")}`}>{group.title}</h4>
              <div class="shortcut-list">
                {#each group.shortcuts as shortcut}
                  <div class="shortcut-row">
                    <div class="shortcut-keys" aria-label={shortcut.keys.join(" plus ")}>
                      {#each shortcut.keys as key, index}
                        {#if index > 0}
                          <span class="shortcut-plus" aria-hidden="true">+</span>
                        {/if}
                        <kbd>{key}</kbd>
                      {/each}
                    </div>
                    <p>{shortcut.description}</p>
                  </div>
                {/each}
              </div>
            </section>
          {/each}
        </div>
      </div>
    </div>
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
    onOpenNowPlaying={handleNowPlayingSelect}
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

  :global(select) {
    appearance: none;
    -webkit-appearance: none;
    min-height: 36px;
    border: 1px solid #303844;
    border-radius: 8px;
    background-color: #12161c;
    background-image:
      linear-gradient(45deg, transparent 50%, #aeb9c6 50%),
      linear-gradient(135deg, #aeb9c6 50%, transparent 50%);
    background-position:
      calc(100% - 14px) 50%,
      calc(100% - 9px) 50%;
    background-repeat: no-repeat;
    background-size: 5px 5px;
    color: #f4f7fb;
    color-scheme: dark;
    font: inherit;
    font-size: 0.86rem;
    font-weight: 750;
    outline: none;
    padding: 0 32px 0 10px;
  }

  :global(select:not(:disabled):hover) {
    border-color: #35544f;
    background-color: #1b2027;
  }

  :global(select:focus-visible),
  :global(select:focus) {
    border-color: #2f8f83;
    box-shadow: 0 0 0 2px rgba(47, 143, 131, 0.18);
  }

  :global(select:disabled) {
    border-color: #252c35;
    background-color: #10141a;
    color: #66717f;
  }

  :global(select option) {
    background-color: #12161c;
    color: #f4f7fb;
  }

  :global(select option:disabled) {
    color: #66717f;
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
    background-color: #12161c;
    color: #f4f7fb;
    font: inherit;
    font-size: 0.86rem;
    font-weight: 750;
    outline: none;
    padding: 0 32px 0 10px;
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

  .scan-error.status-message {
    border-color: #2f5f58;
    background: #142521;
    color: #9ee3d9;
  }

  .playlist-warning {
    border: 1px solid #45412a;
    border-radius: 8px;
    background: #1d1b12;
    color: #eadb94;
    font-size: 0.9rem;
    font-weight: 700;
    padding: 12px 14px;
  }

  .scan-error + :global(.library-section),
  .playlist-warning + :global(.library-section) {
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
  .playlist-card > div:last-child,
  .playlist-create-card > div:last-child {
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

  .playlist-card,
  .playlist-create-card {
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

  .playlist-create-card label {
    display: block;
    margin-bottom: 8px;
    color: #f4f7fb;
    font-size: 0.98rem;
    font-weight: 800;
    line-height: 1.25;
  }

  .playlist-create-card > div:last-child > div {
    display: flex;
    gap: 8px;
  }

  .playlist-create-card input {
    width: 100%;
    min-width: 0;
    min-height: 36px;
    border: 1px solid #303844;
    border-radius: 8px;
    background: #0f1318;
    color: #f4f7fb;
    font: inherit;
    font-size: 0.88rem;
    font-weight: 650;
    outline: none;
    padding: 0 10px;
  }

  .playlist-create-card input:focus {
    border-color: #2f8f83;
    box-shadow: 0 0 0 2px rgba(47, 143, 131, 0.18);
  }

  .playlist-create-card button {
    min-height: 36px;
    border: 1px solid #35544f;
    border-radius: 8px;
    background: #17332f;
    color: #d8fffa;
    cursor: default;
    font: inherit;
    font-size: 0.84rem;
    font-weight: 850;
    padding: 0 10px;
  }

  .playlist-create-card button:disabled {
    border-color: #303844;
    background: #151a21;
    color: #626c79;
  }

  .form-message {
    margin: 8px 0 0;
    color: #9ee3d9;
    font-size: 0.78rem;
    font-weight: 750;
  }

  .form-message.error {
    color: #ffcbc8;
  }

  .liked-mark,
  .mix-mark,
  .playlist-mark,
  .health-mark {
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

  .playlist-mark {
    background: #1b2633;
    color: #8fb9f2;
  }

  .health-mark {
    background: #1b2633;
    color: #8fb9f2;
  }

  .detail-view {
    display: grid;
    gap: 22px;
  }

  .now-playing-page {
    display: grid;
    gap: 22px;
  }

  .now-playing-hero {
    display: grid;
    grid-template-columns: minmax(220px, 340px) minmax(0, 1fr);
    gap: 26px;
    align-items: center;
  }

  .now-playing-cover {
    position: relative;
    display: grid;
    aspect-ratio: 1;
    overflow: hidden;
    place-items: center;
    border-radius: 8px;
    background:
      linear-gradient(135deg, rgba(255, 255, 255, 0.18), transparent 46%),
      #2f8f83;
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.14);
  }

  .now-playing-cover img {
    position: absolute;
    inset: 0;
    z-index: 1;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .now-playing-cover span {
    display: block;
    width: 34%;
    aspect-ratio: 1;
    border: 14px solid rgba(13, 15, 19, 0.55);
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.6);
  }

  .now-playing-copy {
    min-width: 0;
  }

  .now-playing-copy h3 {
    margin: 0 0 12px;
    overflow-wrap: anywhere;
    color: #f7f9fc;
    font-size: clamp(2.1rem, 5vw, 4.8rem);
    line-height: 1.02;
  }

  .now-playing-links,
  .now-playing-genres,
  .now-playing-stats,
  .now-playing-actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }

  .now-playing-links {
    color: #9aa4b1;
    font-size: 1rem;
    font-weight: 750;
  }

  .now-playing-links button,
  .now-playing-genres button {
    border: 0;
    background: transparent;
    color: #d5dce5;
    cursor: default;
    font: inherit;
    font-weight: 800;
    padding: 0;
  }

  .now-playing-links button:hover,
  .now-playing-links button:focus-visible,
  .now-playing-genres button:hover,
  .now-playing-genres button:focus-visible {
    color: #ffffff;
    outline: none;
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  .now-playing-genres {
    margin-top: 16px;
  }

  .now-playing-genres button {
    min-height: 30px;
    border: 1px solid #35544f;
    border-radius: 8px;
    background: #17332f;
    color: #d8fffa;
    font-size: 0.82rem;
    padding: 0 10px;
  }

  .now-playing-stats {
    margin-top: 16px;
    color: #9aa4b1;
    font-size: 0.9rem;
    font-weight: 750;
  }

  .now-playing-stats span {
    min-height: 28px;
    border: 1px solid #242b35;
    border-radius: 8px;
    background: #12161c;
    padding: 4px 9px;
  }

  .now-playing-actions {
    margin-top: 18px;
  }

  .now-playing-actions button,
  .now-playing-info-panel {
    min-height: 38px;
    border: 1px solid #303844;
    border-radius: 8px;
    background: #161a20;
    color: #d5dce5;
    cursor: default;
    font: inherit;
    font-size: 0.86rem;
    font-weight: 850;
    padding: 0 13px;
  }

  .now-playing-actions button:hover,
  .now-playing-actions button:focus-visible,
  .now-playing-actions button.active {
    border-color: #35544f;
    background: #17332f;
    color: #d8fffa;
    outline: none;
  }

  .now-playing-actions button:disabled {
    border-color: #303844;
    background: #151a21;
    color: #626c79;
  }

  .lyrics-panel {
    display: grid;
    gap: 14px;
    border: 1px solid #242b35;
    border-radius: 8px;
    background: #12161c;
    padding: 16px;
  }

  .lyrics-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
  }

  .lyrics-header h3 {
    margin: 0;
    color: #f4f7fb;
    font-size: 1.05rem;
  }

  .lyrics-header-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;
  }

  .lyrics-header-actions > span {
    display: inline-flex;
    align-items: center;
    min-height: 28px;
    border: 1px solid #303844;
    border-radius: 999px;
    background: #11161d;
    color: #b9c3cf;
    font-size: 0.76rem;
    font-weight: 850;
    padding: 0 10px;
  }

  .lyrics-header-actions button {
    min-height: 28px;
    border: 1px solid #303844;
    border-radius: 8px;
    background: #161a20;
    color: #d5dce5;
    cursor: default;
    font: inherit;
    font-size: 0.78rem;
    font-weight: 850;
    padding: 0 10px;
  }

  .lyrics-header-actions button:hover,
  .lyrics-header-actions button:focus-visible {
    border-color: #35544f;
    background: #1b2027;
    outline: none;
  }

  .lyrics-header-actions button:disabled {
    border-color: #303844;
    background: #151a21;
    color: #626c79;
  }

  .lyrics-lookup-message {
    margin: 0;
    color: #9ee3d9;
    font-size: 0.9rem;
    font-weight: 750;
  }

  .lyrics-lookup-message.error {
    color: #ffcbc8;
  }

  .lyrics-lookup-message.cached {
    color: #9aa4b1;
  }

  .auto-lyrics-button {
    justify-self: center;
    min-height: 38px;
    margin-top: 12px;
    border: 1px solid #35544f;
    border-radius: 8px;
    background: #17332f;
    color: #d8fffa;
    cursor: default;
    font: inherit;
    font-size: 0.86rem;
    font-weight: 850;
    padding: 0 13px;
  }

  .auto-lyrics-button:hover,
  .auto-lyrics-button:focus-visible {
    border-color: #4d766f;
    background: #1b403b;
    outline: none;
  }

  .auto-lyrics-button:disabled {
    border-color: #303844;
    background: #151a21;
    color: #626c79;
  }

  .synced-lyrics {
    display: grid;
    gap: 4px;
    max-height: 320px;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: 24px 6px;
  }

  .synced-lyrics button {
    width: 100%;
    border: 0;
    margin: 0;
    border-radius: 8px;
    background: transparent;
    color: #7f8996;
    cursor: default;
    font: inherit;
    font-size: 1rem;
    font-weight: 750;
    line-height: 1.5;
    padding: 7px 10px;
    text-align: left;
    transition:
      background 150ms ease,
      color 150ms ease;
  }

  .synced-lyrics button:hover,
  .synced-lyrics button:focus-visible {
    background: #171d24;
    color: #d5dce5;
    outline: none;
  }

  .synced-lyrics button.active {
    background: #17332f;
    color: #d8fffa;
  }

  .plain-lyrics {
    max-height: 360px;
    overflow: auto;
    margin: 0;
    color: #d5dce5;
    font: inherit;
    font-size: 0.98rem;
    font-weight: 650;
    line-height: 1.65;
    white-space: pre-wrap;
  }

  .now-playing-info-panel {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 18px;
    border: 1px solid #242b35;
    background: #12161c;
    padding: 16px;
  }

  .now-playing-info-panel h3,
  .now-playing-info-panel p {
    margin: 0;
  }

  .now-playing-info-panel h3 {
    color: #f4f7fb;
    font-size: 1.05rem;
  }

  .now-playing-info-panel > p {
    max-width: 620px;
    color: #9aa4b1;
    font-size: 0.92rem;
    font-weight: 700;
    text-align: right;
  }

  .now-playing-queue-list {
    display: grid;
    gap: 8px;
  }

  .now-playing-queue-list button {
    display: grid;
    grid-template-columns: 54px minmax(0, 1fr) auto;
    align-items: center;
    gap: 12px;
    min-height: 56px;
    border: 1px solid #242b35;
    border-radius: 8px;
    background: #151a21;
    color: inherit;
    cursor: default;
    font: inherit;
    padding: 9px 12px;
    text-align: left;
  }

  .now-playing-queue-list button:hover,
  .now-playing-queue-list button:focus-visible,
  .now-playing-queue-list button.active {
    border-color: #35544f;
    background: #1b2027;
    outline: none;
  }

  .now-playing-queue-list button > span {
    display: grid;
    min-height: 32px;
    place-items: center;
    border-radius: 7px;
    background: #1d252e;
    color: #8f9aa8;
    font-size: 0.76rem;
    font-weight: 900;
  }

  .now-playing-queue-list button.active > span {
    background: #17332f;
    color: #d8fffa;
  }

  .now-playing-queue-list div {
    min-width: 0;
  }

  .now-playing-queue-list strong,
  .now-playing-queue-list small {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .now-playing-queue-list strong {
    color: #f4f7fb;
    font-size: 0.94rem;
    line-height: 1.25;
  }

  .now-playing-queue-list small {
    color: #8f9aa8;
    font-size: 0.82rem;
    font-weight: 700;
  }

  .now-playing-empty {
    display: flex;
    align-items: center;
    gap: 18px;
    min-height: 220px;
    border: 1px dashed #303844;
    border-radius: 8px;
    background: rgba(18, 22, 28, 0.74);
    padding: 24px;
  }

  .now-playing-empty-mark {
    display: grid;
    width: 76px;
    height: 76px;
    flex: 0 0 auto;
    place-items: center;
    border-radius: 8px;
    background: #1b2633;
    color: #8fb9f2;
    font-size: 1.6rem;
    font-weight: 900;
  }

  .now-playing-empty h3 {
    margin: 0 0 6px;
    color: #f4f7fb;
    font-size: 1.35rem;
  }

  .now-playing-empty p:not(.eyebrow) {
    margin: 0;
    color: #98a3b0;
    font-weight: 650;
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

  .back-button.destructive {
    border-color: #5b3434;
    background: #2a1718;
    color: #ffcbc8;
  }

  .back-button.destructive:hover,
  .back-button.destructive:focus-visible {
    border-color: #7a3a3a;
    background: #341c1e;
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

  .album-detail-header .detail-cover {
    width: min(34vw, 260px);
    min-width: 176px;
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

  .album-genre-line {
    margin-top: 8px !important;
    color: #d5dce5 !important;
  }

  .album-detail-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin-top: 18px;
  }

  .album-detail-actions button {
    min-height: 38px;
    border: 1px solid #303844;
    border-radius: 8px;
    background: #161a20;
    color: #d5dce5;
    cursor: default;
    font: inherit;
    font-size: 0.86rem;
    font-weight: 850;
    padding: 0 13px;
  }

  .album-detail-actions button:first-child {
    border-color: #35544f;
    background: #17332f;
    color: #d8fffa;
  }

  .album-detail-actions button:hover,
  .album-detail-actions button:focus-visible {
    border-color: #35544f;
    background: #1b2027;
    outline: none;
  }

  .album-detail-actions button:first-child:hover,
  .album-detail-actions button:first-child:focus-visible {
    border-color: #2f8f83;
    background: #1b403a;
  }

  .album-detail-actions button:disabled {
    border-color: #303844;
    background: #151a21;
    color: #626c79;
  }

  .album-track-list {
    display: grid;
    gap: 8px;
  }

  .album-track-list h4 {
    margin: 12px 0 2px;
    color: #aeb9c6;
    font-size: 0.82rem;
    font-weight: 900;
    text-transform: uppercase;
  }

  .album-track-row {
    display: grid;
    grid-template-columns: 44px minmax(180px, 1fr) auto auto auto;
    align-items: center;
    gap: 14px;
    min-height: 58px;
    border: 1px solid #242b35;
    border-radius: 8px;
    background: rgba(22, 26, 32, 0.86);
    color: inherit;
    cursor: default;
    font: inherit;
    padding: 9px 14px;
    outline: none;
  }

  .album-track-row:hover,
  .album-track-row.active,
  .album-track-row:focus-visible {
    border-color: #35544f;
    background: #1b2027;
  }

  .album-track-number {
    color: #d5dce5;
    font-size: 0.95rem;
    font-variant-numeric: tabular-nums;
    font-weight: 850;
    text-align: right;
  }

  .album-track-number.missing {
    color: #626c79;
  }

  .track-title {
    min-width: 0;
  }

  .track-name,
  .track-link {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .track-name {
    display: block;
    color: #f4f7fb;
    font-size: 0.98rem;
    font-weight: 750;
    line-height: 1.25;
  }

  .track-link {
    display: block;
    width: fit-content;
    max-width: 100%;
    margin: 3px 0 0;
    border: 0;
    background: transparent;
    color: #929daa;
    cursor: default;
    font: inherit;
    font-size: 0.86rem;
    font-weight: 650;
    line-height: 1.3;
    padding: 0;
    text-align: left;
  }

  .track-link:hover,
  .track-link:focus-visible {
    color: #d5dce5;
    outline: none;
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  .album-track-duration,
  .album-track-format {
    color: #8f9aa8;
    font-size: 0.86rem;
    font-variant-numeric: tabular-nums;
    font-weight: 700;
  }

  .album-track-format {
    min-width: 42px;
    text-align: right;
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
    background-color: #0f1318;
    color: #f4f7fb;
    font: inherit;
    font-size: 0.86rem;
    font-weight: 750;
    outline: none;
    padding: 0 32px 0 10px;
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

  .videos-page {
    display: grid;
    gap: 18px;
  }

  .videos-toolbar {
    display: grid;
    grid-template-columns: minmax(220px, 1fr) auto auto auto auto;
    gap: 10px;
    align-items: center;
  }

  .video-search {
    display: flex;
    min-width: 0;
    min-height: 42px;
    overflow: hidden;
    border: 1px solid #303844;
    border-radius: 8px;
    background: #0f1318;
  }

  .video-search input {
    width: 100%;
    min-width: 0;
    border: 0;
    background: transparent;
    color: #f4f7fb;
    font: inherit;
    font-size: 0.92rem;
    font-weight: 650;
    outline: none;
    padding: 0 12px;
  }

  .video-search button,
  .videos-toolbar button,
  .video-actions button,
  .video-edit-actions button {
    min-height: 40px;
    border: 1px solid #303844;
    border-radius: 8px;
    background: #161a20;
    color: #d5dce5;
    cursor: default;
    font: inherit;
    font-size: 0.86rem;
    font-weight: 850;
    padding: 0 12px;
    white-space: nowrap;
  }

  .video-search button {
    border-width: 0 0 0 1px;
    border-radius: 0;
  }

  .videos-toolbar button:hover:not(:disabled),
  .videos-toolbar button:focus-visible:not(:disabled),
  .video-actions button:hover:not(:disabled),
  .video-actions button:focus-visible:not(:disabled),
  .video-edit-actions button:hover:not(:disabled),
  .video-edit-actions button:focus-visible:not(:disabled) {
    border-color: #35544f;
    background: #1b2027;
    outline: none;
  }

  .video-edit-actions button.primary {
    border-color: #35544f;
    background: #17332f;
    color: #d8fffa;
  }

  .videos-toolbar button:disabled,
  .video-actions button:disabled,
  .video-edit-actions button:disabled {
    border-color: #303844;
    background: #151a21;
    color: #626c79;
  }

  .videos-toolbar label {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 40px;
    color: #8f9aa8;
    font-size: 0.82rem;
    font-weight: 800;
  }

  .videos-toolbar select {
    min-height: 40px;
    border: 1px solid #303844;
    border-radius: 8px;
    background-color: #0f1318;
    color: #f4f7fb;
    font: inherit;
    font-size: 0.86rem;
    font-weight: 750;
    outline: none;
    padding: 0 32px 0 10px;
  }

  .video-stat-grid,
  .video-meta-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 10px;
  }

  .video-stat-grid > div,
  .video-meta-grid > div {
    min-width: 0;
    border: 1px solid #242b35;
    border-radius: 8px;
    background: #151a21;
    padding: 12px;
  }

  .video-stat-grid span,
  .video-meta-grid span {
    display: block;
    margin-bottom: 6px;
    color: #8f9aa8;
    font-size: 0.74rem;
    font-weight: 850;
    text-transform: uppercase;
  }

  .video-stat-grid strong,
  .video-meta-grid strong {
    display: block;
    overflow: hidden;
    color: #f4f7fb;
    font-size: 0.98rem;
    font-weight: 900;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dvd-placeholder {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    border: 1px dashed #303844;
    border-radius: 8px;
    background: #11161d;
    padding: 16px;
  }

  .dvd-placeholder h3,
  .dvd-placeholder p {
    margin: 0;
  }

  .dvd-placeholder h3 {
    color: #f4f7fb;
    font-size: 1rem;
  }

  .dvd-placeholder p:not(.eyebrow) {
    margin-top: 4px;
    color: #9aa4b1;
    font-size: 0.9rem;
    font-weight: 700;
  }

  .dvd-placeholder > span {
    border: 1px solid #303844;
    border-radius: 999px;
    color: #8f9aa8;
    font-size: 0.76rem;
    font-weight: 900;
    padding: 6px 10px;
    text-transform: uppercase;
  }

  .video-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 14px;
  }

  .video-card {
    display: grid;
    gap: 10px;
    min-width: 0;
    border: 1px solid #242b35;
    border-radius: 8px;
    background: #151a21;
    color: inherit;
    cursor: default;
    font: inherit;
    padding: 10px;
    text-align: left;
  }

  .video-card:hover,
  .video-card:focus-visible {
    border-color: #35544f;
    background: #1b2027;
    outline: none;
  }

  .video-thumb {
    position: relative;
    display: grid;
    overflow: hidden;
    width: 100%;
    aspect-ratio: 16 / 9;
    place-items: center;
    border-radius: 8px;
    background: #0f1318;
    color: #8fb9f2;
    font-size: 1.4rem;
    font-weight: 900;
  }

  .video-thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .video-progress {
    position: absolute;
    right: 0;
    bottom: 0;
    left: 0;
    height: 4px;
    background: linear-gradient(90deg, #2f8f83 var(--progress), rgba(244, 247, 251, 0.18) var(--progress));
  }

  .video-card-copy {
    display: grid;
    min-width: 0;
    gap: 4px;
  }

  .video-card-copy strong,
  .video-card-copy small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .video-card-copy strong {
    color: #f4f7fb;
    font-size: 0.98rem;
    line-height: 1.25;
  }

  .video-card-copy small {
    color: #8f9aa8;
    font-size: 0.82rem;
    font-weight: 700;
  }

  .video-detail-hero {
    display: grid;
    grid-template-columns: minmax(360px, 1.2fr) minmax(300px, 0.8fr);
    gap: 18px;
    align-items: start;
  }

  .video-player-panel {
    display: grid;
    overflow: hidden;
    min-width: 0;
    aspect-ratio: 16 / 9;
    place-items: center;
    border: 1px solid #242b35;
    border-radius: 8px;
    background: #0b0e12;
  }

  .video-player-panel video,
  .video-player-panel img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }

  .video-placeholder {
    display: grid;
    width: 100%;
    height: 100%;
    place-items: center;
    color: #8fb9f2;
    font-size: 2.4rem;
    font-weight: 900;
  }

  .video-detail-copy {
    display: grid;
    min-width: 0;
    gap: 14px;
  }

  .video-detail-copy h3 {
    overflow: hidden;
    margin: 0;
    color: #f4f7fb;
    font-size: clamp(1.65rem, 4vw, 3rem);
    line-height: 1.04;
    text-overflow: ellipsis;
  }

  .video-detail-copy > p:not(.eyebrow) {
    margin: 0;
    color: #9aa4b1;
    font-weight: 750;
  }

  .video-meta-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .video-location {
    color: #d5dce5 !important;
  }

  .video-actions,
  .video-edit-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }

  .video-edit-panel {
    display: grid;
    gap: 14px;
    border: 1px solid #242b35;
    border-radius: 8px;
    background: #12161c;
    padding: 16px;
  }

  .video-edit-form {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .video-edit-form label {
    display: grid;
    gap: 7px;
    color: #8f9aa8;
    font-size: 0.78rem;
    font-weight: 850;
    text-transform: uppercase;
  }

  .video-edit-form input {
    min-width: 0;
    min-height: 42px;
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

  .video-edit-form input:focus {
    border-color: #2f8f83;
    box-shadow: 0 0 0 2px rgba(47, 143, 131, 0.18);
  }

  .video-edit-actions {
    grid-column: 1 / -1;
  }

  .detail-view :global(.library-section + .library-section) {
    margin-top: 8px;
  }

  .stats-page {
    display: grid;
    gap: 22px;
    max-width: 1120px;
  }

  .stats-overview-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 12px;
  }

  .stats-overview-card {
    min-width: 0;
    border: 1px solid #242b35;
    border-radius: 8px;
    background: #151a21;
    padding: 16px;
  }

  .stats-overview-card.muted {
    background: #11161d;
  }

  .stats-overview-card span {
    display: block;
    margin-bottom: 8px;
    color: #8f9aa8;
    font-size: 0.76rem;
    font-weight: 850;
    text-transform: uppercase;
  }

  .stats-overview-card strong {
    display: block;
    overflow: hidden;
    color: #f4f7fb;
    font-size: 1.55rem;
    font-weight: 900;
    line-height: 1.1;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .stats-overview-card.muted strong {
    color: #8f9aa8;
    font-size: 1rem;
  }

  .stats-section-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 22px;
  }

  .stats-section-grid :global(.library-section) {
    min-width: 0;
  }

  .stats-rank-list,
  .stats-recent-list {
    display: grid;
    gap: 8px;
  }

  .stats-rank-card,
  .stats-recent-card {
    display: grid;
    align-items: center;
    width: 100%;
    min-width: 0;
    min-height: 68px;
    border: 1px solid #242b35;
    border-radius: 8px;
    background: #151a21;
    color: inherit;
    cursor: default;
    font: inherit;
    padding: 10px;
    text-align: left;
  }

  .stats-rank-card {
    grid-template-columns: 32px 48px minmax(0, 1fr);
    gap: 10px;
  }

  .stats-recent-card {
    grid-template-columns: 48px minmax(0, 1fr) minmax(120px, auto);
    gap: 12px;
  }

  .stats-rank-card:hover,
  .stats-rank-card:focus-visible,
  .stats-recent-card:hover,
  .stats-recent-card:focus-visible {
    border-color: #35544f;
    background: #1b2027;
    outline: none;
  }

  .stats-rank-number {
    display: grid;
    width: 30px;
    height: 30px;
    place-items: center;
    border-radius: 7px;
    background: #11161d;
    color: #8f9aa8;
    font-size: 0.78rem;
    font-weight: 900;
  }

  .stats-avatar,
  .stats-cover,
  .stats-genre-mark,
  .stats-mini-cover {
    width: 48px;
    height: 48px;
    margin: 0;
    flex: 0 0 auto;
  }

  .stats-cover span {
    border-width: 7px;
  }

  .stats-genre-mark,
  .stats-mini-cover {
    display: grid;
    position: relative;
    overflow: hidden;
    place-items: center;
    border-radius: 8px;
  }

  .stats-genre-mark {
    background: var(--item-color);
    color: #0d0f13;
    font-size: 1.12rem;
    font-weight: 900;
  }

  .stats-mini-cover {
    background: #202832;
    color: #9aa4b1;
    font-size: 0.66rem;
    font-weight: 900;
  }

  .stats-mini-cover img {
    position: absolute;
    inset: 0;
    z-index: 1;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .stats-mini-cover > span {
    z-index: 0;
  }

  .stats-rank-copy {
    display: grid;
    min-width: 0;
    gap: 3px;
  }

  .stats-rank-copy strong,
  .stats-rank-copy small,
  .stats-played-at {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .stats-rank-copy strong {
    color: #f4f7fb;
    font-size: 0.95rem;
    font-weight: 820;
    line-height: 1.2;
  }

  .stats-rank-copy small,
  .stats-played-at {
    color: #8f9aa8;
    font-size: 0.8rem;
    font-weight: 750;
  }

  .stats-played-at {
    justify-self: end;
    max-width: 180px;
  }

  .settings-panel {
    display: grid;
    max-width: 1040px;
    gap: 16px;
  }

  .cd-ripper-page {
    display: grid;
    max-width: 1040px;
    gap: 16px;
  }

  .settings-intro {
    max-width: 760px;
  }

  .settings-intro h3 {
    margin: 0 0 8px;
    color: #f4f7fb;
    font-size: 1.55rem;
  }

  .settings-intro p:not(.eyebrow),
  .settings-note {
    margin: 0;
    color: #98a3b0;
    font-weight: 650;
    line-height: 1.45;
  }

  .settings-section {
    display: grid;
    gap: 14px;
    border: 1px solid #242b35;
    border-radius: 8px;
    background: #151a21;
    padding: 18px;
  }

  .cd-status-card,
  .cd-metadata-section,
  .cd-track-section,
  .cd-debug-output {
    display: grid;
    gap: 14px;
    border: 1px solid #242b35;
    border-radius: 8px;
    background: #151a21;
    padding: 18px;
  }

  .settings-section-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
  }

  .settings-section-header h4 {
    margin: 0;
    color: #f4f7fb;
    font-size: 1.08rem;
    line-height: 1.2;
  }

  .settings-pill {
    display: inline-flex;
    align-items: center;
    min-height: 28px;
    border: 1px solid #303844;
    border-radius: 999px;
    background: #11161d;
    color: #b9c3cf;
    font-size: 0.78rem;
    font-weight: 850;
    padding: 0 10px;
    white-space: nowrap;
  }

  .library-folder-card {
    min-width: 0;
    border: 1px solid #2a313c;
    border-radius: 8px;
    background: #12161c;
    padding: 14px;
  }

  .cd-output-folder {
    min-width: 0;
    border: 1px solid #2a313c;
    border-radius: 8px;
    background: #12161c;
    padding: 14px;
  }

  .library-folder-card span,
  .cd-status-grid span,
  .cd-output-folder span,
  .settings-stat-tile span,
  .settings-status-list span,
  .settings-control-list span,
  .about-grid span {
    display: block;
    margin-bottom: 5px;
    color: #8f9aa8;
    font-size: 0.76rem;
    font-weight: 850;
    text-transform: uppercase;
  }

  .library-folder-card strong {
    display: block;
    overflow: hidden;
    color: #f4f7fb;
    font-size: 0.95rem;
    font-weight: 760;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cd-output-folder strong {
    display: block;
    overflow-wrap: anywhere;
    color: #f4f7fb;
    font-size: 0.95rem;
    font-weight: 760;
  }

  .settings-stat-grid,
  .cd-status-grid,
  .settings-status-list,
  .settings-control-list,
  .about-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 10px;
  }

  .settings-control-list,
  .about-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .settings-stat-tile,
  .cd-status-grid > div,
  .settings-status-list > div,
  .settings-control-list > div,
  .about-grid > div {
    min-width: 0;
    border: 1px solid #2a313c;
    border-radius: 8px;
    background: #12161c;
    padding: 14px;
  }

  .settings-stat-tile.wide {
    grid-column: span 2;
  }

  .settings-stat-tile strong,
  .cd-status-grid strong,
  .settings-status-list strong,
  .settings-control-list strong,
  .about-grid strong {
    display: flex;
    align-items: center;
    gap: 8px;
    overflow: hidden;
    color: #f4f7fb;
    font-size: 0.98rem;
    font-weight: 820;
    line-height: 1.25;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .settings-stat-tile strong {
    font-size: 1.35rem;
  }

  .settings-control-list small {
    display: block;
    margin-top: 6px;
    color: #8f9aa8;
    font-size: 0.8rem;
    font-weight: 760;
  }

  .accent-swatch {
    width: 14px;
    height: 14px;
    flex: 0 0 auto;
    border-radius: 50%;
    background: #2f8f83;
    box-shadow: 0 0 0 2px rgba(47, 143, 131, 0.22);
  }

  .settings-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }

  .cd-rip-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }

  .settings-actions button,
  .cd-rip-actions button,
  .settings-tool-grid button {
    min-height: 40px;
    border: 1px solid #303844;
    border-radius: 8px;
    background: #161a20;
    color: #d5dce5;
    cursor: default;
    font: inherit;
    font-size: 0.86rem;
    font-weight: 850;
  }

  .settings-actions button {
    padding: 0 13px;
  }

  .cd-rip-actions button {
    padding: 0 13px;
  }

  .settings-actions button:hover:not(:disabled),
  .settings-actions button:focus-visible:not(:disabled),
  .cd-rip-actions button:hover:not(:disabled),
  .cd-rip-actions button:focus-visible:not(:disabled),
  .settings-tool-grid button:hover,
  .settings-tool-grid button:focus-visible {
    border-color: #35544f;
    background: #1b2027;
    outline: none;
  }

  .settings-actions button.primary {
    border-color: #35544f;
    background: #17332f;
    color: #d8fffa;
  }

  .cd-rip-actions button.primary {
    border-color: #35544f;
    background: #17332f;
    color: #d8fffa;
  }

  .settings-actions button.danger {
    border-color: #4a3030;
    color: #ffc8c8;
  }

  .settings-actions button:disabled,
  .cd-rip-actions button:disabled {
    border-color: #2a313c;
    background: #11161d;
    color: #6d7784;
  }

  .settings-actions button span {
    margin-left: 8px;
    color: #8f9aa8;
    font-size: 0.76rem;
    font-weight: 850;
  }

  .settings-tool-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 10px;
  }

  .settings-tool-grid button {
    display: grid;
    min-width: 0;
    min-height: 138px;
    gap: 8px;
    justify-items: start;
    padding: 14px;
    text-align: left;
  }

  .settings-tool-grid .health-mark,
  .settings-tool-grid .shortcut-mark,
  .mix-tool-mark {
    width: 48px;
    height: 48px;
    font-size: 1.18rem;
  }

  .settings-tool-grid strong,
  .settings-tool-grid small {
    overflow: hidden;
    max-width: 100%;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .settings-tool-grid strong {
    color: #f4f7fb;
    font-size: 0.98rem;
    line-height: 1.2;
  }

  .settings-tool-grid small {
    color: #8f9aa8;
    font-size: 0.8rem;
    font-weight: 750;
  }

  .cd-track-table {
    display: grid;
    overflow: hidden;
    border: 1px solid #2a313c;
    border-radius: 8px;
    background: #12161c;
  }

  .cd-track-row {
    display: grid;
    grid-template-columns: 60px minmax(150px, 1.25fr) minmax(130px, 1fr) 92px minmax(112px, 0.7fr) minmax(0, 1.4fr);
    min-height: 48px;
    align-items: center;
    gap: 12px;
    border-top: 1px solid #242b35;
    color: #d5dce5;
    font-size: 0.9rem;
    font-weight: 700;
    padding: 10px 14px;
  }

  .cd-track-row:first-child {
    border-top: 0;
  }

  .cd-track-head {
    min-height: 40px;
    background: #10141a;
    color: #8f9aa8;
    font-size: 0.76rem;
    font-weight: 850;
    text-transform: uppercase;
  }

  .cd-track-row.active {
    background: #172521;
  }

  .cd-track-row.error {
    background: #241719;
    color: #ffcbc8;
  }

  .cd-track-row span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cd-track-row input,
  .cd-metadata-form input {
    width: 100%;
    min-width: 0;
    min-height: 34px;
    border: 1px solid #303844;
    border-radius: 8px;
    background: #0f1318;
    color: #f4f7fb;
    font: inherit;
    font-size: 0.86rem;
    font-weight: 650;
    outline: none;
    padding: 0 10px;
  }

  .cd-track-row input:focus,
  .cd-metadata-form input:focus {
    border-color: #2f8f83;
    box-shadow: 0 0 0 2px rgba(47, 143, 131, 0.18);
  }

  .cd-track-row input:disabled,
  .cd-metadata-form input:disabled,
  .cd-release-list button:disabled {
    border-color: #252c35;
    background: #11161d;
    color: #8b95a3;
  }

  .cd-track-row strong {
    color: inherit;
    text-transform: capitalize;
  }

  .cd-track-row small {
    display: block;
    overflow: hidden;
    margin-top: 3px;
    color: #d99b98;
    font-size: 0.76rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cd-debug-output summary {
    color: #d5dce5;
    cursor: default;
    font-weight: 800;
  }

  .cd-debug-output pre {
    overflow: auto;
    max-height: 260px;
    margin: 0;
    border: 1px solid #2a313c;
    border-radius: 8px;
    background: #0f1318;
    color: #b9c3cf;
    font-size: 0.78rem;
    line-height: 1.45;
    padding: 12px;
    white-space: pre-wrap;
  }

  .cd-release-list {
    display: grid;
    gap: 8px;
  }

  .cd-release-list button,
  .cd-selected-release {
    display: grid;
    min-width: 0;
    gap: 5px;
    border: 1px solid #2a313c;
    border-radius: 8px;
    background: #12161c;
    color: inherit;
    cursor: default;
    font: inherit;
    padding: 12px;
    text-align: left;
  }

  .cd-release-list button:hover,
  .cd-release-list button:focus-visible,
  .cd-release-list button.active {
    border-color: #35544f;
    background: #17332f;
    outline: none;
  }

  .cd-release-list strong,
  .cd-selected-release strong {
    overflow: hidden;
    color: #f4f7fb;
    font-size: 0.95rem;
    font-weight: 850;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cd-release-list small,
  .cd-selected-release small {
    overflow: hidden;
    color: #9aa4b1;
    font-size: 0.8rem;
    font-weight: 700;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cd-selected-release span,
  .cd-metadata-form span {
    color: #8f9aa8;
    font-size: 0.76rem;
    font-weight: 850;
    text-transform: uppercase;
  }

  .cd-metadata-form {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 10px;
  }

  .cd-metadata-form label {
    display: grid;
    gap: 6px;
    min-width: 0;
  }

  .cd-cover-panel {
    display: grid;
    grid-template-columns: 120px minmax(0, 1fr);
    gap: 14px;
    align-items: center;
    border: 1px solid #2a313c;
    border-radius: 8px;
    background: #12161c;
    padding: 12px;
  }

  .cd-cover-preview {
    display: grid;
    aspect-ratio: 1;
    overflow: hidden;
    place-items: center;
    border: 1px solid #303844;
    border-radius: 8px;
    background: #0f1318;
    color: #7f8996;
    font-size: 0.78rem;
    font-weight: 850;
  }

  .cd-cover-preview img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .cd-cover-copy {
    display: grid;
    min-width: 0;
    gap: 6px;
  }

  .cd-cover-copy span {
    color: #8f9aa8;
    font-size: 0.76rem;
    font-weight: 850;
    text-transform: uppercase;
  }

  .cd-cover-copy strong {
    overflow: hidden;
    color: #f4f7fb;
    font-size: 1rem;
    font-weight: 850;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cd-cover-copy small {
    overflow: hidden;
    color: #9aa4b1;
    font-size: 0.82rem;
    font-weight: 700;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cd-cover-copy small.error {
    color: #ffcbc8;
  }

  .cd-cover-copy button {
    justify-self: start;
    min-height: 36px;
    border: 1px solid #303844;
    border-radius: 8px;
    background: #161a20;
    color: #d5dce5;
    cursor: default;
    font: inherit;
    font-size: 0.84rem;
    font-weight: 850;
    padding: 0 12px;
  }

  .cd-cover-copy button:hover:not(:disabled),
  .cd-cover-copy button:focus-visible:not(:disabled) {
    border-color: #35544f;
    background: #1b2027;
    outline: none;
  }

  .cd-cover-copy button:disabled {
    border-color: #252c35;
    background: #11161d;
    color: #8b95a3;
  }

  .mix-tool-mark {
    display: grid;
    flex: 0 0 auto;
    place-items: center;
    border-radius: 8px;
    background: #17332f;
    color: #9ee3d9;
    font-weight: 900;
  }

  .about-section {
    margin-bottom: 8px;
  }

  .shortcut-mark {
    display: grid;
    flex: 0 0 auto;
    width: 54px;
    height: 54px;
    place-items: center;
    border-radius: 8px;
    background: #27303b;
    color: #dbe7f3;
    font-size: 1.35rem;
    font-weight: 900;
  }

  .shortcuts-backdrop,
  .confirmation-backdrop {
    position: fixed;
    z-index: 80;
    inset: 0;
    display: grid;
    place-items: center;
    overflow: auto;
    background: rgba(5, 7, 10, 0.72);
    padding: 24px;
  }

  .shortcuts-modal,
  .confirmation-modal {
    width: min(720px, 100%);
    max-height: min(720px, calc(100dvh - 48px));
    overflow: auto;
    border: 1px solid #303844;
    border-radius: 8px;
    background: #12161c;
    box-shadow: 0 24px 70px rgba(0, 0, 0, 0.46);
    outline: none;
    padding: 22px;
  }

  .confirmation-modal {
    width: min(460px, 100%);
  }

  .shortcuts-header,
  .confirmation-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 18px;
    margin-bottom: 18px;
  }

  .shortcuts-header h3,
  .confirmation-header h3 {
    margin: 0;
    color: #f4f7fb;
    font-size: 1.35rem;
    line-height: 1.2;
  }

  .confirmation-modal > p {
    margin: 0 0 18px;
    color: #aab4c0;
    font-size: 0.95rem;
    font-weight: 650;
  }

  .confirmation-modal .scan-error {
    margin-bottom: 18px;
  }

  .confirmation-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 10px;
  }

  .confirmation-actions button {
    min-height: 38px;
    border: 1px solid #303844;
    border-radius: 8px;
    background: #171d24;
    color: #d5dce5;
    cursor: default;
    font: inherit;
    font-size: 0.86rem;
    font-weight: 850;
    padding: 0 13px;
  }

  .confirmation-actions button:hover,
  .confirmation-actions button:focus-visible {
    border-color: #35544f;
    background: #1d242c;
    color: #f4f7fb;
    outline: none;
  }

  .confirmation-actions button.destructive {
    border-color: #5b3434;
    background: #2a1718;
    color: #ffcbc8;
  }

  .confirmation-actions button.destructive:hover,
  .confirmation-actions button.destructive:focus-visible {
    border-color: #7a3a3a;
    background: #341c1e;
  }

  .confirmation-actions button:disabled {
    border-color: #303844;
    background: #151a21;
    color: #626c79;
  }

  .shortcuts-header button {
    min-height: 34px;
    border: 1px solid #303844;
    border-radius: 8px;
    background: #171d24;
    color: #d5dce5;
    cursor: default;
    font: inherit;
    font-size: 0.84rem;
    font-weight: 800;
    padding: 0 12px;
  }

  .shortcuts-header button:hover,
  .shortcuts-header button:focus-visible {
    border-color: #35544f;
    background: #1d242c;
    color: #f4f7fb;
    outline: none;
  }

  .shortcut-group-list {
    display: grid;
    gap: 14px;
  }

  .shortcut-group {
    border: 1px solid #242b35;
    border-radius: 8px;
    background: #151a21;
    padding: 14px;
  }

  .shortcut-group h4 {
    margin: 0 0 10px;
    color: #f4f7fb;
    font-size: 0.94rem;
  }

  .shortcut-list {
    display: grid;
    gap: 8px;
  }

  .shortcut-row {
    display: grid;
    grid-template-columns: minmax(150px, 0.45fr) minmax(0, 1fr);
    align-items: center;
    gap: 14px;
    min-height: 38px;
  }

  .shortcut-keys {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }

  .shortcut-plus {
    color: #6f7b88;
    font-size: 0.78rem;
    font-weight: 900;
  }

  kbd {
    min-width: 28px;
    border: 1px solid #3a4350;
    border-bottom-color: #242b35;
    border-radius: 6px;
    background: #0d1117;
    color: #eef3f8;
    font-family: inherit;
    font-size: 0.78rem;
    font-weight: 850;
    line-height: 1;
    padding: 7px 9px;
    text-align: center;
  }

  .shortcut-row p {
    margin: 0;
    color: #aab4c0;
    font-size: 0.9rem;
    font-weight: 650;
  }

  .health-summary-grid {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 10px;
  }

  .health-summary-grid div {
    min-width: 0;
    border: 1px solid #242b35;
    border-radius: 8px;
    background: #12161c;
    padding: 12px;
  }

  .health-summary-grid div.issue {
    border-color: #45412a;
    background: #1d1b12;
  }

  .health-summary-grid span {
    display: block;
    overflow: hidden;
    margin-bottom: 5px;
    color: #8f9aa8;
    font-size: 0.75rem;
    font-weight: 850;
    text-overflow: ellipsis;
    text-transform: uppercase;
    white-space: nowrap;
  }

  .health-summary-grid strong {
    color: #f4f7fb;
    font-size: 1.25rem;
    line-height: 1;
  }

  .group-empty.compact {
    min-height: 82px;
  }

  .diagnostic-album-list,
  .duplicate-album-list {
    display: grid;
    gap: 10px;
  }

  .diagnostic-album-list {
    grid-template-columns: repeat(3, minmax(0, 1fr));
    margin-bottom: 12px;
  }

  .diagnostic-album-card {
    display: grid;
    grid-template-columns: 42px minmax(0, 1fr);
    align-items: center;
    gap: 10px;
    min-height: 66px;
    border: 1px solid #242b35;
    border-radius: 8px;
    background: #151a21;
    color: inherit;
    cursor: default;
    font: inherit;
    padding: 10px;
    text-align: left;
  }

  .diagnostic-album-card:hover,
  .diagnostic-album-card:focus-visible {
    border-color: #35544f;
    background: #1b2027;
    outline: none;
  }

  .diagnostic-album-card > span:first-child {
    display: grid;
    width: 42px;
    height: 42px;
    place-items: center;
    border-radius: 8px;
    background: #17332f;
    color: #9ee3d9;
    font-weight: 900;
  }

  .diagnostic-album-card div {
    min-width: 0;
  }

  .diagnostic-album-card strong,
  .diagnostic-album-card small {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .diagnostic-album-card strong {
    color: #f4f7fb;
    font-size: 0.9rem;
    line-height: 1.25;
  }

  .diagnostic-album-card small {
    margin-top: 3px;
    color: #8f9aa8;
    font-size: 0.78rem;
    font-weight: 700;
  }

  .duplicate-album-card {
    display: grid;
    gap: 12px;
    border: 1px solid #242b35;
    border-radius: 8px;
    background: #151a21;
    padding: 14px;
  }

  .duplicate-album-card h3 {
    margin-bottom: 5px;
  }

  .duplicate-album-card p:not(.eyebrow) {
    margin: 0;
    color: #8f9aa8;
    font-weight: 700;
  }

  .duplicate-album-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .duplicate-album-actions button {
    min-height: 34px;
    border: 1px solid #303844;
    border-radius: 8px;
    background: #12161c;
    color: #d5dce5;
    cursor: default;
    font: inherit;
    font-size: 0.82rem;
    font-weight: 800;
    padding: 0 11px;
  }

  .duplicate-album-actions button:hover,
  .duplicate-album-actions button:focus-visible {
    border-color: #35544f;
    background: #1b2027;
    outline: none;
  }

  .duplicate-folders {
    overflow-wrap: anywhere;
    font-size: 0.82rem;
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
    .playlist-grid,
    .video-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .videos-toolbar,
    .video-detail-hero {
      grid-template-columns: 1fr;
    }

    .video-stat-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .mix-option-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .health-summary-grid {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }

    .diagnostic-album-list {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .settings-stat-grid,
    .cd-status-grid,
    .cd-metadata-form,
    .settings-status-list,
    .settings-tool-grid,
    .stats-overview-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .cd-cover-panel {
      grid-template-columns: 96px minmax(0, 1fr);
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

    .now-playing-hero {
      grid-template-columns: 1fr;
    }

    .now-playing-cover {
      width: min(100%, 320px);
    }

    .now-playing-copy h3 {
      font-size: 2.3rem;
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

    .album-detail-header .detail-cover {
      width: min(100%, 260px);
    }

    .album-track-row {
      grid-template-columns: 36px minmax(0, 1fr) auto auto;
    }

    .album-track-format {
      display: none;
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

    .health-summary-grid,
    .diagnostic-album-list {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .settings-section-header {
      display: grid;
    }

    .settings-control-list,
    .about-grid,
    .stats-section-grid {
      grid-template-columns: 1fr;
    }

    .cd-track-row {
      grid-template-columns: 52px minmax(130px, 1.1fr) minmax(110px, 0.9fr) 80px minmax(100px, 0.8fr) minmax(0, 1fr);
    }

    .stats-recent-card {
      grid-template-columns: 48px minmax(0, 1fr);
    }

    .stats-played-at {
      grid-column: 2;
      justify-self: start;
      max-width: 100%;
    }

    .queue-panel {
      left: 16px;
      right: 16px;
      bottom: 150px;
      width: auto;
      max-height: calc(100dvh - 178px);
    }

    .shortcuts-backdrop,
    .confirmation-backdrop {
      align-items: start;
      padding: 16px;
    }
  }

  @media (max-width: 520px) {
    .album-grid,
    .artist-grid,
    .genre-grid,
    .playlist-grid,
    .video-grid,
    .mix-option-grid,
    .health-summary-grid,
    .diagnostic-album-list,
    .settings-stat-grid,
    .video-stat-grid,
    .video-meta-grid,
    .video-edit-form,
    .cd-status-grid,
    .cd-metadata-form,
    .settings-status-list,
    .settings-tool-grid,
    .stats-overview-grid {
      grid-template-columns: 1fr;
    }

    .cd-track-head {
      display: none;
    }

    .cd-track-row {
      grid-template-columns: 48px minmax(0, 1fr);
      gap: 8px 12px;
      align-items: start;
    }

    .cd-track-row span:nth-child(n + 2) {
      grid-column: 2;
    }

    .cd-cover-panel {
      grid-template-columns: 1fr;
    }

    .cd-cover-preview {
      width: min(100%, 180px);
    }

    .settings-stat-tile.wide {
      grid-column: auto;
    }

    .genre-editor form {
      flex-direction: column;
    }

    .now-playing-queue-list button {
      grid-template-columns: 42px minmax(0, 1fr);
    }

    .now-playing-queue-list button > small {
      display: none;
    }

    .now-playing-empty {
      align-items: flex-start;
      flex-direction: column;
    }

    .album-track-row {
      gap: 10px;
      padding: 9px 10px;
    }

    .album-track-duration {
      display: none;
    }

    .shortcuts-modal,
    .confirmation-modal {
      max-height: calc(100dvh - 32px);
      padding: 18px;
    }

    .shortcuts-header,
    .confirmation-header {
      align-items: stretch;
      flex-direction: column;
    }

    .shortcuts-header button {
      width: fit-content;
    }

    .shortcut-row {
      grid-template-columns: 1fr;
      gap: 6px;
    }
  }
</style>
