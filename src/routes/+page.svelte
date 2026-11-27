<script lang="ts">
  import {
    addTrackToPlaylist,
    addTracksToPlaylist,
    autoFindTrackLyrics,
    chooseCoverImage,
    chooseDvdImportOutputFolder,
    chooseOutputFolder,
    chooseLibraryFolder,
    chooseVideoTsFolder,
    chooseVideoFolder,
    createPlaylist,
    deletePlaylist,
    detectAudioCd,
    detectDvd,
    getLibraryCache,
    getAlbumTagEditorData,
    getPlatformCapabilities,
    getTrackTagEditorData,
    getVideoLibrary,
    importDvdTitle,
    lookupCdMetadata,
    lookupCdCover,
    movePlaylistTrack,
    recordTrackPlay,
    getListeningStats,
    getListeningTimeByTrack,
    saveListeningMinutes,
    removeListeningEvent,
    chooseHistoryExportPath,
    chooseHistoryImportPath,
    exportListeningHistory,
    previewListeningHistoryImport,
    importListeningHistory,
    associateListeningHistoryTrack,
    readTrackLyrics,
    removeCachedTrackLyrics,
    restoreOriginalLyrics,
    saveEditedLyrics,
    removeTrackFromPlaylist,
    renamePlaylist,
    ripCdToFlac,
    inspectCoverImage,
    scanLibrary,
    setIgnoredTracks,
    scanDvdTitles,
    scanVideoFolder,
    saveTrackLyricsResult,
    searchTrackLyricsResults,
    setAlbumGenres,
    setAlbumCover,
    setArtistGenres,
    setTrackLyricsOffset,
    toggleTrackFavorite,
    setTracksFavorite,
    updateTrackTags,
    updateAlbumTags,
    updateVideoInfo,
    updateVideoProgress,
  } from "$lib/api/library";
  import {
    bringVideoWindowToFront,
    closeVideoWindow,
    fullscreenVideoWindow,
    getPlaybackStatus,
    getVideoCodecInfo,
    getVideoState,
    pausePlayback,
    pauseVideo,
    playTrack,
    playVideo,
    resumePlayback,
    resumeVideo,
    seekPlayback,
    seekVideo,
    setPlaybackVolume,
    stageGaplessTrack,
    setVideoVolume,
    stopVideo,
  } from "$lib/api/playback";
  import {
    AUTOMATIC_UPDATE_SETTING_KEY,
    LAST_SUCCESSFUL_UPDATE_CHECK_KEY,
    LAST_AUTOMATIC_UPDATE_ATTEMPT_KEY,
    type AppImageMenuStatus,
    type CassetteUpdate,
    addAppImageToMenu,
    checkForCassetteUpdate,
    getAppImageMenuStatus,
    getUpdateRuntimeInfo,
    installCassetteUpdate,
    loadUpdaterStorage,
    persistUpdaterStorage,
    removeAppImageFromMenu,
    shouldRunAutomaticUpdateCheck,
    updateReleaseUrl,
    type UpdateDownloadProgress,
    type UpdateRuntimeInfo,
  } from "$lib/api/updates";
  import CompactDropdown, { type DropdownOption } from "$lib/components/CompactDropdown.svelte";
  import ContextMenu from "$lib/components/ContextMenu.svelte";
  import CoverArtViewer from "$lib/components/CoverArtViewer.svelte";
  import LyricsEditor from "$lib/components/LyricsEditor.svelte";
  import LibrarySection from "$lib/components/LibrarySection.svelte";
  import NowPlayingBar from "$lib/components/NowPlayingBar.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import TrackSortMenu from "$lib/components/TrackSortMenu.svelte";
  import TrackList from "$lib/components/TrackList.svelte";
  import { buildAlbums, buildArtists, buildGenres } from "$lib/data/libraryViews";
  import { statsRangeForPeriod, type StatsPeriod } from "$lib/utils/statsPeriod";
  import { listeningDurationLabel, listeningTotalLabel, resolveStatsPlayback, statsDayValueLabel } from "$lib/utils/listeningStats";
  import type { ListeningStats, ListeningImportPreview, RecentListen } from "$lib/types/listening";
  import { albums as mockAlbums, artists as mockArtists, genres as mockGenres, navItems } from "$lib/data/mockLibrary";
  import { ENABLE_EXPERIMENTAL_VIDEOS } from "$lib/featureFlags";
  import type {
    Album,
    AlbumTagEditorData,
    AlbumTagUpdateProgress,
    AlbumTagUpdateResult,
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
    DvdDetectResult,
    DvdImportEvent,
    DvdImportMetadata,
    DvdTitle,
    DvdTitleScanResult,
    Genre,
    LrclibLyricsResult,
    PlatformCapabilities,
    PlaybackStatus,
    Playlist,
    Track,
    TrackTagEditorData,
    TrackTagValues,
    TrackLyrics,
    LibraryFolderScan,
    LibraryScanProgress,
    UpdateTrackTagsRequest,
    UpdateAlbumTagsRequest,
    VideoEntry,
    VideoCodecInfo,
    VideoInfoUpdate,
    VideoLibrary,
    VideoPlaybackStatus,
    VideoType,
  } from "$lib/types/library";
  import { sendLinuxPlaybackNotification } from "$lib/utils/linuxNotifications";
  import { localImageSource } from "$lib/utils/localImage";
  import { ListeningTimeBuffer, PlaybackListenProgress } from "$lib/utils/playbackListenProgress";
  import {
    PlaybackSeekCoordinator,
    failedSeekRecoveryPosition,
    type PlaybackSeekRequest,
  } from "$lib/utils/playbackSeek";
  import {
    hasSyncedLyricsIntro,
    parseLrcLyrics,
    resolveSyncedLyricsState,
    startsSyncedLyricBreak,
    type SyncedLyricCue,
  } from "$lib/utils/syncedLyrics";
  import { sortAlbumDisplayTracks, topPlayedTracks, type AlbumTrackSortKey } from "$lib/utils/albumTrackSort";
  import { stepView, visitView, type ViewHistory } from "$lib/utils/viewHistory";
  import {
    STATS_PAGE_SIZE,
    activitySortOptions,
    compareListeningActivity,
    isActivitySortOrder,
    type ActivitySortOrder,
    type ListeningActivity,
    nextStatsLimit,
    rankMostPlayedTracks,
    rankRecentlyPlayedTracks,
    sortAlbumsByActivity,
    sortArtistsByActivity,
    sortGenresByActivity,
    visibleStatsItems,
  } from "$lib/utils/playStats";
  import { listen } from "@tauri-apps/api/event";
  import { isTauri } from "@tauri-apps/api/core";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { openPath, openUrl } from "@tauri-apps/plugin-opener";
  import { onMount, untrack } from "svelte";
  import packageInfo from "../../package.json";

  type SongSortKey = "title" | "artist" | "album" | "duration" | "recentlyAdded" | "recentlyPlayed" | ActivitySortOrder;
  type AlbumSortKey = "title" | "artist" | "year" | "trackCount" | ActivitySortOrder;
  type ArtistSortKey = "name" | "songCount" | "albumCount" | ActivitySortOrder;
  type GenreSortKey = "name" | "songCount" | "artistCount" | "albumCount" | ActivitySortOrder;
  type VideoSortKey = "title" | "artist" | "year" | "recentlyPlayed" | "duration";
  type VideoTypeFilter = "all" | "music_video" | "live" | "interview_documentary" | "behind_the_scenes" | "other";
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
    action?: () => void | Promise<void>;
    items?: ContextMenuItem[];
    checked?: boolean;
    toggle?: boolean;
    radio?: boolean;
  };
  type ContextMenuState = {
    x: number;
    y: number;
    items: ContextMenuItem[];
  };
  type AlbumNavigationOrigin =
    | { view: "Albums" }
    | { view: "Artists"; artistName: string }
    | { view: "Genres"; genreName: string };
  type ArtistNavigationOrigin =
    | { view: "Artists" }
    | { view: "Genres"; genreName: string };
  type ViewLocation = {
    view: string;
    statsDetailSection: StatsSectionId | null;
    albumId: string | null;
    albumOrigin: AlbumNavigationOrigin;
    artistName: string | null;
    artistOrigin: ArtistNavigationOrigin;
    genreName: string | null;
    detailSongsView: "artist" | "genre" | null;
    playlistId: string | null;
    likedSongs: boolean;
    mixBuilder: boolean;
    libraryHealth: boolean;
    videoId: string | null;
  };
  type ThemeId = "cassette-teal" | "rose-noir" | "royal-gold" | "glacier" | "obsidian" | "light";
  type ThemePreset = {
    id: ThemeId;
    name: string;
    description: string;
    swatches: [string, string, string];
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
    playCount: number;
    listenedMs: number;
    songCount: number;
  };
  type TopAlbumStat = {
    album: Album;
    playCount: number;
    listenedMs: number;
    songCount: number;
  };
  type TopGenreStat = {
    genre: Genre;
    playCount: number;
    listenedMs: number;
    songCount: number;
  };
  type ListeningActivityDisplay = "0" | "1" | "2";
  type StatsSectionId = "tracks" | "artists" | "albums" | "genres" | "recent";
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
    videoType: VideoType;
    releaseOrCollection: string;
    year: string;
    venue: string;
    city: string;
    country: string;
    descriptionOrNotes: string;
  };
  type DvdImportDraft = {
    title: string;
    artist: string;
    videoType: VideoType;
    releaseOrCollection: string;
    year: string;
    venue: string;
    city: string;
    country: string;
    descriptionOrNotes: string;
    outputFilename: string;
  };
  type TagEditorDraft = {
    title: string;
    artist: string;
    album: string;
    albumArtist: string;
    genre: string;
    year: string;
    trackNumber: string;
    discNumber: string;
  };
  type AlbumTagFieldMode = "unchanged" | "set" | "clear";
  type AlbumTagFieldDraft = {
    mode: AlbumTagFieldMode;
    value: string;
  };
  type AlbumTagEditorDraft = {
    album: AlbumTagFieldDraft;
    albumArtist: AlbumTagFieldDraft;
    artist: AlbumTagFieldDraft;
    genre: AlbumTagFieldDraft;
    year: AlbumTagFieldDraft;
  };
  type AlbumTagEditorStage = "edit" | "confirm" | "result";
  type AlbumTagMoveNotice = {
    editedAlbumId: string;
    editedTrackCount: number;
    excludedTrackCount: number;
    originalAlbumId: string;
    originalAlbumTitle: string;
  };

  const mixFormatOptions = ["All", "FLAC", "MP3", "OGG", "OPUS", "WAV", "M4A"];
  const AUTO_LYRICS_COOLDOWN_MS = 7 * 24 * 60 * 60 * 1000;
  const AUTO_LYRICS_SETTING_KEY = "cassette:auto-find-lyrics";
  const TRACK_NOTIFICATIONS_SETTING_KEY = "cassette:track-change-notifications";
  const ALBUM_PLAY_COUNTS_SETTING_KEY = "cassette:show-album-play-counts";
  const ARTIST_PLAY_COUNTS_SETTING_KEY = "cassette:show-artist-play-counts";
  const GENRE_PLAY_COUNTS_SETTING_KEY = "cassette:show-genre-play-counts";
  const ALBUM_TRACK_PLAY_COUNTS_SETTING_KEY = "cassette:show-album-track-play-counts";
  const SONG_LIST_NUMBERS_SETTING_KEY = "cassette:show-song-list-numbers";
  const ALBUM_TAG_FIELDS: Array<{ key: keyof AlbumTagEditorDraft; label: string; help?: string; inputMode?: "numeric" }> = [
    { key: "album", label: "Album title" },
    {
      key: "albumArtist",
      label: "Album artist",
      help: "Use one shared album artist to group compilation or multi-artist albums. Individual track artists remain separate unless changed below.",
    },
    {
      key: "artist",
      label: "Artist",
      help: "Each track can keep its own artist. Setting this field replaces Artist on every editable track; leave it unchanged to preserve multiple performers.",
    },
    { key: "genre", label: "Genre" },
    { key: "year", label: "Year", inputMode: "numeric" },
  ];
  const STATS_PREVIEW_LIMITS: Record<StatsSectionId, number> = {
    tracks: 5,
    artists: 5,
    albums: 5,
    genres: 5,
    recent: 5,
  };
  const statsDayFormat = new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric", timeZone: "UTC" });
  const statsRangeFormat = new Intl.DateTimeFormat(undefined, { year: "numeric", month: "short", day: "numeric", timeZone: "UTC" });
  const THEME_SETTING_KEY = "cassette:theme";
  const DEFAULT_THEME: ThemeId = "obsidian";
  const themePresets: ThemePreset[] = [
    {
      id: "cassette-teal",
      name: "Cassette Teal",
      description: "Near-black, charcoal, and Cassette teal.",
      swatches: ["#0d0f13", "#151a21", "#2f8f83"],
    },
    {
      id: "glacier",
      name: "Glacier",
      description: "Blue-black, cold slate, and icy blue.",
      swatches: ["#071018", "#111d28", "#79c7e8"],
    },
    {
      id: "obsidian",
      name: "Obsidian",
      description: "OLED black, graphite panels, and cool silver accents.",
      swatches: ["#000000", "#111216", "#9aa6b2"],
    },
    {
      id: "light",
      name: "Light",
      description: "White panels, soft gray, and deep teal accents.",
      swatches: ["#f1f5f4", "#ffffff", "#216e61"],
    },
  ];
  const songSortOptions: DropdownOption[] = [
    { value: "title", label: "Title" },
    { value: "artist", label: "Artist" },
    { value: "album", label: "Album" },
    { value: "duration", label: "Duration" },
    { value: "recentlyAdded", label: "Recently added" },
    { value: "recentlyPlayed", label: "Recently played" },
    ...activitySortOptions,
  ];
  const albumTrackSortOptions: { value: AlbumTrackSortKey; label: string }[] = [
    { value: "trackNumber", label: "Track number" },
    { value: "title", label: "Title" },
    { value: "artist", label: "Artist" },
    { value: "duration", label: "Duration" },
    ...activitySortOptions,
  ];
  const groupedTrackSortOptions: { value: AlbumTrackSortKey; label: string }[] = [
    { value: "title", label: "Title" },
    { value: "album", label: "Album" },
    { value: "artist", label: "Artist" },
    { value: "trackNumber", label: "Track number" },
    { value: "duration", label: "Duration" },
    ...activitySortOptions,
  ];
  const videoTypeOptions: Array<{ value: VideoType; label: string }> = [
    { value: "music_video", label: "Music Video / PV" },
    { value: "live_show", label: "Live Show" },
    { value: "concert", label: "Concert" },
    { value: "interview_documentary", label: "Interview / Documentary" },
    { value: "behind_the_scenes", label: "Behind the Scenes" },
    { value: "other", label: "Other" },
  ];
  const videoTypeFilters: Array<{ value: VideoTypeFilter; label: string }> = [
    { value: "all", label: "All" },
    { value: "music_video", label: "Music Videos / PVs" },
    { value: "live", label: "Live Shows / Concerts" },
    { value: "interview_documentary", label: "Interviews / Documentaries" },
    { value: "behind_the_scenes", label: "Behind the Scenes" },
    { value: "other", label: "Other" },
  ];
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
        { keys: ["Ctrl or ⌘", "/"], description: "Open keyboard shortcuts" },
      ],
    },
    {
      title: "Album track selection",
      shortcuts: [
        { keys: ["Ctrl or ⌘", "Click"], description: "Start or extend a range of tracks" },
        { keys: ["Click"], description: "Toggle one track while selecting" },
        { keys: ["Right-click"], description: "Choose Select tracks, or open actions for the selection" },
        { keys: ["Enter or Space"], description: "Toggle the focused track while selecting" },
        { keys: ["Escape"], description: "Clear the track selection" },
      ],
    },
    {
      title: "Search and panels",
      shortcuts: [
        { keys: ["Escape"], description: "Clear search or close an open panel or menu" },
      ],
    },
  ];

  let libraryTracks = $state<Track[]>([]);
  let libraryFolders = $state<LibraryFolderScan[]>([]);
  let scanProgress = $state<LibraryScanProgress | null>(null);
  let scanSummary = $state<string | null>(null);
  let unavailableTrackIds = $state(new Set<string>());
  let ignoredTrackIds = $state(new Set<string>());
  let listeningActivityDisplay = $state<ListeningActivityDisplay>("0");
  let showPlayCounts = $derived(listeningActivityDisplay !== "1");
  let showListeningTime = $derived(listeningActivityDisplay !== "0");
  let statsMetric = $state<"plays" | "time">("plays");
  let statsUseListeningTime = $derived(!showPlayCounts || (showListeningTime && statsMetric === "time"));
  let listeningTimeByTrack = $state<Record<string, number>>({});
  let savingIgnoredTracks = $state(false);
  let ignoredTracksError = $state<string | null>(null);
  let customAlbumCovers = $state<Record<string, string>>({});
  let tracks = $derived(libraryTracks.map((track) => {
    const coverArtPath = customAlbumCovers[albumIdForTrack(track)] ?? track.coverArtPath;
    const ignored = ignoredTrackIds.has(track.id);
    const unavailable = unavailableTrackIds.has(track.id);
    const listenedMs = ignored ? 0 : listeningTimeByTrack[track.id] ?? 0;
    return ignored || unavailable || listenedMs || coverArtPath !== track.coverArtPath
      ? { ...track, coverArtPath, ignored, unavailable, listenedMs, playCount: ignored ? 0 : track.playCount, lastPlayedAt: ignored ? null : track.lastPlayedAt }
      : track;
  }));
  let editingCoverForAlbum = $state<string | null>(null);
  let albumCoverFeedback = $state<{ albumId: string; message: string; error: boolean } | null>(null);
  let videos = $state<VideoEntry[]>([]);
  let isScanning = $state(false);
  let isScanningVideos = $state(false);
  let scanError = $state<string | null>(null);
  let libraryRecoveryNotice = $state<string | null>(null);
  let videoError = $state<string | null>(null);
  let videoMessage = $state<string | null>(null);
  let videoFolder = $state<string | null>(null);
  let lastVideoScannedAt = $state<number | null>(null);
  let selectedVideoId = $state<string | null>(null);
  let videoSearchQuery = $state("");
  let videoSort = $state<VideoSortKey>("title");
  let videoSortDirection = $state<SortDirection>("asc");
  let videoTypeFilter = $state<VideoTypeFilter>("all");
  let isEditingVideo = $state(false);
  let isSavingVideoInfo = $state(false);
  let videoEditDraft = $state<VideoEditDraft>(emptyVideoEditDraft());
  let videoBackendStatus = $state<VideoPlaybackStatus | null>(null);
  let videoCodecInfo = $state<VideoCodecInfo | null>(null);
  let isLoadingVideoCodecInfo = $state(false);
  let videoPlaybackError = $state<string | null>(null);
  let videoPlaybackMessage = $state<string | null>(null);
  let videoSessionVideoId = $state<string | null>(null);
  let videoSessionWatchedSeconds = 0;
  let videoObservedPositionSeconds = 0;
  let videoLastProgressSaveMs = 0;
  let videoPlayCountRecorded = false;
  let dvdDetectResult = $state<DvdDetectResult | null>(null);
  let dvdSourcePath = $state<string | null>(null);
  let dvdSourceType = $state<"physical_device" | "video_ts_folder" | null>(null);
  let dvdTitleScan = $state<DvdTitleScanResult | null>(null);
  let dvdTitles = $state<DvdTitle[]>([]);
  let selectedDvdTitleNumber = $state<number | null>(null);
  let dvdOutputFolder = $state<string | null>(null);
  let dvdImportDraft = $state<DvdImportDraft>(defaultDvdImportDraft(null));
  let isDetectingDvd = $state(false);
  let isScanningDvdTitles = $state(false);
  let isImportingDvd = $state(false);
  let dvdImportStatus = $state("No DVD detected");
  let dvdImportError = $state<string | null>(null);
  let dvdImportMessage = $state<string | null>(null);
  let cdRipError = $state<string | null>(null);
  let cdRipMessage = $state<string | null>(null);
  let isDetectingCd = $state(false);
  let isRippingCd = $state(false);
  let cdDriveFound = $state<boolean | null>(null);
  let audioCdFound = $state<boolean | null>(null);
  let cdTracks = $state<CdRipTrack[]>([]);
  let cdCompletedCount = $derived(cdTracks.filter((track) => track.status === "done").length);
  let cdFailedCount = $derived(cdTracks.filter((track) => track.status === "error").length);
  let cdDurationSeconds = $derived(cdTracks.reduce((total, track) => total + (track.durationSeconds ?? 0), 0));
  let cdRawOutput = $state("");
  let detectedCdKey: string | null = null;
  let cdDetectionRevision = 0;
  let cdCoverRequest = 0;
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
  let artworkViewer = $state<{ source: string; title: string; opener: HTMLElement } | null>(null);
  let currentTrackIndex = $state<number | null>(null);
  let playbackQueue = $state<Track[]>([]);
  let currentQueueIndex = $state<number | null>(null);
  let isQueueOpen = $state(false);
  let isShuffleEnabled = $state(false);
  let shuffledQueueOrder = $state<number[]>([]);
  let repeatMode = $state<RepeatMode>("off");
  let isHandlingTrackEnd = $state(false);
  let platformCapabilities = $state<PlatformCapabilities | null>(null);
  let trackNotificationsEnabled = $state(false);
  let gaplessEnabled = $state(true);
  const listenPercentages = [25, 50, 75, 100];
  let listenPercentage = $state(50);
  let trackNotificationStatus = $state<string | null>(null);
  let trackNotificationStatusIsError = $state(false);
  let automaticUpdateChecksEnabled = $state(true);
  let lastSuccessfulUpdateCheck = $state<number | null>(null);
  let updateRuntime = $state<UpdateRuntimeInfo | null>(null);
  const isWindows = $derived(platformCapabilities?.os === "windows" || updateRuntime?.platform === "windows");
  let appImageMenuStatus = $state<AppImageMenuStatus | null>(null);
  let appImageMenuBusy = $state(false);
  let appImageMenuMessage = $state<string | null>(null);
  let appImageMenuIsError = $state(false);
  let updateCheckState = $state<"idle" | "checking" | "confirming" | "available" | "current" | "downloading" | "installing" | "error">("idle");
  let updateStatusMessage = $state<string | null>(null);
  let updateStatusIsError = $state(false);
  let availableUpdateVersion = $state<string | null>(null);
  let availableUpdateNotes = $state<string | null>(null);
  let updateDownloadProgress = $state<UpdateDownloadProgress | null>(null);
  let showUpdateNotice = $state(false);
  let availableUpdate: CassetteUpdate | null = null;
  let lastAutomaticUpdateAttempt: number | null = null;
  let updaterStorageAvailable = true;
  let automaticUpdateCheckAttemptedThisSession = false;
  let lastNotifiedTrackKey: string | null = null;
  let lastNotifiedPlaybackStartId: number | null = null;
  let playbackStartId = 0;
  let isStartingTrack = false;
  let playbackControlRevision = 0;
  let isChangingPlayback = false;
  let isPollingPlayback = false;
  let volumeRequestId = 0;
  let isSettingVolume = false;
  let mainElement: HTMLElement | undefined = $state();
  let activeView = $state("Albums");
  let statsDetailSection = $state<StatsSectionId | null>(null);
  let selectedAlbumId = $state<string | null>(null);
  let trackSelectionAlbumId = $state<string | null>(null);
  let selectedTrackIds = $state(new Set<string>());
  let selectionAnchor = $state<string | null>(null);
  let bulkTrackFeedback = $state<{ albumId: string; message: string; error: boolean } | null>(null);
  let savingBulkTracks = false;
  let isAlbumSelectingTracks = $derived(trackSelectionAlbumId !== null && trackSelectionAlbumId === selectedAlbumId);
  let albumNavigationOrigin = $state<AlbumNavigationOrigin>({ view: "Albums" });
  let selectedArtistName = $state<string | null>(null);
  let artistNavigationOrigin = $state<ArtistNavigationOrigin>({ view: "Artists" });
  let selectedGenreName = $state<string | null>(null);
  let detailSongsView = $state<"artist" | "genre" | null>(null);
  let viewHistory = $state<ViewHistory<ViewLocation>>({
    entries: [{
      view: "Albums", statsDetailSection: null, albumId: null, albumOrigin: { view: "Albums" },
      artistName: null, artistOrigin: { view: "Artists" }, genreName: null,
      detailSongsView: null,
      playlistId: null, likedSongs: false, mixBuilder: false, libraryHealth: false, videoId: null,
    }],
    index: 0,
  });
  let canNavigateBack = $derived(viewHistory.index > 0);
  let canNavigateForward = $derived(viewHistory.index < viewHistory.entries.length - 1);
  let isLikedSongsOpen = $state(false);
  let isMixBuilderOpen = $state(false);
  let isCreatePlaylistOpen = $state(false);
  let isPlaylistEditMode = $state(false);
  let isMovingPlaylistTrack = $state(false);
  let isLibraryHealthOpen = $state(false);
  let selectedPlaylistId = $state<string | null>(null);
  let isShortcutHelpOpen = $state(false);
  let searchQuery = $state("");
  let songsSearchQuery = $state("");
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
  let albumTrackSort = $state<AlbumTrackSortKey>("trackNumber");
  let albumTrackSortDirection = $state<SortDirection>("asc");
  let artistTrackSort = $state<AlbumTrackSortKey>("mostPlayed");
  let artistTrackSortDirection = $state<SortDirection>("asc");
  let genreTrackSort = $state<AlbumTrackSortKey>("mostPlayed");
  let genreTrackSortDirection = $state<SortDirection>("asc");
  let artistTrackVisibleLimit = $state(STATS_PAGE_SIZE);
  let genreTrackVisibleLimit = $state(STATS_PAGE_SIZE);
  let artistSort = $state<ArtistSortKey>("name");
  let artistSortDirection = $state<SortDirection>("asc");
  let genreSort = $state<GenreSortKey>("name");
  let genreSortDirection = $state<SortDirection>("asc");
  let showAlbumPlayCounts = $state(true);
  let showArtistPlayCounts = $state(true);
  let showGenrePlayCounts = $state(true);
  let showAlbumTrackPlayCounts = $state(true);
  let showSongListNumbers = $state(true);
  let statsVisibleLimits = $state<Record<StatsSectionId, number>>({
    tracks: STATS_PAGE_SIZE,
    artists: STATS_PAGE_SIZE,
    albums: STATS_PAGE_SIZE,
    genres: STATS_PAGE_SIZE,
    recent: STATS_PAGE_SIZE,
  });
  let statsPeriod = $state<StatsPeriod>("all");
  let statsCustomFrom = $state("");
  let statsCustomTo = $state("");
  let statsSelectedMonth = $state("");
  let statsSelectedYear = $state("");
  let statsClock = $state(Date.now());
  let statsRevision = $state(0);
  let statsSnapshot = $state<ListeningStats | null>(null);
  let statsLoading = $state(false);
  let statsError = $state<string | null>(null);
  let removingListenId = $state<string | null>(null);
  let removeListenError = $state<string | null>(null);
  let historyImportPath = $state<string | null>(null);
  let historyImportPreview = $state<ListeningImportPreview | null>(null);
  let historyBusy = $state(false);
  let historyMessage = $state<string | null>(null);
  let historyError = $state<string | null>(null);
  let historyAssociationTargets = $state<Record<string, string>>({});
  let historyPreviewLimit = $state(STATS_PAGE_SIZE);
  let statsChartOffset = $state(0);
  let statsSelectedDay = $state<string | null>(null);
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
  let playbackSessionEventId: string | null = null;
  const playbackListenProgress = new PlaybackListenProgress();
  const listeningTimeBuffer = new ListeningTimeBuffer();
  let listeningTimeWrite: Promise<boolean> | null = null;
  let changesDuringListeningTimeRead: Record<string, number> | null = null;
  let isTagWriteSuspendingPlayback = false;
  let tagWritePlaybackGeneration = 0;
  let positionSeconds = $state(0);
  let lastConfirmedPlaybackPositionSeconds = 0;
  const playbackSeekCoordinator = new PlaybackSeekCoordinator();
  let playbackSeekLoop: Promise<void> | null = null;
  let durationSeconds = $state<number | null>(null);
  let volume = $state(1);
  let contextMenu = $state<ContextMenuState | null>(null);
  let currentLyrics = $state<TrackLyrics | null>(null);
  let lyricsEdit = $state<{ track: Track; kind: TrackLyrics["kind"]; text: string } | null>(null);
  let autoFindLyricsEnabled = $state(true);
  let selectedTheme = $state<ThemeId>(DEFAULT_THEME);
  let isLoadingLyrics = $state(false);
  let isAutoFindingLyrics = $state(false);
  let isSavingLyricsSelection = $state(false);
  let isSavingLyricsOffset = $state(false);
  let lyricsPickerResults = $state<LrclibLyricsResult[]>([]);
  let isLyricsPickerOpen = $state(false);
  let lyricsPickerReplaceCached = $state(false);
  let lyricsLookupMessage = $state<string | null>(null);
  let lyricsLookupError = $state<string | null>(null);
  let isLyricsOptionsOpen = $state(false);
  let tagEditorData = $state<TrackTagEditorData | null>(null);
  let tagEditorTrack = $state<Track | null>(null);
  let tagEditorDraft = $state<TagEditorDraft>(emptyTagEditorDraft());
  let tagEditorOriginal = $state<TagEditorDraft>(emptyTagEditorDraft());
  let isLoadingTagEditor = $state(false);
  let isSavingTagEditor = $state(false);
  let isTagEditorDiscardConfirmationOpen = $state(false);
  let tagEditorPendingTrack = $state<Track | null>(null);
  let tagEditorError = $state<string | null>(null);
  let tagEditorMessage = $state<string | null>(null);
  let albumTagEditorData = $state<AlbumTagEditorData | null>(null);
  let albumTagEditorDraft = $state<AlbumTagEditorDraft>(emptyAlbumTagEditorDraft());
  let albumTagEditorStage = $state<AlbumTagEditorStage>("edit");
  let albumTagEditorError = $state<string | null>(null);
  let albumTagUpdateResult = $state<AlbumTagUpdateResult | null>(null);
  let albumTagUpdateProgress = $state<AlbumTagUpdateProgress | null>(null);
  let albumTagMoveNotice = $state<AlbumTagMoveNotice | null>(null);
  let isAlbumTagEditorOpen = $state(false);
  let isLoadingAlbumTagEditor = $state(false);
  let isSavingAlbumTagEditor = $state(false);
  let shortcutModalElement: HTMLElement | undefined = $state();
  let deletePlaylistModalElement: HTMLElement | undefined = $state();
  let tagEditorModalElement: HTMLElement | undefined = $state();
  let tagEditorDiscardModalElement: HTMLElement | undefined = $state();
  let albumTagEditorModalElement: HTMLElement | undefined = $state();
  let lyricsPanelElement: HTMLElement | undefined = $state();
  let displayAlbums = $derived(!hasLoadedCache ? mockAlbums : buildAlbums(tracks));
  let displayArtists = $derived(!hasLoadedCache ? mockArtists : buildArtists(tracks));
  let displayGenres = $derived(!hasLoadedCache ? mockGenres : buildGenres(tracks));
  let visibleNavItems = $derived(navItems.filter((item) =>
    item.label !== "Home"
      && (ENABLE_EXPERIMENTAL_VIDEOS || item.label !== "Videos")
      && (platformCapabilities?.cdRipping !== false || item.label !== "CD Rip")
  ));
  let favoriteTracks = $derived(tracks.filter((track) => track.isFavorite));
  let availableFormats = $derived(availableTrackFormats(tracks));
  let sortedSongTracks = $derived(sortTracks(tracks, songSort, songSortDirection));
  let filteredSongTracks = $derived(filterTracksByFormat(sortedSongTracks, songFormatFilter));
  let sortedAlbums = $derived(sortAlbums(displayAlbums, albumSort, albumSortDirection));
  let sortedArtists = $derived(sortArtists(displayArtists, artistSort, artistSortDirection));
  let sortedGenres = $derived(sortGenres(displayGenres, genreSort, genreSortDirection));
  let statsRange = $derived(statsRangeForPeriod(statsPeriod, new Date(statsClock), statsCustomFrom, statsCustomTo, statsSelectedMonth, statsSelectedYear));
  let statsDailyActivity = $derived((statsUseListeningTime ? statsSnapshot?.listeningTime?.dailyListenedMs : statsSnapshot?.dailyPlays) ?? []);
  let statsSelectedDayLabel = $derived(statsDayValueLabel(statsSelectedDay, statsRange?.chartDates ?? [], statsDailyActivity, statsChartValueLabel));
  let statsTrackCounts = $derived(new Map(statsSnapshot?.trackCounts.map((item) => [item.trackId, item]) ?? []));
  let statsPendingTracks = $derived<Track[]>((statsSnapshot?.pendingTracks ?? []).map((item) => ({
    id: item.trackId, filePath: "", fileName: "Retained listening history (not in library)", extension: "history",
    title: item.title, artist: item.artist, album: item.album, albumArtist: item.albumArtist, genres: item.genres,
    trackNumber: null, discNumber: null, year: null, durationSeconds: null, modifiedTime: null,
    fileSize: null, scannedAt: null, coverArtPath: null, lyricsPath: null, lyricsKind: null, isFavorite: false,
    playCount: statsTrackCounts.get(item.trackId)?.plays ?? 0,
    listenedMs: statsSnapshot?.listeningTime?.trackTimes[item.trackId] ?? 0,
    lastPlayedAt: statsTrackCounts.get(item.trackId)?.lastPlayedAt ?? null,
  })));
  let statsUnavailableIds = $derived(new Set(statsPendingTracks.map((track) => track.id)));
  let periodTracks = $derived([...tracks.map((track) => {
    if (statsPeriod === "all") return track;
    const count = statsTrackCounts.get(track.id);
    return { ...track, playCount: count?.plays ?? 0, lastPlayedAt: count?.lastPlayedAt ?? null,
      listenedMs: statsSnapshot?.listeningTime?.trackTimes[track.id] ?? 0 };
  }), ...statsPendingTracks]);
  let allStatsTopTracks = $derived(mostPlayed(periodTracks));
  let statsTracksById = $derived(new Map(periodTracks.map((track) => [track.id, track])));
  let statsRecentListens = $derived((statsUseListeningTime ? statsSnapshot?.listeningTime?.recentSessions : statsSnapshot?.recentListens) ?? []);
  let statsRecentQueue = $derived(statsUseListeningTime
    ? [...new Set(statsRecentListens.map((listen) => listen.trackId))].flatMap((id) => {
      const track = statsTracksById.get(id);
      return track ? [track] : [];
    })
    : recentlyPlayed(periodTracks));
  let statsRecentCount = $derived((statsUseListeningTime
    ? statsSnapshot?.listeningTime?.sessionCount
    : statsPeriod === "all" ? statsSnapshot?.datedPlays : statsSnapshot?.totalPlays) ?? 0);
  let allStatsTopArtists = $derived(buildTopArtistStats(periodTracks, buildArtists(periodTracks)));
  let allStatsTopAlbums = $derived(buildTopAlbumStats(periodTracks, buildAlbums(periodTracks)));
  let allStatsTopGenres = $derived(buildTopGenreStats(periodTracks, buildGenres(periodTracks)));
  let statsTopTracks = $derived(visibleStatsItems(allStatsTopTracks, statsSectionExpanded("tracks"), statsVisibleLimits.tracks, STATS_PREVIEW_LIMITS.tracks));
  let statsTopArtists = $derived(visibleStatsItems(allStatsTopArtists, statsSectionExpanded("artists"), statsVisibleLimits.artists, STATS_PREVIEW_LIMITS.artists));
  let statsTopAlbums = $derived(visibleStatsItems(allStatsTopAlbums, statsSectionExpanded("albums"), statsVisibleLimits.albums, STATS_PREVIEW_LIMITS.albums));
  let statsTopGenres = $derived(visibleStatsItems(allStatsTopGenres, statsSectionExpanded("genres"), statsVisibleLimits.genres, STATS_PREVIEW_LIMITS.genres));
  let statsTotalPlays = $derived(statsSnapshot?.totalPlays
    ?? (statsPeriod === "all" ? tracks.reduce((total, track) => total + track.playCount, 0) : 0));
  let statsTotalListenedMs = $derived(statsSnapshot?.listeningTime?.totalListenedMs
    ?? (statsPeriod === "all" ? tracks.reduce((total, track) => total + (track.listenedMs ?? 0), 0) : 0));
  let statsDistinctTracks = $derived(periodTracks.filter((track) => statsActivity(track) > 0).length);
  let statsDistinctArtists = $derived(new Set(periodTracks.filter((track) => statsActivity(track) > 0)
    .map((track) => track.artist ?? track.albumArtist ?? "Unknown Artist")).size);
  let statsChartUnit = $derived(statsUseListeningTime ? 1000 : 1);
  let statsChartStep = $derived(Math.max(1, Math.ceil(Math.max(0, ...statsDailyActivity) / (4 * statsChartUnit))) * statsChartUnit);
  let statsActiveDays = $derived(statsDailyActivity.filter((amount) => amount > 0).length);
  let statsOverview = $derived([
    ...(showPlayCounts ? [{ label: "Plays", value: statsTotalPlays }] : []),
    ...(showListeningTime ? [{ label: "Listening time", value: listeningTotalLabel(statsTotalListenedMs, statsTotalPlays) }] : []),
    { label: "Songs played", value: statsDistinctTracks },
    { label: "Artists played", value: statsDistinctArtists },
    { label: "Active days", value: statsActiveDays },
  ]);
  let statsChartDates = $derived(statsRange?.chartDates.slice(statsChartOffset, statsChartOffset + 90) ?? []);
  let statsChartDateLabel = $derived(statsChartDates.length
    ? statsRangeFormat.formatRange(new Date(statsChartDates[0]), new Date(statsChartDates[statsChartDates.length - 1]))
    : "");
  let songFormatOptions = $derived<DropdownOption[]>(availableFormats.map((format) => ({ value: format, label: format })));
  let normalizedSearchQuery = $derived(normalizeSearch(activeView === "Songs" ? songsSearchQuery : searchQuery));
  let libraryTracksById = $derived(new Map(tracks.map((track) => [track.id, track])));
  let selectedAlbum = $derived(displayAlbums.find((album) => album.id === selectedAlbumId) ?? null);
  let selectedArtist = $derived(displayArtists.find((artist) => artist.name === selectedArtistName) ?? null);
  let selectedGenre = $derived(displayGenres.find((genre) => genre.name === selectedGenreName) ?? null);
  let isAlbumDetailView = $derived(activeView === "Albums" && Boolean(selectedAlbum));
  let isArtistDetailView = $derived(activeView === "Artists" && Boolean(selectedArtist));
  let isGenreDetailView = $derived(activeView === "Genres" && Boolean(selectedGenre));
  let selectedPlaylist = $derived(playlists.find((playlist) => playlist.id === selectedPlaylistId) ?? null);
  let isPlaylistDetailView = $derived(activeView === "Playlists" && (Boolean(selectedPlaylist) || isLikedSongsOpen));
  let selectedVideo = $derived(videos.find((video) => video.id === selectedVideoId) ?? null);
  let selectedVideoThumbnailSrc = $derived(localImageSource(selectedVideo?.thumbnailPath));
  let activeVideoStatus = $derived(videoBackendStatus?.videoId === selectedVideoId ? videoBackendStatus : null);
  let isNativeVideoPlaying = $derived(Boolean(activeVideoStatus?.isPlaying));
  let isVideoPlaybackActive = $derived(Boolean(videoBackendStatus?.videoId && !videoBackendStatus.hasEnded));
  let selectedVideoIsActive = $derived(Boolean(activeVideoStatus && !activeVideoStatus.hasEnded));
  let nativeVideoPositionSeconds = $derived(activeVideoStatus?.positionSeconds ?? selectedVideo?.lastPositionSeconds ?? 0);
  let nativeVideoDurationSeconds = $derived(activeVideoStatus?.durationSeconds ?? selectedVideo?.durationSeconds ?? videoCodecInfo?.durationSeconds ?? null);
  let nativeVideoVolume = $derived(activeVideoStatus?.volume ?? 1);
  let nativeVideoProgressPercent = $derived(progressPercent(nativeVideoPositionSeconds, nativeVideoDurationSeconds ?? selectedVideo?.durationSeconds ?? null));
  let selectedVideoPlaybackStatusLabel = $derived(videoPlaybackStatusLabel(activeVideoStatus));
  let selectedCdRelease = $derived(cdMetadataResults.find((release) => release.id === selectedCdReleaseId) ?? null);
  let selectedCdCover = $derived(activeCdRipMetadata()?.cover ?? null);
  let selectedCdCoverSrc = $derived(localImageSource(selectedCdCover?.path));
  let selectedPlaylistTracks = $derived(selectedPlaylist ? tracksForPlaylist(selectedPlaylist) : []);
  let selectedPlaylistDurationLabel = $derived(totalTrackDurationLabel(selectedPlaylistTracks));
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
  let selectedAlbumDisplayTracks = $derived(sortAlbumDisplayTracks(selectedAlbumSearchTracks, albumTrackSort, albumTrackSortDirection));
  let selectedAlbumDiscGroups = $derived(albumTrackSort === "trackNumber"
    ? albumDiscGroups(selectedAlbumDisplayTracks)
    : [{ discNumber: null, tracks: selectedAlbumDisplayTracks }]);
  let selectedAlbumIsMultiDisc = $derived(albumTrackSort === "trackNumber" && albumHasMultipleDiscs(selectedAlbumSearchTracks));
  let selectedAlbumDurationLabel = $derived(albumTotalDurationLabel(selectedAlbumTracks));
  let selectedAlbumFormatSummary = $derived(albumFormatSummary(selectedAlbumTracks));
  let selectedArtistTracks = $derived(selectedArtist ? tracksForArtist(selectedArtist) : []);
  let selectedArtistDisplayTracks = $derived(sortAlbumDisplayTracks(selectedArtistTracks, artistTrackSort, artistTrackSortDirection));
  let selectedArtistRankedTracks = $derived(topPlayedTracks(selectedArtistTracks, selectedArtistTracks.length, showPlayCounts ? "mostPlayed" : "mostListened"));
  let selectedArtistTopTracks = $derived(selectedArtistRankedTracks.slice(0, 10));
  let selectedArtistAlbums = $derived(
    selectedArtist ? sortedAlbums.filter((album) => album.artist === selectedArtist.name) : [],
  );
  let selectedArtistSearchAlbums = $derived(searchFilterAlbums(selectedArtistAlbums, normalizedSearchQuery));
  let selectedGenreTracks = $derived(selectedGenre ? tracksForGenre(selectedGenre) : []);
  let selectedGenreDisplayTracks = $derived(sortAlbumDisplayTracks(selectedGenreTracks, genreTrackSort, genreTrackSortDirection));
  let selectedGenreRankedTracks = $derived(topPlayedTracks(selectedGenreTracks, selectedGenreTracks.length, showPlayCounts ? "mostPlayed" : "mostListened"));
  let selectedGenreTopTracks = $derived(selectedGenreRankedTracks.slice(0, 10));
  let selectedGenreAlbums = $derived(selectedGenre ? albumsForTracks(selectedGenreTracks, sortedAlbums) : []);
  let selectedGenreArtists = $derived(selectedGenre ? buildArtists(selectedGenreTracks) : []);
  let selectedGenreSearchAlbums = $derived(searchFilterGenreAlbums(selectedGenreAlbums, normalizedSearchQuery));
  let selectedGenreSearchArtists = $derived(searchFilterArtists(selectedGenreArtists, normalizedSearchQuery));
  let visibleSongTracks = $derived(searchFilterTracks(filteredSongTracks, normalizedSearchQuery));
  let visibleAlbums = $derived(searchFilterAlbums(sortedAlbums, normalizedSearchQuery));
  let visibleArtists = $derived(searchFilterArtists(sortedArtists, normalizedSearchQuery));
  let visibleGenres = $derived(searchFilterGenres(sortedGenres, normalizedSearchQuery));
  let normalizedVideoSearchQuery = $derived(normalizeSearch(videoSearchQuery));
  let visibleVideos = $derived(searchFilterVideos(
    filterVideosByType(sortVideos(videos, videoSort, videoSortDirection), videoTypeFilter),
    normalizedVideoSearchQuery,
  ));
  let videoLibraryStats = $derived([
    { label: "Videos", value: String(videos.length) },
    { label: "Partially watched", value: String(videos.filter((video) => video.lastPositionSeconds > 0).length) },
    { label: "Played", value: String(videos.filter((video) => video.playCount > 0).length) },
    { label: "Last scan", value: lastVideoScannedAt ? formatDateTime(lastVideoScannedAt) : "Not available" },
  ]);
  let selectedAlbumGenres = $derived(uniqueGenresForTracks(selectedAlbumTracks));
  let selectedArtistGenres = $derived(uniqueGenresForTracks(selectedArtistTracks));
  let selectedPlaylistMissingTrackCount = $derived(selectedPlaylist ? missingTrackCountForPlaylist(selectedPlaylist) : 0);
  let canCreatePlaylist = $derived(normalizePlaylistName(playlistNameDraft).length > 0);
  let libraryDiagnostics = $derived(buildLibraryDiagnostics(tracks, displayAlbums, displayArtists));
  let libraryHealthIssueCount = $derived(libraryHealthTotalIssueCount(libraryDiagnostics));
  let librarySettingsStats = $derived([
    { label: "Tracks", value: tracks.length },
    { label: "Albums", value: displayAlbums.length },
    { label: "Artists", value: displayArtists.length },
    { label: "Genres", value: displayGenres.length },
  ]);
  let volumePercentLabel = $derived(`${Math.round(volume * 100)}%`);
  let lastScanLabel = $derived(lastScannedAt ? formatDateTime(lastScannedAt) : "Not available");
  let updateIsBusy = $derived(updateCheckState === "checking" || updateCheckState === "confirming" || updateCheckState === "downloading" || updateCheckState === "installing");
  let updateWaitsForLibrary = $derived(isScanning || isRippingCd || isImportingDvd
    || isSavingTagEditor || isSavingAlbumTagEditor || !!editingCoverForAlbum || historyBusy);
  let updatePackageLabel = $derived(
    !updateRuntime
      ? "Detecting..."
      : updateRuntime.packageKind === "appimage"
      ? "AppImage"
      : updateRuntime.packageKind === "nsis"
      ? "Windows installer"
      : updateRuntime.packageKind === "native-or-unknown"
        ? "DEB, RPM, or other"
        : updateRuntime.development
          ? "Development build"
          : "Unsupported platform",
  );
  let updateProgressLabel = $derived(
    updateDownloadProgress?.percent !== null && updateDownloadProgress?.percent !== undefined
      ? `${updateDownloadProgress.percent}%`
      : updateDownloadProgress
        ? `${formatByteCount(updateDownloadProgress.downloadedBytes)} downloaded`
        : "Preparing download",
  );
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
  let playableQueueOrder = $derived(playbackOrder.filter((index) => index === currentQueueIndex || isQueueEligible(playbackQueue[index])));
  let queueLengthLabel = $derived(`${playableQueueOrder.length} ${playableQueueOrder.length === 1 ? "track" : "tracks"}`);
  let currentOrderIndex = $derived(queueOrderIndex(playableQueueOrder, currentQueueIndex));
  let queuePanelEntries = $derived(buildQueuePanelEntries(playbackQueue, playableQueueOrder, currentQueueIndex));
  let currentTrackGenres = $derived(currentTrack ? trackGenres(currentTrack).filter((genre) => genre !== "Unknown Genre") : []);
  let currentTrackCoverArtPath = $derived(currentTrack ? trackCoverPath(currentTrack) : null);
  let currentTrackCoverArtSrc = $derived(localImageSource(currentTrackCoverArtPath));
  let currentTrackDuration = $derived(durationSeconds ?? currentTrack?.durationSeconds ?? null);
  let syncedLyricLines = $derived(currentLyrics?.kind === "synced" ? parseLrcLyrics(currentLyrics.text) : []);
  let plainLyricStanzas = $derived(
    currentLyrics?.kind === "plain" && currentLyrics.text.trim()
      ? currentLyrics.text.replace(/\r\n?/g, "\n").trim().split(/\n(?:[^\S\n]*\n)+/)
      : [],
  );
  let lyricsOffsetSeconds = $derived(currentLyrics?.offsetSeconds ?? 0);
  let syncedLyricsState = $derived(resolveSyncedLyricsState(syncedLyricLines, positionSeconds, lyricsOffsetSeconds));
  let hasLyricsIntro = $derived(hasSyncedLyricsIntro(syncedLyricLines, lyricsOffsetSeconds));
  let activeLyricsScrollKey = $derived(
    syncedLyricsState.kind === "intro"
      ? "intro"
      : syncedLyricsState.activeCueIndex >= 0
        ? `cue-${syncedLyricsState.activeCueIndex}`
        : null,
  );
  let lyricsBadgeLabel = $derived(currentLyrics ? lyricsStatusText(currentLyrics) : null);
  let cachedLyricsLabel = $derived(currentLyrics?.source === "lrclib" ? cachedLyricsStatus(currentLyrics) : null);
  let canPlayPrevious = $derived(
    currentQueueIndex !== null
      && playableQueueOrder.length > 1
      && (currentOrderIndex > 0 || repeatMode === "all"),
  );
  let canPlayNext = $derived(
    currentQueueIndex !== null
      && playableQueueOrder.length > 1
      && (currentOrderIndex < playableQueueOrder.length - 1 || repeatMode === "all"),
  );
  let hadSearchQuery = false;
  let lyricsRequestId = 0;
  let tagEditorRequestId = 0;
  let statsRequestId = 0;
  let statsLastRangeKey: string | null = null;

  $effect(() => {
    void statsPeriod;
    void statsCustomFrom;
    void statsCustomTo;
    void statsSelectedMonth;
    void statsSelectedYear;
    statsChartOffset = 0;
    statsSelectedDay = null;
  });

  $effect(() => {
    const requestId = ++statsRequestId;
    const range = statsRange;
    const revision = statsRevision;
    const ready = hasLoadedCache;
    if (activeView !== "Stats" || !ready || !range) {
      statsSnapshot = null;
      statsLoading = false;
      return;
    }
    void revision;
    const rangeKey = `${range.startUtc}:${range.endUtc}:${range.dayBoundariesUtc[0]}:${range.dayBoundariesUtc.at(-1)}`;
    if (rangeKey !== statsLastRangeKey) statsSnapshot = null;
    statsLastRangeKey = rangeKey;
    statsLoading = true;
    statsError = null;
    const recentLimit = statsSectionExpanded("recent") ? statsVisibleLimits.recent : STATS_PREVIEW_LIMITS.recent;
    void getListeningStats(range.startUtc, range.endUtc, range.dayBoundariesUtc, recentLimit)
      .then((result) => {
        if (requestId === statsRequestId) statsSnapshot = result;
      })
      .catch((error) => {
        if (requestId === statsRequestId) statsError = error instanceof Error ? error.message : String(error);
      })
      .finally(() => {
        if (requestId === statsRequestId) statsLoading = false;
      });
  });

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
    if (!tagEditorTrack) {
      return;
    }

    window.requestAnimationFrame(() => {
      tagEditorModalElement?.focus();
    });
  });

  $effect(() => {
    if (!isTagEditorDiscardConfirmationOpen) {
      return;
    }

    window.requestAnimationFrame(() => {
      tagEditorDiscardModalElement?.focus();
    });
  });

  $effect(() => {
    if (!isAlbumTagEditorOpen) {
      return;
    }

    window.requestAnimationFrame(() => {
      albumTagEditorModalElement?.focus();
    });
  });

  function readPreference(key: string): string | null {
    try {
      return window.localStorage.getItem(key);
    } catch {
      return null;
    }
  }

  function writePreference(key: string, value: string) {
    try {
      window.localStorage.setItem(key, value);
    } catch {
      // A denied or full store does not prevent using the current selection.
    }
  }

  function removePreference(key: string) {
    try {
      window.localStorage.removeItem(key);
    } catch {
      // Storage can be unavailable even if the rest of the interface works.
    }
  }

  function isThemeId(value: string | null): value is ThemeId {
    return themePresets.some((theme) => theme.id === value);
  }

  function savedThemeOrDefault(value: string | null): ThemeId {
    return isThemeId(value) ? value : DEFAULT_THEME;
  }

  function applyTheme(theme: ThemeId) {
    if (typeof document !== "undefined") {
      document.documentElement.dataset.theme = theme;
    }
  }

  function handleThemeSelect(theme: ThemeId) {
    selectedTheme = theme;
    applyTheme(theme);
    writePreference(THEME_SETTING_KEY, theme);
  }

  function initializeSavedTheme() {
    if (typeof window === "undefined") {
      return;
    }

    const storedTheme = readPreference(THEME_SETTING_KEY);
    selectedTheme = savedThemeOrDefault(storedTheme);
    applyTheme(selectedTheme);
    if (storedTheme !== selectedTheme) writePreference(THEME_SETTING_KEY, selectedTheme);
  }

  // Normalize hidden legacy selections before the component is mounted, so
  // Rose Noir or Royal Gold is never applied during startup.
  initializeSavedTheme();

  $effect(() => {
    if (isWindows && isTauri()) {
      const theme = selectedTheme === "light" ? "light" : "dark";
      void applyWindowsTheme(theme);
    }
  });

  async function applyWindowsTheme(theme: "light" | "dark") {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      if (theme === (selectedTheme === "light" ? "light" : "dark")) {
        await getCurrentWindow().setTheme(theme);
      }
    } catch (error) {
      console.warn("Could not apply the Windows title bar theme:", error);
    }
  }

  $effect(() => {
    void loadCurrentTrackLyrics(currentTrack?.filePath ?? null);
  });

  $effect(() => {
    if (activeView === "Home" || (!ENABLE_EXPERIMENTAL_VIDEOS && activeView === "Videos")) {
      activeView = "Albums";
      selectedVideoId = null;
      isEditingVideo = false;
    }
  });

  $effect(() => {
    const location = currentViewLocation();
    const [previousHistory, nextHistory] = untrack(() => [
      viewHistory,
      visitView(viewHistory, location, sameViewLocation),
    ]);
    if (nextHistory !== previousHistory) viewHistory = nextHistory;
  });

  $effect(() => {
    if (!activeLyricsScrollKey || !lyricsPanelElement) {
      return;
    }

    const activeLine = lyricsPanelElement.querySelector<HTMLElement>("[data-active='true']");
    const prefersReducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    activeLine?.scrollIntoView({ block: "center", behavior: prefersReducedMotion ? "auto" : "smooth" });
  });

  $effect(() => {
    if (!availableFormats.includes(songFormatFilter)) {
      songFormatFilter = "All";
    }
  });

  onMount(() => {
    const refreshStatsCalendar = () => {
      const now = Date.now();
      if (new Date(now).toDateString() !== new Date(statsClock).toDateString()) statsClock = now;
    };
    const statsCalendarIntervalId = window.setInterval(refreshStatsCalendar, 60_000);
    window.addEventListener("focus", refreshStatsCalendar);
    autoFindLyricsEnabled = readPreference(AUTO_LYRICS_SETTING_KEY) !== "off";
    const storedListenPercentage = Number(readPreference("cassette:listen-percentage"));
    const activityDisplayPreference = readPreference("cassette:listening-time");
    listeningActivityDisplay = activityDisplayPreference === "1" || activityDisplayPreference === "2" ? activityDisplayPreference : "0";
    if (listenPercentages.includes(storedListenPercentage)) listenPercentage = storedListenPercentage;
    trackNotificationsEnabled = readPreference(TRACK_NOTIFICATIONS_SETTING_KEY) === "on";
    gaplessEnabled = readPreference("cassette:gapless") !== "off";
    showAlbumPlayCounts = readPreference(ALBUM_PLAY_COUNTS_SETTING_KEY) !== "off";
    showArtistPlayCounts = readPreference(ARTIST_PLAY_COUNTS_SETTING_KEY) !== "off";
    showGenrePlayCounts = readPreference(GENRE_PLAY_COUNTS_SETTING_KEY) !== "off";
    showAlbumTrackPlayCounts = readPreference(ALBUM_TRACK_PLAY_COUNTS_SETTING_KEY) !== "off";
    showSongListNumbers = readPreference(SONG_LIST_NUMBERS_SETTING_KEY) !== "off";
    try {
      const updaterStorage = loadUpdaterStorage(window.localStorage);
      automaticUpdateChecksEnabled = updaterStorage.automaticChecksEnabled;
      lastSuccessfulUpdateCheck = updaterStorage.lastSuccessfulCheck;
      lastAutomaticUpdateAttempt = updaterStorage.lastAutomaticAttempt;
      updaterStorageAvailable = updaterStorage.storageAvailable;
    } catch {
      // Accessing the browser's Storage object itself can be denied. Manual
      // checks remain available once native runtime detection completes.
      automaticUpdateChecksEnabled = false;
      lastSuccessfulUpdateCheck = null;
      lastAutomaticUpdateAttempt = null;
      updaterStorageAvailable = false;
    }
    void initializeUpdater();
    void loadPlatformCapabilities();
    void loadLibraryCache();
    if (ENABLE_EXPERIMENTAL_VIDEOS) {
      void loadVideoLibrary();
    }
    const mediaControlUnlisteners = [
      listen("media-play-pause", () => void handleTogglePlayback(), { target: "main" }),
      listen("media-play", () => void handleMediaPlay(), { target: "main" }),
      listen("media-pause", () => void handleMediaPause(), { target: "main" }),
      listen("media-stop", () => void handleMediaStop(), { target: "main" }),
      listen("media-next", () => void handleNextTrack(), { target: "main" }),
      listen("media-previous", () => void handlePreviousTrack(), { target: "main" }),
      listen<number>("media-seek", (event) => void handleSeek(event.payload), { target: "main" }),
      listen<CdRipEvent>("cd-rip-started", (event) => handleCdRipStarted(event.payload)),
      listen<CdRipEvent>("cd-rip-track-started", (event) => handleCdRipTrackStarted(event.payload)),
      listen<CdRipEvent>("cd-rip-track-finished", (event) => handleCdRipTrackFinished(event.payload)),
      listen<CdRipEvent>("cd-rip-track-error", (event) => handleCdRipTrackError(event.payload)),
      listen<CdRipEvent>("cd-rip-finished", (event) => handleCdRipFinished(event.payload)),
      listen<DvdImportEvent>("dvd-import-started", (event) => handleDvdImportEvent(event.payload, "Importing...")),
      listen<DvdImportEvent>("dvd-import-title-started", (event) => handleDvdImportEvent(event.payload, "Importing...")),
      listen<DvdImportEvent>("dvd-import-progress", (event) => handleDvdImportEvent(event.payload, "Importing...")),
      listen<DvdImportEvent>("dvd-import-finished", (event) => handleDvdImportEvent(event.payload, "Import complete")),
      listen<DvdImportEvent>("dvd-import-error", (event) => handleDvdImportEvent(event.payload, "Error")),
      listen<AlbumTagUpdateProgress>("album-tag-update-progress", (event) => {
        if (isSavingAlbumTagEditor) {
          albumTagUpdateProgress = event.payload;
        }
      }),
    ];

    const statusIntervalId = window.setInterval(async () => {
      if (isStartingTrack || isChangingPlayback || isPollingPlayback || isTagWriteSuspendingPlayback || !currentTrack || !isPlaying) {
        return;
      }

      const statusTrackPath = currentTrack.filePath;
      const startId = playbackStartId;
      const controlRevision = playbackControlRevision;
      const tagWriteGeneration = tagWritePlaybackGeneration;
      const positionSnapshot = playbackSeekCoordinator.capturePositionUpdate(statusTrackPath);
      isPollingPlayback = true;

      try {
        const requestedAt = performance.now();
        const status = await getPlaybackStatus();

        if (isStartingTrack || startId !== playbackStartId || controlRevision !== playbackControlRevision || isTagWriteSuspendingPlayback
          || tagWriteGeneration !== tagWritePlaybackGeneration || currentTrack?.filePath !== statusTrackPath
          || !playbackSeekCoordinator.allowsPositionUpdate(positionSnapshot)) {
          return;
        }
        if (status.filePath !== statusTrackPath) {
          await acceptGaplessTransition(status);
          return;
        }

        applyPlaybackStatus(status, { requestedAt });
        await handlePlaybackStatusUpdate(status);
      } catch (error) {
        if (!isStartingTrack && startId === playbackStartId && controlRevision === playbackControlRevision && !isTagWriteSuspendingPlayback
          && tagWriteGeneration === tagWritePlaybackGeneration) {
          playbackError = error instanceof Error ? error.message : String(error);
        }
      } finally {
        isPollingPlayback = false;
      }
    }, 250);

    const listeningTimeIntervalId = window.setInterval(() => void flushListeningTime(), 1000);
    const flushBeforeHide = () => { void flushListeningTime(); };
    window.addEventListener("pagehide", flushBeforeHide);

    const videoProgressIntervalId = ENABLE_EXPERIMENTAL_VIDEOS
      ? window.setInterval(() => {
          void refreshNativeVideoStatus(false);
        }, 1000)
      : null;

    function handleKeydown(event: KeyboardEvent) {
      if (artworkViewer || lyricsEdit) {
        // The viewer handles Escape/Tab. Do not run page shortcuts behind it.
        return;
      }
      if (isAlbumTagEditorOpen) {
        if (event.key === "Escape" && !isSavingAlbumTagEditor) {
          event.preventDefault();
          event.stopPropagation();
          closeAlbumTagEditor();
        }

        return;
      }

      if (tagEditorTrack) {
        if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          if (isTagEditorDiscardConfirmationOpen) {
            keepEditingTagEditor();
          } else {
            requestCloseTagEditor();
          }
        }

        return;
      }

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

      if (event.key === "Escape" && isQueueOpen) {
        event.preventDefault();
        event.stopPropagation();
        isQueueOpen = false;
        return;
      }
      if (event.key === "Escape" && isAlbumSelectingTracks && !contextMenu) {
        event.preventDefault();
        clearAlbumTrackSelection();
        return;
      }

      if (shouldIgnoreShortcut(event.target)
        || (isAlbumSelectingTracks && event.code === "Space" && event.target instanceof HTMLElement
          && event.target.classList.contains("album-track-row"))) {
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
      window.clearInterval(statsCalendarIntervalId);
      window.removeEventListener("focus", refreshStatsCalendar);
      window.clearInterval(statusIntervalId);
      window.clearInterval(listeningTimeIntervalId);
      window.removeEventListener("pagehide", flushBeforeHide);
      if (videoProgressIntervalId !== null) {
        window.clearInterval(videoProgressIntervalId);
      }
      window.removeEventListener("keydown", handleKeydown, true);
      for (const unlisten of mediaControlUnlisteners) {
        void unlisten.then((cleanup) => cleanup());
      }
    };
  });

  async function loadPlatformCapabilities() {
    try {
      platformCapabilities = await getPlatformCapabilities();

      if (!platformCapabilities.linuxNotifications) {
        trackNotificationsEnabled = false;
        writePreference(TRACK_NOTIFICATIONS_SETTING_KEY, "off");
      }
    } catch {
      // Older backends do not expose capabilities. Keep the established Linux UI.
    }
  }

  async function loadLibraryCache() {
    try {
      const cache = await getLibraryCache();
      libraryRecoveryNotice = cache.recoveryNotice ?? null;
      libraryTracks = cache.tracks;
      void reloadListeningTime();
      libraryFolders = cache.folders ?? [];
      unavailableTrackIds = new Set(cache.unavailableTrackIds ?? []);
      ignoredTrackIds = new Set(cache.ignoredTrackIds ?? []);
      customAlbumCovers = cache.albumCovers ?? {};
      playlists = cache.playlists;
      scannedFolder = cache.lastScannedFolder;
      lastScannedAt = cache.lastScannedAt;
      scanCount = cache.tracks.length;
      hasLoadedCache = true;
      statsRevision += 1;
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

  async function reloadListeningTime() {
    const changes: Record<string, number> = Object.create(null);
    changesDuringListeningTimeRead = changes;
    try {
      const totals = await getListeningTimeByTrack();
      if (changesDuringListeningTimeRead === changes) listeningTimeByTrack = { ...totals, ...changes };
    } catch (error) {
      console.warn("Could not read listening time", error);
    } finally {
      if (changesDuringListeningTimeRead === changes) changesDuringListeningTimeRead = null;
    }
  }

  function applyListeningTimeTotals(totals: Record<string, number>) {
    if (changesDuringListeningTimeRead) Object.assign(changesDuringListeningTimeRead, totals);
    listeningTimeByTrack = { ...listeningTimeByTrack, ...totals };
  }

  function flushListeningTime(batch = listeningTimeBuffer.batch()): Promise<boolean> {
    if (listeningTimeWrite) return listeningTimeWrite;
    if (removingListenId) return Promise.resolve(false);
    if (batch.length === 0) return Promise.resolve(true);
    listeningTimeWrite = saveListeningMinutes(batch)
      .then((totals) => {
        listeningTimeBuffer.saved(batch);
        applyListeningTimeTotals(totals);
        if (activeView === "Stats" && showListeningTime) {
          statsClock = Date.now();
          statsRevision += 1;
        }
        return true;
      })
      .catch((error) => {
        console.warn("Could not save listening time", error);
        return false;
      })
      .finally(() => { listeningTimeWrite = null; });
    return listeningTimeWrite;
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
      lyricsPickerResults = [];
      isLyricsPickerOpen = false;
      isLyricsOptionsOpen = false;
      return;
    }

    isLoadingLyrics = true;
    currentLyrics = null;
    isAutoFindingLyrics = false;
    lyricsLookupMessage = null;
    lyricsLookupError = null;
    lyricsPickerResults = [];
    isLyricsPickerOpen = false;
    isLyricsOptionsOpen = false;

    try {
      const lyrics = await readTrackLyrics(trackPath);

      if (requestId === lyricsRequestId) {
        currentLyrics = lyrics;
        if (!lyrics) {
          void handleAutoFindLyrics({ mode: "auto", requestId });
        }
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

  async function handleAutoFindLyrics(options: {
    mode: "auto" | "manual";
    replaceCached?: boolean;
    requestId?: number;
  }) {
    const track = currentTrack;
    const requestId = options.requestId ?? lyricsRequestId;
    const isManual = options.mode === "manual";
    const replaceCached = options.replaceCached ?? false;

    if (!track || isAutoFindingLyrics) {
      return;
    }

    if (!canSearchLyricsForTrack(track)) {
      if (isManual) {
        lyricsLookupMessage = null;
        lyricsLookupError = "Title and artist metadata are required before searching lyrics.";
      }
      return;
    }

    if (!isManual) {
      if (
        !autoFindLyricsEnabled
        || currentLyrics
        || isAutoLyricsCoolingDown(track)
        || (options.requestId !== undefined && options.requestId !== lyricsRequestId)
      ) {
        return;
      }
    } else {
      clearAutoLyricsCooldown(track);
    }

    isAutoFindingLyrics = true;
    lyricsPickerReplaceCached = replaceCached;
    lyricsPickerResults = [];
    isLyricsPickerOpen = false;
    lyricsLookupMessage = "Searching lyrics...";
    lyricsLookupError = null;

    try {
      if (isManual) {
        const results = await searchTrackLyricsResults(track.filePath);

        if (!lyricsRequestIsCurrent(track.filePath, requestId)) {
          return;
        }

        lyricsPickerResults = results;
        isLyricsPickerOpen = results.length > 0;
        isLyricsOptionsOpen = true;
        lyricsLookupMessage = results.length > 0 ? "Select lyrics result." : "No lyrics found";
        return;
      }

      const result = await autoFindTrackLyrics(track.filePath, replaceCached);

      if (!lyricsRequestIsCurrent(track.filePath, requestId)) {
        return;
      }

      if (result.lyrics) {
        currentLyrics = result.lyrics;
        lyricsLookupMessage = null;
        clearAutoLyricsCooldown(track);
      } else if (result.status === "not_found") {
        lyricsLookupMessage = "No lyrics found";
        setAutoLyricsCooldown(track, "not_found");
      } else {
        lyricsLookupMessage = null;
      }
    } catch (error) {
      if (!lyricsRequestIsCurrent(track.filePath, requestId)) return;
      const message = error instanceof Error ? error.message : "Could not search LRCLIB. Check your connection.";

      if (isManual) {
        lyricsLookupMessage = null;
        lyricsLookupError = message;
      } else {
        lyricsLookupMessage = "Offline / lookup failed";
        lyricsLookupError = null;
        setAutoLyricsCooldown(track, "failed", message);
      }
    } finally {
      if (lyricsRequestIsCurrent(track.filePath, requestId)) isAutoFindingLyrics = false;
    }
  }

  function canSearchLyricsForTrack(track: Track) {
    const artist = track.artist ?? track.albumArtist;

    return Boolean(
      track.title.trim()
      && artist?.trim()
      && !textMissingOrUnknown(artist, "Unknown Artist"),
    );
  }

  function autoLyricsCooldownKey(track: Track) {
    return `cassette:auto-lyrics:${hashString(track.filePath)}`;
  }

  function isAutoLyricsCoolingDown(track: Track) {
    const stored = readPreference(autoLyricsCooldownKey(track));

    if (!stored) {
      return false;
    }

    try {
      const cooldown = JSON.parse(stored) as { attemptedAt?: number };
      const attemptedAt = typeof cooldown.attemptedAt === "number" ? cooldown.attemptedAt : 0;

      if (Date.now() - attemptedAt < AUTO_LYRICS_COOLDOWN_MS) {
        return true;
      }
    } catch {
      // Ignore unreadable cooldown entries and replace them on the next failed lookup.
    }

    removePreference(autoLyricsCooldownKey(track));
    return false;
  }

  function setAutoLyricsCooldown(track: Track, status: "not_found" | "failed", message: string | null = null) {
    writePreference(autoLyricsCooldownKey(track), JSON.stringify({
      status,
      message,
      attemptedAt: Date.now(),
    }));
  }

  function clearAutoLyricsCooldown(track: Track) {
    removePreference(autoLyricsCooldownKey(track));
  }

  function hashString(value: string) {
    let hash = 2166136261;

    for (let index = 0; index < value.length; index += 1) {
      hash ^= value.charCodeAt(index);
      hash = Math.imul(hash, 16777619);
    }

    return (hash >>> 0).toString(16);
  }

  function handleAutoFindLyricsSettingChange(event: Event) {
    autoFindLyricsEnabled = event.currentTarget instanceof HTMLInputElement
      ? event.currentTarget.checked
      : autoFindLyricsEnabled;
    writePreference(AUTO_LYRICS_SETTING_KEY, autoFindLyricsEnabled ? "on" : "off");
  }

  function handleTrackNotificationsSettingChange(event: Event) {
    trackNotificationsEnabled = event.currentTarget instanceof HTMLInputElement
      ? event.currentTarget.checked
      : trackNotificationsEnabled;
    trackNotificationStatus = null;
    trackNotificationStatusIsError = false;
    writePreference(TRACK_NOTIFICATIONS_SETTING_KEY, trackNotificationsEnabled ? "on" : "off");
  }

  function updatePlayCountVisibility(
    event: Event,
    setting: "albums" | "artists" | "genres" | "albumTracks",
  ) {
    if (!(event.currentTarget instanceof HTMLInputElement)) {
      return;
    }

    const enabled = event.currentTarget.checked;

    if (setting === "albums") {
      showAlbumPlayCounts = enabled;
      writePreference(ALBUM_PLAY_COUNTS_SETTING_KEY, enabled ? "on" : "off");
    } else if (setting === "artists") {
      showArtistPlayCounts = enabled;
      writePreference(ARTIST_PLAY_COUNTS_SETTING_KEY, enabled ? "on" : "off");
    } else if (setting === "genres") {
      showGenrePlayCounts = enabled;
      writePreference(GENRE_PLAY_COUNTS_SETTING_KEY, enabled ? "on" : "off");
    } else {
      showAlbumTrackPlayCounts = enabled;
      writePreference(ALBUM_TRACK_PLAY_COUNTS_SETTING_KEY, enabled ? "on" : "off");
    }
  }

  function handleSongListNumbersSettingChange(event: Event) {
    if (!(event.currentTarget instanceof HTMLInputElement)) return;
    showSongListNumbers = event.currentTarget.checked;
    writePreference(SONG_LIST_NUMBERS_SETTING_KEY, showSongListNumbers ? "on" : "off");
  }

  function handleAutomaticUpdateSettingChange(event: Event) {
    automaticUpdateChecksEnabled = event.currentTarget instanceof HTMLInputElement
      ? event.currentTarget.checked
      : automaticUpdateChecksEnabled;
    if (!persistUpdaterValue(
      AUTOMATIC_UPDATE_SETTING_KEY,
      automaticUpdateChecksEnabled ? "on" : "off",
    )) {
      updaterStorageAvailable = false;
      automaticUpdateChecksEnabled = false;
    }

    if (
      automaticUpdateChecksEnabled
      && updateRuntime
      && shouldRunAutomaticUpdateCheck(true, lastAutomaticUpdateAttempt)
    ) {
      void runUpdateCheck("automatic");
    }
  }

  function persistUpdaterValue(key: string, value: string | null): boolean {
    try {
      return persistUpdaterStorage(window.localStorage, key, value);
    } catch {
      return false;
    }
  }

  async function initializeUpdater() {
    try {
      updateRuntime = await getUpdateRuntimeInfo();
    } catch (error) {
      updateRuntime = {
        development: true,
        platform: "development",
        packageKind: "unsupported",
        canSelfInstall: false,
        updaterAvailable: false,
      };
      console.warn("Could not determine updater runtime:", safeUpdateError(error));
      return;
    }

    if (!updateRuntime.development && updateRuntime.packageKind === "appimage") {
      try {
        appImageMenuStatus = await getAppImageMenuStatus();
      } catch (error) {
        appImageMenuIsError = true;
        appImageMenuMessage = `Could not inspect the applications-menu launcher: ${safeUpdateError(error)}`;
      }
    }

    if (
      !updateRuntime.development
      && updateRuntime.updaterAvailable
      && shouldRunAutomaticUpdateCheck(automaticUpdateChecksEnabled, lastAutomaticUpdateAttempt)
    ) {
      await runUpdateCheck("automatic");
    }
  }

  function unavailableUpdateMessage(runtime: UpdateRuntimeInfo | null) {
    if (!runtime || runtime.development) {
      return "Update checks are unavailable in development builds. No update server was contacted.";
    }

    if (!runtime.updaterAvailable) {
      return "Updater configuration is unavailable in this build or session. Cassette remains usable; downloads are available on GitHub Releases.";
    }

    return null;
  }

  async function runUpdateCheck(kind: "manual" | "automatic") {
    if (updateIsBusy) {
      return;
    }

    if (kind === "automatic") {
      if (automaticUpdateCheckAttemptedThisSession || !updaterStorageAvailable
        || !shouldRunAutomaticUpdateCheck(automaticUpdateChecksEnabled, lastAutomaticUpdateAttempt)) {
        return;
      }
      automaticUpdateCheckAttemptedThisSession = true;
      lastAutomaticUpdateAttempt = Date.now();
      if (!persistUpdaterValue(
        LAST_AUTOMATIC_UPDATE_ATTEMPT_KEY,
        String(lastAutomaticUpdateAttempt),
      )) {
        updaterStorageAvailable = false;
        automaticUpdateChecksEnabled = false;
        return;
      }
    }

    const unavailable = unavailableUpdateMessage(updateRuntime);
    if (unavailable) {
      if (kind === "manual") {
        updateCheckState = "error";
        updateStatusIsError = false;
        updateStatusMessage = unavailable;
      }
      return;
    }

    const runtime = updateRuntime;
    if (!runtime) {
      return;
    }

    const previousState = availableUpdate ? "available" : "idle";
    updateCheckState = "checking";
    updateStatusIsError = false;
    updateStatusMessage = kind === "manual" ? "Checking the signed beta channel..." : null;

    try {
      const nextUpdate = await checkForCassetteUpdate(runtime);
      const checkedAt = Date.now();
      lastSuccessfulUpdateCheck = checkedAt;
      if (!persistUpdaterValue(LAST_SUCCESSFUL_UPDATE_CHECK_KEY, String(checkedAt))) {
        updaterStorageAvailable = false;
        automaticUpdateChecksEnabled = false;
      }

      if (!nextUpdate) {
        availableUpdate = null;
        availableUpdateVersion = null;
        availableUpdateNotes = null;
        showUpdateNotice = false;
        updateCheckState = kind === "manual" ? "current" : "idle";
        updateStatusMessage = kind === "manual" ? `Cassette ${appVersion} is up to date.` : null;
        return;
      }

      availableUpdate = nextUpdate;
      availableUpdateVersion = nextUpdate.version;
      availableUpdateNotes = nextUpdate.body?.trim() || "No release notes were provided.";
      updateCheckState = "available";
      updateStatusMessage = `Cassette ${nextUpdate.version} is available.`;
      showUpdateNotice = kind === "automatic";
    } catch (error) {
      if (kind === "automatic") {
        updateCheckState = previousState;
        updateStatusMessage = null;
        console.warn("Automatic update check failed:", safeUpdateError(error));
      } else {
        updateCheckState = "error";
        updateStatusIsError = true;
        updateStatusMessage = `Could not check for updates: ${safeUpdateError(error)} You can try again or use GitHub Releases.`;
      }
    }
  }

  async function handleManualUpdateCheck() {
    await runUpdateCheck("manual");
  }

  async function handleAddAppImageToMenu() {
    if (appImageMenuBusy || updateIsBusy || updateRuntime?.packageKind !== "appimage") return;
    appImageMenuBusy = true;
    appImageMenuMessage = null;
    try {
      appImageMenuStatus = await addAppImageToMenu();
      if (!appImageMenuStatus.installed || appImageMenuStatus.needsRefresh) {
        throw new Error("The launcher or icon is not ready; please retry.");
      }
      appImageMenuIsError = false;
      appImageMenuMessage = "Cassette's AppImage launcher and icon are ready in your applications menu.";
    } catch (error) {
      appImageMenuIsError = true;
      appImageMenuMessage = `Could not add the applications-menu launcher: ${safeUpdateError(error)}`;
    } finally {
      appImageMenuBusy = false;
    }
  }

  async function handleRemoveAppImageFromMenu() {
    if (appImageMenuBusy || updateIsBusy || updateRuntime?.packageKind !== "appimage") return;
    appImageMenuBusy = true;
    appImageMenuMessage = null;
    try {
      appImageMenuStatus = await removeAppImageFromMenu();
      if (appImageMenuStatus.installed) {
        throw new Error("The launcher is still installed; please retry.");
      }
      appImageMenuIsError = false;
      appImageMenuMessage = "Cassette's managed AppImage launcher and icon were removed.";
    } catch (error) {
      appImageMenuIsError = true;
      appImageMenuMessage = `Could not remove the applications-menu launcher: ${safeUpdateError(error)}`;
    } finally {
      appImageMenuBusy = false;
    }
  }

  async function handleViewUpdateDownload() {
    const downloadUrl = availableUpdateVersion
      ? updateReleaseUrl(availableUpdateVersion, updateRuntime?.platform)
      : "https://github.com/atilla-m/cassette/releases";
    if (!downloadUrl) {
      return;
    }

    try {
      await openUrl(downloadUrl);
    } catch (error) {
      updateCheckState = "error";
      updateStatusIsError = true;
      updateStatusMessage = `Could not open the release page: ${safeUpdateError(error)}`;
    }
  }

  async function handleInstallCassetteUpdate() {
    if (updateIsBusy || updateWaitsForLibrary || !availableUpdate || !availableUpdateVersion || !updateRuntime?.canSelfInstall) {
      return;
    }

    // Bind consent to the exact opaque native operation visible when the
    // dialog opened. A later same-version check must require new consent.
    const confirmedUpdate = availableUpdate;
    const confirmedVersion = availableUpdateVersion;
    let confirmed = false;
    updateCheckState = "confirming";
    try {
      confirmed = await ask(
        `Download and verify Cassette ${confirmedVersion}, install the update, and restart Cassette?`,
        {
          title: "Update Cassette",
          kind: "info",
          okLabel: "Update and restart",
          cancelLabel: "Later",
        },
      );
    } catch (error) {
      updateCheckState = "error";
      updateStatusIsError = true;
      updateStatusMessage = `Could not open update confirmation: ${safeUpdateError(error)}`;
      return;
    }

    if (!confirmed || updateWaitsForLibrary) {
      updateCheckState = "available";
      showUpdateNotice = false;
      return;
    }

    updateCheckState = "downloading";
    updateStatusIsError = false;
    updateStatusMessage = "Downloading and verifying the signed update...";
    updateDownloadProgress = { downloadedBytes: 0, totalBytes: null, percent: null, installing: false };

    try {
      await installCassetteUpdate(confirmedUpdate, (progress) => {
        updateDownloadProgress = progress;
        if (progress.installing) {
          updateCheckState = "installing";
          updateStatusMessage = "Installing the verified update and restarting Cassette...";
        }
      });
    } catch (error) {
      updateCheckState = "error";
      updateStatusIsError = true;
      updateStatusMessage = `Update failed: ${safeUpdateError(error)} Your library data is separate from the application. If installation completed but restart failed, reopen Cassette.`;
      updateDownloadProgress = null;
    }
  }

  function handleUpdateLater() {
    showUpdateNotice = false;
    updateStatusIsError = false;
    updateStatusMessage = availableUpdateVersion
      ? `Cassette ${availableUpdateVersion} remains available when you are ready.`
      : null;
  }

  function handleReviewAvailableUpdate() {
    showUpdateNotice = false;
    activeView = "Settings";
    isLibraryHealthOpen = false;
    requestAnimationFrame(() => jumpToSetting("settings-updates-title"));
  }

  function jumpToSetting(headingId: string) {
    const heading = document.getElementById(headingId);
    heading?.focus({ preventScroll: true });
    heading?.closest("section")?.scrollIntoView({ block: "start" });
  }

  function safeUpdateError(error: unknown) {
    const raw = error instanceof Error ? error.message : String(error);
    const normalized = raw.replace(/[\r\n]+/g, " ").trim();

    if (/endpoint|pubkey|public key|configuration|plugin:updater|updater.*not found/i.test(normalized)) {
      return "The signed update service is not configured yet.";
    }

    return normalized || "Unknown updater error.";
  }

  async function handleTestTrackNotification() {
    if (!trackNotificationsEnabled) {
      return;
    }

    trackNotificationStatus = null;
    trackNotificationStatusIsError = false;

    const result = await sendLinuxPlaybackNotification("Cassette", "Notifications are working.");

    if (result.ok) {
      trackNotificationStatus = "Notification sent.";
      return;
    }

    trackNotificationStatusIsError = true;
    trackNotificationStatus = result.unavailable
      ? "Desktop notification service is unavailable."
      : `Notification failed: ${safeNotificationError(result.error)}`;
  }

  async function handleSelectLyricsResult(result: LrclibLyricsResult) {
    if (!currentTrack || isSavingLyricsSelection) {
      return;
    }

    const track = currentTrack;
    const requestId = lyricsRequestId;
    isSavingLyricsSelection = true;
    lyricsLookupError = null;
    lyricsLookupMessage = "Saving selected lyrics...";

    try {
      const lyrics = await saveTrackLyricsResult(track.filePath, result, true);
      if (!lyricsRequestIsCurrent(track.filePath, requestId)) return;
      currentLyrics = lyrics;
      clearAutoLyricsCooldown(track);
      lyricsPickerResults = [];
      isLyricsPickerOpen = false;
      isLyricsOptionsOpen = false;
      lyricsLookupMessage = null;
    } catch (error) {
      if (!lyricsRequestIsCurrent(track.filePath, requestId)) return;
      lyricsLookupMessage = null;
      lyricsLookupError = error instanceof Error ? error.message : "Could not save selected lyrics.";
    } finally {
      isSavingLyricsSelection = false;
    }
  }

  function lyricsRequestIsCurrent(trackPath: string, requestId: number) {
    return requestId === lyricsRequestId && currentTrack?.filePath === trackPath;
  }

  function openLyricsEditor() {
    if (!currentTrack) return;
    lyricsEdit = { track: currentTrack, kind: currentLyrics?.kind ?? "plain", text: currentLyrics?.text ?? "" };
  }

  async function handleSaveEditedLyrics(kind: TrackLyrics["kind"], text: string) {
    const trackPath = lyricsEdit?.track.filePath;
    if (!trackPath) return;
    const lyrics = await saveEditedLyrics(trackPath, kind, text);
    if (currentTrack?.filePath === trackPath) {
      lyricsRequestId += 1;
      currentLyrics = lyrics;
      isLoadingLyrics = false;
      isAutoFindingLyrics = false;
      isLyricsPickerOpen = false;
      lyricsLookupError = null;
      lyricsLookupMessage = "Lyrics saved.";
    }
  }

  async function handleRestoreOriginalLyrics() {
    const trackPath = currentTrack?.filePath;
    if (!trackPath || !window.confirm("Discard your edits and restore the original lyrics?")) return;
    try {
      await restoreOriginalLyrics(trackPath);
      if (currentTrack?.filePath === trackPath) await loadCurrentTrackLyrics(trackPath);
    } catch (error) {
      if (currentTrack?.filePath === trackPath) lyricsLookupError = String(error);
    }
  }

  async function handleRemoveCachedLyrics() {
    if (!currentTrack) {
      return;
    }

    const trackPath = currentTrack.filePath;
    const requestId = lyricsRequestId;
    lyricsLookupError = null;
    lyricsLookupMessage = "Removing cached lyrics...";

    try {
      await removeCachedTrackLyrics(trackPath);
      if (!lyricsRequestIsCurrent(trackPath, requestId)) return;
      const reloadId = lyricsRequestId + 1;
      await loadCurrentTrackLyrics(trackPath);
      if (lyricsRequestIsCurrent(trackPath, reloadId)) lyricsLookupMessage = "Cached lyrics removed.";
    } catch (error) {
      if (!lyricsRequestIsCurrent(trackPath, requestId)) return;
      lyricsLookupMessage = null;
      lyricsLookupError = error instanceof Error ? error.message : "Could not remove cached lyrics.";
    }
  }

  async function handleLyricsOffsetChange(nextOffsetSeconds: number) {
    if (!currentTrack || !currentLyrics || currentLyrics.kind !== "synced" || isSavingLyricsOffset) {
      return;
    }

    const trackPath = currentTrack.filePath;
    const requestId = lyricsRequestId;
    const roundedOffset = Math.round(Math.max(-5, Math.min(5, nextOffsetSeconds)) * 10) / 10;
    isSavingLyricsOffset = true;
    lyricsLookupError = null;

    try {
      const savedOffset = await setTrackLyricsOffset(trackPath, roundedOffset);
      if (!lyricsRequestIsCurrent(trackPath, requestId) || !currentLyrics) return;
      currentLyrics = { ...currentLyrics, offsetSeconds: savedOffset };
      lyricsLookupMessage = Math.abs(savedOffset) > 0.01 ? `Lyrics offset saved (${formatSignedSeconds(savedOffset)}).` : "Lyrics offset saved.";
    } catch (error) {
      if (!lyricsRequestIsCurrent(trackPath, requestId)) return;
      lyricsLookupMessage = null;
      lyricsLookupError = error instanceof Error ? error.message : "Could not save lyrics offset.";
    } finally {
      isSavingLyricsOffset = false;
    }
  }

  async function handleScanLibrary(changeFolder = false, append = false, onlyFolder: string | null = null) {
    if (isScanning) return;
    isScanning = true;
    scanError = null;

    try {
      const folder = changeFolder || append || !scannedFolder ? await chooseLibraryFolder() : scannedFolder;

      if (!folder) {
        return;
      }

      await scanFolderIntoLibrary(folder, append, onlyFolder);
    } catch (error) {
      scanError = error instanceof Error ? error.message : String(error);
    } finally {
      isScanning = false;
    }
  }

  async function scanFolderIntoLibrary(folder: string, append = false, onlyFolder: string | null = null) {
    isScanning = true;
    hasLoadedCache = true;

    const sameLibrary = scannedFolder !== null && (append || scannedFolder === folder);
    scanProgress = null;
    scanSummary = null;
    const result = await scanLibrary(folder, append, onlyFolder, (progress) => {
      if (isScanning) scanProgress = progress;
    });
    const scannedTracks = result.tracks;
    const reports = onlyFolder ? result.folders.filter((item) => item.path === onlyFolder) : result.folders;
    const total = (key: "added" | "updated" | "skipped") => reports.reduce((count, report) => count + report[key], 0);
    const unavailable = reports.filter((report) => report.error).length;
    scanSummary = `${total("added")} added · ${total("updated")} updated · ${total("skipped")} skipped${unavailable ? ` · ${unavailable} unavailable` : ""}`;
    scannedFolder = result.primaryFolder;
    libraryTracks = scannedTracks;
    void reloadListeningTime();
    libraryFolders = result.folders;
    unavailableTrackIds = new Set(result.unavailableTrackIds);
    scanCount = scannedTracks.length;
    lastScannedAt = result.scannedAt;
    statsRevision += 1;
    if (sameLibrary) {
      const scannedById = new Map(scannedTracks.map((track, index) => [track.id, { track, index }]));
      playbackQueue = playbackQueue.map((track) => scannedById.get(track.id)?.track ?? track);
      if (currentTrack) {
        const scannedTrack = scannedById.get(currentTrack.id);
        currentTrack = scannedTrack?.track ?? currentTrack;
        currentTrackIndex = scannedTrack?.index ?? null;
      }
      return;
    }
    selectedAlbumId = null;
    selectedArtistName = null;
    selectedGenreName = null;
    clearGenreEditState();
    isLikedSongsOpen = false;
    isMixBuilderOpen = false;
    isCreatePlaylistOpen = false;
    isPlaylistEditMode = false;
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
  }

  async function handleScanRippedFolder() {
    if (!lastRippedFolder || isScanning) {
      return;
    }

    scanError = null;
    cdRipError = null;
    cdRipMessage = "Scanning ripped folder...";

    try {
      await scanFolderIntoLibrary(lastRippedFolder, true);
      cdRipMessage = "Ripped tracks added to your library.";
    } catch (error) {
      cdRipMessage = null;
      cdRipError = error instanceof Error ? error.message : String(error);
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
    videoCodecInfo = null;
    void loadVideoCodecInfo(video.id);
    void refreshNativeVideoStatus(false);
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

    try {
      if (activeVideoStatus?.videoId === video.id) {
        videoBackendStatus = await stopVideo();
      }
      applyUpdatedVideo(await updateVideoProgress(video.id, 0, false));
      resetVideoSession(video.id);
      videoPlaybackMessage = "Progress reset.";
    } catch (error) {
      videoPlaybackError = friendlyVideoPlaybackError(error);
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

    if (!video) {
      return;
    }

    videoPlaybackError = null;
    videoPlaybackMessage = null;

    try {
      if (isPlaying) {
        applyPlaybackStatus(await pausePlayback());
      }

      resetVideoSession(video.id);
      videoBackendStatus = await playVideo(video.id, Math.max(0, positionSeconds));
      videoObservedPositionSeconds = videoBackendStatus.positionSeconds;
      videoPlaybackMessage = "Playing in Cassette video window.";
    } catch (error) {
      videoPlaybackError = friendlyVideoPlaybackError(error);
    }
  }

  async function handleNativeVideoPause() {
    await saveActiveVideoProgress(true);

    try {
      videoBackendStatus = await pauseVideo();
    } catch (error) {
      videoPlaybackError = friendlyVideoPlaybackError(error);
    }
  }

  async function handleNativeVideoResume() {
    try {
      videoBackendStatus = await resumeVideo();
    } catch (error) {
      videoPlaybackError = friendlyVideoPlaybackError(error);
    }
  }

  async function handleNativeVideoStop() {
    await saveActiveVideoProgress(true);

    try {
      videoBackendStatus = await stopVideo();
      await loadVideoLibrary();
      videoPlaybackMessage = "Video stopped.";
    } catch (error) {
      videoPlaybackError = friendlyVideoPlaybackError(error);
    }
  }

  async function handleBringVideoWindowToFront() {
    try {
      videoBackendStatus = await bringVideoWindowToFront();
      videoPlaybackMessage = "Video is playing in Cassette video window.";
    } catch (error) {
      videoPlaybackError = friendlyVideoPlaybackError(error);
    }
  }

  async function handleFullscreenVideoWindow() {
    try {
      videoBackendStatus = await fullscreenVideoWindow();
    } catch (error) {
      videoPlaybackError = friendlyVideoPlaybackError(error);
    }
  }

  async function handleCloseVideoWindow() {
    await saveActiveVideoProgress(true);

    try {
      videoBackendStatus = await closeVideoWindow();
      await loadVideoLibrary();
      videoPlaybackMessage = "Video window closed.";
    } catch (error) {
      videoPlaybackError = friendlyVideoPlaybackError(error);
    }
  }

  async function handleNativeVideoSeek(event: Event) {
    const value = event.currentTarget instanceof HTMLInputElement ? Number(event.currentTarget.value) : 0;

    try {
      videoBackendStatus = await seekVideo(value);
      videoObservedPositionSeconds = videoBackendStatus.positionSeconds;
      await saveActiveVideoProgress(true);
    } catch (error) {
      videoPlaybackError = friendlyVideoPlaybackError(error);
    }
  }

  async function handleNativeVideoVolume(event: Event) {
    const value = event.currentTarget instanceof HTMLInputElement ? Number(event.currentTarget.value) : 1;

    try {
      videoBackendStatus = await setVideoVolume(value);
    } catch (error) {
      videoPlaybackError = friendlyVideoPlaybackError(error);
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

      await saveActiveVideoProgress(true);
      if (videoBackendStatus?.videoId === video.id) {
        videoBackendStatus = await stopVideo();
      }

      await openPath(video.filePath);
      videoPlaybackMessage = "Opened in external player.";
    } catch (error) {
      videoPlaybackError = error instanceof Error ? error.message : String(error);
    }
  }

  function friendlyVideoPlaybackError(error: unknown) {
    const message = error instanceof Error ? error.message : String(error);

    if (message.trim() === "The operation is not supported.") {
      return "Cassette could not play this video internally.\nTry Open External as a backup.";
    }

    return message;
  }

  async function saveActiveVideoProgress(force: boolean, explicitStatus: VideoPlaybackStatus | null = activeVideoStatus) {
    const video = selectedVideo;
    const status = explicitStatus;

    if (!video || !status || status.videoId !== video.id || status.hasEnded || (!force && !status.isPlaying)) {
      return;
    }

    const now = Date.now();

    if (!force && now - videoLastProgressSaveMs < 5000) {
      return;
    }

    videoLastProgressSaveMs = now;
    const duration = status.durationSeconds ?? video.durationSeconds;
    const threshold = duration ? Math.min(30, duration * 0.5) : 30;
    const shouldIncrementPlayCount = !videoPlayCountRecorded && videoSessionWatchedSeconds >= threshold;

    await saveVideoProgress(video, Math.floor(status.positionSeconds), shouldIncrementPlayCount);

    if (shouldIncrementPlayCount) {
      videoPlayCountRecorded = true;
    }
  }

  async function refreshNativeVideoStatus(forceSave: boolean) {
    if (!selectedVideoId && !videoBackendStatus?.videoId) {
      return;
    }

    try {
      const previousVideoId = videoBackendStatus?.videoId ?? null;
      const status = await getVideoState();
      videoBackendStatus = status;
      videoPlaybackError = status.error;

      if (previousVideoId && !status.videoId) {
        resetVideoSession(null);
        await loadVideoLibrary();
        videoPlaybackMessage = "Video window closed.";
        return;
      }

      if (selectedVideo && status.videoId === selectedVideo.id) {
        if (videoSessionVideoId !== selectedVideo.id) {
          resetVideoSession(selectedVideo.id);
        }

        const delta = status.positionSeconds - videoObservedPositionSeconds;

        if (status.isPlaying && delta > 0 && delta < 10) {
          videoSessionWatchedSeconds += delta;
        }

        videoObservedPositionSeconds = status.positionSeconds;

        if (status.hasEnded) {
          const shouldIncrementPlayCount = !videoPlayCountRecorded;
          resetVideoSession(selectedVideo.id);
          await saveVideoProgress(selectedVideo, 0, shouldIncrementPlayCount);
          return;
        }

        await saveActiveVideoProgress(forceSave, status);
      }
    } catch (error) {
      videoPlaybackError = error instanceof Error ? error.message : String(error);
    }
  }

  async function loadVideoCodecInfo(videoId: string) {
    isLoadingVideoCodecInfo = true;

    try {
      videoCodecInfo = await getVideoCodecInfo(videoId);
    } catch (error) {
      videoCodecInfo = {
        container: null,
        videoCodec: null,
        audioCodec: null,
        resolution: null,
        durationSeconds: null,
        error: error instanceof Error ? error.message : String(error),
      };
    } finally {
      isLoadingVideoCodecInfo = false;
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
    videoEditDraft = {
      ...videoEditDraft,
      [key]: key === "videoType" ? value as VideoType : value,
    };
  }

  async function handleDetectDvd() {
    if (isDetectingDvd || isImportingDvd) {
      return;
    }

    isDetectingDvd = true;
    dvdImportError = null;
    dvdImportMessage = null;
    dvdImportStatus = "Scanning titles...";

    try {
      const result = await detectDvd();
      dvdDetectResult = result;

      if (!result.found || !result.devicePath) {
        dvdSourcePath = null;
        dvdSourceType = null;
        dvdTitleScan = null;
        dvdTitles = [];
        selectedDvdTitleNumber = null;
        dvdImportStatus = "No DVD detected";
        dvdImportError = result.error ?? "No DVD drive found.";
        return;
      }

      dvdSourcePath = result.devicePath;
      dvdSourceType = "physical_device";
      dvdTitleScan = null;
      dvdTitles = [];
      selectedDvdTitleNumber = null;
      dvdImportStatus = result.readable ? "DVD detected" : "DVD detected";
      dvdImportMessage = result.readable ? `DVD device detected at ${result.devicePath}.` : `DVD device detected at ${result.devicePath}, but it may not be readable.`;
    } catch (error) {
      dvdImportStatus = "Error";
      dvdImportError = error instanceof Error ? error.message : String(error);
    } finally {
      isDetectingDvd = false;
    }
  }

  async function handleChooseVideoTsFolder() {
    if (isImportingDvd) {
      return;
    }

    const folder = await chooseVideoTsFolder();
    if (!folder) {
      return;
    }

    dvdSourcePath = folder;
    dvdSourceType = "video_ts_folder";
    dvdDetectResult = null;
    dvdTitleScan = null;
    dvdTitles = [];
    selectedDvdTitleNumber = null;
    dvdImportStatus = "VIDEO_TS folder selected";
    dvdImportError = null;
    dvdImportMessage = folder;
  }

  async function handleChooseDvdOutputFolder() {
    if (isImportingDvd) {
      return;
    }

    const folder = await chooseDvdImportOutputFolder();
    if (folder) {
      dvdOutputFolder = folder;
      dvdImportError = null;
    }
  }

  async function handleScanDvdTitles() {
    if (!dvdSourcePath) {
      dvdImportError = "Detect a DVD or choose a VIDEO_TS folder before scanning titles.";
      dvdImportStatus = "Error";
      return;
    }

    if (isScanningDvdTitles || isImportingDvd) {
      return;
    }

    isScanningDvdTitles = true;
    dvdImportStatus = "Scanning titles...";
    dvdImportError = null;
    dvdImportMessage = null;

    try {
      applyDvdTitleScan(await scanDvdTitles(dvdSourcePath));
    } catch (error) {
      dvdImportStatus = "Error";
      dvdImportError = error instanceof Error ? error.message : String(error);
    } finally {
      isScanningDvdTitles = false;
    }
  }

  function applyDvdTitleScan(result: DvdTitleScanResult) {
    dvdTitleScan = result;
    dvdSourcePath = result.sourcePath;
    dvdSourceType = result.sourceType;
    dvdTitles = result.titles;
    selectedDvdTitleNumber = result.titles.find((title) => title.likelyMainTitle)?.number ?? result.titles[0]?.number ?? null;
    dvdImportDraft = defaultDvdImportDraft(selectedDvdTitleNumber);

    if (result.error) {
      dvdImportStatus = "Error";
      dvdImportError = result.error;
      dvdImportMessage = null;
      return;
    }

    dvdImportStatus = result.titles.length > 0 ? "Ready to import" : "Error";
    dvdImportError = result.titles.length > 0 ? null : "No titles found.";
    dvdImportMessage = result.titles.length > 0
      ? `${result.titles.length} ${result.titles.length === 1 ? "title" : "titles"} found.`
      : null;
  }

  function handleSelectDvdTitle(titleNumber: number) {
    if (isImportingDvd) {
      return;
    }

    selectedDvdTitleNumber = titleNumber;
    dvdImportDraft = defaultDvdImportDraft(titleNumber, dvdImportDraft);
  }

  function updateDvdImportDraftField(key: keyof DvdImportDraft, value: string) {
    if (isImportingDvd) {
      return;
    }

    dvdImportDraft = {
      ...dvdImportDraft,
      [key]: key === "videoType" ? value as VideoType : value,
    };

    if (key === "artist" || key === "title" || key === "year") {
      dvdImportDraft = { ...dvdImportDraft, outputFilename: defaultDvdOutputFilename(dvdImportDraft) };
    }
  }

  async function handleImportDvdTitle() {
    if (isImportingDvd) {
      return;
    }

    if (!dvdSourcePath) {
      dvdImportError = "Detect a DVD or choose a VIDEO_TS folder before importing.";
      dvdImportStatus = "Error";
      return;
    }

    if (!selectedDvdTitleNumber) {
      dvdImportError = "Select a DVD title before importing.";
      dvdImportStatus = "Error";
      return;
    }

    if (!dvdOutputFolder) {
      dvdImportError = "Choose an output folder before importing.";
      dvdImportStatus = "Error";
      return;
    }

    const metadata = dvdImportMetadataFromDraft(dvdImportDraft);

    if (!metadata.title.trim()) {
      dvdImportError = "Title is required.";
      dvdImportStatus = "Error";
      return;
    }

    isImportingDvd = true;
    dvdImportStatus = "Importing...";
    dvdImportError = null;
    dvdImportMessage = "Starting DVD import...";

    try {
      const result = await importDvdTitle(dvdSourcePath, selectedDvdTitleNumber, dvdOutputFolder, metadata);
      applyUpdatedVideo(result.video);
      selectedVideoId = result.video.id;
      videoEditDraft = draftFromVideo(result.video);
      void loadVideoCodecInfo(result.video.id);
      dvdImportStatus = "Import complete";
      dvdImportMessage = `Imported to ${result.outputPath}`;
    } catch (error) {
      dvdImportStatus = "Error";
      dvdImportMessage = null;
      dvdImportError = error instanceof Error ? error.message : String(error);
    } finally {
      isImportingDvd = false;
    }
  }

  function handleDvdImportEvent(payload: DvdImportEvent, status: string) {
    dvdImportStatus = status;
    dvdImportMessage = payload.message ?? dvdImportMessage;

    if (status === "Error") {
      dvdImportError = payload.message ?? "DVD import failed.";
    }
  }

  function emptyVideoEditDraft(): VideoEditDraft {
    return {
      title: "",
      artist: "",
      videoType: "other",
      releaseOrCollection: "",
      year: "",
      venue: "",
      city: "",
      country: "",
      descriptionOrNotes: "",
    };
  }

  function draftFromVideo(video: VideoEntry): VideoEditDraft {
    return {
      title: video.title,
      artist: video.artist ?? "",
      videoType: video.videoType,
      releaseOrCollection: video.releaseOrCollection ?? "",
      year: video.year?.toString() ?? "",
      venue: video.venue ?? "",
      city: video.city ?? "",
      country: video.country ?? "",
      descriptionOrNotes: video.descriptionOrNotes ?? "",
    };
  }

  function videoInfoUpdateFromDraft(draft: VideoEditDraft): VideoInfoUpdate {
    const parsedYear = Number.parseInt(draft.year, 10);

    return {
      title: draft.title.trim(),
      artist: nullableDraftValue(draft.artist),
      videoType: draft.videoType,
      releaseOrCollection: nullableDraftValue(draft.releaseOrCollection),
      year: Number.isFinite(parsedYear) && parsedYear > 0 ? parsedYear : null,
      venue: nullableDraftValue(draft.venue),
      city: nullableDraftValue(draft.city),
      country: nullableDraftValue(draft.country),
      descriptionOrNotes: nullableDraftValue(draft.descriptionOrNotes),
    };
  }

  function defaultDvdImportDraft(titleNumber: number | null, existing?: DvdImportDraft): DvdImportDraft {
    const title = existing?.title.trim() || (titleNumber ? `DVD Title ${String(titleNumber).padStart(2, "0")}` : "DVD Title 01");
    const artist = existing?.artist.trim() || "Unknown Artist";
    const draft = {
      title,
      artist,
      videoType: existing?.videoType ?? ("other" as VideoType),
      releaseOrCollection: existing?.releaseOrCollection ?? "",
      year: existing?.year ?? "",
      venue: existing?.venue ?? "",
      city: existing?.city ?? "",
      country: existing?.country ?? "",
      descriptionOrNotes: existing?.descriptionOrNotes ?? "",
      outputFilename: existing?.outputFilename ?? "",
    };

    return {
      ...draft,
      outputFilename: draft.outputFilename || defaultDvdOutputFilename(draft),
    };
  }

  function dvdImportMetadataFromDraft(draft: DvdImportDraft): DvdImportMetadata {
    const parsedYear = Number.parseInt(draft.year, 10);

    return {
      title: draft.title.trim() || "DVD Title 01",
      artist: draft.artist.trim() || "Unknown Artist",
      videoType: draft.videoType,
      releaseOrCollection: nullableDraftValue(draft.releaseOrCollection),
      year: Number.isFinite(parsedYear) && parsedYear > 0 ? parsedYear : null,
      venue: nullableDraftValue(draft.venue),
      city: nullableDraftValue(draft.city),
      country: nullableDraftValue(draft.country),
      descriptionOrNotes: nullableDraftValue(draft.descriptionOrNotes),
      outputFilename: nullableDraftValue(draft.outputFilename),
    };
  }

  function defaultDvdOutputFilename(draft: Pick<DvdImportDraft, "artist" | "title" | "year">) {
    const artist = draft.artist.trim();
    const title = draft.title.trim();

    if (!artist || !title) {
      return `DVD Import ${ripFolderTimestampPreview()}.mkv`;
    }

    const year = draft.year.trim();
    return `${sanitizeFilenamePreview(year ? `${artist} - ${title} (${year})` : `${artist} - ${title}`, "DVD Import")}.mkv`;
  }

  function nullableDraftValue(value: string) {
    const trimmed = value.trim();
    return trimmed.length > 0 ? trimmed : null;
  }

  function videoDetailLine(video: VideoEntry) {
    return [
      video.artist ?? "Unknown Artist",
      videoTypeLabel(video.videoType),
      video.releaseOrCollection,
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
      video.releaseOrCollection ?? videoTypeLabel(video.videoType),
    ].filter(Boolean).join(" · ");
  }

  function videoTypeLabel(value: VideoType) {
    return videoTypeOptions.find((option) => option.value === value)?.label ?? "Other";
  }

  function sourceLabel(source: VideoEntry["source"]) {
    return source === "dvd_import" ? "DVD import" : "Local file";
  }

  function filterVideosByType(videoEntries: VideoEntry[], filter: VideoTypeFilter) {
    if (filter === "all") {
      return videoEntries;
    }

    if (filter === "live") {
      return videoEntries.filter((video) => video.videoType === "live_show" || video.videoType === "concert");
    }

    return videoEntries.filter((video) => video.videoType === filter);
  }

  function ripFolderTimestampPreview() {
    const now = new Date();
    const date = now.toISOString().slice(0, 10);
    const time = now.toTimeString().slice(0, 8).replaceAll(":", "-");

    return `${date} ${time}`;
  }

  function videoProgressPercent(video: VideoEntry) {
    if (!video.durationSeconds || video.durationSeconds <= 0) {
      return 0;
    }

    return progressPercent(video.lastPositionSeconds, video.durationSeconds);
  }

  function progressPercent(position: number, duration: number | null) {
    if (!duration || duration <= 0) {
      return 0;
    }

    return Math.max(0, Math.min(100, (position / duration) * 100));
  }

  function videoPlaybackStatusLabel(status: VideoPlaybackStatus | null) {
    if (!status?.videoId) {
      return "Stopped";
    }

    if (status.hasEnded) {
      return "Finished";
    }

    return status.isPlaying ? "Playing" : "Paused";
  }

  async function handleDetectCd() {
    if (isDetectingCd || isRippingCd) {
      return;
    }

    isDetectingCd = true;
    cdDetectionRevision += 1;
    cdCoverRequest += 1;
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
      detectedCdKey = null;
      audioCdFound = false;
      cdTracks = [];
      cdRipMetadata = null;
      cdRipError = error instanceof Error ? error.message : String(error);
    } finally {
      isDetectingCd = false;
    }
  }

  function applyCdDetection(result: CdDetectResult) {
    const key = result.discFound ? result.discKey ?? null : null;
    const sameDisc = key !== null && key === detectedCdKey;
    detectedCdKey = key;
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
    cdRipMetadata = defaultCdRipMetadata(cdTracks, sameDisc ? cdRipMetadata : null);
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
    isLookingUpCdMetadata = false;
  }

  async function handleLookupCdMetadata() {
    if (isLookingUpCdMetadata || isDetectingCd || isRippingCd) {
      return;
    }

    if (cdTracks.length === 0 || !audioCdFound) {
      await handleDetectCd();
      if (cdTracks.length === 0 || !audioCdFound) {
        cdMetadataError = cdRipError ?? "Detect an audio CD before looking up metadata.";
        return;
      }
    }

    const revision = cdDetectionRevision;
    isLookingUpCdMetadata = true;
    cdMetadataError = null;
    cdMetadataMessage = "Looking up metadata...";
    cdMetadataResults = [];
    selectedCdReleaseId = null;

    try {
      const result = await lookupCdMetadata(detectedCdKey);
      if (revision === cdDetectionRevision && !isRippingCd) applyCdMetadataLookup(result);
    } catch (error) {
      if (revision !== cdDetectionRevision || isRippingCd) return;
      cdMetadataMessage = null;
      cdMetadataError = error instanceof Error ? error.message : String(error);
    } finally {
      if (revision === cdDetectionRevision) isLookingUpCdMetadata = false;
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

    cdCoverRequest += 1;
    isLookingUpCdCover = false;
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
    if (isDetectingCd || isRippingCd) {
      return;
    }

    const request = ++cdCoverRequest;
    isLookingUpCdCover = true;
    cdCoverError = null;
    cdCoverMessage = "Loading cover...";

    try {
      const result = await lookupCdCover(releaseId);
      if (request === cdCoverRequest && !isRippingCd) applyCdCoverLookup(result, "cover-art-archive");
    } catch (error) {
      if (request !== cdCoverRequest || isRippingCd) return;
      cdCoverMessage = null;
      cdCoverError = error instanceof Error ? error.message : String(error);
    } finally {
      if (request === cdCoverRequest) isLookingUpCdCover = false;
    }
  }

  async function handleChooseCoverImage() {
    if (isRippingCd) {
      return;
    }

    const revision = cdDetectionRevision;
    const path = await chooseCoverImage();
    if (!path || revision !== cdDetectionRevision || isRippingCd) {
      return;
    }

    const request = ++cdCoverRequest;
    isLookingUpCdCover = false;
    cdCoverError = null;
    cdCoverMessage = "Loading cover...";

    try {
      const result = await inspectCoverImage(path);
      if (request === cdCoverRequest && !isRippingCd) applyCdCoverLookup(result, "manual");
    } catch (error) {
      if (request !== cdCoverRequest || isRippingCd) return;
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
      const result = await ripCdToFlac(cdOutputFolder, ripMetadataSnapshot, detectedCdKey);
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

  function selectValue(event: Event) {
    return event.currentTarget instanceof HTMLSelectElement ? event.currentTarget.value : "";
  }

  function textAreaValue(event: Event) {
    return event.currentTarget instanceof HTMLTextAreaElement ? event.currentTarget.value : "";
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

  function emptyTagEditorDraft(): TagEditorDraft {
    return {
      title: "",
      artist: "",
      album: "",
      albumArtist: "",
      genre: "",
      year: "",
      trackNumber: "",
      discNumber: "",
    };
  }

  function tagEditorDraftFromValues(values: TrackTagValues): TagEditorDraft {
    return {
      title: values.title ?? "",
      artist: values.artist ?? "",
      album: values.album ?? "",
      albumArtist: values.albumArtist ?? "",
      genre: values.genre ?? "",
      year: values.year?.toString() ?? "",
      trackNumber: values.trackNumber?.toString() ?? "",
      discNumber: values.discNumber?.toString() ?? "",
    };
  }

  function handleEditTrackTags(track: Track) {
    closeContextMenu();

    if (tagEditorTrack) {
      if (tagEditorTrack.id !== track.id) {
        requestCloseTagEditor(track);
      }

      return;
    }

    void openTagEditor(track);
  }

  async function openTagEditor(track: Track) {
    const requestId = ++tagEditorRequestId;
    tagEditorTrack = track;
    tagEditorData = null;
    tagEditorDraft = emptyTagEditorDraft();
    tagEditorOriginal = emptyTagEditorDraft();
    tagEditorError = null;
    tagEditorMessage = null;
    isLoadingTagEditor = true;
    isSavingTagEditor = false;

    try {
      const data = await getTrackTagEditorData(track.id);

      if (requestId !== tagEditorRequestId) {
        return;
      }

      tagEditorData = data;
      tagEditorTrack = data.track;
      tagEditorDraft = tagEditorDraftFromValues(data.fileValues);
      tagEditorOriginal = tagEditorDraftFromValues(data.fileValues);
    } catch (error) {
      if (requestId === tagEditorRequestId) {
        tagEditorError = error instanceof Error ? error.message : String(error);
      }
    } finally {
      if (requestId === tagEditorRequestId) {
        isLoadingTagEditor = false;
      }
    }
  }

  function requestCloseTagEditor(replacementTrack: Track | null = null) {
    if (isSavingTagEditor) {
      return;
    }

    if (tagEditorHasChanges()) {
      tagEditorPendingTrack = replacementTrack;
      isTagEditorDiscardConfirmationOpen = true;
      return;
    }

    closeTagEditor();

    if (replacementTrack) {
      void openTagEditor(replacementTrack);
    }
  }

  function keepEditingTagEditor() {
    isTagEditorDiscardConfirmationOpen = false;
    tagEditorPendingTrack = null;
    window.requestAnimationFrame(() => {
      tagEditorModalElement?.focus();
    });
  }

  function discardTagEditorChanges() {
    const replacementTrack = tagEditorPendingTrack;
    closeTagEditor();

    if (replacementTrack) {
      void openTagEditor(replacementTrack);
    }
  }

  function handleTagEditorDiscardBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      keepEditingTagEditor();
    }
  }

  function closeTagEditor() {
    tagEditorRequestId += 1;
    tagEditorTrack = null;
    tagEditorData = null;
    tagEditorDraft = emptyTagEditorDraft();
    tagEditorOriginal = emptyTagEditorDraft();
    isTagEditorDiscardConfirmationOpen = false;
    tagEditorPendingTrack = null;
    tagEditorError = null;
    tagEditorMessage = null;
    isLoadingTagEditor = false;
    isSavingTagEditor = false;
  }

  function resetTagEditorDraft() {
    tagEditorDraft = { ...tagEditorOriginal };
    tagEditorError = null;
    tagEditorMessage = null;
  }

  function updateTagEditorField(key: keyof TagEditorDraft, value: string) {
    tagEditorDraft = {
      ...tagEditorDraft,
      [key]: value,
    };
    tagEditorError = null;
    tagEditorMessage = null;
  }

  function normalizeTagEditorText(value: string) {
    return value.trim();
  }

  function normalizeTagEditorNumber(value: string) {
    const trimmed = value.trim();

    if (!/^\d+$/.test(trimmed)) {
      return trimmed;
    }

    const parsed = Number.parseInt(trimmed, 10);
    return Number.isSafeInteger(parsed) ? String(parsed) : trimmed;
  }

  function normalizedTagEditorDraft(draft: TagEditorDraft): TagEditorDraft {
    return {
      title: normalizeTagEditorText(draft.title),
      artist: normalizeTagEditorText(draft.artist),
      album: normalizeTagEditorText(draft.album),
      albumArtist: normalizeTagEditorText(draft.albumArtist),
      genre: normalizeTagEditorText(draft.genre),
      year: normalizeTagEditorNumber(draft.year),
      trackNumber: normalizeTagEditorNumber(draft.trackNumber),
      discNumber: normalizeTagEditorNumber(draft.discNumber),
    };
  }

  function tagEditorFieldChanged(key: keyof TagEditorDraft) {
    const draft = normalizedTagEditorDraft(tagEditorDraft);
    const original = normalizedTagEditorDraft(tagEditorOriginal);
    return draft[key] !== original[key];
  }

  function tagEditorHasChanges() {
    return (Object.keys(tagEditorDraft) as Array<keyof TagEditorDraft>).some(tagEditorFieldChanged);
  }

  function tagEditorValidationError() {
    if (!parsePositiveIntegerField(tagEditorDraft.year, "Year", 65535)) {
      return "Year must be blank or a valid positive integer.";
    }

    if (!parsePositiveIntegerField(tagEditorDraft.trackNumber, "Track number")) {
      return "Track number must be blank or an integer greater than or equal to 1.";
    }

    if (!parsePositiveIntegerField(tagEditorDraft.discNumber, "Disc number")) {
      return "Disc number must be blank or an integer greater than or equal to 1.";
    }

    return null;
  }

  function parsePositiveIntegerField(value: string, _label: string, maxValue: number | null = null) {
    const trimmed = value.trim();

    if (!trimmed) {
      return true;
    }

    if (!/^\d+$/.test(trimmed)) {
      return false;
    }

    const parsed = Number.parseInt(trimmed, 10);

    return Number.isSafeInteger(parsed) && parsed >= 1 && (maxValue === null || parsed <= maxValue);
  }

  function nullableTagText(value: string) {
    const trimmed = normalizeTagEditorText(value);

    return trimmed ? trimmed : null;
  }

  function nullableTagNumber(value: string) {
    const trimmed = normalizeTagEditorNumber(value);

    return trimmed ? Number.parseInt(trimmed, 10) : null;
  }

  function tagUpdateRequestFromDraft(trackId: string): UpdateTrackTagsRequest {
    return {
      trackId,
      title: nullableTagText(tagEditorDraft.title),
      artist: nullableTagText(tagEditorDraft.artist),
      album: nullableTagText(tagEditorDraft.album),
      albumArtist: nullableTagText(tagEditorDraft.albumArtist),
      genre: nullableTagText(tagEditorDraft.genre),
      year: nullableTagNumber(tagEditorDraft.year),
      trackNumber: nullableTagNumber(tagEditorDraft.trackNumber),
      discNumber: nullableTagNumber(tagEditorDraft.discNumber),
    };
  }

  function holdPlaybackClockForTagWrite(paths: string[]): boolean {
    if (platformCapabilities?.os !== "windows" || !currentTrack || !paths.includes(currentTrack.filePath)) {
      return false;
    }
    playbackListenProgress.forgetPosition();
    isTagWriteSuspendingPlayback = true;
    tagWritePlaybackGeneration += 1;
    return true;
  }

  async function releasePlaybackClockAfterTagWrite(held: boolean) {
    if (!held) return;
    try {
      const status = await getPlaybackStatus();
      if (currentTrack?.filePath === status.filePath) {
        applyPlaybackStatus(status);
        isTagWriteSuspendingPlayback = false;
        await handlePlaybackStatusUpdate(status);
      }
    } catch (error) {
      playbackListenProgress.forgetPosition();
      isPlaying = false;
      playbackError = error instanceof Error ? error.message : String(error);
    } finally {
      isTagWriteSuspendingPlayback = false;
      tagWritePlaybackGeneration += 1;
    }
  }

  async function handleSaveTagEditor() {
    if (!tagEditorTrack || !tagEditorData || isSavingTagEditor) {
      return;
    }

    const validationError = tagEditorValidationError();

    if (validationError) {
      tagEditorError = validationError;
      return;
    }

    if (!tagEditorHasChanges()) {
      return;
    }

    isSavingTagEditor = true;
    tagEditorError = null;
    tagEditorMessage = "Writing tags...";
    const savedSnapshot = normalizedTagEditorDraft(tagEditorDraft);
    let shouldCloseAfterSave = false;
    const heldPlaybackClock = holdPlaybackClockForTagWrite([tagEditorTrack.filePath]);

    try {
      const changedFields = (Object.keys(tagEditorDraft) as Array<keyof TagEditorDraft>).filter(tagEditorFieldChanged);
      const updatedTrack = await updateTrackTags(tagUpdateRequestFromDraft(tagEditorTrack.id), changedFields);
      applyUpdatedTrack(updatedTrack);
      tagEditorOriginal = savedSnapshot;
      tagEditorMessage = "Tags updated successfully.";
      shouldCloseAfterSave = true;
    } catch (error) {
      tagEditorMessage = null;
      tagEditorError = error instanceof Error ? error.message : String(error);
    } finally {
      await releasePlaybackClockAfterTagWrite(heldPlaybackClock);
      if (shouldCloseAfterSave) {
        window.setTimeout(() => {
          isSavingTagEditor = false;
          closeTagEditor();
        }, 450);
      } else {
        isSavingTagEditor = false;
      }
    }
  }

  function tagEditorCanSave() {
    return Boolean(
      tagEditorData?.tagEditingSupported
      && tagEditorHasChanges()
      && !tagEditorValidationError()
      && !isSavingTagEditor
      && !isLoadingTagEditor,
    );
  }

  function tagEditorCoverSource(track: Track | null) {
    return track?.coverArtPath ? localImageSource(track.coverArtPath) : null;
  }

  function emptyAlbumTagFieldDraft(): AlbumTagFieldDraft {
    return { mode: "unchanged", value: "" };
  }

  function emptyAlbumTagEditorDraft(): AlbumTagEditorDraft {
    return {
      album: emptyAlbumTagFieldDraft(),
      albumArtist: emptyAlbumTagFieldDraft(),
      artist: emptyAlbumTagFieldDraft(),
      genre: emptyAlbumTagFieldDraft(),
      year: emptyAlbumTagFieldDraft(),
    };
  }

  function albumTagSharedValue(key: keyof AlbumTagEditorDraft) {
    const shared = albumTagEditorData?.sharedValues[key];

    if (!shared || shared.mixed) {
      return "";
    }

    return shared.value?.toString() ?? "";
  }

  function albumTagSharedLabel(key: keyof AlbumTagEditorDraft) {
    const shared = albumTagEditorData?.sharedValues[key];

    if (!shared) {
      return "Unavailable";
    }
    if (shared.mixed) {
      return "Mixed";
    }

    return shared.value?.toString() || "Blank";
  }

  async function openAlbumTagEditor() {
    const album = selectedAlbum;
    if (!album || isAlbumTagEditorOpen) {
      return;
    }

    isAlbumTagEditorOpen = true;
    isLoadingAlbumTagEditor = true;
    albumTagEditorData = null;
    albumTagEditorDraft = emptyAlbumTagEditorDraft();
    albumTagEditorStage = "edit";
    albumTagEditorError = null;
    albumTagUpdateResult = null;
    albumTagUpdateProgress = null;

    try {
      albumTagEditorData = await getAlbumTagEditorData(album.id);
    } catch (error) {
      albumTagEditorError = error instanceof Error ? error.message : String(error);
    } finally {
      isLoadingAlbumTagEditor = false;
    }
  }

  function closeAlbumTagEditor() {
    if (isSavingAlbumTagEditor) {
      return;
    }

    isAlbumTagEditorOpen = false;
    isLoadingAlbumTagEditor = false;
    albumTagEditorData = null;
    albumTagEditorDraft = emptyAlbumTagEditorDraft();
    albumTagEditorStage = "edit";
    albumTagEditorError = null;
    albumTagUpdateResult = null;
    albumTagUpdateProgress = null;
  }

  function setAlbumTagFieldMode(key: keyof AlbumTagEditorDraft, mode: AlbumTagFieldMode) {
    const current = albumTagEditorDraft[key];
    albumTagEditorDraft = {
      ...albumTagEditorDraft,
      [key]: {
        mode,
        value: mode === "set" && current.mode === "unchanged"
          ? albumTagSharedValue(key)
          : current.value,
      },
    };
    albumTagEditorError = null;
  }

  function setAlbumTagFieldValue(key: keyof AlbumTagEditorDraft, value: string) {
    albumTagEditorDraft = {
      ...albumTagEditorDraft,
      [key]: { mode: "set", value },
    };
    albumTagEditorError = null;
  }

  function albumTagEditorHasChanges() {
    return (Object.values(albumTagEditorDraft) as AlbumTagFieldDraft[])
      .some((field) => field.mode !== "unchanged");
  }

  function albumTagGroupingMayChange() {
    const albumArtist = albumTagEditorData?.sharedValues.albumArtist;
    const albumUsesTrackArtist = Boolean(albumArtist && !albumArtist.mixed && albumArtist.value === null);

    return albumTagEditorDraft.album.mode !== "unchanged"
      || albumTagEditorDraft.albumArtist.mode !== "unchanged"
      || (albumUsesTrackArtist && albumTagEditorDraft.artist.mode !== "unchanged");
  }

  function albumTagEditorValidationError() {
    if (!albumTagEditorHasChanges()) {
      return "Choose at least one album field to change.";
    }

    for (const [key, field] of Object.entries(albumTagEditorDraft) as Array<[keyof AlbumTagEditorDraft, AlbumTagFieldDraft]>) {
      if (field.mode === "set" && !field.value.trim()) {
        return `${albumTagFieldName(key)} cannot be blank when set. Choose Clear to remove it.`;
      }
    }

    const year = albumTagEditorDraft.year;
    if (year.mode === "set") {
      const parsed = Number.parseInt(year.value.trim(), 10);
      if (!/^\d+$/.test(year.value.trim()) || !Number.isSafeInteger(parsed) || parsed < 1 || parsed > 65535) {
        return "Year must be an integer from 1 through 65535.";
      }
    }

    if ((albumTagEditorData?.preflightErrors.length ?? 0) > 0) {
      return "Resolve the listed tag preflight problems before saving.";
    }
    if (!albumTagEditorData || albumTagEditorData.editableTrackCount === 0) {
      return "This album has no safely editable tracks.";
    }

    return null;
  }

  function albumTagFieldName(key: keyof AlbumTagEditorDraft) {
    return {
      album: "Album title",
      albumArtist: "Album artist",
      artist: "Artist",
      genre: "Genre",
      year: "Year",
    }[key];
  }

  function albumTagChangeSummary(key: keyof AlbumTagEditorDraft) {
    const field = albumTagEditorDraft[key];
    if (field.mode === "clear") {
      return `${albumTagFieldName(key)}: clear`;
    }
    if (field.mode === "set") {
      return `${albumTagFieldName(key)}: ${field.value.trim()}`;
    }

    return null;
  }

  function albumTagFieldRequest(key: keyof AlbumTagEditorDraft) {
    const field = albumTagEditorDraft[key];
    if (field.mode === "unchanged") {
      return { mode: "unchanged" } as const;
    }
    if (field.mode === "clear") {
      return { mode: "clear" } as const;
    }
    if (key === "year") {
      return { mode: "set", value: Number.parseInt(field.value.trim(), 10) } as const;
    }

    return { mode: "set", value: field.value.trim() } as const;
  }

  function albumTagUpdateRequest(): UpdateAlbumTagsRequest | null {
    if (!albumTagEditorData) {
      return null;
    }

    return {
      albumId: albumTagEditorData.albumId,
      album: albumTagFieldRequest("album") as UpdateAlbumTagsRequest["album"],
      albumArtist: albumTagFieldRequest("albumArtist") as UpdateAlbumTagsRequest["albumArtist"],
      artist: albumTagFieldRequest("artist") as UpdateAlbumTagsRequest["artist"],
      genre: albumTagFieldRequest("genre") as UpdateAlbumTagsRequest["genre"],
      year: albumTagFieldRequest("year") as UpdateAlbumTagsRequest["year"],
    };
  }

  function showAlbumTagConfirmation() {
    const validationError = albumTagEditorValidationError();
    if (validationError) {
      albumTagEditorError = validationError;
      return;
    }

    albumTagEditorError = null;
    albumTagEditorStage = "confirm";
  }

  async function saveAlbumTags() {
    if (isSavingAlbumTagEditor) {
      return;
    }
    const request = albumTagUpdateRequest();
    const validationError = albumTagEditorValidationError();
    if (!request || validationError) {
      albumTagEditorError = validationError ?? "Album tag changes are unavailable.";
      albumTagEditorStage = "edit";
      return;
    }

    isSavingAlbumTagEditor = true;
    albumTagEditorError = null;
    const originalAlbumId = request.albumId;
    const originalAlbumTitle = selectedAlbum?.title ?? "the original album";
    const excludedTrackCount = albumTagEditorData?.excludedTrackCount ?? 0;
    albumTagUpdateProgress = {
      phase: "preflight",
      completed: 0,
      total: albumTagEditorData?.editableTrackCount ?? 0,
      fileName: null,
    };
    const heldPlaybackClock = holdPlaybackClockForTagWrite(
      albumTagEditorData?.tracks.filter((item) => item.editable).map((item) => item.track.filePath) ?? [],
    );

    try {
      const result = await updateAlbumTags(request);
      albumTagUpdateResult = result;
      if (result.outcome === "success") {
        for (const track of result.updatedTracks) {
          applyUpdatedTrack(track);
        }
        const nextAlbumId = result.updatedTracks[0] ? albumIdForTrack(result.updatedTracks[0]) : null;
        if (nextAlbumId) {
          selectedAlbumId = nextAlbumId;
          albumTagMoveNotice = nextAlbumId !== originalAlbumId && excludedTrackCount > 0
            ? {
                editedAlbumId: nextAlbumId,
                editedTrackCount: result.updatedTracks.length,
                excludedTrackCount,
                originalAlbumId,
                originalAlbumTitle,
              }
            : null;
        }
      }
      albumTagEditorStage = "result";
    } catch (error) {
      albumTagEditorError = error instanceof Error ? error.message : String(error);
      albumTagEditorStage = "edit";
    } finally {
      await releasePlaybackClockAfterTagWrite(heldPlaybackClock);
      isSavingAlbumTagEditor = false;
    }
  }

  function albumTagProgressLabel() {
    const progress = albumTagUpdateProgress;
    if (!progress) {
      return "Preparing album update...";
    }
    const phase = progress.phase === "preflight" ? "Checking" : "Writing";
    const file = progress.fileName ? ` · ${progress.fileName}` : "";
    return `${phase} ${progress.completed} of ${progress.total}${file}`;
  }

  function openAlbumMoreMenu(event: MouseEvent) {
    event.preventDefault();
    event.stopPropagation();
    const album = selectedAlbum;
    if (!album) return;
    const button = event.currentTarget as HTMLElement;
    const bounds = button.getBoundingClientRect();
    const items: ContextMenuItem[] = [
      {
        label: "Edit album tags…",
        disabled: selectedAlbumTracks.length === 0,
        action: () => openAlbumTagEditor(),
      },
      {
        label: "Edit album art",
        disabled: !!editingCoverForAlbum || selectedAlbumTracks.length === 0,
        action: () => changeAlbumCover(album.id),
      },
    ];
    if (customAlbumCovers[album.id]) {
      items.push({
        label: "Restore original art",
        disabled: !!editingCoverForAlbum,
        action: () => changeAlbumCover(album.id, true),
      });
    }
    openContextMenu(Math.max(8, bounds.right - 210), bounds.bottom + 6, items);
  }

  function viewOriginalAlbumAfterTagEdit() {
    const notice = albumTagMoveNotice;
    if (!notice || !displayAlbums.some((album) => album.id === notice.originalAlbumId)) {
      return;
    }

    selectedAlbumId = notice.originalAlbumId;
    searchQuery = "";
    mainElement?.scrollTo({ top: 0, behavior: "smooth" });
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

    await playQueuedTrackAtIndex(queueIndex, {
      tracks: nextQueue,
      shuffle: isShuffleEnabled,
      order: isShuffleEnabled ? buildShuffledQueueOrder(nextQueue.length, queueIndex) : [],
    });
  }

  function openContextMenu(x: number, y: number, items: ContextMenuItem[]) {
    contextMenu = { x, y, items };
  }

  function closeContextMenu() {
    contextMenu = null;
  }

  function openTrackContextMenu(track: Track, queue: Track[], x: number, y: number, firstItems: ContextMenuItem[] = []) {
    openContextMenu(x, y, [
      ...firstItems,
      { label: "Play", action: () => handleTrackSelect(track, queue) },
      { label: "Play next", action: () => insertTracksNext([track]) },
      { label: "Add to queue", action: () => appendTracksToQueue([track]) },
      { label: "Edit Tags", action: () => handleEditTrackTags(track) },
      ...ignoredTrackMenuItems([track.id]),
      ...playlistAddMenuItems(track),
      { label: "Go to artist", action: () => handleTrackArtistSelect(track) },
      { label: "Go to album", action: () => handleTrackAlbumSelect(track) },
      {
        label: track.isFavorite ? "Remove from liked songs" : "Add to liked songs",
        action: () => handleToggleFavorite(track),
      },
    ]);
  }

  function ignoredTrackMenuItems(ids: string[]): ContextMenuItem[] {
    const items: ContextMenuItem[] = [];
    if (ids.some((id) => !ignoredTrackIds.has(id))) {
      items.push({ label: ids.length === 1 ? "Ignore track" : "Ignore selected tracks", action: () => updateIgnoredTracks(ids, true) });
    }
    if (ids.some((id) => ignoredTrackIds.has(id))) {
      items.push({ label: ids.length === 1 ? "Stop ignoring track" : "Stop ignoring selected tracks", action: () => updateIgnoredTracks(ids, false) });
    }
    return items;
  }

  async function updateIgnoredTracks(ids: string[], ignored: boolean) {
    if (savingIgnoredTracks) return;
    savingIgnoredTracks = true;
    ignoredTracksError = null;
    try {
      ignoredTrackIds = new Set(await setIgnoredTracks(ids, ignored));
      statsRevision += 1;
      clearAlbumTrackSelection();
    } catch (error) {
      ignoredTracksError = error instanceof Error ? error.message : String(error);
    } finally {
      savingIgnoredTracks = false;
    }
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

    const hasTracks = albumHasTracks(album);
    const hasArtist = album.artist.trim().length > 0 && album.artist !== "Unknown Artist";

    openContextMenu(event.clientX, event.clientY, [
      { label: "Play album", disabled: !hasTracks, action: () => playTrackSet(tracksForAlbum(album)) },
      { label: "Shuffle album", disabled: !hasTracks, action: () => playTrackSet(tracksForAlbum(album), true) },
      { label: "Add album to queue", disabled: !hasTracks, action: () => appendTracksToQueue(tracksForAlbum(album)) },
      { label: "Go to artist", disabled: !hasArtist, action: () => selectArtistName(album.artist, artistOriginForCurrentView()) },
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

  function openPlaylistMoreMenu(event: MouseEvent, playlist: Playlist) {
    event.preventDefault();
    event.stopPropagation();

    const trigger = (event.currentTarget as HTMLElement).getBoundingClientRect();
    openContextMenu(trigger.left, trigger.bottom, [
      { label: "Rename playlist", action: () => handleRenamePlaylist(playlist) },
      { label: "Delete playlist", action: () => openDeletePlaylistConfirmation(playlist) },
    ]);
  }

  function handleNavigate(label: string) {
    const targetView = label === "Home" || (!ENABLE_EXPERIMENTAL_VIDEOS && label === "Videos")
      ? "Albums"
      : label;

    void saveActiveVideoProgress(true);
    activeView = targetView;
    statsDetailSection = null;
    selectedAlbumId = null;
    albumNavigationOrigin = { view: "Albums" };
    selectedArtistName = null;
    artistNavigationOrigin = { view: "Artists" };
    selectedGenreName = null;
    detailSongsView = null;
    clearGenreEditState();
    isLikedSongsOpen = false;
    isMixBuilderOpen = false;
    isCreatePlaylistOpen = false;
    isPlaylistEditMode = false;
    isLibraryHealthOpen = false;
    selectedPlaylistId = null;
    selectedVideoId = targetView === "Videos" ? selectedVideoId : null;
    isEditingVideo = false;
    searchQuery = "";
    mainElement?.scrollTo({ top: 0 });
  }

  function currentViewLocation(): ViewLocation {
    return {
      view: activeView,
      statsDetailSection: activeView === "Stats" ? statsDetailSection : null,
      albumId: selectedAlbumId,
      albumOrigin: albumNavigationOrigin,
      artistName: selectedArtistName,
      artistOrigin: artistNavigationOrigin,
      genreName: selectedGenreName,
      detailSongsView: (activeView === "Artists" && selectedArtistName) || (activeView === "Genres" && selectedGenreName) ? detailSongsView : null,
      playlistId: selectedPlaylistId,
      likedSongs: isLikedSongsOpen,
      mixBuilder: isMixBuilderOpen,
      libraryHealth: isLibraryHealthOpen,
      videoId: selectedVideoId,
    };
  }

  function sameViewLocation(left: ViewLocation, right: ViewLocation) {
    return JSON.stringify(left) === JSON.stringify(right);
  }

  function navigateViewHistory(direction: -1 | 1) {
    const nextHistory = stepView(viewHistory, direction);
    if (nextHistory === viewHistory) return;

    viewHistory = nextHistory;
    const location = nextHistory.entries[nextHistory.index];
    void saveActiveVideoProgress(true);
    activeView = location.view;
    statsDetailSection = location.statsDetailSection;
    selectedAlbumId = location.albumId;
    albumNavigationOrigin = location.albumOrigin;
    selectedArtistName = location.artistName;
    artistNavigationOrigin = location.artistOrigin;
    selectedGenreName = location.genreName;
    detailSongsView = location.detailSongsView;
    selectedPlaylistId = location.playlistId;
    isLikedSongsOpen = location.likedSongs;
    isMixBuilderOpen = location.mixBuilder;
    isLibraryHealthOpen = location.libraryHealth;
    selectedVideoId = location.videoId;
    isCreatePlaylistOpen = false;
    isPlaylistEditMode = false;
    isLyricsOptionsOpen = false;
    isLyricsPickerOpen = false;
    clearGenreEditState();
    searchQuery = "";
    mainElement?.scrollTo({ top: 0 });
  }

  function handleLyricsSelect() {
    activeView = "Now Playing";
    isQueueOpen = false;
    mainElement?.scrollTo({ top: 0 });
  }

  function toggleLyricsOptions() {
    isLyricsOptionsOpen = !isLyricsOptionsOpen;
  }

  function handleAlbumSelect(album: Album) {
    selectAlbumId(album.id, albumOriginForCurrentView());
  }

  function handleAlbumArtistSelect(artistName: string) {
    const artist = displayArtists.find((candidate) => candidate.name === artistName);
    if (artist) handleArtistSelect(artist);
  }

  function handleGenreNameSelect(genreName: string) {
    const genre = displayGenres.find((candidate) => candidate.name === genreName);
    if (genre) handleGenreSelect(genre);
  }

  function artistActivity(artistName: string): ListeningActivity {
    return displayArtists.find((candidate) => candidate.name === artistName) ?? { playCount: 0 };
  }

  async function handleAlbumCardPlay(event: MouseEvent, album: Album, shouldShuffle = false) {
    event.stopPropagation();
    await playTrackSet(tracksForAlbum(album), shouldShuffle);
  }

  function handleAlbumCardAddToQueue(event: MouseEvent, album: Album) {
    event.stopPropagation();
    appendTracksToQueue(tracksForAlbum(album));
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

  async function handlePlaySelectedArtist() {
    if (selectedArtistTracks.length === 0) {
      return;
    }

    await playTrackSet(selectedArtistTracks);
  }

  async function handleShuffleSelectedArtist() {
    if (selectedArtistTracks.length === 0) {
      return;
    }

    await playTrackSet(selectedArtistTracks, true);
  }

  function handleAddSelectedArtistToQueue() {
    appendTracksToQueue(selectedArtistTracks);
  }

  async function handlePlaySelectedGenre() {
    if (selectedGenreTracks.length === 0) {
      return;
    }

    await playTrackSet(selectedGenreTracks);
  }

  async function handleShuffleSelectedGenre() {
    if (selectedGenreTracks.length === 0) {
      return;
    }

    await playTrackSet(selectedGenreTracks, true);
  }

  function handleAddSelectedGenreToQueue() {
    appendTracksToQueue(selectedGenreTracks);
  }

  async function handlePlayLikedSongs(shouldShuffle = false) {
    await playTrackSet(favoriteTracks, shouldShuffle);
  }

  function handleAddLikedSongsToQueue() {
    appendTracksToQueue(favoriteTracks);
  }

  async function handlePlayPlaylist(playlist: Playlist, shouldShuffle = false) {
    await playTrackSet(tracksForPlaylist(playlist), shouldShuffle);
  }

  function handleAddPlaylistToQueue(playlist: Playlist) {
    appendTracksToQueue(tracksForPlaylist(playlist));
  }

  function stopCardAction(event: MouseEvent) {
    event.stopPropagation();
  }

  function clearAlbumTrackSelection() {
    trackSelectionAlbumId = null;
    selectedTrackIds = new Set();
    selectionAnchor = null;
  }

  function dismissAlbumTrackSelection(event: PointerEvent) {
    if (isAlbumSelectingTracks && event.target instanceof Element
      && !event.target.closest(".album-track-row, .album-selection-controls, .context-menu")) {
      clearAlbumTrackSelection();
    }
  }

  function albumSelectionMenuItems(albumId: string): ContextMenuItem[] {
    const selection = selectedAlbumTracks.filter((track) => selectedTrackIds.has(track.id));
    const ids = selection.map((track) => track.id);
    const queueTracks = selection.filter(isQueueEligible);

    function changeSelection(action: () => Promise<string>) {
      return async () => {
        if (savingBulkTracks) return;
        savingBulkTracks = true;
        try {
          bulkTrackFeedback = { albumId, message: await action(), error: false };
        } catch (error) {
          bulkTrackFeedback = { albumId, message: String(error), error: true };
        } finally {
          savingBulkTracks = false;
        }
      };
    }

    function addToPlaylist(playlistId: string | null, name: string | null) {
      return changeSelection(async () => {
        const playlist = await addTracksToPlaylist(playlistId, name, ids);
        applyUpdatedPlaylist(playlist);
        return `Selection added to ${playlist.name}.`;
      });
    }

    function likeSelection(favorite: boolean) {
      return changeSelection(async () => {
        await setTracksFavorite(ids, favorite);
        applyTrackFavorites(new Set(ids), favorite);
        return favorite ? "Selected tracks liked." : "Selected tracks unliked.";
      });
    }

    return [
      { label: "Play next", disabled: queueTracks.length === 0, action: () => insertTracksNext(queueTracks) },
      { label: "Add to queue", disabled: queueTracks.length === 0, action: () => appendTracksToQueue(queueTracks) },
      { label: "Add to playlist", items: [
        ...playlists.map((playlist) => ({ label: playlist.name, action: addToPlaylist(playlist.id, null) })),
        { label: "New playlist…", action: async () => {
          const name = window.prompt("Playlist name")?.trim();
          if (name) await addToPlaylist(null, name)();
        } },
      ] },
      ...(selection.some((track) => !track.isFavorite) ? [{ label: "Like selected tracks", action: likeSelection(true) }] : []),
      ...(selection.some((track) => track.isFavorite) ? [{ label: "Unlike selected tracks", action: likeSelection(false) }] : []),
      ...ignoredTrackMenuItems(ids),
    ];
  }

  function handleAlbumTrackSelect(track: Track, event: Pick<MouseEvent, "ctrlKey" | "metaKey">) {
    const extendRange = event.ctrlKey || event.metaKey;
    if (!selectedAlbumId || !(isAlbumSelectingTracks || extendRange)) {
      void handleTrackSelect(track, selectedAlbumTracks);
      return;
    }
    selectAlbumTrack(track, extendRange);
  }

  function selectAlbumTrack(track: Track, extendRange = false) {
    if (!selectedAlbumId) return;
    const selection = new Set(isAlbumSelectingTracks ? selectedTrackIds : []);
    trackSelectionAlbumId = selectedAlbumId;
    const displayed = selectedAlbumDiscGroups.flatMap((group) => group.tracks);
    const anchorIndex = displayed.findIndex((entry) => entry.id === selectionAnchor);
    const trackIndex = displayed.findIndex((entry) => entry.id === track.id);
    if (extendRange && anchorIndex >= 0 && trackIndex >= 0) {
      for (const entry of displayed.slice(Math.min(anchorIndex, trackIndex), Math.max(anchorIndex, trackIndex) + 1)) {
        selection.add(entry.id);
      }
    } else {
      if (!selection.delete(track.id)) selection.add(track.id);
      selectionAnchor = track.id;
    }
    selectedTrackIds = selection;
  }

  function openAlbumTrackContextMenu(event: MouseEvent, track: Track) {
    event.preventDefault();
    event.stopPropagation();
    if (selectedAlbumId && isAlbumSelectingTracks && selectedTrackIds.has(track.id)) {
      openContextMenu(event.clientX, event.clientY, albumSelectionMenuItems(selectedAlbumId));
      return;
    }
    openTrackContextMenu(track, selectedAlbumTracks, event.clientX, event.clientY, [{
      label: isAlbumSelectingTracks ? "Add to selection" : "Select tracks",
      action: () => selectAlbumTrack(track),
    }]);
  }

  function handleAlbumTrackKeydown(event: KeyboardEvent, track: Track) {
    if (event.target !== event.currentTarget) {
      return;
    }

    if (event.key === "Enter" || (isAlbumSelectingTracks && event.code === "Space")) {
      event.preventDefault();
      event.stopPropagation();
      handleAlbumTrackSelect(track, event);
    }
  }

  function handleTrackAlbumSelect(track: Track) {
    selectAlbumId(albumIdForTrack(track), albumOriginForCurrentView());
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

  function albumOriginForCurrentView(): AlbumNavigationOrigin {
    if (activeView === "Artists" && selectedArtistName) {
      return { view: "Artists", artistName: selectedArtistName };
    }

    if (activeView === "Genres" && selectedGenreName) {
      return { view: "Genres", genreName: selectedGenreName };
    }

    return { view: "Albums" };
  }

  function selectAlbumId(albumId: string, origin: AlbumNavigationOrigin = { view: "Albums" }) {
    searchQuery = "";
    activeView = "Albums";
    selectedAlbumId = albumId;
    albumNavigationOrigin = origin;
    selectedArtistName = null;
    selectedGenreName = null;
    detailSongsView = null;
    clearGenreEditState();
    albumGenreDraft = genreDraftForTracks(tracks.filter((track) => albumIdForTrack(track) === albumId));
    isLikedSongsOpen = false;
    isMixBuilderOpen = false;
    isCreatePlaylistOpen = false;
    isPlaylistEditMode = false;
    isLibraryHealthOpen = false;
    selectedPlaylistId = null;
    mainElement?.scrollTo({ top: 0 });
  }

  function handleArtistSelect(artist: Artist) {
    selectArtistName(artist.name, artistOriginForCurrentView());
  }

  function handleTrackArtistSelect(track: Track) {
    selectArtistName(artistNameForTrack(track), artistOriginForCurrentView());
  }

  function artistOriginForCurrentView(): ArtistNavigationOrigin {
    if (activeView === "Genres" && selectedGenreName) {
      return { view: "Genres", genreName: selectedGenreName };
    }

    if (activeView === "Albums" && albumNavigationOrigin.view === "Genres") {
      return { view: "Genres", genreName: albumNavigationOrigin.genreName };
    }

    return { view: "Artists" };
  }

  function selectArtistName(artistName: string, origin: ArtistNavigationOrigin = { view: "Artists" }) {
    artistTrackVisibleLimit = STATS_PAGE_SIZE;
    searchQuery = "";
    activeView = "Artists";
    selectedArtistName = artistName;
    detailSongsView = null;
    artistNavigationOrigin = origin;
    selectedAlbumId = null;
    albumNavigationOrigin = { view: "Albums" };
    selectedGenreName = null;
    clearGenreEditState();
    artistGenreDraft = genreDraftForTracks(tracks.filter((track) => artistNameForTrack(track) === artistName));
    isLikedSongsOpen = false;
    isMixBuilderOpen = false;
    isCreatePlaylistOpen = false;
    isPlaylistEditMode = false;
    isLibraryHealthOpen = false;
    selectedPlaylistId = null;
    mainElement?.scrollTo({ top: 0 });
  }

  function handleGenreSelect(genre: Genre) {
    genreTrackVisibleLimit = STATS_PAGE_SIZE;
    searchQuery = "";
    activeView = "Genres";
    selectedGenreName = genre.name;
    detailSongsView = null;
    selectedAlbumId = null;
    albumNavigationOrigin = { view: "Albums" };
    selectedArtistName = null;
    artistNavigationOrigin = { view: "Artists" };
    clearGenreEditState();
    isLikedSongsOpen = false;
    isMixBuilderOpen = false;
    isCreatePlaylistOpen = false;
    isPlaylistEditMode = false;
    isLibraryHealthOpen = false;
    selectedPlaylistId = null;
    mainElement?.scrollTo({ top: 0 });
  }

  function openGroupSongs(view: "artist" | "genre") {
    detailSongsView = view;
    if (view === "artist") artistTrackVisibleLimit = STATS_PAGE_SIZE;
    else genreTrackVisibleLimit = STATS_PAGE_SIZE;
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
    isCreatePlaylistOpen = false;
    isPlaylistEditMode = false;
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
    isCreatePlaylistOpen = false;
    isPlaylistEditMode = false;
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
    isCreatePlaylistOpen = false;
    isPlaylistEditMode = false;
    isLibraryHealthOpen = false;
    searchQuery = "";
    playlistMessage = null;
    playlistError = null;
    mainElement?.scrollTo({ top: 0 });
  }

  function handleOpenCreatePlaylist() {
    isCreatePlaylistOpen = true;
    playlistNameDraft = "";
    playlistError = null;
    playlistMessage = null;
    requestAnimationFrame(() => document.getElementById("playlist-name")?.focus());
  }

  function handleCancelCreatePlaylist() {
    isCreatePlaylistOpen = false;
    playlistNameDraft = "";
    playlistError = null;
    requestAnimationFrame(() => document.getElementById("new-playlist-button")?.focus());
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
      isCreatePlaylistOpen = false;
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

    if (!playlist || isMovingPlaylistTrack) {
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
    if (!playlist || !canMovePlaylistTrack(playlist, track, direction)) return;

    isMovingPlaylistTrack = true;
    playlistError = null;
    playlistMessage = null;
    try {
      applyUpdatedPlaylist(await movePlaylistTrack(playlist.id, track.id, direction));
    } catch (error) {
      playlistError = error instanceof Error ? error.message : String(error);
    } finally {
      isMovingPlaylistTrack = false;
    }
  }

  function canMoveSelectedPlaylistTrack(track: Track, direction: "up" | "down") {
    return selectedPlaylist ? canMovePlaylistTrack(selectedPlaylist, track, direction) : false;
  }

  function canMovePlaylistTrack(playlist: Playlist, track: Track, direction: "up" | "down") {
    const trackIndex = playlist.trackIds.indexOf(track.id);
    if (isMovingPlaylistTrack || trackIndex < 0) return false;
    const candidates = direction === "up"
      ? playlist.trackIds.slice(0, trackIndex)
      : playlist.trackIds.slice(trackIndex + 1);
    return candidates.some((id) => libraryTracksById.has(id));
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
      isCreatePlaylistOpen = false;
      isPlaylistEditMode = false;
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
    isCreatePlaylistOpen = false;
    isPlaylistEditMode = false;
    isLibraryHealthOpen = true;
    selectedPlaylistId = null;
    searchQuery = "";
    mainElement?.scrollTo({ top: 0 });
  }

  function handleBackToPlaylists() {
    isLikedSongsOpen = false;
    isMixBuilderOpen = false;
    isCreatePlaylistOpen = false;
    isPlaylistEditMode = false;
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
    if (activeView === "Songs") songsSearchQuery = "";
    else searchQuery = "";
  }

  function handleSearchKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      clearSearch();
    }
  }

  function isQueueEligible(track: Track) {
    return !unavailableTrackIds.has(track.id) && !ignoredTrackIds.has(track.id);
  }

  function gaplessCandidate(queue: Track[], queueIndex: number, order: number[]) {
    if (!gaplessEnabled || repeatMode === "one") return null;
    const filtered = order.filter((index) => index === queueIndex
      || isQueueEligible(queue[index]));
    const nextIndex = filtered[filtered.indexOf(queueIndex) + 1]
      ?? (repeatMode === "all" ? filtered[0] : undefined);
    const next = nextIndex === undefined ? null : queue[nextIndex];
    return next && next.filePath !== queue[queueIndex].filePath
      && isQueueEligible(next) ? next.filePath : null;
  }

  $effect(() => {
    if (isStartingTrack || isTagWriteSuspendingPlayback || !currentTrack) return;
    const path = currentTrack.filePath;
    const next = currentQueueIndex === null ? null
      : gaplessCandidate(playbackQueue, currentQueueIndex, playbackOrder);
    void stageGaplessTrack(path, next).catch(() => { /* Existing end-of-track playback is the fallback. */ });
  });

  async function acceptGaplessTransition(status: PlaybackStatus) {
    if (!status.filePath || !currentTrack) return;
    const changedAt = performance.now() - Math.max(0, status.positionSeconds) * 1000;
    observeGaplessEnd(changedAt);
    const nextIndex = getNextQueueIndex(true);
    if (nextIndex !== null && playbackQueue[nextIndex]?.filePath !== status.filePath) {
      await playQueuedTrackAtIndex(nextIndex);
      return;
    }
    if (nextIndex === null) {
      await pausePlayback();
      isPlaying = false;
      hasCurrentTrackEnded = true;
      return;
    }
    const track = playbackQueue[nextIndex];
    const trackIndex = tracks.findIndex((candidate) => candidate.id === track.id);
    currentTrack = trackIndex >= 0 ? tracks[trackIndex] : track;
    currentTrackIndex = trackIndex >= 0 ? trackIndex : null;
    currentQueueIndex = nextIndex;
    playbackSeekCoordinator.reset(track.filePath);
    resetPlaybackListenSession(currentTrack, changedAt);
    applyPlaybackStatus(status);
    void notifyTrackPlaybackStarted(currentTrack, ++playbackStartId);
    await handlePlaybackStatusUpdate(status);
  }

  async function playQueuedTrackAtIndex(
    queueIndex: number,
    nextQueue?: { tracks: Track[]; shuffle: boolean; order: number[] },
  ) {
    const track = (nextQueue?.tracks ?? playbackQueue)[queueIndex];
    if (!track || isStartingTrack || isTagWriteSuspendingPlayback) {
      return;
    }
    if (unavailableTrackIds.has(track.id)) {
      playbackError = "This track is unavailable. Reconnect its music folder and rescan it.";
      return;
    }

    isStartingTrack = true;
    playbackError = null;
    const currentPlaybackStartId = ++playbackStartId;
    try {
      await saveActiveVideoProgress(true);
      if (videoBackendStatus?.videoId) {
        try {
          videoBackendStatus = await stopVideo();
        } catch {
          videoBackendStatus = null;
        }
      }
      const queue = nextQueue?.tracks ?? playbackQueue;
      const shuffle = nextQueue?.shuffle ?? isShuffleEnabled;
      const order = nextQueue
        ? (shuffle ? normalizedQueueOrder(nextQueue.order, queue.length) : queue.map((_, index) => index))
        : playbackOrder;
      const nextFilePath = queue[queueIndex]?.id === track.id
        ? gaplessCandidate(queue, queueIndex, order)
        : null;
      await sampleListenBeforeTrackChange();
      const startedAt = performance.now();
      const status = await playTrack(track.filePath, nextFilePath);
      if (status.filePath !== track.filePath) {
        throw new Error("The selected track could not be started.");
      }
      if (nextQueue) {
        playbackQueue = nextQueue.tracks;
        isShuffleEnabled = nextQueue.shuffle;
        shuffledQueueOrder = nextQueue.order;
      }
      const trackIndex = tracks.findIndex((candidate) => candidate.id === track.id);
      currentTrack = trackIndex >= 0 ? tracks[trackIndex] : track;
      currentTrackIndex = trackIndex >= 0 ? trackIndex : null;
      currentQueueIndex = playbackQueue[queueIndex]?.id === track.id
        ? queueIndex
        : playbackQueue.findIndex((candidate) => candidate.id === track.id);
      if (currentQueueIndex < 0) currentQueueIndex = null;
      playbackSeekCoordinator.reset(track.filePath);
      hasCurrentTrackEnded = false;
      resetPlaybackListenSession(currentTrack, startedAt);
      applyPlaybackStatus(status);
      void notifyTrackPlaybackStarted(currentTrack, currentPlaybackStartId);
    } catch (error) {
      try {
        const status = await getPlaybackStatus();
        if (currentTrack?.filePath === status.filePath) {
          applyPlaybackStatus(status);
        } else if (currentTrack && !status.filePath) {
          isPlaying = false;
          hasCurrentTrackEnded = true;
        }
      } catch {
        // Keep the original track opening error available to the user.
      }
      playbackError = error instanceof Error ? error.message : String(error);
    } finally {
      isStartingTrack = false;
    }
  }

  async function notifyTrackPlaybackStarted(track: Track, currentPlaybackStartId: number) {
    if (!trackNotificationsEnabled) {
      return;
    }

    const trackKey = trackNotificationKey(track);

    if (lastNotifiedTrackKey === trackKey && lastNotifiedPlaybackStartId === currentPlaybackStartId) {
      return;
    }

    lastNotifiedTrackKey = trackKey;
    lastNotifiedPlaybackStartId = currentPlaybackStartId;
    const result = await sendLinuxPlaybackNotification(track.title, trackNotificationBody(track));

    if (!result.ok) {
      console.warn("Track notification failed:", safeNotificationError(result.error));
    }
  }

  function trackNotificationKey(track: Track) {
    return track.id || track.filePath;
  }

  function trackNotificationBody(track: Track) {
    const artist = track.artist ?? track.albumArtist ?? "Unknown Artist";
    const album = track.album ?? "Unknown Album";

    return `${artist} · ${album}`;
  }

  function safeNotificationError(error: string) {
    const normalized = error.replace(/[\r\n]+/g, " ").trim();

    return normalized || "Unknown error";
  }

  async function handlePlaybackStatusUpdate(status: PlaybackStatus) {
    void maybeRecordTrackPlay(status.hasEnded);
    if (status.hasEnded) await handleTrackEnd();
  }

  async function handleTrackEnd() {
    if (!currentTrack || isStartingTrack || isHandlingTrackEnd || hasCurrentTrackEnded) {
      return;
    }

    isHandlingTrackEnd = true;
    hasCurrentTrackEnded = true;
    isPlaying = false;
    positionSeconds = durationSeconds ?? currentTrack.durationSeconds ?? positionSeconds;
    const nextQueueIndex = getNextQueueIndex(true);
    try {
      if (nextQueueIndex !== null) await playQueuedTrackAtIndex(nextQueueIndex);
    } finally {
      isHandlingTrackEnd = false;
    }
  }

  async function handleToggleFavorite(track: Track) {
    playbackError = null;

    try {
      const isFavorite = await toggleTrackFavorite(track.id);
      applyTrackFavorites(new Set([track.id]), isFavorite);
    } catch (error) {
      playbackError = error instanceof Error ? error.message : String(error);
    }
  }

  function handleToggleCurrentTrackFavorite() {
    if (currentTrack) {
      void handleToggleFavorite(currentTrack);
    }
  }

  function applyTrackFavorites(trackIds: Set<string>, isFavorite: boolean) {
    libraryTracks = libraryTracks.map((track) => (trackIds.has(track.id) ? { ...track, isFavorite } : track));
    playbackQueue = playbackQueue.map((track) => (trackIds.has(track.id) ? { ...track, isFavorite } : track));

    if (currentTrack && trackIds.has(currentTrack.id)) {
      currentTrack = { ...currentTrack, isFavorite };
    }
  }

  async function maybeRecordTrackPlay(naturalEnd = false) {
    if (
      !currentTrack
      || !playbackSessionEventId
      || ignoredTrackIds.has(currentTrack.id)
      || countedPlaybackTrackId === currentTrack.id
      || !playbackListenProgress.qualifies(durationSeconds ?? currentTrack.durationSeconds, listenPercentage, naturalEnd)
    ) {
      return;
    }

    const trackId = currentTrack.id;
    const eventId = playbackSessionEventId;
    countedPlaybackTrackId = trackId;

    try {
      const samples = listeningTimeBuffer.batch(eventId);
      if (samples.length > 0) {
        while (listeningTimeWrite) await listeningTimeWrite;
        if (!await flushListeningTime(samples)) throw new Error("Could not save listening time for this play.");
      }
      applyUpdatedTrack(await recordTrackPlay(trackId, eventId));
      statsClock = Date.now();
      statsRevision += 1;
    } catch (error) {
      if (playbackSessionEventId === eventId) {
        countedPlaybackTrackId = null;
        playbackError = error instanceof Error ? error.message : String(error);
      }
    }
  }

  function resetPlaybackListenSession(track: Track | null, startedAt = performance.now()) {
    void flushListeningTime();
    playbackSessionEventId = track ? crypto.randomUUID() : null;
    listeningTimeBuffer.start(track?.id ?? null, playbackSessionEventId);
    playbackListenProgress.reset(startedAt);
    countedPlaybackTrackId = null;
  }

  function observeListenPosition(status: PlaybackStatus, requestedAt?: number) {
    if (!currentTrack || status.filePath !== currentTrack.filePath) return;
    const endedAt = status.durationSeconds ?? currentTrack.durationSeconds;
    const now = performance.now();
    observeListeningAdvance(
      status.hasEnded && endedAt !== null ? endedAt : status.positionSeconds,
      now,
      isPlaying || status.isPlaying,
      requestedAt,
    );
    if (status.hasEnded || !status.isPlaying) void flushListeningTime();
  }

  function observeGaplessEnd(changedAt: number) {
    const duration = durationSeconds ?? currentTrack?.durationSeconds ?? null;
    if (duration !== null) observeListeningAdvance(duration, changedAt, isPlaying);
    void maybeRecordTrackPlay(true);
  }

  function observeListeningAdvance(position: number, observedAt: number, wasPlaying: boolean, requestedAt?: number) {
    if (!currentTrack || ignoredTrackIds.has(currentTrack.id)) {
      playbackListenProgress.forgetPosition();
      return;
    }
    const advance = playbackListenProgress.observe(position, observedAt, wasPlaying, requestedAt);
    listeningTimeBuffer.observe(advance, performance.timeOrigin + observedAt);
  }

  async function sampleListenBeforeTrackChange() {
    const path = currentTrack?.filePath;
    if (!path || !isPlaying || playbackSeekCoordinator.hasPendingSeek(path)) return;
    try {
      const requestedAt = performance.now();
      const status = await getPlaybackStatus();
      if (status.filePath === path) {
        observeListenPosition(status, requestedAt);
        void maybeRecordTrackPlay(status.hasEnded);
      } else if (status.filePath) {
        observeGaplessEnd(performance.now() - Math.max(0, status.positionSeconds) * 1000);
      }
    } catch {
      // Opening the next track remains possible when the old player has failed.
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

    libraryTracks = updatedTracks;
    playbackQueue = playbackQueue.map((track) => tracksById.get(track.id) ?? track);

    if (currentTrack) {
      currentTrack = tracksById.get(currentTrack.id) ?? currentTrack;
      currentTrackIndex = tracks.findIndex((track) => track.id === currentTrack?.id);
    }
  }

  function applyUpdatedTrack(updatedTrack: Track) {
    libraryTracks = libraryTracks.map((track) => (track.id === updatedTrack.id ? updatedTrack : track));
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
    isQueueOpen = true;
    await playQueuedTrackAtIndex(0, { tracks: shuffledMix, shuffle: true, order: shuffledMix.map((_, index) => index) });
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

  function handleClearQueue(shouldConfirm = false) {
    if (playbackQueue.length === 0) {
      return;
    }

    if (shouldConfirm && !window.confirm("Clear the current Up Next queue? Playback will not delete any files.")) {
      return;
    }

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

  function handleRemoveQueueTrack(event: MouseEvent, queueIndex: number) {
    event.stopPropagation();

    if (queueIndex < 0 || queueIndex >= playbackQueue.length || queueIndex === currentQueueIndex) {
      return;
    }

    const previousLength = playbackQueue.length;
    const nextQueue = playbackQueue.filter((_, index) => index !== queueIndex);

    if (nextQueue.length === 0) {
      playbackQueue = [];
      currentQueueIndex = null;
      shuffledQueueOrder = [];
      return;
    }

    playbackQueue = nextQueue;

    if (currentQueueIndex !== null && queueIndex < currentQueueIndex) {
      currentQueueIndex -= 1;
    }

    shuffledQueueOrder = isShuffleEnabled
      ? normalizedQueueOrder(shuffledQueueOrder, previousLength)
          .filter((index) => index !== queueIndex)
          .map((index) => index > queueIndex ? index - 1 : index)
      : [];
  }

  async function playTrackSet(libraryTracks: Track[], shouldShuffle = false) {
    const available = libraryTracks.filter(isQueueEligible);
    const queue = shouldShuffle ? shuffleTracks(available) : orderedContextTracks(available);

    if (queue.length === 0) {
      return;
    }

    await playQueuedTrackAtIndex(0, { tracks: queue, shuffle: false, order: [] });
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
    if (!currentTrack || isStartingTrack || isChangingPlayback || isTagWriteSuspendingPlayback) return;

    if (!isPlaying && hasCurrentTrackEnded) {
      if (currentQueueIndex === null) {
        await playQueuedTrackAtIndex(0, { tracks: [currentTrack], shuffle: isShuffleEnabled, order: isShuffleEnabled ? [0] : [] });
      } else {
        await playQueuedTrackAtIndex(currentQueueIndex);
      }
      return;
    }

    await changePlayback(isPlaying ? "pause" : "resume");
  }

  async function handleMediaPlay() {
    if (currentTrack && !isPlaying) await handleTogglePlayback();
  }

  async function handleMediaPause() {
    if (currentTrack && isPlaying) await changePlayback("pause");
  }

  async function handleMediaStop() {
    await changePlayback("stop");
  }

  async function changePlayback(action: "pause" | "resume" | "stop") {
    if (!currentTrack || isStartingTrack || isChangingPlayback || isTagWriteSuspendingPlayback) return;

    const trackPath = currentTrack.filePath;
    const startId = playbackStartId;
    const tagWriteGeneration = tagWritePlaybackGeneration;
    const positionSnapshot = playbackSeekCoordinator.capturePositionUpdate(trackPath);
    const isCurrent = () => startId === playbackStartId && currentTrack?.filePath === trackPath
      && tagWriteGeneration === tagWritePlaybackGeneration;
    playbackControlRevision += 1;
    isChangingPlayback = true;
    playbackError = null;

    try {
      if (action === "resume" && videoBackendStatus?.videoId) {
        await saveActiveVideoProgress(true);
        videoBackendStatus = await stopVideo();
        if (!isCurrent()) return;
      }
      const status = action === "resume" ? await resumePlayback() : await pausePlayback();
      if (!isCurrent()) return;
      if (status.filePath !== trackPath) {
        await acceptGaplessTransition(status);
        return;
      }

      applyPlaybackStatus(status, {
        applyPosition: playbackSeekCoordinator.allowsPositionUpdate(positionSnapshot),
      });
      if (action === "stop" && playbackSeekCoordinator.allowsPositionUpdate(positionSnapshot)) {
        await handleSeek(0);
      }
      await maybeRecordTrackPlay();
    } catch (error) {
      if (isCurrent()) playbackError = error instanceof Error ? error.message : String(error);
    } finally {
      isChangingPlayback = false;
    }
  }

  async function handleSeek(nextPositionSeconds: number) {
    const duration = durationSeconds ?? currentTrack?.durationSeconds ?? null;

    if (!currentTrack || isStartingTrack || isTagWriteSuspendingPlayback || !duration || duration <= 0 || !Number.isFinite(nextPositionSeconds)) {
      return;
    }

    const clampedPositionSeconds = Math.min(Math.max(nextPositionSeconds, 0), duration);
    const playbackTrackPath = currentTrack.filePath;
    playbackError = null;
    playbackSeekCoordinator.request(playbackTrackPath, clampedPositionSeconds);
    playbackListenProgress.forgetPosition();
    positionSeconds = clampedPositionSeconds;
    hasCurrentTrackEnded = false;

    if (!playbackSeekLoop) {
      playbackSeekLoop = drainPlaybackSeekQueue();
    }

    await playbackSeekLoop;
  }

  function handleLyricLineSeek(timeSeconds: number) {
    void handleSeek(Math.max(0, timeSeconds + lyricsOffsetSeconds));
  }

  async function drainPlaybackSeekQueue() {
    try {
      let request: PlaybackSeekRequest | null;

      while ((request = playbackSeekCoordinator.takeNext()) !== null) {
        await performPlaybackSeek(request);
      }
    } finally {
      playbackSeekLoop = null;
    }
  }

  async function performPlaybackSeek(request: PlaybackSeekRequest) {
    const startId = playbackStartId;
    const controlRevision = playbackControlRevision;
    try {
      const status = await seekPlayback(request.positionSeconds, request.contextKey);
      const shouldApply = playbackSeekCoordinator.complete(request);
      if (startId !== playbackStartId || currentTrack?.filePath !== request.contextKey || status.filePath !== request.contextKey) {
        return;
      }
      lastConfirmedPlaybackPositionSeconds = request.positionSeconds;
      if (!shouldApply) return;

      applyPlaybackStatus(status, {
        positionOverride: request.positionSeconds,
        applyTransport: controlRevision === playbackControlRevision && !isChangingPlayback,
      });
      hasCurrentTrackEnded = false;
      await maybeRecordTrackPlay();
    } catch (error) {
      await recoverFromFailedPlaybackSeek(request, error);
    }
  }

  async function recoverFromFailedPlaybackSeek(request: PlaybackSeekRequest, seekError: unknown) {
    let recoveredStatus: PlaybackStatus | null = null;
    const controlRevision = playbackControlRevision;

    try {
      const status = await getPlaybackStatus();

      if (status.filePath === request.contextKey) {
        recoveredStatus = status;
      }
    } catch {
      // The original seek error remains the actionable failure. If status
      // recovery is also unavailable, restore the last confirmed position.
    }

    const shouldApply = playbackSeekCoordinator.complete(request);

    if (!shouldApply || currentTrack?.filePath !== request.contextKey) {
      return;
    }

    const recoveryPositionSeconds = failedSeekRecoveryPosition(
      recoveredStatus?.positionSeconds ?? null,
      lastConfirmedPlaybackPositionSeconds,
    );

    if (recoveredStatus) {
      applyPlaybackStatus(recoveredStatus, {
        positionOverride: recoveryPositionSeconds,
        applyTransport: controlRevision === playbackControlRevision && !isChangingPlayback,
      });
    } else {
      positionSeconds = recoveryPositionSeconds;
    }

    playbackError = seekError instanceof Error ? seekError.message : String(seekError);
  }

  async function handleVolumeChange(nextVolume: number) {
    const requestId = ++volumeRequestId;
    isSettingVolume = true;
    playbackError = null;
    volume = nextVolume;

    try {
      const status = await setPlaybackVolume(nextVolume);
      if (requestId === volumeRequestId) volume = status.volume;
    } catch (error) {
      if (requestId === volumeRequestId) playbackError = error instanceof Error ? error.message : String(error);
    } finally {
      if (requestId === volumeRequestId) isSettingVolume = false;
    }
  }

  function applyPlaybackStatus(
    status: PlaybackStatus,
    options: { applyPosition?: boolean; positionOverride?: number; applyTransport?: boolean; requestedAt?: number } = {},
  ) {
    if (options.applyPosition !== false && options.positionOverride === undefined) observeListenPosition(status, options.requestedAt);
    if (options.applyTransport !== false) {
      isPlaying = status.isPlaying;
      if (status.isPlaying) hasCurrentTrackEnded = false;
    }

    if (options.applyPosition !== false) {
      const nextPositionSeconds = options.positionOverride ?? status.positionSeconds;
      positionSeconds = nextPositionSeconds;
      lastConfirmedPlaybackPositionSeconds = nextPositionSeconds;
    }

    durationSeconds = status.durationSeconds ?? currentTrack?.durationSeconds ?? null;
    if (!isSettingVolume) volume = status.volume;

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

  function albumInitials(album: Album) {
    const source = album.title.trim() || album.artist.trim() || "Album";
    const words = source.split(/\s+/).filter(Boolean);

    if (words.length === 1) {
      return words[0].slice(0, 2).toUpperCase();
    }

    return words.slice(0, 2).map((word) => word.slice(0, 1).toUpperCase()).join("");
  }

  function albumHeroDetails(album: Album, albumTracks: Track[]) {
    const details = [album.artist];

    if (album.year) {
      details.push(String(album.year));
    }

    details.push(songCountLabel(albumTracks.length));
    if (showAlbumPlayCounts) details.push(activityLabel(album));

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

  function trackCoverPath(track: Track): string | null {
    const cached = libraryTracksById.get(track.id);
    return cached ? cached.coverArtPath : track.coverArtPath;
  }

  async function changeAlbumCover(albumId: string, reset = false) {
    if (editingCoverForAlbum) return;
    editingCoverForAlbum = albumId;
    albumCoverFeedback = null;
    try {
      const imagePath = reset ? null : await chooseCoverImage();
      if (!reset && !imagePath) return;
      customAlbumCovers = await setAlbumCover(albumId, imagePath);
      albumCoverFeedback = { albumId, message: reset ? "Original artwork restored." : "Album artwork saved in Cassette.", error: false };
    } catch (error) {
      albumCoverFeedback = { albumId, message: error instanceof Error ? error.message : String(error), error: true };
    } finally {
      editingCoverForAlbum = null;
    }
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

  function albumHasTracks(album: Album) {
    return album.trackCount > 0;
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

  function formatByteCount(bytes: number) {
    if (bytes < 1024) {
      return `${bytes} B`;
    }

    const units = ["KB", "MB", "GB"];
    let value = bytes / 1024;
    let unitIndex = 0;

    while (value >= 1024 && unitIndex < units.length - 1) {
      value /= 1024;
      unitIndex += 1;
    }

    return `${value.toFixed(value >= 10 ? 1 : 2)} ${units[unitIndex]}`;
  }

  function lyricBreakLabel(cue: SyncedLyricCue | null) {
    if (!cue) {
      return "Intro before lyrics";
    }

    const breakKind = cue.breakSource === "instrumental" ? "Instrumental break" : "Lyrics break";
    return `${breakKind} at ${formatPlaybackTime(Math.max(0, cue.timeSeconds + lyricsOffsetSeconds))}`;
  }

  function lyricsKindLabel(lyrics: TrackLyrics) {
    if (lyrics.source === "edited") return lyrics.kind === "synced" ? "Edited synced lyrics" : "Edited plain lyrics";
    if (lyrics.source === "lrclib") {
      return lyrics.kind === "synced" ? "Cached LRCLIB synced" : "Cached LRCLIB plain";
    }

    return lyrics.kind === "synced" ? "Local LRC" : "Local TXT";
  }

  function cachedLyricsStatus(lyrics: TrackLyrics) {
    const selected = [lyrics.selectedArtistName, lyrics.selectedTrackName]
      .filter((value): value is string => Boolean(value))
      .join(" - ");
    const album = lyrics.selectedAlbumName ? ` · ${lyrics.selectedAlbumName}` : "";
    const selectedLabel = selected ? ` · ${selected}${album}` : "";
    const offsetLabel = Math.abs(lyrics.offsetSeconds) > 0.01 ? ` · Offset: ${formatSignedSeconds(lyrics.offsetSeconds)}` : "";

    if (lyrics.fetchedAt) {
      return `Cached from LRCLIB · ${formatDateTime(lyrics.fetchedAt)}${selectedLabel}${offsetLabel}`;
    }

    return `Cached from LRCLIB${selectedLabel}${offsetLabel}`;
  }

  function lyricsStatusText(lyrics: TrackLyrics) {
    const offsetLabel = Math.abs(lyrics.offsetSeconds) > 0.01 ? ` · Offset: ${formatSignedSeconds(lyrics.offsetSeconds)}` : "";
    return `${lyricsKindLabel(lyrics)}${offsetLabel}`;
  }

  function formatSignedSeconds(seconds: number) {
    const roundedSeconds = Math.round(seconds * 10) / 10;
    return `${roundedSeconds >= 0 ? "+" : ""}${roundedSeconds.toFixed(1)}s`;
  }

  function lyricsResultDurationLabel(result: LrclibLyricsResult) {
    if (result.durationDifferenceSeconds === null) {
      return result.durationSeconds === null ? null : `Duration: ${formatPlaybackTime(result.durationSeconds)}`;
    }

    return `Duration: ${formatSignedSeconds(result.durationDifferenceSeconds)}`;
  }

  function lyricsResultAvailabilityLabel(result: LrclibLyricsResult) {
    if (result.hasSyncedLyrics) {
      return "Synced lyrics available";
    }

    return result.hasPlainLyrics ? "Plain lyrics available" : "No lyrics text";
  }

  function lyricsMatchLabel(value: string, field: "title" | "artist") {
    if (value === "Exact match") {
      return `Exact ${field} match`;
    }

    if (value === "Close match") {
      return `Close ${field} match`;
    }

    if (value === "Different") {
      return `Different ${field}`;
    }

    return `${field[0].toUpperCase()}${field.slice(1)}: ${value}`;
  }

  function lyricsResultMatchLabels(result: LrclibLyricsResult) {
    return [
      lyricsMatchLabel(result.titleMatch, "title"),
      lyricsMatchLabel(result.artistMatch, "artist"),
      lyricsResultDurationLabel(result),
      lyricsResultAvailabilityLabel(result),
      result.source,
    ].filter((label): label is string => Boolean(label));
  }

  function statsActivity(activity: ListeningActivity) {
    return statsUseListeningTime ? activity.listenedMs ?? 0 : activity.playCount;
  }

  function trackActivityLabel(track: Track) {
    const hasVisibleActivity = (showPlayCounts && track.playCount > 0)
      || (showListeningTime && ((track.listenedMs ?? 0) > 0 || track.playCount > 0));
    return hasVisibleActivity ? activityLabel(track) : "";
  }

  function playCountLabel(count: number) {
    return `${count.toLocaleString()} ${count === 1 ? "play" : "plays"}`;
  }

  function activityLabel({ playCount, listenedMs = 0 }: ListeningActivity) {
    const labels = [];
    if (showPlayCounts) labels.push(playCountLabel(playCount));
    if (showListeningTime) labels.push(listeningTotalLabel(listenedMs, playCount));
    return labels.join(" · ");
  }

  function statsChartValueLabel(amount: number) {
    return statsUseListeningTime ? listeningDurationLabel(amount) : playCountLabel(amount);
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

  function overviewTracks(libraryTracks: Track[]) {
    const recent = recentlyPlayed(libraryTracks);
    const recentIds = new Set(recent.map((track) => track.id));
    const fallback = orderedContextTracks(libraryTracks).filter((track) => !recentIds.has(track.id));

    return [...recent, ...fallback].slice(0, 10);
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

  function totalTrackDurationLabel(libraryTracks: Track[]) {
    const knownDurations = libraryTracks
      .map((track) => track.durationSeconds)
      .filter((duration): duration is number => duration !== null);

    if (knownDurations.length === 0) {
      return null;
    }

    return formatDurationSummary(knownDurations.reduce((total, duration) => total + duration, 0));
  }

  function playlistMetaLabel(playlist: Playlist) {
    const playlistTracks = tracksForPlaylist(playlist);
    const durationLabel = totalTrackDurationLabel(playlistTracks);

    return [playlistTrackLabel(playlist), durationLabel].filter(Boolean).join(" · ");
  }

  function smartPlaylistMetaLabel(libraryTracks: Track[]) {
    return [songCountLabel(libraryTracks.length), totalTrackDurationLabel(libraryTracks)].filter(Boolean).join(" · ");
  }

  function playlistCoverTracks(libraryTracks: Track[]) {
    return libraryTracks.slice(0, 4);
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
    if (ignoredTrackIds.has(track.id) || unavailableTrackIds.has(track.id)) return false;
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
    const shuffled = libraryTracks.filter((track) => !ignoredTrackIds.has(track.id) && !unavailableTrackIds.has(track.id));

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

  function handleSongSortChange(value: string) {
    const nextSort = value as SongSortKey;

    songSort = nextSort;

    if ((nextSort === "recentlyAdded" || nextSort === "recentlyPlayed") && songSortDirection === "asc") {
      songSortDirection = "desc";
    }
  }

  function handleSongFormatFilterChange(value: string) {
    songFormatFilter = value;
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

    if (isAutoAdvance && repeatMode === "one" && isQueueEligible(playbackQueue[currentQueueIndex])) {
      return currentQueueIndex;
    }

    const nextOrderIndex = currentOrderIndex + 1;

    if (nextOrderIndex < playableQueueOrder.length) {
      return playableQueueOrder[nextOrderIndex];
    }

    if (repeatMode === "all") {
      return playableQueueOrder.find((index) => isQueueEligible(playbackQueue[index])) ?? null;
    }

    return null;
  }

  function getPreviousQueueIndex() {
    if (currentQueueIndex === null || playbackQueue.length === 0) {
      return null;
    }

    const previousOrderIndex = currentOrderIndex - 1;

    if (previousOrderIndex >= 0) {
      return playableQueueOrder[previousOrderIndex];
    }

    if (repeatMode === "all") {
      return [...playableQueueOrder].reverse().find((index) => isQueueEligible(playbackQueue[index])) ?? null;
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

      if (isActivitySortOrder(sortKey)) {
        return compareListeningActivity(left, right, sortKey)
          || (right.lastPlayedAt ?? 0) - (left.lastPlayedAt ?? 0)
          || compareText(left.title, right.title);
      }

      result = compareText(left.title, right.title);
      return applySortDirection(result, direction);
    });
  }

  function recentlyPlayed(libraryTracks: Track[]) {
    return rankRecentlyPlayedTracks(libraryTracks);
  }

  function handleStatsTrackSelect(track: Track, queue: Track[]) {
    const selection = resolveStatsPlayback(track.id, queue, libraryTracksById);
    if (selection) void handleTrackSelect(selection.track, selection.queue);
  }

  function openStatsTrackContextMenu(track: Track, queue: Track[], x: number, y: number) {
    const selection = resolveStatsPlayback(track.id, queue, libraryTracksById);
    if (selection) openTrackContextMenu(selection.track, selection.queue, x, y);
  }

  async function handleRemoveListen(listen: RecentListen, title: string) {
    if (removingListenId || !window.confirm(`Remove the listen for “${title}” recorded on ${formatDateTime(listen.playedAtUtc)}?`)) return;
    removingListenId = listen.eventId;
    removeListenError = null;
    try {
      await listeningTimeWrite;
      const removed = await removeListeningEvent(listen.eventId);
      if (removed) {
        if (removed.track) applyUpdatedTrack(removed.track);
        applyListeningTimeTotals({ [removed.trackId]: removed.listenedMs });
      }
      statsRevision += 1;
    } catch (error) {
      removeListenError = error instanceof Error ? error.message : String(error);
    } finally {
      removingListenId = null;
    }
  }

  async function handleExportListeningHistory() {
    if (historyBusy) return;
    historyBusy = true;
    historyError = null;
    historyMessage = null;
    try {
      const path = await chooseHistoryExportPath();
      if (!path) return;
      const result = await exportListeningHistory(path);
      historyMessage = `Exported ${result.eventCount} dated events and ${result.undatedPlays} undated legacy plays for ${result.trackCount} tracks. Existing backup files are never overwritten.`;
    } catch (error) {
      historyError = error instanceof Error ? error.message : String(error);
    } finally {
      historyBusy = false;
    }
  }

  async function handlePreviewListeningHistory() {
    if (historyBusy) return;
    historyBusy = true;
    historyError = null;
    historyMessage = null;
    try {
      const path = await chooseHistoryImportPath();
      if (!path) return;
      historyImportPreview = null;
      historyPreviewLimit = STATS_PAGE_SIZE;
      historyAssociationTargets = {};
      historyImportPath = path;
      historyImportPreview = await previewListeningHistoryImport(path);
    } catch (error) {
      historyError = error instanceof Error ? error.message : String(error);
    } finally {
      historyBusy = false;
    }
  }

  async function handleApplyListeningHistory() {
    if (!historyImportPath || !historyImportPreview || historyImportPreview.conflicts.length > 0 || historyBusy) return;
    historyBusy = true;
    historyError = null;
    historyMessage = null;
    try {
      const result = await importListeningHistory(historyImportPath, historyImportPreview.approvalToken);
      historyImportPreview = null;
      historyMessage = `Imported ${result.importedEvents} new events and ${result.importedUndatedPlays} undated plays. ${result.pendingTracks} track references remain pending; reopen this backup after adding music or associate an ambiguous match below.`;
      await loadLibraryCache();
      statsRevision += 1;
      try {
        historyImportPreview = await previewListeningHistoryImport(historyImportPath);
      } catch (error) {
        historyError = `History was imported, but the updated preview could not be loaded: ${error instanceof Error ? error.message : String(error)}`;
      }
    } catch (error) {
      historyError = error instanceof Error ? error.message : String(error);
    } finally {
      historyBusy = false;
    }
  }

  async function handleAssociateListeningHistory(pendingTrackId: string) {
    const targetTrackId = historyAssociationTargets[pendingTrackId];
    if (!targetTrackId || historyBusy) return;
    historyBusy = true;
    historyError = null;
    try {
      await associateListeningHistoryTrack(pendingTrackId, targetTrackId);
      await loadLibraryCache();
      statsRevision += 1;
      historyMessage = "Pending history associated with the selected audio-identical track.";
      if (historyImportPath) historyImportPreview = await previewListeningHistoryImport(historyImportPath);
    } catch (error) {
      historyError = error instanceof Error ? error.message : String(error);
    } finally {
      historyBusy = false;
    }
  }

  function mostPlayed(libraryTracks: Track[]) {
    return rankMostPlayedTracks(libraryTracks, statsActivity);
  }

  function statsSectionExpanded(section: StatsSectionId) {
    return statsDetailSection === section;
  }

  function openStatsSection(section: StatsSectionId) {
    statsDetailSection = section;
    statsVisibleLimits[section] = STATS_PAGE_SIZE;
    mainElement?.scrollTo({ top: 0 });
  }

  function closeStatsSection() {
    statsDetailSection = null;
    mainElement?.scrollTo({ top: 0 });
  }

  function loadMoreStats(section: StatsSectionId, total: number) {
    statsVisibleLimits[section] = nextStatsLimit(statsVisibleLimits[section], total);
  }

  function statsViewAllLabel(section: StatsSectionId, total: number) {
    if (statsSectionExpanded(section)) {
      return `${total} total`;
    }

    if (total > STATS_PREVIEW_LIMITS[section]) {
      return "View all";
    }

    return "";
  }

  function hasMoreStats(section: StatsSectionId, total: number) {
    return statsSectionExpanded(section) && statsVisibleLimits[section] < total;
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
        playCount: 0,        listenedMs: 0,
        songCount: 0,
      };

      current.playCount += track.playCount;
      current.listenedMs += track.listenedMs ?? 0;
      current.songCount += 1;
      statsByName.set(name, current);
    }

    return Array.from(statsByName.values())
      .filter((stat) => statsActivity(stat) > 0)
      .sort((left, right) =>
        statsActivity(right) - statsActivity(left)
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
        playCount: 0,        listenedMs: 0,
        songCount: 0,
      };

      current.playCount += track.playCount;
      current.listenedMs += track.listenedMs ?? 0;
      current.songCount += 1;
      statsByAlbumId.set(albumId, current);
    }

    return Array.from(statsByAlbumId.values())
      .filter((stat) => statsActivity(stat) > 0)
      .sort((left, right) =>
        statsActivity(right) - statsActivity(left)
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
          playCount: 0,          listenedMs: 0,
          songCount: 0,
        };

        current.playCount += track.playCount;
        current.listenedMs += track.listenedMs ?? 0;
        current.songCount += 1;
        statsByGenreName.set(genreName, current);
      }
    }

    return Array.from(statsByGenreName.values())
      .filter((stat) => statsActivity(stat) > 0)
      .sort((left, right) =>
        statsActivity(right) - statsActivity(left)
        || right.songCount - left.songCount
        || compareText(left.genre.name, right.genre.name),
      );
  }

  function sortAlbums(albums: Album[], sortKey: AlbumSortKey, direction: SortDirection) {
    if (isActivitySortOrder(sortKey)) {
      return sortAlbumsByActivity(albums, sortKey);
    }

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
    if (isActivitySortOrder(sortKey)) {
      return sortArtistsByActivity(artists, sortKey);
    }

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
    if (isActivitySortOrder(sortKey)) {
      return sortGenresByActivity(genres, sortKey);
    }

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
      videoTypeLabel(video.videoType),
      video.releaseOrCollection,
      video.descriptionOrNotes,
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

    if (activeView === "Videos") {
      return "";
    }

    return "";
  }

  function isSearchAvailable() {
    return searchPlaceholder().length > 0;
  }

  function viewTitle() {
    if (activeView === "Now Playing") {
      return currentTrack ? currentTrack.title : "Current Track";
    }

    return activeView;
  }

  function viewEyebrow() {
    if (activeView === "Now Playing") {
      return "Lyrics";
    }

    return activeView;
  }

  function viewStatus() {
    if (isScanning) {
      return scanProgress ? `Scanning… ${scanProgress.checked} audio files checked` : "Scanning your local music files…";
    }

    if (!hasLoadedCache) {
      return "Loading cached library...";
    }

    if (activeView === "Now Playing") {
      return currentTrack
        ? "Current song details and lyrics. Use the player bar for playback and queue controls."
        : "Select a song to view lyrics.";
    }

    if (activeView === "Albums") {
      return [
        songCountLabel(tracks.length),
        `${hasLoadedCache ? displayAlbums.length : 0} ${(hasLoadedCache ? displayAlbums.length : 0) === 1 ? "album" : "albums"}`,
        scanSummary ?? scannedFolder,
      ].filter(Boolean).join(" · ");
    }

    if (activeView === "Settings") {
      return scannedFolder
        ? `Library folder: ${scannedFolder}`
        : "No library folder is cached yet.";
    }

    if (activeView === "Videos") {
      if (isScanningVideos) {
        return "Scanning local video files...";
      }

      if (videoFolder) {
        return `${videos.length} ${videos.length === 1 ? "video" : "videos"} from ${videoFolder}`;
      }

      return "Add local music videos, concerts, interviews, documentaries, and related videos.";
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

  function openArtworkViewer(event: MouseEvent, path: string, title: string) {
    const source = localImageSource(path);
    if (!source || !(event.currentTarget instanceof HTMLElement)) return;
    // Snapshot the clicked cover; later track changes must not replace it.
    artworkViewer = { source, title, opener: event.currentTarget };
  }
</script>

<svelte:window onpointerdown={dismissAlbumTrackSelection} />

<svelte:head>
  <title>Cassette</title>
</svelte:head>

<div class="app-shell">
  <div class:lyrics-mode={activeView === "Now Playing"} class="workspace">
    <div class="workspace-navigation">
      <span class="brand-mark" aria-label="Cassette">C</span>
      <div class="view-history-controls" role="group" aria-label="View history">
        <button type="button" aria-label="Back to previous view" title="Back" disabled={!canNavigateBack} onclick={() => navigateViewHistory(-1)}>‹</button>
        <button type="button" aria-label="Forward to next view" title="Forward" disabled={!canNavigateForward} onclick={() => navigateViewHistory(1)}>›</button>
      </div>
    </div>
    {#if activeView !== "Now Playing"}
      <Sidebar items={visibleNavItems} active={activeView} onNavigate={handleNavigate} />
    {/if}

    <main
      class:album-detail-view={isAlbumDetailView}
      class:albums-landing-view={activeView === "Albums" && !selectedAlbum}
      class:artist-detail-view={isArtistDetailView}
      class:genre-detail-view={isGenreDetailView}
      class:lyrics-mode={activeView === "Now Playing"}
      class:playlist-detail-view={isPlaylistDetailView}
      class:playlists-view={activeView === "Playlists"}
      class:songs-library-view={activeView === "Songs"}
      class="home"
      bind:this={mainElement}
      tabindex="-1"
    >
      {#if activeView !== "Now Playing" && activeView !== "Stats" && activeView !== "CD Rip" && activeView !== "Settings" && activeView !== "Playlists" && !isAlbumDetailView && !isArtistDetailView && !isGenreDetailView}
        <header class="home-header">
          <div>
            <p class="eyebrow">{viewEyebrow()}</p>
            <h2>{viewTitle()}</h2>
            <p class="scan-status">{viewStatus()}</p>
          </div>
          <div class="home-header-actions">
            {#if ENABLE_EXPERIMENTAL_VIDEOS && activeView === "Videos"}
              <button type="button" disabled={isScanningVideos} onclick={() => void (videoFolder ? handleRescanVideos() : handleAddVideoFolder())}>
                {isScanningVideos ? "Scanning..." : videoFolder ? "Rescan Videos" : "Add Video Folder"}
              </button>
            {:else}
              <button type="button" disabled={isScanning} onclick={() => void handleScanLibrary()}>
                {isScanning ? "Scanning..." : scannedFolder ? "Rescan Library" : "Scan Library"}
              </button>
            {/if}
          </div>
        </header>
      {/if}

      {#if isSearchAvailable() && !isAlbumDetailView && !isArtistDetailView && !isGenreDetailView && !isPlaylistDetailView}
        <div class="search-bar">
          {#if activeView === "Songs"}
            <input
              type="search"
              bind:value={songsSearchQuery}
              placeholder={searchPlaceholder()}
              aria-label={searchPlaceholder()}
              onkeydown={handleSearchKeydown}
            />
          {:else}
            <input
              type="search"
              bind:value={searchQuery}
              placeholder={searchPlaceholder()}
              aria-label={searchPlaceholder()}
              onkeydown={handleSearchKeydown}
            />
          {/if}
          {#if normalizedSearchQuery}
            <button type="button" aria-label="Clear search" onclick={clearSearch}>Clear</button>
          {/if}
        </div>
      {/if}

      {#if libraryRecoveryNotice}
        <div class="scan-error" role="alert">{libraryRecoveryNotice}</div>
      {/if}
      {#if scanError}
        <div class="scan-error" role="alert">{scanError}</div>
      {/if}
      {#if playbackError}
        <div class="scan-error" role="alert">{playbackError}</div>
      {/if}
      {#if ignoredTracksError}
        <div class="scan-error" role="alert">{ignoredTracksError}</div>
      {/if}
      {#if activeView === "Videos" && videoError}
        <div class="scan-error" role="alert">{videoError}</div>
      {:else if activeView === "Videos" && videoMessage}
        <div class="scan-error status-message" role="status">{videoMessage}</div>
      {/if}
      {#if activeView !== "Playlists" && playlistError}
        <div class="scan-error" role="alert">{playlistError}</div>
      {:else if activeView !== "Playlists" && playlistMessage}
        <div class="scan-error status-message" role="status">{playlistMessage}</div>
      {/if}

      {#if activeView === "Now Playing"}
        <section class="lyrics-view" aria-labelledby="lyrics-view-title">
          {#if currentTrack}
            <header class="lyrics-view-top">
              <div class="lyrics-header-spacer" aria-hidden="true"></div>
              <div class="lyrics-title-block">
                <h2 id="lyrics-view-title">Lyrics</h2>
              </div>
              <button
                class:active={isLyricsOptionsOpen}
                class="lyrics-options-button"
                type="button"
                onclick={toggleLyricsOptions}
              >
                {isLyricsOptionsOpen ? "Hide Options" : "Lyrics Options"}
                {#if Math.abs(lyricsOffsetSeconds) > 0.01}
                  <span>{formatSignedSeconds(lyricsOffsetSeconds)}</span>
                {/if}
              </button>
            </header>

            <div class="lyrics-experience">
              <aside class="lyrics-sidecar" aria-label="Current song">
                {#if currentTrackCoverArtSrc}
                  <img
                    class="lyrics-cover-ambient"
                    src={currentTrackCoverArtSrc}
                    alt=""
                    aria-hidden="true"
                    onload={showLoadedImage}
                    onerror={hideBrokenImage}
                  />
                {/if}
                {#if currentTrackCoverArtSrc && currentTrackCoverArtPath}
                  <button
                    class="lyrics-cover cover-open-button"
                    type="button"
                    aria-label={`View full cover art for ${currentTrack.title}`}
                    onclick={(event) => openArtworkViewer(event, currentTrackCoverArtPath ?? "", currentTrack?.title ?? "Current track")}
                  >
                    <img
                      src={currentTrackCoverArtSrc}
                      alt=""
                      onload={showLoadedImage}
                      onerror={hideBrokenImage}
                    />
                    <span></span>
                  </button>
                {:else}
                  <div class="lyrics-cover" aria-hidden="true"><span></span></div>
                {/if}
                <div class="lyrics-sidecar-copy">
                  <h3>{currentTrack.title}</h3>
                  <p>
                    <button type="button" onclick={handleNowPlayingArtistSelect}>
                      {currentTrack.artist ?? currentTrack.albumArtist ?? "Unknown Artist"}
                    </button>
                  </p>
                  {#if currentTrack.album}
                    <p>
                      <button type="button" onclick={handleNowPlayingAlbumSelect}>{currentTrack.album}</button>
                    </p>
                  {/if}
                  <div class="lyrics-sidecar-meta">
                    {#if currentTrack.year}
                      <span>{currentTrack.year}</span>
                    {/if}
                    <span>{currentTrack.extension.toUpperCase()}</span>
                    {#if !ignoredTrackIds.has(currentTrack.id) && trackActivityLabel(libraryTracksById.get(currentTrack.id) ?? currentTrack)}
                      <span>{trackActivityLabel(libraryTracksById.get(currentTrack.id) ?? currentTrack)}</span>
                    {/if}
                  </div>
                  {#if currentTrackGenres.length > 0}
                    <div class="lyrics-genre-links" aria-label="Genres">
                      {#each currentTrackGenres as genre}
                        <button type="button" onclick={() => handleNowPlayingGenreSelect(genre)}>{genre}</button>
                      {/each}
                    </div>
                  {/if}
                </div>
              </aside>

              <section class="lyrics-focus" aria-labelledby="lyrics-view-title">
                {#if lyricsLookupError || isAutoFindingLyrics || (!currentLyrics && lyricsLookupMessage)}
                  <p class:error={lyricsLookupError} class="lyrics-status-line">
                    {lyricsLookupError ?? (isAutoFindingLyrics ? "Searching lyrics..." : lyricsLookupMessage)}
                  </p>
                {/if}

                {#if isLoadingLyrics}
                  <div class="lyrics-empty-state" role="status">
                    <h3>Loading lyrics...</h3>
                    <p>Checking local and cached lyrics.</p>
                  </div>
                {:else if currentLyrics?.kind === "synced" && syncedLyricLines.length > 0}
                  <div class="synced-lyrics" bind:this={lyricsPanelElement}>
                    {#if hasLyricsIntro}
                      <button
                        class:active={syncedLyricsState.kind === "intro"}
                        class="lyrics-break-cue"
                        data-active={syncedLyricsState.kind === "intro" ? "true" : undefined}
                        type="button"
                        aria-label={lyricBreakLabel(null)}
                        aria-current={syncedLyricsState.kind === "intro" ? "true" : undefined}
                        onclick={() => void handleSeek(0)}
                      >
                        <span aria-hidden="true">♪</span>
                      </button>
                    {/if}
                    {#each syncedLyricLines as line, index}
                      {#if startsSyncedLyricBreak(syncedLyricLines, index)}
                        <button
                          class:active={syncedLyricsState.kind === "break" && index === syncedLyricsState.activeCueIndex}
                          class="lyrics-break-cue"
                          data-active={syncedLyricsState.kind === "break" && index === syncedLyricsState.activeCueIndex ? "true" : undefined}
                          type="button"
                          aria-label={lyricBreakLabel(line)}
                          aria-current={syncedLyricsState.kind === "break" && index === syncedLyricsState.activeCueIndex ? "true" : undefined}
                          onclick={() => handleLyricLineSeek(line.timeSeconds)}
                        >
                          <span aria-hidden="true">♪</span>
                        </button>
                      {:else}
                        <button
                          class:active={syncedLyricsState.kind === "lyric" && index === syncedLyricsState.activeCueIndex}
                          data-active={syncedLyricsState.kind === "lyric" && index === syncedLyricsState.activeCueIndex ? "true" : undefined}
                          type="button"
                          aria-current={syncedLyricsState.kind === "lyric" && index === syncedLyricsState.activeCueIndex ? "true" : undefined}
                          onclick={() => handleLyricLineSeek(line.timeSeconds)}
                        >
                          {line.text}
                        </button>
                      {/if}
                    {/each}
                  </div>
                {:else if plainLyricStanzas.length > 0}
                  <!-- svelte-ignore a11y_no_noninteractive_tabindex (Allow keyboard scrolling in the lyrics pane.) -->
                  <div class="plain-lyrics" role="region" aria-label="Plain lyrics" tabindex="0">
                    {#each plainLyricStanzas as stanza}
                      <p>{stanza}</p>
                    {/each}
                  </div>
                {:else if currentLyrics?.kind === "synced"}
                  <div class="lyrics-empty-state">
                    <h3>No lyric lines found</h3>
                    <p>This .lrc file exists, but it does not contain timestamped lyric lines.</p>
                  </div>
                {:else}
                  <div class="lyrics-empty-state">
                    <h3>{isAutoFindingLyrics ? "Searching lyrics..." : "No lyrics found"}</h3>
                    <p>{isAutoFindingLyrics ? "Cassette is checking LRCLIB in the background." : "Search LRCLIB manually or add a local .lrc/.txt file next to the song."}</p>
                    <button class="auto-lyrics-button" type="button" disabled={isAutoFindingLyrics} onclick={() => void handleAutoFindLyrics({ mode: "manual" })}>
                      {isAutoFindingLyrics ? "Searching..." : "Search Lyrics"}
                    </button>
                  </div>
                {/if}
              </section>
            </div>

            {#if isLyricsOptionsOpen}
              <section class="lyrics-options-drawer" aria-label="Lyrics options">
                <div class="lyrics-options-header">
                  <div>
                    <p class="eyebrow">Lyrics Options</p>
                    <h3>Source and timing</h3>
                  </div>
                  <button type="button" aria-label="Close lyrics options" onclick={toggleLyricsOptions}>Close</button>
                </div>

                <div class="lyrics-option-actions">
                  <button type="button" disabled={isLoadingLyrics} onclick={openLyricsEditor}>
                    {currentLyrics ? "Edit lyrics" : "Add lyrics"}
                  </button>
                  {#if currentLyrics?.source === "edited"}
                    <button type="button" onclick={() => void handleRestoreOriginalLyrics()}>Restore original lyrics</button>
                  {:else}
                  <button type="button" disabled={isAutoFindingLyrics} onclick={() => void handleAutoFindLyrics({ mode: "manual", replaceCached: true })}>
                    {isAutoFindingLyrics ? "Searching..." : currentLyrics ? "Change Lyrics" : "Search Lyrics"}
                  </button>
                  {/if}
                  {#if currentLyrics?.source === "lrclib"}
                    <button type="button" onclick={() => void handleRemoveCachedLyrics()}>
                      Remove Cached Lyrics
                    </button>
                  {/if}
                </div>

                <div class="lyrics-source-summary">
                  <div>
                    <span>Source</span>
                    <strong>{lyricsBadgeLabel ?? (isAutoFindingLyrics ? "Searching lyrics..." : "No lyrics loaded")}</strong>
                  </div>
                  {#if cachedLyricsLabel}
                    <p>{cachedLyricsLabel}</p>
                  {/if}
                  {#if lyricsLookupMessage}
                    <p>{lyricsLookupMessage}</p>
                  {:else if lyricsLookupError}
                    <p class="error">{lyricsLookupError}</p>
                  {/if}
                </div>

                {#if currentLyrics?.kind === "synced"}
                  <div class="lyrics-offset-controls" aria-label="Lyrics Offset">
                    <span>Lyrics Offset</span>
                    <button type="button" disabled={isSavingLyricsOffset} onclick={() => void handleLyricsOffsetChange(lyricsOffsetSeconds - 1)}>
                      -1.0s
                    </button>
                    <button type="button" disabled={isSavingLyricsOffset} onclick={() => void handleLyricsOffsetChange(lyricsOffsetSeconds - 0.5)}>
                      -0.5s
                    </button>
                    <button type="button" disabled={isSavingLyricsOffset} onclick={() => void handleLyricsOffsetChange(0)}>
                      Reset
                    </button>
                    <button type="button" disabled={isSavingLyricsOffset} onclick={() => void handleLyricsOffsetChange(lyricsOffsetSeconds + 0.5)}>
                      +0.5s
                    </button>
                    <button type="button" disabled={isSavingLyricsOffset} onclick={() => void handleLyricsOffsetChange(lyricsOffsetSeconds + 1)}>
                      +1.0s
                    </button>
                    <input
                      aria-label="Lyrics offset seconds"
                      type="number"
                      min="-5"
                      max="5"
                      step="0.1"
                      value={lyricsOffsetSeconds.toFixed(1)}
                      disabled={isSavingLyricsOffset}
                      onchange={(event) => void handleLyricsOffsetChange(Number(event.currentTarget.value))}
                    />
                  </div>
                {/if}

                {#if isLyricsPickerOpen}
                  <div class="lyrics-result-picker" role="list" aria-label="Select lyrics result">
                    <div class="lyrics-result-picker-heading">
                      <h4>Select lyrics result</h4>
                      <p>Synced LRCLIB results are listed first. Choose the version that best matches this track.</p>
                    </div>
                    {#each lyricsPickerResults as result}
                      <button
                        type="button"
                        class="lyrics-result-card"
                        disabled={isSavingLyricsSelection}
                        onclick={() => void handleSelectLyricsResult(result)}
                      >
                        <span class="lyrics-result-title">{result.trackName}</span>
                        <span class="lyrics-result-artist">{result.artistName}</span>
                        {#if result.albumName}
                          <span class="lyrics-result-album">{result.albumName}</span>
                        {/if}
                        <span class="lyrics-result-tags">
                          {#each lyricsResultMatchLabels(result) as label}
                            <span>{label}</span>
                          {/each}
                        </span>
                      </button>
                    {/each}
                  </div>
                {/if}
              </section>
            {/if}
          {:else}
            <header class="lyrics-view-top">
              <div class="lyrics-header-spacer" aria-hidden="true"></div>
              <div class="lyrics-title-block">
                <h2 id="lyrics-view-title">Lyrics</h2>
              </div>
            </header>
            <div class="lyrics-empty-shell">
              <div class="lyrics-empty-art" aria-hidden="true">L</div>
              <div>
                <p class="eyebrow">Lyrics</p>
                <h3>No current song</h3>
                <p>Play a track, then open lyrics again from the bottom player.</p>
              </div>
            </div>
          {/if}
        </section>
      {:else if activeView === "Stats"}
        <section class="stats-page" aria-label="Listening and library statistics">
          {#snippet statsTrackArt(track: Track)}
            <span class="stats-mini-cover" aria-hidden="true">
              <span>♪</span>
              {#if track.coverArtPath}
                <img src={localImageSource(track.coverArtPath) ?? ""} alt="" loading="lazy" onload={showLoadedImage} onerror={hideBrokenImage} />
              {/if}
            </span>
          {/snippet}
          <header class="stats-header">
          {#if statsDetailSection}
            <button class="back-button stats-back-button" type="button" onclick={closeStatsSection}>Back to Stats</button>
          {:else}
          <h2>Stats</h2>
          <div class="stats-period-toolbar">
            <div class="stats-period-field">
              <label for="stats-period">Listening period</label>
              <select id="stats-period" bind:value={statsPeriod} onchange={() => { statsClock = Date.now(); }}>
              <option value="all">All time</option>
              <option value="today">Today</option>
              <option value="week">This week</option>
              <option value="month">This month</option>
              <option value="previousMonth">Previous month</option>
              <option value="year">This year</option>
              <option value="previousYear">Previous year</option>
              <option value="custom">Custom date range</option>
              </select>
            </div>
            {#if listeningActivityDisplay === "2"}
              <div class="stats-period-field">
                <label for="stats-metric">Chart, rankings and history</label>
                <select id="stats-metric" bind:value={statsMetric}>
                  <option value="plays">Play count</option>
                  <option value="time">Listening time</option>
                </select>
              </div>
            {/if}
            {#if statsPeriod === "previousMonth"}
              <div class="stats-period-field">
                <label for="stats-month">Month</label>
                <input id="stats-month" type="month" bind:value={statsSelectedMonth} />
              </div>
            {:else if statsPeriod === "previousYear"}
              <div class="stats-period-field">
                <label for="stats-year">Year</label>
                <input id="stats-year" type="text" inputmode="numeric" maxlength="4" placeholder={String(new Date(statsClock).getFullYear() - 1)} bind:value={statsSelectedYear} />
              </div>
            {/if}
            {#if statsPeriod === "custom"}
              <div class="stats-period-field">
                <label for="stats-from">From</label>
                <input id="stats-from" type="date" bind:value={statsCustomFrom} />
              </div>
              <div class="stats-period-field">
                <label for="stats-to">Through</label>
                <input id="stats-to" type="date" bind:value={statsCustomTo} />
              </div>
            {/if}
          </div>
          {/if}
          </header>
          {#if !statsRange}
            <p class="stats-status" role="alert">Choose valid dates with “From” on or before “Through” (up to 100 years).</p>
          {:else if statsError}
            <p class="stats-status" role="alert">{statsError}</p>
          {:else if statsLoading && !statsSnapshot}
            <p class="stats-status" aria-live="polite">Loading listening statistics…</p>
          {/if}
          {#if !statsDetailSection}
          <section class="stats-chart-panel" aria-labelledby="daily-plays-title">
          <dl class="stats-overview-grid" class:both-activities={listeningActivityDisplay === "2"} aria-label="Selected listening period">
            {#each statsOverview as metric (metric.label)}
              {@const isActiveDays = metric.label === "Active days"}
              <div class="stats-overview-card" class:highlight={metric.label === "Plays" || metric.label === "Listening time"}>
                <dt>{metric.label}</dt>
                <dd>{statsRange && !statsError && (statsSnapshot || (!statsLoading && statsPeriod === "all" && !isActiveDays)) ? metric.value.toLocaleString() : "—"}</dd>
                {#if isActiveDays && statsPeriod === "all"}<span>Last 30 days</span>{/if}
              </div>
            {/each}
          </dl>

            <div class="stats-chart-heading">
              <h2 id="daily-plays-title">{statsUseListeningTime ? "Daily listening time" : "Daily plays"}</h2>
              <span>{statsChartDateLabel}</span>
            </div>
            {#if statsSnapshot && statsRange && statsActiveDays > 0}
              <div class="stats-chart">
                <div class="stats-chart-scale" aria-hidden="true">
                  {#each [4, 3, 2, 1, 0] as tick}
                    <span>{statsUseListeningTime ? listeningDurationLabel(tick * statsChartStep) : (tick * statsChartStep).toLocaleString()}</span>
                  {/each}
                </div>
                <div class="stats-chart-scroll" role="region" aria-labelledby="daily-plays-title">
                <div class="stats-chart-bars" style:--chart-days={statsChartDates.length} class:dense={statsChartDates.length > 35}>
                  {#each statsChartDates as day, index (day)}
                    {@const count = statsDailyActivity[index + statsChartOffset] ?? 0}
                    {@const caption = `${day}: ${statsChartValueLabel(count)}`}
                    <button type="button" class="stats-chart-day" aria-pressed={statsSelectedDay === day} aria-label={caption} title={caption} onclick={() => {
                      statsSelectedDay = day;
                    }}>
                      <span class="stats-chart-plot" aria-hidden="true">
                        <span class="stats-chart-bar" class:zero={!count} style:--bar-height={`${count / (4 * statsChartStep) * 100}%`}></span>
                      </span>
                      <span class="stats-chart-date" aria-hidden="true">
                        {#if index % Math.max(1, Math.ceil(statsChartDates.length / 6)) === 0}
                          {statsDayFormat.format(new Date(day))}
                        {/if}
                      </span>
                    </button>
                  {/each}
                </div>
                </div>
              </div>
              {#if statsSelectedDayLabel}<p class="stats-coverage-note" aria-live="polite">{statsSelectedDayLabel}</p>{/if}
              {#if statsRange.chartDates.length > 90}
                <div class="stats-pagination">
                  <button type="button" disabled={statsChartOffset === 0} onclick={() => { statsChartOffset = Math.max(0, statsChartOffset - 90); statsSelectedDay = null; }}>Earlier days</button>
                  <button type="button" disabled={statsChartOffset + 90 >= statsRange.chartDates.length} onclick={() => { statsChartOffset += 90; statsSelectedDay = null; }}>Later days</button>
                </div>
              {/if}
            {:else if statsSnapshot}
              <p class="stats-chart-empty">{statsUseListeningTime ? "No listening time recorded in this period." : "No plays with dates in this period."}</p>
            {/if}
            {#if statsSnapshot}
              <details class="stats-coverage">
                <summary>Tracking coverage</summary>
                <p class="stats-coverage-note">
                  {#if statsUseListeningTime}
                    Listening time includes recorded sessions with at least five seconds of playback. Older plays may have no saved duration. {statsPeriod === "all" ? "The chart covers the last 30 days." : ""}
                  {:else if statsPeriod === "all"}
                    All-time totals include {statsSnapshot.undatedLegacyPlays} plays without dates. The chart covers the last 30 days.
                  {:else}
                    This period includes plays with recorded dates.
                  {/if}
                  {#if !statsUseListeningTime && statsSnapshot.coverageSources.length > 0}
                    Detailed tracking began on {statsSnapshot.coverageSources.map((source) => source.detailedTrackingStartedAtUtc !== null ? new Date(source.detailedTrackingStartedAtUtc * 1000).toLocaleDateString() : "an unknown date").join(", ")}.
                  {/if}
                  {#if statsSnapshot.pendingTracks.length > 0}
                    History includes {statsSnapshot.pendingTracks.length} tracks outside the current library.
                  {/if}
                </p>
              </details>
            {/if}
          </section>
          {/if}

          {#if statsRange && (!statsLoading || statsSnapshot) && !statsError}
          <div class="stats-section-grid" class:full-list={statsDetailSection !== null}>
          {#if !statsDetailSection || statsDetailSection === "tracks"}
          <LibrarySection
            title="Top Tracks"
            viewAllLabel={statsViewAllLabel("tracks", allStatsTopTracks.length)}
            onViewAll={!statsDetailSection && allStatsTopTracks.length > STATS_PREVIEW_LIMITS.tracks ? () => openStatsSection("tracks") : undefined}
          >
            {#if statsTopTracks.length === 0}
              <p class="stats-empty">{statsUseListeningTime ? "No listening time recorded for these tracks." : "No track plays in this period."}</p>
            {:else if statsDetailSection === "tracks"}
              <TrackList
                activityLabel={trackActivityLabel}
                tracks={statsTopTracks}
                unavailableTrackIds={statsUnavailableIds}
                isScanning={false}
                selectedTrackId={currentTrack?.id}
                onTrackSelect={handleStatsTrackSelect}
                onTrackContextMenu={openStatsTrackContextMenu}
                onArtistSelect={handleTrackArtistSelect}
                onAlbumSelect={handleTrackAlbumSelect}
                onToggleFavorite={handleToggleFavorite}
              />
              {#if statsSectionExpanded("tracks")}
                <div class="stats-pagination" aria-live="polite">
                  <span>Showing {statsTopTracks.length} of {allStatsTopTracks.length}</span>
                  {#if hasMoreStats("tracks", allStatsTopTracks.length)}
                    <button type="button" onclick={() => loadMoreStats("tracks", allStatsTopTracks.length)}>Load 50 more</button>
                  {/if}
                </div>
              {/if}
            {:else}
              <ol class="stats-rank-list">
                {#each statsTopTracks as track, index (track.id)}
                  <li>
                    <button class="stats-rank-card" type="button" disabled={statsUnavailableIds.has(track.id)}
                      onclick={() => handleStatsTrackSelect(track, statsTopTracks)}
                      oncontextmenu={(event) => {
                        event.preventDefault();
                        openStatsTrackContextMenu(track, statsTopTracks, event.clientX, event.clientY);
                      }}>
                      <span class="stats-rank-number">{index + 1}</span>
                      {@render statsTrackArt(track)}
                      <span class="stats-rank-copy">
                        <strong title={track.title}>{track.title}</strong>
                        <small>{track.artist ?? track.albumArtist ?? "Unknown Artist"}</small>
                      </span>
                      <span class="stats-rank-count">{activityLabel(track)}</span>
                    </button>
                  </li>
                {/each}
              </ol>
            {/if}
          </LibrarySection>
          {/if}

            {#if !statsDetailSection || statsDetailSection === "artists"}
            <LibrarySection
              title="Top Artists"
              viewAllLabel={statsViewAllLabel("artists", allStatsTopArtists.length)}
              onViewAll={!statsDetailSection && allStatsTopArtists.length > STATS_PREVIEW_LIMITS.artists ? () => openStatsSection("artists") : undefined}
            >
              {#if statsTopArtists.length === 0}
                <p class="stats-empty">{statsUseListeningTime ? "No listening time recorded for these artists." : "No artist plays in this period."}</p>
              {:else}
                <div class="stats-rank-list">
                  {#each statsTopArtists as stat, index}
                    <button class="stats-rank-card" type="button" disabled={!displayArtists.some((artist) => artist.name === stat.name)} onclick={() => selectArtistName(stat.name)}>
                      <span class="stats-rank-number">{index + 1}</span>
                      <span class="artist-avatar stats-avatar" style={`--item-color: ${stat.color}`} aria-hidden="true">
                        {stat.name.slice(0, 1)}
                      </span>
                      <span class="stats-rank-copy">
                        <strong>{stat.name}</strong>
                        <small>{songCountLabel(stat.songCount)}</small>
                      </span>
                      <span class="stats-rank-count">{activityLabel(stat)}</span>
                    </button>
                  {/each}
                </div>
                {#if statsSectionExpanded("artists")}
                  <div class="stats-pagination" aria-live="polite">
                    <span>Showing {statsTopArtists.length} of {allStatsTopArtists.length}</span>
                    {#if hasMoreStats("artists", allStatsTopArtists.length)}
                      <button type="button" onclick={() => loadMoreStats("artists", allStatsTopArtists.length)}>Load 50 more</button>
                    {/if}
                  </div>
                {/if}
              {/if}
            </LibrarySection>
            {/if}

            {#if !statsDetailSection || statsDetailSection === "albums"}
            <LibrarySection
              title="Top Albums"
              viewAllLabel={statsViewAllLabel("albums", allStatsTopAlbums.length)}
              onViewAll={!statsDetailSection && allStatsTopAlbums.length > STATS_PREVIEW_LIMITS.albums ? () => openStatsSection("albums") : undefined}
            >
              {#if statsTopAlbums.length === 0}
                <p class="stats-empty">{statsUseListeningTime ? "No listening time recorded for these albums." : "No album plays in this period."}</p>
              {:else}
                <div class="stats-rank-list">
                  {#each statsTopAlbums as stat, index}
                    <button class="stats-rank-card" type="button" disabled={!displayAlbums.some((album) => album.id === stat.album.id)} onclick={() => handleAlbumSelect(stat.album)}>
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
                        <span class="album-art-disc"></span>
                      </span>
                      <span class="stats-rank-copy">
                        <strong>{stat.album.title}</strong>
                        <small>{stat.album.artist}</small>
                      </span>
                      <span class="stats-rank-count">{activityLabel(stat)}</span>
                    </button>
                  {/each}
                </div>
                {#if statsSectionExpanded("albums")}
                  <div class="stats-pagination" aria-live="polite">
                    <span>Showing {statsTopAlbums.length} of {allStatsTopAlbums.length}</span>
                    {#if hasMoreStats("albums", allStatsTopAlbums.length)}
                      <button type="button" onclick={() => loadMoreStats("albums", allStatsTopAlbums.length)}>Load 50 more</button>
                    {/if}
                  </div>
                {/if}
              {/if}
            </LibrarySection>
            {/if}

            {#if !statsDetailSection || statsDetailSection === "genres"}
            <LibrarySection
              title="Top Genres"
              viewAllLabel={statsViewAllLabel("genres", allStatsTopGenres.length)}
              onViewAll={!statsDetailSection && allStatsTopGenres.length > STATS_PREVIEW_LIMITS.genres ? () => openStatsSection("genres") : undefined}
            >
              {#if statsTopGenres.length === 0}
                <p class="stats-empty">{statsUseListeningTime ? "No listening time recorded for these genres." : "No genre plays in this period."}</p>
              {:else}
                <div class="stats-rank-list">
                  {#each statsTopGenres as stat, index}
                    <button class="stats-rank-card" type="button" disabled={!displayGenres.some((genre) => genre.name === stat.genre.name)} onclick={() => handleGenreSelect(stat.genre)}>
                      <span class="stats-rank-number">{index + 1}</span>
                      <span class="genre-pill stats-genre-mark" style={`--item-color: ${stat.genre.color}`} aria-hidden="true">
                        {stat.genre.name.slice(0, 1)}
                      </span>
                      <span class="stats-rank-copy">
                        <strong>{stat.genre.name}</strong>
                        <small>{songCountLabel(stat.songCount)}</small>
                      </span>
                      <span class="stats-rank-count">{activityLabel(stat)}</span>
                    </button>
                  {/each}
                </div>
                {#if statsSectionExpanded("genres")}
                  <div class="stats-pagination" aria-live="polite">
                    <span>Showing {statsTopGenres.length} of {allStatsTopGenres.length}</span>
                    {#if hasMoreStats("genres", allStatsTopGenres.length)}
                      <button type="button" onclick={() => loadMoreStats("genres", allStatsTopGenres.length)}>Load 50 more</button>
                    {/if}
                  </div>
                {/if}
              {/if}
            </LibrarySection>
            {/if}

            {#if !statsDetailSection || statsDetailSection === "recent"}
            <LibrarySection
              title="Recently Played"
              viewAllLabel={statsViewAllLabel("recent", statsRecentCount)}
              onViewAll={!statsDetailSection && statsRecentCount > STATS_PREVIEW_LIMITS.recent ? () => openStatsSection("recent") : undefined}
            >
              {#if removeListenError}<p class="stats-status" role="alert">{removeListenError}</p>{/if}
              {#if statsRecentListens.length === 0}
                <p class="stats-empty">No listens recorded in this period.</p>
              {:else}
                <div class="stats-recent-list">
                  {#each statsRecentListens as listen (listen.eventId)}
                    {@const track = statsTracksById.get(listen.trackId)}
                    {@const title = track?.title ?? "Unavailable track"}
                    <div class="stats-recent-entry">
                      <button
                        class="stats-recent-card"
                        type="button"
                        title={track?.filePath || "Track unavailable"}
                        disabled={!track?.filePath}
                        onclick={() => track && handleStatsTrackSelect(track, statsRecentQueue)}
                        oncontextmenu={(event) => {
                          if (!track?.filePath) return;
                          event.preventDefault();
                          event.stopPropagation();
                          openStatsTrackContextMenu(track, statsRecentQueue, event.clientX, event.clientY);
                        }}
                      >
                        {#if track}{@render statsTrackArt(track)}{:else}<span aria-hidden="true"></span>{/if}
                        <span class="stats-rank-copy">
                          <strong>{title}</strong>
                          <small>{track?.artist ?? "Unknown Artist"}{track?.album ? ` · ${track.album}` : ""}</small>
                        </span>
                        <span class="stats-played-at">{listen.listenedMs ? `${listeningDurationLabel(listen.listenedMs)} · ` : ""}{formatDateTime(listen.playedAtUtc)}</span>
                      </button>
                      <button
                        class="stats-remove-listen"
                        type="button"
                        aria-label={`Remove listen for ${title} from ${formatDateTime(listen.playedAtUtc)}`}
                        disabled={removingListenId !== null}
                        onclick={() => handleRemoveListen(listen, title)}
                      >{removingListenId === listen.eventId ? "Removing…" : "Remove listen"}</button>
                    </div>
                  {/each}
                </div>
                {#if statsSectionExpanded("recent")}
                  <div class="stats-pagination" aria-live="polite">
                    <span>Showing {statsRecentListens.length} of {statsRecentCount} listens</span>
                    {#if hasMoreStats("recent", statsRecentCount)}
                      <button type="button" onclick={() => loadMoreStats("recent", statsRecentCount)}>Load 50 more</button>
                    {/if}
                  </div>
                {/if}
              {/if}
            </LibrarySection>
            {/if}

            {#if !statsDetailSection}
              <section class="stats-library-summary" aria-labelledby="stats-library-title">
                <h2 id="stats-library-title">Library totals</h2>
                <dl>
                  {#each [
                    { label: "Songs", value: tracks.length },
                    { label: "Albums", value: displayAlbums.length },
                    { label: "Artists", value: displayArtists.length },
                    { label: "Genres", value: displayGenres.length },
                    { label: "Liked songs", value: favoriteTracks.length },
                  ] as item (item.label)}
                    <div><dt>{item.label}</dt><dd>{hasLoadedCache ? item.value.toLocaleString() : "—"}</dd></div>
                  {/each}
                </dl>
              </section>
            {/if}
          </div>
          {/if}
        </section>
      {:else if activeView === "Albums"}
        {#if selectedAlbum}
          <section class="detail-view" aria-labelledby="album-detail-title">
            <div class="album-detail-header" style={`--item-color: ${selectedAlbum.color}`}>
              <button
                class="album-detail-overflow"
                type="button"
                aria-label="More album actions"
                aria-haspopup="menu"
                title="More album actions"
                onclick={openAlbumMoreMenu}
              >⋮</button>
              {#if selectedAlbum.coverArtPath}
                <img
                  class="album-detail-ambient"
                  src={localImageSource(selectedAlbum.coverArtPath) ?? ""}
                  alt=""
                  aria-hidden="true"
                  onload={showLoadedImage}
                  onerror={hideBrokenImage}
                />
              {/if}
              <div class="album-detail-cover-shell">
                {#if selectedAlbum.coverArtPath}
                  <button
                    class="album-art detail-cover cover-open-button"
                    style={`--item-color: ${selectedAlbum.color}`}
                    type="button"
                    aria-label={`View full cover art for ${selectedAlbum.title}`}
                    onclick={(event) => openArtworkViewer(event, selectedAlbum.coverArtPath!, selectedAlbum.title)}
                  >
                    <img
                      src={localImageSource(selectedAlbum.coverArtPath) ?? ""}
                      alt=""
                      onload={showLoadedImage}
                      onerror={hideBrokenImage}
                    />
                    <span class="album-art-disc"></span>
                  </button>
                {:else}
                  <div class="album-art detail-cover" style={`--item-color: ${selectedAlbum.color}`} aria-hidden="true">
                    <span class="album-art-placeholder">
                      <strong>{albumInitials(selectedAlbum)}</strong>
                      <small>{selectedAlbum.artist}</small>
                    </span>
                  </div>
                {/if}
              </div>
              <div class="detail-copy album-detail-copy">
                <p class="eyebrow">Album</p>
                <h3 id="album-detail-title">{selectedAlbum.title}</h3>
                <p class="album-detail-artist">
                  {#if displayArtists.some((artist) => artist.name === selectedAlbum.artist)}
                    <button type="button" onclick={() => handleAlbumArtistSelect(selectedAlbum.artist)}>{selectedAlbum.artist}</button>
                  {:else}
                    {selectedAlbum.artist}
                  {/if}
                </p>
                <div class="album-detail-meta" aria-label={albumHeroDetails(selectedAlbum, selectedAlbumTracks)}>
                  {#if selectedAlbum.year}
                    <span>{selectedAlbum.year}</span>
                  {/if}
                  <span>{songCountLabel(selectedAlbumTracks.length)}</span>
                  {#if showAlbumPlayCounts}
                    <span>{activityLabel(selectedAlbum)}</span>
                  {/if}
                  {#if selectedAlbumDurationLabel}
                    <span>{selectedAlbumDurationLabel}</span>
                  {/if}
                  {#if selectedAlbumFormatSummary}
                    <span>{selectedAlbumFormatSummary}</span>
                  {/if}
                </div>
                <div class="album-genre-chip-list" aria-label="Album genres">
                  {#each selectedAlbumGenres as genre}
                    <button type="button" class:muted={genre === "Unknown Genre"} onclick={() => handleGenreNameSelect(genre)}>{genre}</button>
                  {/each}
                </div>
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
                </div>
                {#if editingCoverForAlbum === selectedAlbum.id}
                  <p class="form-message" role="status">Updating artwork…</p>
                {:else if albumCoverFeedback?.albumId === selectedAlbum.id}
                  <p class="form-message" class:error={albumCoverFeedback.error} role={albumCoverFeedback.error ? "alert" : "status"}>{albumCoverFeedback.message}</p>
                {/if}
              </div>
            </div>

            {#if albumTagMoveNotice?.editedAlbumId === selectedAlbum.id}
              <div class="album-tag-move-notice" role="status">
                <div>
                  <strong>{albumTagMoveNotice.excludedTrackCount} read-only {albumTagMoveNotice.excludedTrackCount === 1 ? "track was" : "tracks were"} not deleted</strong>
                  <p>
                    {albumTagMoveNotice.editedTrackCount} {albumTagMoveNotice.editedTrackCount === 1 ? "edited track moved" : "edited tracks moved"} to this album. The excluded {albumTagMoveNotice.excludedTrackCount === 1 ? "track remains" : "tracks remain"} under “{albumTagMoveNotice.originalAlbumTitle}”.
                  </p>
                </div>
                <div class="album-tag-move-actions">
                  <button type="button" onclick={viewOriginalAlbumAfterTagEdit}>View original album</button>
                  <button type="button" aria-label="Dismiss album split notice" onclick={() => albumTagMoveNotice = null}>Dismiss</button>
                </div>
              </div>
            {/if}

            {#if selectedAlbumTracks.length > 10}
              <div class="search-bar album-track-search">
                <input
                  type="search"
                  bind:value={searchQuery}
                  placeholder="Search this album..."
                  aria-label="Search this album"
                  onkeydown={handleSearchKeydown}
                />
                {#if searchQuery}
                  <button type="button" aria-label="Clear album search" onclick={clearSearch}>Clear</button>
                {/if}
              </div>
            {/if}

            <LibrarySection title="Album Songs" viewAllLabel="">
              {#snippet headerActions()}
                <div class="album-selection-controls">
                <button class="album-selection-button" type="button" aria-pressed={isAlbumSelectingTracks}
                  onclick={() => {
                    const selecting = isAlbumSelectingTracks;
                    clearAlbumTrackSelection();
                    if (!selecting) trackSelectionAlbumId = selectedAlbumId;
                  }}>
                  {isAlbumSelectingTracks ? "Done selecting" : "Select tracks"}
                </button>
                {#if isAlbumSelectingTracks && selectedTrackIds.size > 0}
                  <button class="album-selection-button" type="button" aria-haspopup="menu" onclick={(event) => {
                    const bounds = event.currentTarget.getBoundingClientRect();
                    openContextMenu(bounds.left, bounds.bottom, albumSelectionMenuItems(selectedAlbum.id));
                  }}>
                    {selectedTrackIds.size} selected ▾
                  </button>
                {/if}
                </div>
                <TrackSortMenu
                  value={albumTrackSort}
                  direction={albumTrackSortDirection}
                  options={albumTrackSortOptions}
                  onSortChange={(value) => albumTrackSort = value}
                  onDirectionChange={(direction) => albumTrackSortDirection = direction}
                />
              {/snippet}
              {#if bulkTrackFeedback?.albumId === selectedAlbum.id}
                <p class="form-message" class:error={bulkTrackFeedback.error} role={bulkTrackFeedback.error ? "alert" : "status"}>{bulkTrackFeedback.message}</p>
              {/if}
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
                        class:with-play-counts={showAlbumTrackPlayCounts}
                        class:bulk-selected={isAlbumSelectingTracks && selectedTrackIds.has(track.id)}
                        class="album-track-row"
                        role="button"
                        tabindex="0"
                        title={track.filePath}
                        aria-pressed={isAlbumSelectingTracks ? selectedTrackIds.has(track.id) : undefined}
                        onclick={(event) => handleAlbumTrackSelect(track, event)}
                        oncontextmenu={(event) => openAlbumTrackContextMenu(event, track)}
                        onkeydown={(event) => handleAlbumTrackKeydown(event, track)}
                      >
                        <span class:missing={track.trackNumber === null} class="album-track-number">
                          {albumTrackNumberLabel(track)}
                        </span>
                        <div class="track-title">
                          <span class="track-name">{track.title}{track.unavailable ? " · Unavailable" : track.ignored ? " · Ignored" : ""}</span>
                          <button class="track-link" type="button" onclick={(event) => { event.stopPropagation(); handleTrackArtistSelect(track); }}>
                            {track.artist ?? "Unknown Artist"}
                          </button>
                        </div>
                        {#if showAlbumTrackPlayCounts}
                          <span class="album-track-play-count">{track.ignored ? "" : activityLabel(track)}</span>
                        {/if}
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
          <div class="albums-landing">
            <LibrarySection title="All Albums" viewAllLabel={`${visibleAlbums.length} total`}>
            <div class="control-bar">
              <label>
                <span>Sort</span>
                <select bind:value={albumSort}>
                  <option value="title">Album title</option>
                  <option value="artist">Artist</option>
                  <option value="year">Year</option>
                  <option value="trackCount">Song count</option>
                  {#each activitySortOptions as option (option.value)}
                    <option value={option.value}>{option.label}</option>
                  {/each}
                </select>
              </label>
              {#if !isActivitySortOrder(albumSort)}
                <button
                  class="direction-toggle"
                  type="button"
                  aria-label={`Album sort direction: ${sortDirectionLabel(albumSortDirection)}`}
                  onclick={() => albumSortDirection = nextSortDirection(albumSortDirection)}
                >
                  {sortDirectionLabel(albumSortDirection)}
                </button>
              {/if}
            </div>
            {#if visibleAlbums.length === 0}
              <div class="group-empty">
                <h3>{normalizedSearchQuery ? "No albums matched" : "No albums found"}</h3>
                <p>{normalizedSearchQuery ? "Search is limited to album titles, artists, and years." : "Scan a music folder to build your local album library."}</p>
              </div>
            {:else}
              <div class="album-grid">
                {#each visibleAlbums as album}
                  <article class="album-card" oncontextmenu={(event) => openAlbumContextMenu(event, album)}>
                    <div class="album-card-main">
                      <div class="album-art" style={`--item-color: ${album.color}`}>
                        {#if album.coverArtPath}
                          <img
                            src={localImageSource(album.coverArtPath) ?? ""}
                            alt=""
                            loading="lazy"
                            onload={showLoadedImage}
                            onerror={hideBrokenImage}
                          />
                          <span class="album-art-disc"></span>
                        {:else}
                          <span class="album-art-placeholder">
                            <strong>{albumInitials(album)}</strong>
                            <small>{album.artist}</small>
                          </span>
                        {/if}
                        <button class="album-art-open" type="button" aria-label={`Open ${album.title}`} onclick={() => handleAlbumSelect(album)}></button>
                        <div class="album-card-actions" role="group" aria-label={`${album.title} actions`}>
                          <button type="button" disabled={!albumHasTracks(album)} onclick={(event) => void handleAlbumCardPlay(event, album)}>
                            Play
                          </button>
                          <button type="button" disabled={!albumHasTracks(album)} onclick={(event) => void handleAlbumCardPlay(event, album, true)}>
                            Shuffle
                          </button>
                          <button type="button" disabled={!albumHasTracks(album)} onclick={(event) => handleAlbumCardAddToQueue(event, album)}>
                            Queue
                          </button>
                        </div>
                      </div>
                      <button class="album-card-copy" type="button" onclick={() => handleAlbumSelect(album)}>
                        <strong>{album.title}</strong>
                        <small>{albumDetail(album)}{showAlbumPlayCounts ? ` · ${activityLabel(album)}` : ""}</small>
                      </button>
                    </div>
                  </article>
                {/each}
              </div>
            {/if}
            </LibrarySection>
          </div>
        {/if}
      {:else if activeView === "Artists"}
        {#if selectedArtist}
          <section class="detail-view" aria-labelledby={detailSongsView === "artist" ? "artist-songs-title" : "artist-detail-title"}>
            {#if detailSongsView === "artist"}
              <div class="group-songs-intro">
                <p class="eyebrow">Artist songs</p>
                <h3 id="artist-songs-title">All songs by {selectedArtist.name}</h3>
                <p>{songCountLabel(selectedArtistTracks.length)} · Sort the list without changing the current queue.</p>
              </div>
            {:else}
            <div class="artist-detail-header" style={`--item-color: ${selectedArtist.color}`}>
              <div class="artist-avatar detail-avatar" style={`--item-color: ${selectedArtist.color}`} aria-hidden="true">
                {selectedArtist.name.slice(0, 1)}
              </div>
              <div class="detail-copy">
                <p class="eyebrow">Artist</p>
                <h3 id="artist-detail-title">{selectedArtist.name}</h3>
                <div class="album-detail-meta" aria-label={`${selectedArtist.name} summary${showArtistPlayCounts ? `, ${activityLabel(selectedArtist)}` : ""}`}>
                  <span>{artistSongCount(selectedArtist)}</span>
                  <span>{selectedArtistAlbums.length} {selectedArtistAlbums.length === 1 ? "album" : "albums"}</span>
                  {#if showArtistPlayCounts}<span>{activityLabel(selectedArtist)}</span>{/if}
                </div>
                <div class="album-genre-chip-list" aria-label="Artist genres">
                  {#each selectedArtistGenres as genre}
                    <button type="button" class:muted={genre === "Unknown Genre"} onclick={() => handleGenreNameSelect(genre)}>{genre}</button>
                  {/each}
                </div>
                <div class="album-detail-actions artist-detail-actions">
                  <button type="button" disabled={selectedArtistTracks.length === 0} onclick={() => void handlePlaySelectedArtist()}>
                    Play Artist
                  </button>
                  <button type="button" disabled={selectedArtistTracks.length === 0} onclick={() => void handleShuffleSelectedArtist()}>
                    Shuffle Artist
                  </button>
                  <button type="button" disabled={selectedArtistTracks.length === 0} onclick={handleAddSelectedArtistToQueue}>
                    Add to Queue
                  </button>
                </div>
              </div>
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
                    <article class="album-card" oncontextmenu={(event) => openAlbumContextMenu(event, album)}>
                      <div class="album-card-main">
                        <div class="album-art" style={`--item-color: ${album.color}`}>
                          {#if album.coverArtPath}
                            <img
                              src={localImageSource(album.coverArtPath) ?? ""}
                              alt=""
                              loading="lazy"
                              onload={showLoadedImage}
                              onerror={hideBrokenImage}
                            />
                            <span class="album-art-disc"></span>
                          {:else}
                            <span class="album-art-placeholder">
                              <strong>{albumInitials(album)}</strong>
                              <small>{album.artist}</small>
                            </span>
                          {/if}
                          <button class="album-art-open" type="button" aria-label={`Open ${album.title}`} onclick={() => handleAlbumSelect(album)}></button>
                          <div class="album-card-actions" role="group" aria-label={`${album.title} actions`}>
                            <button type="button" disabled={!albumHasTracks(album)} onclick={(event) => void handleAlbumCardPlay(event, album)}>
                              Play
                            </button>
                            <button type="button" disabled={!albumHasTracks(album)} onclick={(event) => void handleAlbumCardPlay(event, album, true)}>
                              Shuffle
                            </button>
                            <button type="button" disabled={!albumHasTracks(album)} onclick={(event) => handleAlbumCardAddToQueue(event, album)}>
                              Queue
                            </button>
                          </div>
                        </div>
                        <button class="album-card-copy" type="button" onclick={() => handleAlbumSelect(album)}>
                          <strong>{album.title}</strong>
                          <small>{albumDetail(album)}{showAlbumPlayCounts ? ` · ${activityLabel(album)}` : ""}</small>
                        </button>
                      </div>
                    </article>
                  {/each}
                </div>
              {/if}
            </LibrarySection>
            {/if}
            <LibrarySection
              title={detailSongsView === "artist" ? "All Songs" : "Most Played Songs"}
              viewAllLabel={detailSongsView === "artist" ? `${selectedArtistTracks.length} total` : "View all"}
              onViewAll={detailSongsView === "artist" ? undefined : () => openGroupSongs("artist")}
            >
              {#snippet headerActions()}
                {#if detailSongsView === "artist"}
                  <TrackSortMenu
                    value={artistTrackSort}
                    direction={artistTrackSortDirection}
                    options={groupedTrackSortOptions}
                    onSortChange={(value) => artistTrackSort = value}
                    onDirectionChange={(direction) => artistTrackSortDirection = direction}
                  />
                {/if}
              {/snippet}
              {#if selectedArtistTracks.length === 0}
                <div class="group-empty"><h3>No songs found for this artist</h3></div>
              {:else}
                <TrackList
                  activityLabel={trackActivityLabel}
                  tracks={detailSongsView === "artist" ? selectedArtistDisplayTracks.slice(0, artistTrackVisibleLimit) : selectedArtistTopTracks}
                  isScanning={false}
                  variant="library"
                  selectedTrackId={currentTrack?.id}
                  onTrackSelect={(track) => void handleTrackSelect(track, detailSongsView === "artist" ? selectedArtistDisplayTracks : selectedArtistRankedTracks)}
                  onTrackContextMenu={(track, _queue, x, y) => openTrackContextMenu(track, detailSongsView === "artist" ? selectedArtistDisplayTracks : selectedArtistRankedTracks, x, y)}
                  onArtistSelect={handleTrackArtistSelect}
                  onAlbumSelect={handleTrackAlbumSelect}
                  onToggleFavorite={handleToggleFavorite}
                />
                {#if detailSongsView === "artist" && artistTrackVisibleLimit < selectedArtistDisplayTracks.length}
                  <button class="detail-songs-load-more" type="button" onclick={() => artistTrackVisibleLimit = nextStatsLimit(artistTrackVisibleLimit, selectedArtistDisplayTracks.length)}>Show more songs</button>
                {/if}
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
                  {#each activitySortOptions as option (option.value)}
                    <option value={option.value}>{option.label}</option>
                  {/each}
                </select>
              </label>
              {#if !isActivitySortOrder(artistSort)}
                <button
                  class="direction-toggle"
                  type="button"
                  aria-label={`Artist sort direction: ${sortDirectionLabel(artistSortDirection)}`}
                  onclick={() => artistSortDirection = nextSortDirection(artistSortDirection)}
                >
                  {sortDirectionLabel(artistSortDirection)}
                </button>
              {/if}
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
                      <p>{artistSongCount(artist)}{showArtistPlayCounts ? ` · ${activityLabel(artist)}` : ""}</p>
                    </div>
                  </button>
                {/each}
              </div>
            {/if}
          </LibrarySection>
        {/if}
      {:else if activeView === "Genres"}
        {#if selectedGenre}
          <section class="detail-view" aria-labelledby={detailSongsView === "genre" ? "genre-songs-title" : "genre-detail-title"}>
            {#if detailSongsView === "genre"}
              <div class="group-songs-intro">
                <p class="eyebrow">Genre songs</p>
                <h3 id="genre-songs-title">All {selectedGenre.name} songs</h3>
                <p>{songCountLabel(selectedGenreTracks.length)} · Sort the list without changing the current queue.</p>
              </div>
            {:else}
            <div class="genre-detail-header" style={`--item-color: ${selectedGenre.color}`}>
              <div class="genre-mark detail-avatar" style={`--item-color: ${selectedGenre.color}`} aria-hidden="true">
                {selectedGenre.name.slice(0, 1)}
              </div>
              <div class="detail-copy">
                <p class="eyebrow">Genre</p>
                <h3 id="genre-detail-title">{selectedGenre.name}</h3>
                <div class="album-detail-meta" aria-label={`${selectedGenre.name} summary${showGenrePlayCounts ? `, ${activityLabel(selectedGenre)}` : ""}`}>
                  <span>{songCountLabel(selectedGenreTracks.length)}</span>
                  <span>{selectedGenreArtists.length} {selectedGenreArtists.length === 1 ? "artist" : "artists"}</span>
                  <span>{selectedGenreAlbums.length} {selectedGenreAlbums.length === 1 ? "album" : "albums"}</span>
                  {#if showGenrePlayCounts}<span>{activityLabel(selectedGenre)}</span>{/if}
                </div>
                <div class="album-detail-actions genre-detail-actions">
                  <button type="button" disabled={selectedGenreTracks.length === 0} onclick={() => void handlePlaySelectedGenre()}>
                    Play Genre
                  </button>
                  <button type="button" disabled={selectedGenreTracks.length === 0} onclick={() => void handleShuffleSelectedGenre()}>
                    Shuffle Genre
                  </button>
                  <button type="button" disabled={selectedGenreTracks.length === 0} onclick={handleAddSelectedGenreToQueue}>
                    Add to Queue
                  </button>
                </div>
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
                    <article class="album-card" oncontextmenu={(event) => openAlbumContextMenu(event, album)}>
                      <div class="album-card-main">
                        <div class="album-art" style={`--item-color: ${album.color}`}>
                          {#if album.coverArtPath}
                            <img
                              src={localImageSource(album.coverArtPath) ?? ""}
                              alt=""
                              loading="lazy"
                              onload={showLoadedImage}
                              onerror={hideBrokenImage}
                            />
                            <span class="album-art-disc"></span>
                          {:else}
                            <span class="album-art-placeholder">
                              <strong>{albumInitials(album)}</strong>
                              <small>{album.artist}</small>
                            </span>
                          {/if}
                          <button class="album-art-open" type="button" aria-label={`Open ${album.title}`} onclick={() => handleAlbumSelect(album)}></button>
                          <div class="album-card-actions" role="group" aria-label={`${album.title} actions`}>
                            <button type="button" disabled={!albumHasTracks(album)} onclick={(event) => void handleAlbumCardPlay(event, album)}>
                              Play
                            </button>
                            <button type="button" disabled={!albumHasTracks(album)} onclick={(event) => void handleAlbumCardPlay(event, album, true)}>
                              Shuffle
                            </button>
                            <button type="button" disabled={!albumHasTracks(album)} onclick={(event) => handleAlbumCardAddToQueue(event, album)}>
                              Queue
                            </button>
                          </div>
                        </div>
                        <button class="album-card-copy" type="button" onclick={() => handleAlbumSelect(album)}>
                          <strong>{album.title}</strong>
                          <small>{albumDetail(album)}{showAlbumPlayCounts ? ` · ${activityLabel(album)}` : ""}</small>
                        </button>
                      </div>
                    </article>
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
                        <p>{artistSongCount(artist)}{showArtistPlayCounts ? ` · ${activityLabel(artistActivity(artist.name))}` : ""}</p>
                      </div>
                    </button>
                  {/each}
                </div>
              {/if}
            </LibrarySection>
            {/if}
            <LibrarySection
              title={detailSongsView === "genre" ? "All Songs" : "Most Played Songs"}
              viewAllLabel={detailSongsView === "genre" ? `${selectedGenreTracks.length} total` : "View all"}
              onViewAll={detailSongsView === "genre" ? undefined : () => openGroupSongs("genre")}
            >
              {#snippet headerActions()}
                {#if detailSongsView === "genre"}
                  <TrackSortMenu
                    value={genreTrackSort}
                    direction={genreTrackSortDirection}
                    options={groupedTrackSortOptions}
                    onSortChange={(value) => genreTrackSort = value}
                    onDirectionChange={(direction) => genreTrackSortDirection = direction}
                  />
                {/if}
              {/snippet}
              {#if selectedGenreTracks.length === 0}
                <div class="group-empty"><h3>No songs found for this genre</h3></div>
              {:else}
                <TrackList
                  activityLabel={trackActivityLabel}
                  tracks={detailSongsView === "genre" ? selectedGenreDisplayTracks.slice(0, genreTrackVisibleLimit) : selectedGenreTopTracks}
                  isScanning={false}
                  variant="library"
                  selectedTrackId={currentTrack?.id}
                  onTrackSelect={(track) => void handleTrackSelect(track, detailSongsView === "genre" ? selectedGenreDisplayTracks : selectedGenreRankedTracks)}
                  onTrackContextMenu={(track, _queue, x, y) => openTrackContextMenu(track, detailSongsView === "genre" ? selectedGenreDisplayTracks : selectedGenreRankedTracks, x, y)}
                  onArtistSelect={handleTrackArtistSelect}
                  onAlbumSelect={handleTrackAlbumSelect}
                  onToggleFavorite={handleToggleFavorite}
                />
                {#if detailSongsView === "genre" && genreTrackVisibleLimit < selectedGenreDisplayTracks.length}
                  <button class="detail-songs-load-more" type="button" onclick={() => genreTrackVisibleLimit = nextStatsLimit(genreTrackVisibleLimit, selectedGenreDisplayTracks.length)}>Show more songs</button>
                {/if}
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
                  {#each activitySortOptions as option (option.value)}
                    <option value={option.value}>{option.label}</option>
                  {/each}
                </select>
              </label>
              {#if !isActivitySortOrder(genreSort)}
                <button
                  class="direction-toggle"
                  type="button"
                  aria-label={`Genre sort direction: ${sortDirectionLabel(genreSortDirection)}`}
                  onclick={() => genreSortDirection = nextSortDirection(genreSortDirection)}
                >
                  {sortDirectionLabel(genreSortDirection)}
                </button>
              {/if}
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
                      <p>{genreDetail(genre)}{showGenrePlayCounts ? ` · ${activityLabel(genre)}` : ""}</p>
                    </div>
                  </button>
                {/each}
              </div>
            {/if}
          </LibrarySection>
        {/if}
      {:else if activeView === "Songs"}
        <LibrarySection title="All Songs" viewAllLabel={`${visibleSongTracks.length} ${visibleSongTracks.length === 1 ? "song" : "songs"}`}>
          <div class="control-bar song-browser-controls">
            <CompactDropdown
              label="Sort"
              value={songSort}
              options={songSortOptions}
              onChange={handleSongSortChange}
            />
            {#if !isActivitySortOrder(songSort)}
              <button
                class="direction-toggle"
                type="button"
                aria-label={`Song sort direction: ${sortDirectionLabel(songSortDirection)}`}
                onclick={() => songSortDirection = nextSortDirection(songSortDirection)}
              >
                {sortDirectionLabel(songSortDirection)}
              </button>
            {/if}
            <CompactDropdown
              label="Format"
              value={songFormatFilter}
              options={songFormatOptions}
              onChange={handleSongFormatFilterChange}
            />
          </div>
          {#if isScanning}
            <TrackList
              tracks={visibleSongTracks}
              {isScanning}
              variant="songs"
              {showPlayCounts}
              {showListeningTime}
              showOrder={showSongListNumbers}
              selectedTrackId={currentTrack?.id}
              onTrackSelect={handleTrackSelect}
              onTrackContextMenu={openTrackContextMenu}
              onArtistSelect={handleTrackArtistSelect}
              onAlbumSelect={handleTrackAlbumSelect}
              onToggleFavorite={handleToggleFavorite}
            />
          {:else if tracks.length === 0}
            <div class="group-empty songs-empty-state">
              <h3>No songs in your library</h3>
              <p>Scan a music folder to build a full-library track browser.</p>
            </div>
          {:else if visibleSongTracks.length === 0}
            <div class="group-empty songs-empty-state">
              {#if normalizedSearchQuery}
                <h3>No songs matched</h3>
                <p>Try another title, artist, album, or file name.</p>
              {:else if songFormatFilter !== "All"}
                <h3>No {songFormatFilter} songs found</h3>
                <p>Choose another format or scan folders that contain {songFormatFilter} files.</p>
              {:else}
                <h3>No songs found</h3>
                <p>Scan a music folder to build your local track library.</p>
              {/if}
            </div>
          {:else}
            <TrackList
              tracks={visibleSongTracks}
              {isScanning}
              variant="songs"
              {showPlayCounts}
              {showListeningTime}
              showOrder={showSongListNumbers}
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
        {#snippet playlistSearch()}
          <div class="playlist-search" role="search">
            <input
              type="search"
              bind:value={searchQuery}
              placeholder={searchPlaceholder()}
              aria-label={searchPlaceholder()}
              onkeydown={handleSearchKeydown}
            />
          </div>
        {/snippet}
        {#if isLikedSongsOpen}
          <section class="playlist-detail-page" aria-labelledby="liked-songs-title">
            <button class="back-button playlist-detail-back" type="button" onclick={handleBackToPlaylists}>Back to Playlists</button>
            <section class="playlist-hero smart" aria-labelledby="liked-songs-title">
              <div class="playlist-hero-art liked" aria-hidden="true">★</div>
              <div class="playlist-hero-copy">
                <h2 id="liked-songs-title">Liked Songs</h2>
                <div class="playlist-hero-meta">
                  <span>{smartPlaylistMetaLabel(favoriteTracks)}</span>
                </div>
                <div class="playlist-hero-actions">
                  <button type="button" disabled={favoriteTracks.length === 0} onclick={() => void handlePlayLikedSongs()}>Play Playlist</button>
                  <button type="button" disabled={favoriteTracks.length === 0} onclick={() => void handlePlayLikedSongs(true)}>Shuffle Playlist</button>
                  <button type="button" disabled={favoriteTracks.length === 0} onclick={handleAddLikedSongsToQueue}>Add to Queue</button>
                </div>
              </div>
            </section>

            {#if playlistError}
              <div class="scan-error" role="alert">{playlistError}</div>
            {:else if playlistMessage}
              <p class="playlist-status-pill" role="status">{playlistMessage}</p>
            {/if}

            {#if favoriteTracks.length}{@render playlistSearch()}{/if}
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
                  activityLabel={trackActivityLabel}
                  tracks={filteredLikedTracks}
                  isScanning={false}
                  variant="library"
                  showOrder
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
          <section class="playlist-detail-page" aria-labelledby="custom-playlist-title">
            <button class="back-button playlist-detail-back" type="button" onclick={handleBackToPlaylists}>Back to Playlists</button>
            <section class="playlist-hero" aria-labelledby="custom-playlist-title">
              <div class="playlist-hero-art mosaic" aria-hidden="true">
                {#each playlistCoverTracks(selectedPlaylistTracks) as track}
                  <span>
                    {#if track.coverArtPath}
                      <img src={localImageSource(track.coverArtPath) ?? ""} alt="" onload={showLoadedImage} onerror={hideBrokenImage} />
                    {:else}
                      {track.title.slice(0, 1)}
                    {/if}
                  </span>
                {/each}
                {#if selectedPlaylistTracks.length === 0}
                  <span>P</span>
                {/if}
              </div>
              <div class="playlist-hero-copy">
                <h2 id="custom-playlist-title">{selectedPlaylist.name}</h2>
                <div class="playlist-hero-meta">
                  <span>{playlistTrackLabel(selectedPlaylist)}</span>
                  {#if selectedPlaylistDurationLabel}
                    <span>{selectedPlaylistDurationLabel}</span>
                  {/if}
                </div>
                <div class="playlist-hero-action-row">
                  <div class="playlist-hero-actions">
                    <button type="button" disabled={selectedPlaylistTracks.length === 0} onclick={() => void handlePlayPlaylist(selectedPlaylist)}>Play Playlist</button>
                    <button type="button" disabled={selectedPlaylistTracks.length === 0} onclick={() => void handlePlayPlaylist(selectedPlaylist, true)}>Shuffle Playlist</button>
                    <button type="button" disabled={selectedPlaylistTracks.length === 0} onclick={() => handleAddPlaylistToQueue(selectedPlaylist)}>Add to Queue</button>
                  </div>
                  <div class="playlist-hero-actions playlist-management-actions">
                    <button class:active={isPlaylistEditMode} class="secondary" type="button" onclick={() => isPlaylistEditMode = !isPlaylistEditMode}>{isPlaylistEditMode ? "Done" : "Edit Order"}</button>
                    <button class="secondary more" type="button" aria-label="More playlist actions" onclick={(event) => openPlaylistMoreMenu(event, selectedPlaylist)}>⋯</button>
                  </div>
                </div>
              </div>
            </section>

            {#if playlistError}
              <div class="scan-error" role="alert">{playlistError}</div>
            {:else if playlistMessage}
              <p class="playlist-status-pill" role="status">{playlistMessage}</p>
            {/if}
            {#if selectedPlaylistMissingTrackCount > 0}
              <div class="playlist-warning" role="status">
                {selectedPlaylistMissingTrackCount} {selectedPlaylistMissingTrackCount === 1 ? "track is" : "tracks are"} unavailable because the file is no longer in the scanned library.
              </div>
            {/if}

            {#if selectedPlaylistTracks.length}{@render playlistSearch()}{/if}
            <LibrarySection title="Songs" viewAllLabel={isPlaylistEditMode ? "Editing order" : normalizedSearchQuery ? `${selectedPlaylistSearchTracks.length} ${selectedPlaylistSearchTracks.length === 1 ? "match" : "matches"}` : `${selectedPlaylistTracks.length} playable`}>
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
                  activityLabel={trackActivityLabel}
                  tracks={selectedPlaylistSearchTracks}
                  isScanning={false}
                  variant="library"
                  showOrder
                  selectedTrackId={currentTrack?.id}
                  onTrackSelect={handleTrackSelect}
                  onTrackContextMenu={openTrackContextMenu}
                  onArtistSelect={handleTrackArtistSelect}
                  onAlbumSelect={handleTrackAlbumSelect}
                  onToggleFavorite={handleToggleFavorite}
                  onRemoveTrack={isPlaylistEditMode ? handleRemoveTrackFromSelectedPlaylist : undefined}
                  onMoveTrackUp={isPlaylistEditMode ? (track) => void handleMoveTrackInSelectedPlaylist(track, "up") : undefined}
                  onMoveTrackDown={isPlaylistEditMode ? (track) => void handleMoveTrackInSelectedPlaylist(track, "down") : undefined}
                  canMoveTrackUp={isPlaylistEditMode ? (track) => canMoveSelectedPlaylistTrack(track, "up") : undefined}
                  canMoveTrackDown={isPlaylistEditMode ? (track) => canMoveSelectedPlaylistTrack(track, "down") : undefined}
                />
              {/if}
            </LibrarySection>
          </section>
        {:else if isMixBuilderOpen}
          <section class="detail-view playlists-page" aria-labelledby="mix-builder-title">
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
          <section class="playlists-page" aria-labelledby="playlists-title">
            <header class="organizer-header">
              <h2 id="playlists-title">Playlists</h2>
              {#if !isCreatePlaylistOpen}
                <div class="settings-actions">
                  <button id="new-playlist-button" class="primary" type="button" onclick={handleOpenCreatePlaylist}>New playlist</button>
                </div>
              {/if}
            </header>
            {#if playlistError && !isCreatePlaylistOpen}
              <div class="scan-error" role="alert">{playlistError}</div>
            {:else if playlistMessage && !isCreatePlaylistOpen}
              <p class="playlist-status-pill" role="status">{playlistMessage}</p>
            {/if}

            {#if isCreatePlaylistOpen}
              <form class="playlist-create-panel" onsubmit={(event) => { event.preventDefault(); void handleCreatePlaylist(); }}>
                <label for="playlist-name">Playlist name</label>
                <input id="playlist-name" type="text" bind:value={playlistNameDraft} aria-invalid={Boolean(playlistError)} />
                <div class="playlist-create-actions">
                  <button type="submit" disabled={!canCreatePlaylist}>Create</button>
                  <button class="secondary" type="button" onclick={handleCancelCreatePlaylist}>Cancel</button>
                </div>
                {#if playlistError}<p class="form-message error" role="alert">{playlistError}</p>{/if}
              </form>
            {/if}
            <div class="playlist-feature-grid">
              <article class="playlist-library-card smart">
                <button class="playlist-card-main" type="button" onclick={handleLikedSongsSelect}>
                  <div class="playlist-card-art liked" aria-hidden="true">★</div>
                  <div class="playlist-card-copy">
                    <h3>Liked Songs</h3>
                    <p>{smartPlaylistMetaLabel(favoriteTracks)}</p>
                  </div>
                </button>
                <div class="playlist-card-actions">
                  <button type="button" disabled={favoriteTracks.length === 0} onclick={() => void handlePlayLikedSongs()}>Play</button>
                  <button type="button" disabled={favoriteTracks.length === 0} onclick={() => void handlePlayLikedSongs(true)}>Shuffle</button>
                </div>
              </article>
              <button class="mix-tool-card" type="button" onclick={handleMixBuilderSelect}>
                <span class="mix-mark" aria-hidden="true">♪</span>
                <span class="mix-tool-copy">
                  <strong>Mix Builder</strong>
                  <small>Build a queue from artists, genres, albums, or liked songs.</small>
                </span>
                <span class="mix-tool-arrow" aria-hidden="true">›</span>
              </button>
            </div>

            <LibrarySection title="Your playlists" viewAllLabel={playlists.length ? `${playlists.length} total` : ""}>
              {#if playlists.length === 0}
                <div class="group-empty playlist-empty-state">
                  <h3>No playlists yet</h3>
                  <p>Create a playlist, then add songs from a track’s menu.</p>
                </div>
              {:else}
                <div class="playlist-card-grid">
                  {#each playlists as playlist (playlist.id)}
                    {@const playlistTracks = tracksForPlaylist(playlist)}
                    <article class="playlist-library-card" oncontextmenu={(event) => openPlaylistContextMenu(event, playlist)}>
                      <button class="playlist-card-main" type="button" onclick={() => handlePlaylistSelect(playlist)}>
                        <div class="playlist-card-art mosaic" aria-hidden="true">
                          {#each playlistCoverTracks(playlistTracks) as track}
                            <span>
                              {#if track.coverArtPath}
                                <img src={localImageSource(track.coverArtPath) ?? ""} alt="" onload={showLoadedImage} onerror={hideBrokenImage} />
                              {:else}
                                {track.title.slice(0, 1)}
                              {/if}
                            </span>
                          {/each}
                          {#if playlistTracks.length === 0}
                            <span>P</span>
                          {/if}
                        </div>
                        <div class="playlist-card-copy">
                          <h3 title={playlist.name}>{playlist.name}</h3>
                          <p>{playlistMetaLabel(playlist)}</p>
                        </div>
                      </button>
                      <div class="playlist-card-actions">
                        <button type="button" disabled={playlistTracks.length === 0} onclick={(event) => { stopCardAction(event); void handlePlayPlaylist(playlist); }}>Play</button>
                        <button type="button" disabled={playlistTracks.length === 0} onclick={(event) => { stopCardAction(event); void handlePlayPlaylist(playlist, true); }}>Shuffle</button>
                        <button type="button" disabled={playlistTracks.length === 0} onclick={(event) => { stopCardAction(event); handleAddPlaylistToQueue(playlist); }}>Queue</button>
                        <button class="secondary playlist-card-menu" type="button" aria-label={`More actions for ${playlist.name}`} onclick={(event) => openPlaylistMoreMenu(event, playlist)}>⋯</button>
                      </div>
                    </article>
                  {/each}
                </div>
              {/if}
            </LibrarySection>
          </section>
        {/if}
      {:else if activeView === "Videos"}
        <section class="videos-page" aria-labelledby="videos-title">
          {#if selectedVideo}
            <button class="back-button" type="button" onclick={handleBackToVideos}>Back to Videos</button>

            <section class="video-detail-hero" aria-labelledby="video-detail-title">
              <div class="video-session-card">
                <div class="video-session-thumb" aria-hidden="true">
                  {#if selectedVideoThumbnailSrc}
                    <img src={selectedVideoThumbnailSrc} alt="" onload={showLoadedImage} onerror={hideBrokenImage} />
                  {:else}
                    <span>V</span>
                  {/if}
                </div>

                <div class="video-session-copy">
                  <p class="eyebrow">{selectedVideoIsActive ? "Native video window active" : "Cassette video window"}</p>
                  <h3 id="video-detail-title">{selectedVideo.title}</h3>
                  <p>{selectedVideo.artist ?? "Unknown Artist"}</p>
                  <div class="video-session-progress">
                    <div>
                      <span>{formatVideoDuration(nativeVideoPositionSeconds)}</span>
                      <span>{formatVideoDuration(nativeVideoDurationSeconds)}</span>
                    </div>
                    <div class="video-panel-progress" style={`--progress: ${nativeVideoProgressPercent}%`} aria-hidden="true"></div>
                  </div>
                </div>

                <div class="video-session-status" aria-label="Video session status">
                  <div><span>Status</span><strong>{selectedVideoPlaybackStatusLabel}</strong></div>
                  <div><span>Backend</span><strong>Cassette video engine</strong></div>
                </div>

                <p class="video-window-note">
                  Cassette uses its native video engine for MKV/DVD playback. The video opens in a controlled Cassette video window.
                </p>

                <div class="video-actions video-window-actions">
                  <button type="button" disabled={!activeVideoStatus?.hasVideoWindow} onclick={() => void handleBringVideoWindowToFront()}>Bring Video Window to Front</button>
                  <button type="button" disabled={!activeVideoStatus?.hasVideoWindow} onclick={() => void handleFullscreenVideoWindow()}>
                    {activeVideoStatus?.isFullscreen ? "Exit Fullscreen" : "Fullscreen Video"}
                  </button>
                  <button type="button" disabled={!activeVideoStatus?.hasVideoWindow} onclick={() => void handleCloseVideoWindow()}>Close Video Window</button>
                  <button type="button" onclick={() => void handleOpenVideoExternal()}>Open External as Backup</button>
                </div>
              </div>

              <div class="video-player-controls" aria-label="Cassette video controls">
                <div class="video-actions primary-video-actions">
                  {#if selectedVideoIsActive}
                    <button type="button" onclick={() => void (isNativeVideoPlaying ? handleNativeVideoPause() : handleNativeVideoResume())}>
                      {isNativeVideoPlaying ? "Pause" : "Play"}
                    </button>
                  {:else}
                    <button type="button" onclick={() => void handleVideoPlayFromStart()}>Play</button>
                  {/if}
                  <button type="button" disabled={selectedVideo.lastPositionSeconds <= 0} onclick={() => void handleVideoResume()}>Resume</button>
                  <button type="button" disabled={!selectedVideoIsActive} onclick={() => void handleNativeVideoStop()}>Stop</button>
                </div>

                <div class="video-native-controls">
                  <label>
                    <span>Seek</span>
                    <input
                      type="range"
                      min="0"
                      max={nativeVideoDurationSeconds ?? selectedVideo.durationSeconds ?? 0}
                      value={nativeVideoPositionSeconds}
                      disabled={!selectedVideoIsActive || !(nativeVideoDurationSeconds ?? selectedVideo.durationSeconds)}
                      oninput={(event) => void handleNativeVideoSeek(event)}
                    />
                  </label>
                  <label>
                    <span>Volume</span>
                    <input
                      type="range"
                      min="0"
                      max="1"
                      step="0.01"
                      value={nativeVideoVolume}
                      disabled={!selectedVideoIsActive}
                      oninput={(event) => void handleNativeVideoVolume(event)}
                    />
                  </label>
                </div>

                {#if videoPlaybackError}
                  <p class="form-message error" role="alert">{videoPlaybackError}</p>
                {:else if videoPlaybackMessage}
                  <p class="form-message" role="status">{videoPlaybackMessage}</p>
                {/if}
              </div>

              <div class="video-detail-copy">
                <p class="eyebrow">{sourceLabel(selectedVideo.source)}</p>
                <h3>Details</h3>
                <p>{videoDetailLine(selectedVideo)}</p>
                <div class="video-meta-grid">
                  <div><span>Artist</span><strong>{selectedVideo.artist ?? "Unknown Artist"}</strong></div>
                  <div><span>Type</span><strong>{videoTypeLabel(selectedVideo.videoType)}</strong></div>
                  <div><span>Release</span><strong>{selectedVideo.releaseOrCollection ?? "Not set"}</strong></div>
                  <div><span>Year</span><strong>{selectedVideo.year ?? "Not set"}</strong></div>
                  <div><span>Duration</span><strong>{formatVideoDuration(nativeVideoDurationSeconds)}</strong></div>
                  <div><span>Last watched</span><strong>{formatVideoDuration(selectedVideo.lastPositionSeconds)}</strong></div>
                  <div><span>Plays</span><strong>{selectedVideo.playCount}</strong></div>
                </div>
                <div class="video-codec-grid" aria-label="Codec information">
                  <div><span>Container</span><strong>{isLoadingVideoCodecInfo ? "Loading..." : videoCodecInfo?.container ?? "Unknown"}</strong></div>
                  <div><span>Video codec</span><strong>{videoCodecInfo?.videoCodec ?? "Unknown"}</strong></div>
                  <div><span>Audio codec</span><strong>{videoCodecInfo?.audioCodec ?? "Unknown"}</strong></div>
                  <div><span>Resolution</span><strong>{videoCodecInfo?.resolution ?? "Unknown"}</strong></div>
                </div>
                {#if videoCodecInfo?.error}
                  <p class="video-location">{videoCodecInfo.error}</p>
                {/if}
                {#if videoLocationLine(selectedVideo)}
                  <p class="video-location">{videoLocationLine(selectedVideo)}</p>
                {/if}
                {#if selectedVideo.descriptionOrNotes}
                  <p class="video-location">{selectedVideo.descriptionOrNotes}</p>
                {/if}
                <div class="video-actions secondary-video-actions">
                  <button type="button" onclick={handleEditVideoInfo}>Edit Info</button>
                  <button type="button" onclick={() => void handleResetVideoProgress()}>Reset Progress</button>
                </div>
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
                    <span>Video type</span>
                    <select value={videoEditDraft.videoType} onchange={(event) => updateVideoDraftField("videoType", selectValue(event))}>
                      {#each videoTypeOptions as option}
                        <option value={option.value}>{option.label}</option>
                      {/each}
                    </select>
                  </label>
                  <label>
                    <span>Release / Collection</span>
                    <input type="text" value={videoEditDraft.releaseOrCollection} oninput={(event) => updateVideoDraftField("releaseOrCollection", inputValue(event))} />
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
                  <label class="wide">
                    <span>Description / Notes</span>
                    <textarea value={videoEditDraft.descriptionOrNotes} oninput={(event) => updateVideoDraftField("descriptionOrNotes", textAreaValue(event))}></textarea>
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

            <div class="video-filter-tabs" aria-label="Video type filters">
              {#each videoTypeFilters as filter}
                <button
                  class:active={videoTypeFilter === filter.value}
                  type="button"
                  onclick={() => videoTypeFilter = filter.value}
                >
                  {filter.label}
                </button>
              {/each}
            </div>

            <div class="video-stat-grid" aria-label="Videos summary">
              {#each videoLibraryStats as stat}
                <div>
                  <span>{stat.label}</span>
                  <strong>{stat.value}</strong>
                </div>
              {/each}
            </div>

            <section class="dvd-import-panel" aria-labelledby="dvd-import-title">
              <div class="settings-section-header">
                <div>
                  <p class="eyebrow">Source Import</p>
                  <h3 id="dvd-import-title">DVD Import</h3>
                  <p>DVD Import supports readable/non-DRM DVDs and VIDEO_TS folders. Cassette does not bypass DRM.</p>
                </div>
                <span class="settings-pill">{dvdImportStatus}</span>
              </div>

              <div class="dvd-actions">
                <button type="button" disabled={isDetectingDvd || isImportingDvd} onclick={() => void handleDetectDvd()}>
                  {isDetectingDvd ? "Detecting..." : "Detect DVD"}
                </button>
                <button type="button" disabled={isImportingDvd} onclick={() => void handleChooseVideoTsFolder()}>
                  Choose VIDEO_TS Folder
                </button>
                <button type="button" disabled={!dvdSourcePath || isScanningDvdTitles || isImportingDvd} onclick={() => void handleScanDvdTitles()}>
                  {isScanningDvdTitles ? "Scanning titles..." : "Scan DVD Titles"}
                </button>
                <button type="button" disabled={isImportingDvd} onclick={() => void handleChooseDvdOutputFolder()}>
                  Choose Output Folder
                </button>
                <button class="primary" type="button" disabled={isImportingDvd || !selectedDvdTitleNumber || !dvdSourcePath || !dvdOutputFolder} onclick={() => void handleImportDvdTitle()}>
                  {isImportingDvd ? "Importing..." : "Import Selected Title"}
                </button>
              </div>

              <div class="dvd-status-grid">
                <div><span>Source</span><strong>{dvdSourcePath ?? "No DVD detected"}</strong></div>
                <div><span>Source type</span><strong>{dvdSourceType === "video_ts_folder" ? "VIDEO_TS folder" : dvdSourceType === "physical_device" ? "Physical DVD" : "Not selected"}</strong></div>
                <div><span>Output folder</span><strong>{dvdOutputFolder ?? "Not selected"}</strong></div>
                <div><span>Selected title</span><strong>{selectedDvdTitleNumber ? `Title ${String(selectedDvdTitleNumber).padStart(2, "0")}` : "Not selected"}</strong></div>
              </div>

              {#if dvdImportError}
                <p class="form-message error" role="alert">{dvdImportError}</p>
              {:else if dvdImportMessage}
                <p class="form-message" role="status">{dvdImportMessage}</p>
              {/if}

              {#if dvdTitles.length > 0}
                <div class="dvd-title-table" role="table" aria-label="DVD titles">
                  <div class="dvd-title-row head" role="row">
                    <span>Title</span>
                    <span>Duration</span>
                    <span>Chapters</span>
                    <span>Suggested</span>
                    <span>Select</span>
                  </div>
                  {#each dvdTitles as title (title.number)}
                    <div class:active={selectedDvdTitleNumber === title.number} class="dvd-title-row" role="row">
                      <span>{String(title.number).padStart(2, "0")}</span>
                      <span>{title.duration ?? formatVideoDuration(title.durationSeconds)}</span>
                      <span>{title.chapters ?? "Unknown"}</span>
                      <span>{title.likelyMainTitle ? "Suggested" : ""}</span>
                      <label>
                        <input
                          type="radio"
                          name="dvd-title"
                          checked={selectedDvdTitleNumber === title.number}
                          onchange={() => handleSelectDvdTitle(title.number)}
                        />
                      </label>
                    </div>
                  {/each}
                </div>
              {/if}

              <form class="dvd-import-form" onsubmit={(event) => { event.preventDefault(); void handleImportDvdTitle(); }}>
                <label>
                  <span>Title</span>
                  <input type="text" value={dvdImportDraft.title} disabled={isImportingDvd} oninput={(event) => updateDvdImportDraftField("title", inputValue(event))} />
                </label>
                <label>
                  <span>Artist</span>
                  <input type="text" value={dvdImportDraft.artist} disabled={isImportingDvd} oninput={(event) => updateDvdImportDraftField("artist", inputValue(event))} />
                </label>
                <label>
                  <span>Video Type</span>
                  <select value={dvdImportDraft.videoType} disabled={isImportingDvd} onchange={(event) => updateDvdImportDraftField("videoType", selectValue(event))}>
                    {#each videoTypeOptions as option}
                      <option value={option.value}>{option.label}</option>
                    {/each}
                  </select>
                </label>
                <label>
                  <span>Release / Collection</span>
                  <input type="text" value={dvdImportDraft.releaseOrCollection} disabled={isImportingDvd} oninput={(event) => updateDvdImportDraftField("releaseOrCollection", inputValue(event))} />
                </label>
                <label>
                  <span>Year</span>
                  <input type="number" min="1" max="9999" value={dvdImportDraft.year} disabled={isImportingDvd} oninput={(event) => updateDvdImportDraftField("year", inputValue(event))} />
                </label>
                <label>
                  <span>Venue</span>
                  <input type="text" value={dvdImportDraft.venue} disabled={isImportingDvd} oninput={(event) => updateDvdImportDraftField("venue", inputValue(event))} />
                </label>
                <label>
                  <span>City</span>
                  <input type="text" value={dvdImportDraft.city} disabled={isImportingDvd} oninput={(event) => updateDvdImportDraftField("city", inputValue(event))} />
                </label>
                <label>
                  <span>Country</span>
                  <input type="text" value={dvdImportDraft.country} disabled={isImportingDvd} oninput={(event) => updateDvdImportDraftField("country", inputValue(event))} />
                </label>
                <label class="wide">
                  <span>Description / Notes</span>
                  <textarea disabled={isImportingDvd} value={dvdImportDraft.descriptionOrNotes} oninput={(event) => updateDvdImportDraftField("descriptionOrNotes", textAreaValue(event))}></textarea>
                </label>
                <label class="wide">
                  <span>Output filename</span>
                  <input type="text" value={dvdImportDraft.outputFilename} disabled={isImportingDvd} oninput={(event) => updateDvdImportDraftField("outputFilename", inputValue(event))} />
                </label>
              </form>

              {#if dvdTitleScan?.rawOutput}
                <details class="cd-debug-output">
                  <summary>DVD tool output</summary>
                  <pre>{dvdTitleScan.rawOutput}</pre>
                </details>
              {/if}
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
          <header class="cd-ripper-header">
            <div>
              <h2 id="cd-ripper-title">CD Rip</h2>
              <p>Save your audio CD as lossless FLAC files.</p>
            </div>
            <div class="cd-rip-actions">
              <button class:primary={!audioCdFound} type="button" disabled={isDetectingCd || isRippingCd} onclick={() => void handleDetectCd()}>
                {isDetectingCd ? "Detecting…" : audioCdFound ? "Detect again" : "Detect CD"}
              </button>
              <button class:primary={audioCdFound} type="button" disabled={isDetectingCd || isRippingCd} onclick={() => void handleRipCd()}>
                {isRippingCd ? "Ripping…" : "Rip CD"}
              </button>
            </div>
          </header>

          <dl class="cd-overview" aria-label="Disc and rip status">
            <div>
              <dt>Optical drive</dt>
              <dd>{cdDriveFound === null ? "Not checked" : cdDriveFound ? "Connected" : "Not detected"}</dd>
            </div>
            <div>
              <dt>Audio CD</dt>
              <dd>{audioCdFound === null ? "Not checked" : audioCdFound ? "Ready" : "No disc"}</dd>
            </div>
            <div>
              <dt>Tracks</dt>
              <dd>{cdTracks.length || "—"}</dd>
              {#if cdDurationSeconds > 0}<small>{formatTrackDuration(cdDurationSeconds)}</small>{/if}
            </div>
          </dl>

          <section class="cd-status-card" aria-labelledby="cd-status-title">
            <div class="settings-section-header">
              <h4 id="cd-status-title">Destination</h4>
              <span class="settings-pill">FLAC</span>
            </div>
            <div class="cd-destination">
              <div class="cd-output-folder">
                <span>Output folder</span>
                <strong>{cdOutputFolder ?? "Choose where to save your tracks"}</strong>
              </div>
              <div class="cd-rip-actions">
                <button type="button" disabled={isRippingCd} onclick={() => void handleChooseCdOutputFolder()}>
                  {cdOutputFolder ? "Change folder" : "Choose folder"}
                </button>
                {#if lastRippedFolder}
                  <button type="button" disabled={isScanning || isRippingCd} onclick={() => void handleScanRippedFolder()}>
                    {isScanning ? "Scanning…" : "Add ripped tracks to library"}
                  </button>
                {/if}
              </div>
            </div>

            {#if lastRippedFolder}
              <div class="cd-output-folder">
                <span>Last rip</span>
                <strong>{lastRippedFolder}</strong>
              </div>
            {/if}

            {#if isRippingCd || cdCompletedCount || cdFailedCount}
              <div class="cd-rip-progress">
                <div>
                  <strong>{isRippingCd ? "Ripping tracks" : "Rip results"}</strong>
                  <span>{cdCompletedCount} of {cdTracks.length} saved{cdFailedCount ? ` · ${cdFailedCount} failed` : ""}</span>
                </div>
                <progress aria-label="CD rip progress" max={Math.max(1, cdTracks.length)}
                  value={cdTracks.length ? cdCompletedCount + cdFailedCount : undefined}></progress>
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
                <h4 id="cd-metadata-title">Album details</h4>
              </div>
              <span class="settings-pill">{cdDiscId ? `Disc ID ${cdDiscId}` : "Manual mode"}</span>
            </div>

            <div class="cd-rip-actions">
              <button type="button" disabled={isLookingUpCdMetadata || isDetectingCd || isRippingCd} onclick={() => void handleLookupCdMetadata()}>
                {isLookingUpCdMetadata ? "Looking up metadata…" : "Look up on MusicBrainz"}
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
              <summary>Drive details</summary>
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
                  activityLabel={trackActivityLabel}
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
                  activityLabel={trackActivityLabel}
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
                  activityLabel={trackActivityLabel}
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
                  activityLabel={trackActivityLabel}
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
                  activityLabel={trackActivityLabel}
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
                  activityLabel={trackActivityLabel}
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
            <header class="organizer-header">
              <h2 id="settings-title">Settings</h2>
            </header>
            <div class="settings-layout">
              <nav class="settings-index" aria-label="Settings sections">
                {#each [
                  { id: "settings-library-title", label: "Library" },
                  { id: "settings-playback-title", label: "Playback" },
                  { id: "history-transfer-title", label: "Listening history" },
                  { id: "settings-interface-title", label: "Appearance" },
                  { id: "settings-tools-title", label: "Tools" },
                  { id: "settings-updates-title", label: "Updates" },
                  { id: "settings-about-title", label: "About" },
                ] as section (section.id)}
                  <button type="button" onclick={() => jumpToSetting(section.id)}>{section.label}</button>
                {/each}
              </nav>
              <div class="settings-sections">

                <section class="settings-section" aria-labelledby="settings-library-title">
                  <div class="settings-section-header">
                    <h3 id="settings-library-title" tabindex="-1">Library</h3>
                    {#if !hasLoadedCache}<span class="settings-pill">Loading library…</span>{/if}
                  </div>

                  <dl class="settings-stat-grid" aria-label="Library summary">
                    {#each librarySettingsStats as stat (stat.label)}
                      <div><dt>{stat.label}</dt><dd>{hasLoadedCache ? stat.value.toLocaleString() : "—"}</dd></div>
                    {/each}
                  </dl>
                  <div class="library-folder-card">
                    <span>Music folder</span>
                    <strong>{scannedFolder ?? "No library folder selected"}</strong>
                    {#if lastScannedAt}<small>Last scan {lastScanLabel}</small>{/if}
                  </div>
                  {#if isScanning && scanProgress}
                    <div class="folder-scan-progress" role="status">
                      <progress aria-label="Scanning library"></progress>
                      <span>{scanProgress.checked} audio files checked in {scanProgress.path}</span>
                    </div>
                  {:else if scanSummary}
                    <p class="settings-note" role="status">{scanSummary}</p>
                  {/if}
                  {#if libraryFolders.length > 0}
                    <ul class="folder-scan-list">
                      {#each libraryFolders as folder (folder.path)}
                        <li>
                          <div><strong>{folder.path}</strong>
                            {#if folder.error}
                              <small>{folder.error} · Existing tracks kept</small>
                            {:else if folder.scannedAt}
                              <small>{folder.added} added · {folder.updated} updated · {folder.skipped} skipped</small>
                            {/if}
                          </div>
                          <button type="button" disabled={isScanning} onclick={() => void handleScanLibrary(false, false, folder.path)}>Rescan folder</button>
                        </li>
                      {/each}
                    </ul>
                  {/if}

                  <div class="settings-actions">
                    <button class="primary" type="button" disabled={isScanning} onclick={() => void handleScanLibrary()}>
                      {isScanning ? "Scanning..." : scannedFolder ? "Rescan Library" : "Scan Library"}
                    </button>
                    <button type="button" disabled={isScanning} onclick={() => void handleScanLibrary(true)}>
                      Change Library Folder
                    </button>
                    <button type="button" disabled={isScanning} onclick={() => void handleScanLibrary(false, true)}>Add Music Folder</button>
                  </div>
                </section>

                <section class="settings-section" aria-labelledby="settings-playback-title">
                  <div class="settings-section-header">
                    <h3 id="settings-playback-title" tabindex="-1">Playback</h3>
                    <span class="settings-pill">{isPlaying ? "Playing" : currentTrack ? "Paused" : "Idle"}</span>
                  </div>
                  <div class="settings-status-list">
                    <div>
                      <span>Shuffle</span>
                      <strong>{isShuffleEnabled ? "On" : "Off"}</strong>
                    </div>
                    <div>
                      <span>Repeat</span>
                      <strong>{repeatMode === "all" ? "Queue" : repeatMode === "one" ? "Track" : "Off"}</strong>
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

                  <div class="settings-control-list">
                    <label class="settings-toggle-row">
                      <span>Gapless playback</span>
                      <input type="checkbox" checked={gaplessEnabled} onchange={(event) => {
                        gaplessEnabled = event.currentTarget.checked;
                        writePreference("cassette:gapless", gaplessEnabled ? "on" : "off");
                      }} />
                      <small>Play consecutive tracks without a pause between them.</small>
                    </label>
                    <label class="settings-toggle-row">
                      <span>Auto-find lyrics</span>
                      <input type="checkbox" checked={autoFindLyricsEnabled} onchange={handleAutoFindLyricsSettingChange} />
                      <small>Look for lyrics when a track plays.</small>
                    </label>
                    {#if platformCapabilities?.linuxNotifications !== false}
                      <label class="settings-toggle-row">
                        <span>Track change notifications</span>
                        <input
                          type="checkbox"
                          checked={trackNotificationsEnabled}
                          onchange={handleTrackNotificationsSettingChange}
                        />
                        <small>Show a desktop notification when the track changes.</small>
                      </label>
                    {/if}
                  </div>

                  {#if platformCapabilities?.linuxNotifications !== false && trackNotificationsEnabled}
                    <div class="settings-actions">
                      <button type="button" onclick={handleTestTrackNotification}>
                        Test Notification
                      </button>
                    </div>
                    {#if trackNotificationStatus}
                      <p class="form-message" class:error={trackNotificationStatusIsError} role="status">{trackNotificationStatus}</p>
                    {/if}
                  {/if}

                  <div class="settings-actions">
                    <button type="button" disabled={playbackQueue.length === 0} onclick={handleSettingsClearQueue}>
                      Clear Queue
                    </button>
                  </div>
                </section>

                <section class="settings-section stats-transfer-panel" aria-labelledby="history-transfer-title">
                  <div class="settings-section-header">
                    <h3 id="history-transfer-title" tabindex="-1">Listening history</h3>
                  </div>
                  <label class="listen-threshold">
                    <span>Show listening activity as</span>
                    <select value={listeningActivityDisplay} onchange={(event) => {
                      const value = event.currentTarget.value as ListeningActivityDisplay;
                      listeningActivityDisplay = value;
                      writePreference("cassette:listening-time", value);
                    }}>
                      <option value="0">Play count</option>
                      <option value="1">Listening time</option>
                      <option value="2">Both</option>
                    </select>
                  </label>
                  <label class="listen-threshold">
                    <span>Count a listen after</span>
                    <select value={listenPercentage} onchange={(event) => {
                      listenPercentage = Number(selectValue(event));
                      writePreference("cassette:listen-percentage", String(listenPercentage));
                    }}>
                      {#each listenPercentages as percentage}
                        <option value={percentage}>{percentage}% of the track</option>
                      {/each}
                    </select>
                  </label>
                  <p class="settings-note">A session counts after at least five seconds of playback. Older play counts may have no saved listening time.</p>
                  <details class="ignored-tracks">
                    <summary>Ignored tracks ({ignoredTrackIds.size})</summary>
                    {#if ignoredTrackIds.size === 0}
                      <p>Right-click a song to ignore it. On an album, choose Select tracks to ignore several together.</p>
                    {:else}
                      <ul class="folder-scan-list">
                        {#each [...ignoredTrackIds].sort() as id (id)}
                          {@const track = libraryTracksById.get(id)}
                          <li>
                            <div><strong>{track?.title ?? id}</strong>{#if track?.album}<small>{track.album}</small>{/if}</div>
                            <button type="button" disabled={savingIgnoredTracks} onclick={() => void updateIgnoredTracks([id], false)}>Stop ignoring</button>
                          </li>
                        {/each}
                      </ul>
                    {/if}
                  </details>
                  <div class="settings-subsection">
                    <h4>History backup</h4>
                    <p>Back up your play history and review matches before importing. Listening time is stored on this device.</p>
                    <div class="stats-transfer-actions">
                      <button type="button" disabled={historyBusy} onclick={() => void handleExportListeningHistory()}>Export listening history</button>
                      <button type="button" disabled={historyBusy} onclick={() => void handlePreviewListeningHistory()}>Import listening history…</button>
                    </div>
                    {#if historyBusy}<p role="status" class="stats-status">Preparing listening history… Verifying audio identities can take a while for a large library.</p>{/if}
                    {#if historyError}<p role="alert" class="stats-status">{historyError}</p>{/if}
                    {#if historyMessage}<p role="status" class="stats-status">{historyMessage}</p>{/if}
                    {#if historyImportPreview}
                      <div class="stats-import-preview">
                        <h5>Import preview</h5>
                        <p>{historyImportPreview.matchedTracks} matched · {historyImportPreview.unmatchedTracks} unmatched · {historyImportPreview.ambiguousTracks} ambiguous tracks</p>
                        <p>{historyImportPreview.newEvents} new · {historyImportPreview.duplicateEvents} duplicate dated events · {historyImportPreview.undatedPlays} source undated plays</p>
                        {#if historyImportPreview.conflicts.length > 0}
                          <ul>{#each historyImportPreview.conflicts as conflict}<li>{conflict}</li>{/each}</ul>
                        {/if}
                        <div class="stats-import-tracks">
                          {#each historyImportPreview.tracks.slice(0, historyPreviewLimit) as item (item.referenceId)}
                            <div class="stats-import-track">
                              <strong>{item.title}</strong><span>{item.artist ?? "Unknown Artist"} · {item.status} · {item.newEvents} new / {item.duplicateEvents} duplicate events</span>
                              {#if item.conflict}<small>{item.conflict}</small>{/if}
                              {#if item.status === "ambiguous" && item.candidates.length > 0}
                                <label for={`associate-${item.referenceId}`}>Associate audio-identical track</label>
                                <select id={`associate-${item.referenceId}`} value={historyAssociationTargets[item.pendingTrackId] ?? ""} onchange={(event) => { historyAssociationTargets[item.pendingTrackId] = event.currentTarget.value; }}>
                                  <option value="">Choose a track</option>
                                  {#each item.candidates as candidate}<option value={candidate}>{tracks.find((track) => track.id === candidate)?.title ?? candidate}</option>{/each}
                                </select>
                                <button type="button" disabled={historyBusy || !item.retained || !historyAssociationTargets[item.pendingTrackId]} onclick={() => void handleAssociateListeningHistory(item.pendingTrackId)}>Associate retained history</button>
                                {#if !item.retained}<small>Import first to retain this history, then choose an audio-identical track.</small>{/if}
                              {/if}
                            </div>
                          {/each}
                        </div>
                        {#if historyPreviewLimit < historyImportPreview.tracks.length}
                          <button type="button" onclick={() => { historyPreviewLimit = nextStatsLimit(historyPreviewLimit, historyImportPreview?.tracks.length ?? 0); }}>Show 50 more track references</button>
                        {/if}
                        <p>Showing {Math.min(historyPreviewLimit, historyImportPreview.tracks.length)} of {historyImportPreview.tracks.length} track references. All references are included when importing.</p>
                        <button type="button" disabled={historyBusy || historyImportPreview.conflicts.length > 0} onclick={() => void handleApplyListeningHistory()}>Import new history</button>
                        <button type="button" disabled={historyBusy} onclick={() => { historyImportPreview = null; historyImportPath = null; }}>Cancel preview</button>
                      </div>
                    {/if}
                  </div>
                </section>

                <section class="settings-section" aria-labelledby="settings-interface-title">
                  <div class="settings-section-header">
                    <h3 id="settings-interface-title" tabindex="-1">Appearance</h3>
                  </div>

                  <div class="theme-preset-grid" aria-label="Theme presets">
                    {#each themePresets as theme}
                      <button
                        class:selected={selectedTheme === theme.id}
                        type="button"
                        aria-pressed={selectedTheme === theme.id}
                        onclick={() => handleThemeSelect(theme.id)}
                      >
                        <span class="theme-swatch-row" aria-hidden="true">
                          {#each theme.swatches as swatch}
                            <span style={`--swatch-color: ${swatch}`}></span>
                          {/each}
                        </span>
                        <strong>{theme.name}</strong>
                        <small>{theme.description}</small>
                      </button>
                    {/each}
                  </div>

                  <div class="settings-control-list">
                    <label class="settings-toggle-row">
                      <span>Songs list numbers</span>
                      <input type="checkbox" checked={showSongListNumbers} onchange={handleSongListNumbersSettingChange} />
                      <small>Show each song’s position in the current sort order.</small>
                    </label>
                    <label class="settings-toggle-row">
                      <span>Album listening totals</span>
                      <input type="checkbox" checked={showAlbumPlayCounts} onchange={(event) => updatePlayCountVisibility(event, "albums")} />
                      <small>Show the selected measure for each album.</small>
                    </label>
                    <label class="settings-toggle-row">
                      <span>Artist listening totals</span>
                      <input type="checkbox" checked={showArtistPlayCounts} onchange={(event) => updatePlayCountVisibility(event, "artists")} />
                      <small>Show the selected measure for each artist.</small>
                    </label>
                    <label class="settings-toggle-row">
                      <span>Genre listening totals</span>
                      <input type="checkbox" checked={showGenrePlayCounts} onchange={(event) => updatePlayCountVisibility(event, "genres")} />
                      <small>Each track contributes its listening activity to every assigned genre.</small>
                    </label>
                    <label class="settings-toggle-row">
                      <span>Album track listening totals</span>
                      <input type="checkbox" checked={showAlbumTrackPlayCounts} onchange={(event) => updatePlayCountVisibility(event, "albumTracks")} />
                      <small>Show each song’s listening activity in album detail.</small>
                    </label>
                  </div>
                </section>

                <section class="settings-section" aria-labelledby="settings-tools-title">
                  <div class="settings-section-header">
                    <h3 id="settings-tools-title" tabindex="-1">Tools</h3>
                  </div>

                  <div class="settings-tool-grid">
                    <button type="button" onclick={handleLibraryHealthSelect}>
                      <span class="health-mark" aria-hidden="true">H</span>
                      <span class="settings-tool-copy">
                        <strong>Library Health</strong>
                        <small>{libraryHealthIssueCount} {libraryHealthIssueCount === 1 ? "issue" : "issues"} found</small>
                      </span>
                    </button>
                    <button type="button" onclick={openShortcutHelp}>
                      <span class="shortcut-mark" aria-hidden="true">?</span>
                      <strong>Keyboard Shortcuts</strong>
                    </button>
                    <button type="button" onclick={handleMixBuilderSelect}>
                      <span class="mix-tool-mark" aria-hidden="true">M</span>
                      <span class="settings-tool-copy">
                        <strong>Mix Builder</strong>
                        <small>Build a queue from your library</small>
                      </span>
                    </button>
                  </div>
                </section>

                <section class="settings-section updates-section" aria-labelledby="settings-updates-title">
                  <div class="settings-section-header">
                    <h3 id="settings-updates-title" tabindex="-1">Updates</h3>
                    <span class="settings-pill">{isWindows ? "Windows prerelease" : "Linux beta channel"}</span>
                  </div>

                  <div class="settings-control-list">
                    <label class="settings-toggle-row">
                      <span>Automatic update checks</span>
                      <input
                        type="checkbox"
                        checked={automaticUpdateChecksEnabled}
                        onchange={handleAutomaticUpdateSettingChange}
                      />
                      <small>At most once every 24 hours. Available updates are never installed automatically.</small>
                    </label>
                    <div>
                      <span>Installation type</span>
                      <strong>{updatePackageLabel}</strong>
                      <small>
                        {updateRuntime?.canSelfInstall
                          ? "Cassette installs verified updates after you confirm."
                          : "You can download a new installer from GitHub Releases."}
                      </small>
                    </div>
                  </div>

                  <div class="settings-actions">
                    <button type="button" disabled={!updateRuntime || updateIsBusy} onclick={handleManualUpdateCheck}>
                      {updateCheckState === "checking" ? "Checking..." : "Check for updates"}
                    </button>
                    <button type="button" onclick={handleViewUpdateDownload}>GitHub Releases</button>
                  </div>

                  {#if updateRuntime?.packageKind === "appimage" && !updateRuntime.development}
                    <div class="settings-control-list">
                      <div>
                        <span>Applications menu</span>
                        <strong>
                          {appImageMenuStatus?.needsRefresh
                            ? "Launcher needs refresh"
                            : appImageMenuStatus?.installed
                              ? "Cassette AppImage launcher added"
                              : "Launcher not added"}
                        </strong>
                        <small>
                          The launcher points to this AppImage's current location. If you move or delete the AppImage,
                          launch it from the new location and add or refresh the launcher again.
                        </small>
                        {#if appImageMenuStatus}
                          <small>Current AppImage: {appImageMenuStatus.imagePath}</small>
                        {/if}
                      </div>
                    </div>
                    <div class="settings-actions">
                      <button type="button" disabled={appImageMenuBusy || updateIsBusy} onclick={handleAddAppImageToMenu}>
                        {appImageMenuBusy ? "Working..." : appImageMenuStatus?.installed ? "Refresh applications-menu launcher" : "Add to applications menu"}
                      </button>
                      {#if appImageMenuStatus?.installed}
                        <button class="danger" type="button" disabled={appImageMenuBusy || updateIsBusy} onclick={handleRemoveAppImageFromMenu}>
                          Remove AppImage launcher
                        </button>
                      {/if}
                    </div>
                    {#if appImageMenuMessage}
                      <p class="form-message" class:error={appImageMenuIsError} role={appImageMenuIsError ? "alert" : "status"}>
                        {appImageMenuMessage}
                      </p>
                    {/if}
                  {/if}

                  {#if unavailableUpdateMessage(updateRuntime)}
                    <p class="settings-note">{unavailableUpdateMessage(updateRuntime)}</p>
                  {/if}

                  {#if lastSuccessfulUpdateCheck !== null}
                    <p class="settings-note">Last successful check: {formatDateTime(lastSuccessfulUpdateCheck / 1000)}</p>
                  {/if}

                  {#if availableUpdateVersion}
                    <article class="update-available-card">
                      <div>
                        <p class="eyebrow">Update available</p>
                        <h5>Cassette {availableUpdateVersion}</h5>
                      </div>
                      <div class="update-release-notes">
                        <strong>Release notes</strong>
                        <p>{availableUpdateNotes}</p>
                      </div>

                      {#if updateCheckState === "downloading" || updateCheckState === "installing"}
                        <div class="update-progress" aria-live="polite">
                          {#if updateDownloadProgress?.percent !== null && updateDownloadProgress?.percent !== undefined}
                            <progress max="100" value={updateDownloadProgress.percent}></progress>
                          {:else}
                            <progress></progress>
                          {/if}
                          <strong>{updateProgressLabel}</strong>
                        </div>
                      {:else}
                        <div class="settings-actions">
                          {#if updateRuntime?.canSelfInstall}
                            <button class="primary" type="button" disabled={updateIsBusy || updateWaitsForLibrary} onclick={handleInstallCassetteUpdate}>
                              Update and restart
                            </button>
                          {:else}
                            <button class="primary" type="button" onclick={handleViewUpdateDownload}>
                              View download
                            </button>
                          {/if}
                          <button type="button" onclick={handleUpdateLater}>Later</button>
                        </div>
                        {#if updateWaitsForLibrary}
                          <p class="settings-note">Finish the library operation before installing the update.</p>
                        {/if}
                      {/if}
                    </article>
                  {/if}

                  {#if updateStatusMessage}
                    <p class="form-message" class:error={updateStatusIsError} role={updateStatusIsError ? "alert" : "status"}>
                      {updateStatusMessage}
                    </p>
                  {/if}

                </section>

                <section class="settings-section about-section" aria-labelledby="settings-about-title">
                  <div class="settings-section-header">
                    <h3 id="settings-about-title" tabindex="-1">About</h3>
                  </div>

                  <div class="about-grid">
                    <div>
                      <span>Cassette</span>
                      <strong>{appVersion}</strong>
                    </div>
                    <div>
                      <span>Platform</span>
                      <strong>{isWindows ? "Windows" : "Linux"}</strong>
                    </div>
                  </div>
                </section>
              </div>
            </div>
          </section>
        {/if}
      {/if}
    </main>
  </div>

  {#if showUpdateNotice && availableUpdateVersion}
    <aside class="update-notice" aria-live="polite" aria-label="Cassette update available">
      <div>
        <strong>Cassette {availableUpdateVersion} is available</strong>
        <span>{updateRuntime?.canSelfInstall ? "Signed update" : "Package download"}</span>
      </div>
      <button type="button" onclick={handleReviewAvailableUpdate}>Review</button>
      <button type="button" onclick={handleUpdateLater}>Later</button>
    </aside>
  {/if}

  {#if isQueueOpen}
    {@const current = queuePanelEntries.find((entry) => entry.queueIndex === currentQueueIndex)}
    {@const upcoming = queuePanelEntries.filter((entry) => entry.queueIndex !== currentQueueIndex)}
    {#snippet queueRow(entry: QueuePanelEntry, nowPlaying: boolean)}
      <div class="queue-row" class:active={nowPlaying} role="group" aria-label={`Queue track ${entry.track.title}`}
        oncontextmenu={(event) => {
          event.preventDefault();
          openTrackContextMenu(entry.track, playbackQueue, event.clientX, event.clientY);
        }}>
        <button class="queue-row-main" type="button" title={entry.track.title}
          onclick={() => nowPlaying ? handleTogglePlayback() : playQueuedTrackAtIndex(entry.queueIndex)}>
          {#if !nowPlaying}<span class="queue-row-position">{queuePositionLabel(entry.offset)}</span>{/if}
          <span class="queue-row-art" aria-hidden="true">
            <span>{entry.track.title.slice(0, 1)}</span>
            {#if trackCoverPath(entry.track)}
              <img src={localImageSource(trackCoverPath(entry.track)) ?? ""} alt="" loading="lazy" onload={showLoadedImage} onerror={hideBrokenImage} />
            {/if}
          </span>
          <span class="queue-row-copy">
            {#if nowPlaying}<small class="queue-current-label">Now playing</small>{/if}
            <strong>{entry.track.title}</strong>
            <small>{entry.track.artist ?? "Unknown Artist"}{entry.track.album ? ` · ${entry.track.album}` : ""}</small>
          </span>
        </button>
        <span class="queue-row-duration">{formatPlaybackTime(entry.track.durationSeconds)}</span>
        {#if !nowPlaying}
          <button class="queue-remove" type="button" title="Remove from queue" aria-label={`Remove ${entry.track.title} from queue`}
            onclick={(event) => handleRemoveQueueTrack(event, entry.queueIndex)}>×</button>
        {/if}
      </div>
    {/snippet}

    <button class="queue-backdrop" type="button" aria-label="Dismiss queue" onclick={() => isQueueOpen = false}></button>
    <aside class="queue-panel" aria-label="Up Next">
      <header class="queue-panel-header">
        <div class="queue-panel-title">
          <h3>Up Next</h3>
          <span>{upcoming.length} upcoming</span>
        </div>
        <div class="queue-panel-actions">
          <button type="button" disabled={upcoming.length === 0} onclick={() => handleClearQueue(true)}>Clear</button>
          <button class="icon" type="button" aria-label="Close Up Next" onclick={() => isQueueOpen = false}>×</button>
        </div>
      </header>

      {#if current}{@render queueRow(current, true)}{/if}
      {#if upcoming.length}
        <div class="queue-list" aria-label="Upcoming tracks">
          {#each upcoming as entry (`${entry.track.id}-${entry.queueIndex}`)}
            {@render queueRow(entry, false)}
          {/each}
        </div>
      {:else}
        <div class="queue-empty-state">
          <strong>{current ? "No upcoming tracks" : "Queue is empty"}</strong>
          <p>Add tracks from a song, album, or playlist menu.</p>
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

  {#if lyricsEdit}
    <LyricsEditor title={lyricsEdit.track.title} initial={lyricsEdit}
      position={lyricsEdit.track.id === currentTrack?.id ? positionSeconds : null}
      onSave={handleSaveEditedLyrics} onClose={() => { lyricsEdit = null; }} />
  {/if}

  {#if isAlbumTagEditorOpen}
    <div class="tag-editor-backdrop" role="presentation" onclick={(event) => { if (event.target === event.currentTarget) closeAlbumTagEditor(); }}>
      <div
        bind:this={albumTagEditorModalElement}
        class="tag-editor-modal album-tag-editor-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="album-tag-editor-title"
        tabindex="-1"
      >
        <header class="tag-editor-header">
          <div class="tag-editor-art" aria-hidden="true">
            {#if selectedAlbum?.coverArtPath}
              <img src={localImageSource(selectedAlbum.coverArtPath) ?? ""} alt="" onload={showLoadedImage} onerror={hideBrokenImage} />
            {:else}
              <span>{selectedAlbum ? albumInitials(selectedAlbum) : "A"}</span>
            {/if}
          </div>
          <div class="tag-editor-heading">
            <p class="eyebrow">Album Tags</p>
            <h3 id="album-tag-editor-title">Edit album tags</h3>
            <strong>{selectedAlbum?.title ?? "Selected album"}</strong>
            <small>Only intentionally changed shared fields are written.</small>
          </div>
          <button type="button" aria-label="Close album tag editor" disabled={isSavingAlbumTagEditor} onclick={closeAlbumTagEditor}>Close</button>
        </header>

        {#if isLoadingAlbumTagEditor}
          <div class="tag-editor-loading" role="status">Inspecting album tracks and tags...</div>
        {:else if !albumTagEditorData}
          <p class="form-message error" role="alert">{albumTagEditorError ?? "Album tag information is unavailable."}</p>
          <div class="tag-editor-actions">
            <button type="button" onclick={closeAlbumTagEditor}>Close</button>
          </div>
        {:else if albumTagEditorStage === "edit"}
          <div class="album-tag-scope-summary">
            <strong>{albumTagEditorData.editableTrackCount} editable {albumTagEditorData.editableTrackCount === 1 ? "track" : "tracks"}</strong>
            <span>{albumTagEditorData.excludedTrackCount} read-only/excluded {albumTagEditorData.excludedTrackCount === 1 ? "track" : "tracks"}</span>
          </div>

          {#if albumTagEditorData.genreOverrideActive}
            <div class="tag-editor-warning" role="status">
              A Cassette album or artist genre override currently controls some displayed genres. File tags will still be written, but that separate override is preserved.
            </div>
          {/if}
          {#if albumTagEditorData.preflightErrors.length > 0}
            <div class="tag-editor-warning" role="alert">
              <strong>Tag preflight must pass before saving:</strong>
              <ul>
                {#each albumTagEditorData.preflightErrors as error}
                  <li>{error}</li>
                {/each}
              </ul>
            </div>
          {/if}

          <div class="album-tag-field-grid">
            {#each ALBUM_TAG_FIELDS as field (field.key)}
              <fieldset class:changed={albumTagEditorDraft[field.key].mode !== "unchanged"}>
                <legend>{field.label}</legend>
                <p>Current: <strong>{albumTagSharedLabel(field.key)}</strong></p>
                {#if field.help}
                  <p class="album-tag-field-help">{field.help}</p>
                {/if}
                <label>
                  <span>Action</span>
                  <select
                    value={albumTagEditorDraft[field.key].mode}
                    disabled={isSavingAlbumTagEditor}
                    onchange={(event) => setAlbumTagFieldMode(field.key, selectValue(event) as AlbumTagFieldMode)}
                  >
                    <option value="unchanged">Leave unchanged</option>
                    <option value="set">Set value</option>
                    <option value="clear">Clear field</option>
                  </select>
                </label>
                {#if albumTagEditorDraft[field.key].mode === "set"}
                  <label>
                    <span>New value</span>
                    <input
                      inputmode={field.inputMode}
                      value={albumTagEditorDraft[field.key].value}
                      disabled={isSavingAlbumTagEditor}
                      aria-invalid={field.key === "year" && albumTagEditorValidationError()?.startsWith("Year") ? "true" : "false"}
                      oninput={(event) => setAlbumTagFieldValue(field.key, inputValue(event))}
                    />
                  </label>
                {:else if albumTagEditorDraft[field.key].mode === "clear"}
                  <p class="album-tag-clear-note">This field will be removed from every editable track.</p>
                {/if}
              </fieldset>
            {/each}
          </div>

          <section class="album-tag-track-scope" aria-labelledby="album-tag-track-scope-title">
            <h4 id="album-tag-track-scope-title">Affected tracks</h4>
            <div class="album-tag-track-list">
              {#each albumTagEditorData.tracks as item (item.track.id)}
                <div class:excluded={!item.editable}>
                  <span>
                    <strong>{item.track.title}</strong>
                    <small>{item.track.fileName}</small>
                  </span>
                  <span class="album-tag-track-status">{item.editable ? `Editable ${item.detectedFormat}` : `${item.detectedFormat ?? "Unknown format"}: ${item.exclusionReason}`}</span>
                </div>
              {/each}
            </div>
          </section>

          {#if albumTagEditorError}
            <p class="form-message error" role="alert">{albumTagEditorError}</p>
          {/if}
          <div class="tag-editor-actions">
            <button type="button" onclick={closeAlbumTagEditor}>Cancel</button>
            <button
              class="primary"
              type="button"
              disabled={!albumTagEditorHasChanges() || albumTagEditorData.editableTrackCount === 0 || albumTagEditorData.preflightErrors.length > 0}
              onclick={showAlbumTagConfirmation}
            >
              Review changes
            </button>
          </div>
        {:else if albumTagEditorStage === "confirm"}
          <div class="album-tag-confirmation">
            <div class="tag-editor-warning" role="status">
              Review the exact shared-field changes and editable tracks. Individual titles, track/disc numbers, artwork, lyrics, audio, and unrelated metadata are not selected for change.
            </div>
            {#if albumTagEditorData.excludedTrackCount > 0 && albumTagGroupingMayChange()}
              <div class="tag-editor-warning" role="alert">
                <strong>This can split the album in Cassette.</strong>
                The read-only tracks cannot receive these grouping changes. They will remain visible under the original album; they are not deleted or removed from the library.
              </div>
            {/if}
            {#if albumTagEditorDraft.artist.mode !== "unchanged"}
              <div class="tag-editor-warning" role="alert">
                <strong>Artist applies to every editable track.</strong>
                Continue only if all of those tracks should use the same performer. Album artist is the shared grouping field for multi-artist albums.
              </div>
            {/if}
            <section>
              <h4>Changes</h4>
              <ul>
                {#each ALBUM_TAG_FIELDS as field (field.key)}
                  {@const summary = albumTagChangeSummary(field.key)}
                  {#if summary}<li>{summary}</li>{/if}
                {/each}
              </ul>
            </section>
            <section>
              <h4>Will be updated ({albumTagEditorData.editableTrackCount})</h4>
              <ul>
                {#each albumTagEditorData.tracks.filter((item) => item.editable) as item (item.track.id)}
                  <li>{item.track.title} <small>· {item.track.fileName}</small></li>
                {/each}
              </ul>
            </section>
            {#if albumTagEditorData.excludedTrackCount > 0}
              <section>
                <h4>Will remain read-only ({albumTagEditorData.excludedTrackCount})</h4>
                <ul>
                  {#each albumTagEditorData.tracks.filter((item) => !item.editable) as item (item.track.id)}
                    <li>{item.track.title} <small>· {item.exclusionReason}</small></li>
                  {/each}
                </ul>
              </section>
            {/if}
          </div>

          {#if isSavingAlbumTagEditor}
            <div class="album-tag-progress" role="status" aria-live="polite">
              <strong>{albumTagProgressLabel()}</strong>
              <progress max={Math.max(albumTagUpdateProgress?.total ?? 1, 1)} value={albumTagUpdateProgress?.completed ?? 0}></progress>
            </div>
          {/if}
          {#if albumTagEditorError}
            <p class="form-message error" role="alert">{albumTagEditorError}</p>
          {/if}
          <div class="tag-editor-actions">
            <button type="button" disabled={isSavingAlbumTagEditor} onclick={() => { albumTagEditorStage = "edit"; albumTagEditorError = null; }}>Back</button>
            <button type="button" disabled={isSavingAlbumTagEditor} onclick={closeAlbumTagEditor}>Cancel</button>
            <button class="primary" type="button" disabled={isSavingAlbumTagEditor} onclick={() => void saveAlbumTags()}>
              {isSavingAlbumTagEditor ? "Updating..." : `Update ${albumTagEditorData.editableTrackCount} ${albumTagEditorData.editableTrackCount === 1 ? "track" : "tracks"}`}
            </button>
          </div>
        {:else if albumTagUpdateResult}
          <div class:success={albumTagUpdateResult.outcome === "success"} class:error={albumTagUpdateResult.outcome !== "success"} class="album-tag-result" role="status">
            <strong>{albumTagUpdateResult.outcome === "success" ? "Album tags updated" : "Album update did not complete"}</strong>
            <p>{albumTagUpdateResult.summary}</p>
          </div>
          {#if albumTagMoveNotice}
            {@const moveNotice = albumTagMoveNotice}
            {#if moveNotice.editedAlbumId === selectedAlbum?.id}
              <div class="tag-editor-warning" role="status">
                The excluded read-only {moveNotice.excludedTrackCount === 1 ? "track remains" : "tracks remain"} under “{moveNotice.originalAlbumTitle}”. Nothing was deleted.
              </div>
            {/if}
          {/if}
          <div class="album-tag-result-list">
            {#each albumTagUpdateResult.trackResults as result (result.trackId)}
              <div>
                <span><strong>{result.fileName}</strong><small>{result.status.replaceAll("_", " ")}</small></span>
                <p>{result.message}</p>
                {#if result.backupPath}<code>Backup: {result.backupPath}</code>{/if}
                {#if result.recoveryPath}<code>Recovery copy: {result.recoveryPath}</code>{/if}
              </div>
            {/each}
          </div>
          <div class="tag-editor-actions">
            {#if albumTagUpdateResult.outcome === "rolled_back"}
              <button type="button" onclick={() => { albumTagEditorStage = "edit"; albumTagUpdateResult = null; }}>Review and retry</button>
            {/if}
            <button class="primary" type="button" onclick={closeAlbumTagEditor}>Close</button>
          </div>
        {/if}
      </div>
    </div>
  {/if}

  {#if tagEditorTrack}
    <div class="tag-editor-backdrop" role="presentation" onclick={(event) => { if (event.target === event.currentTarget) requestCloseTagEditor(); }}>
      <div
        bind:this={tagEditorModalElement}
        class="tag-editor-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="tag-editor-title"
        tabindex="-1"
      >
        <header class="tag-editor-header">
          <div class="tag-editor-art" aria-hidden="true">
            {#if tagEditorCoverSource(tagEditorTrack)}
              <img src={tagEditorCoverSource(tagEditorTrack) ?? ""} alt="" onload={showLoadedImage} onerror={hideBrokenImage} />
            {:else}
              <span>{tagEditorTrack.title.slice(0, 1)}</span>
            {/if}
          </div>
          <div class="tag-editor-heading">
            <p class="eyebrow">Audio Tags</p>
            <h3 id="tag-editor-title">Edit Tags</h3>
            <strong>{tagEditorTrack.title}</strong>
            <small>{tagEditorTrack.fileName} · {tagEditorData?.detectedFormat ?? tagEditorTrack.extension.toUpperCase()}</small>
          </div>
          <button type="button" aria-label="Close tag editor" disabled={isSavingTagEditor} onclick={() => requestCloseTagEditor()}>Close</button>
        </header>

        <div class="tag-editor-path">
          <span>File path</span>
          <code>{tagEditorTrack.filePath}</code>
        </div>

        {#if isLoadingTagEditor}
          <div class="tag-editor-loading" role="status">Loading tags...</div>
        {:else}
          {#if tagEditorData?.genreOverrideActive}
            <div class="tag-editor-warning" role="status">
              A Cassette library override currently controls the displayed genre for this album or artist.
            </div>
          {/if}

          {#if tagEditorData?.unsupportedReason}
            <div class="tag-editor-warning" role="status">{tagEditorData.unsupportedReason}</div>
          {/if}

          <form class="tag-editor-form" onsubmit={(event) => { event.preventDefault(); void handleSaveTagEditor(); }}>
            <label class:changed={tagEditorFieldChanged("title")}>
              <span>Title</span>
              <input value={tagEditorDraft.title} oninput={(event) => updateTagEditorField("title", inputValue(event))} disabled={isSavingTagEditor || !tagEditorData?.tagEditingSupported} />
            </label>
            <label class:changed={tagEditorFieldChanged("artist")}>
              <span>Artist</span>
              <input value={tagEditorDraft.artist} oninput={(event) => updateTagEditorField("artist", inputValue(event))} disabled={isSavingTagEditor || !tagEditorData?.tagEditingSupported} />
            </label>
            <label class:changed={tagEditorFieldChanged("album")}>
              <span>Album</span>
              <input value={tagEditorDraft.album} oninput={(event) => updateTagEditorField("album", inputValue(event))} disabled={isSavingTagEditor || !tagEditorData?.tagEditingSupported} />
            </label>
            <label class:changed={tagEditorFieldChanged("albumArtist")}>
              <span>Album Artist</span>
              <input value={tagEditorDraft.albumArtist} oninput={(event) => updateTagEditorField("albumArtist", inputValue(event))} disabled={isSavingTagEditor || !tagEditorData?.tagEditingSupported} />
            </label>
            <label class:changed={tagEditorFieldChanged("genre")}>
              <span>Genre</span>
              <input value={tagEditorDraft.genre} oninput={(event) => updateTagEditorField("genre", inputValue(event))} disabled={isSavingTagEditor || !tagEditorData?.tagEditingSupported} />
            </label>
            <label class:changed={tagEditorFieldChanged("year")}>
              <span>Year</span>
              <input inputmode="numeric" value={tagEditorDraft.year} aria-invalid={tagEditorValidationError()?.startsWith("Year") ? "true" : "false"} oninput={(event) => updateTagEditorField("year", inputValue(event))} disabled={isSavingTagEditor || !tagEditorData?.tagEditingSupported} />
            </label>
            <label class:changed={tagEditorFieldChanged("trackNumber")}>
              <span>Track Number</span>
              <input inputmode="numeric" value={tagEditorDraft.trackNumber} aria-invalid={tagEditorValidationError()?.startsWith("Track number") ? "true" : "false"} oninput={(event) => updateTagEditorField("trackNumber", inputValue(event))} disabled={isSavingTagEditor || !tagEditorData?.tagEditingSupported} />
            </label>
            <label class:changed={tagEditorFieldChanged("discNumber")}>
              <span>Disc Number</span>
              <input inputmode="numeric" value={tagEditorDraft.discNumber} aria-invalid={tagEditorValidationError()?.startsWith("Disc number") ? "true" : "false"} oninput={(event) => updateTagEditorField("discNumber", inputValue(event))} disabled={isSavingTagEditor || !tagEditorData?.tagEditingSupported} />
            </label>

            {#if tagEditorError || tagEditorValidationError()}
              <p class="form-message error" role="alert">{tagEditorError ?? tagEditorValidationError()}</p>
            {:else if tagEditorMessage}
              <p class="form-message" role="status">{tagEditorMessage}</p>
            {/if}

            <div class="tag-editor-actions">
              <button type="button" disabled={isSavingTagEditor || !tagEditorHasChanges()} onclick={resetTagEditorDraft}>Reset</button>
              <button type="button" disabled={isSavingTagEditor} onclick={() => requestCloseTagEditor()}>Cancel</button>
              <button class="primary" type="submit" disabled={!tagEditorCanSave()}>
                {isSavingTagEditor ? "Saving..." : "Save Changes"}
              </button>
            </div>
          </form>
        {/if}
      </div>
    </div>
  {/if}

  {#if isTagEditorDiscardConfirmationOpen}
    <div class="confirmation-backdrop" role="presentation" onclick={handleTagEditorDiscardBackdropClick}>
      <div
        bind:this={tagEditorDiscardModalElement}
        class="confirmation-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="discard-tag-editor-title"
        aria-describedby="discard-tag-editor-description"
        tabindex="-1"
      >
        <header class="confirmation-header">
          <div>
            <p class="eyebrow">Audio Tags</p>
            <h3 id="discard-tag-editor-title">Unsaved Changes</h3>
          </div>
        </header>

        <p id="discard-tag-editor-description">Discard unsaved tag changes?</p>

        <div class="confirmation-actions">
          <button type="button" onclick={keepEditingTagEditor}>Keep Editing</button>
          <button class="destructive" type="button" onclick={discardTagEditorChanges}>Discard Changes</button>
        </div>
      </div>
    </div>
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

  {#if artworkViewer}
    <CoverArtViewer
      source={artworkViewer.source}
      title={artworkViewer.title}
      opener={artworkViewer.opener}
      fallbackFocus={mainElement}
      onClose={() => { artworkViewer = null; }}
    />
  {/if}

  {#if !isVideoPlaybackActive}
    <NowPlayingBar
      track={currentTrack ? { ...currentTrack, coverArtPath: currentTrackCoverArtPath } : null}
      {isPlaying}
      {positionSeconds}
      {durationSeconds}
      {volume}
      {canPlayPrevious}
      {canPlayNext}
      queueCount={playableQueueOrder.length}
      {isQueueOpen}
      {isShuffleEnabled}
      {repeatMode}
      compact={activeView === "Now Playing"}
      onTogglePlayback={handleTogglePlayback}
      onPrevious={handlePreviousTrack}
      onNext={handleNextTrack}
      onSeek={handleSeek}
      onVolumeChange={handleVolumeChange}
      onToggleFavorite={handleToggleFavorite}
      onToggleQueue={handleToggleQueue}
      onToggleShuffle={handleToggleShuffle}
      onToggleRepeat={handleToggleRepeat}
      onArtistSelect={handleTrackArtistSelect}
      onAlbumSelect={handleTrackAlbumSelect}
      onOpenLyrics={handleLyricsSelect}
    />
  {/if}
</div>

<style>
  :global(:root),
  :global(:root[data-theme="cassette-teal"]) {
    color-scheme: dark;
    --bg: #0d0f13;
    --bg-soft: #111317;
    --panel: #12161c;
    --panel-soft: #151a21;
    --panel-strong: #1a2028;
    --panel-hover: #1b2027;
    --border: #242b35;
    --border-strong: #303844;
    --text: #f4f7fb;
    --text-muted: #b9c3cf;
    --text-soft: #8f9aa8;
    --text-dim: #626c79;
    --accent: #2f8f83;
    --accent-soft: #17332f;
    --accent-strong: #35544f;
    --accent-text: #d8fffa;
    --accent-contrast: #07110f;
    --danger: #ffcbc8;
    --danger-soft: #2a1718;
    --warning: #f0c85a;
    --shadow: rgba(0, 0, 0, 0.22);
    --focus-ring: rgba(47, 143, 131, 0.55);
    --range-empty: #2a313c;
  }

  :global(:root[data-theme="rose-noir"]) {
    --bg: #0f0d11;
    --bg-soft: #131116;
    --panel: #15141a;
    --panel-soft: #18171e;
    --panel-strong: #1f1d25;
    --panel-hover: #25212b;
    --border: #2b2731;
    --border-strong: #3a3340;
    --text: #f7f4f7;
    --text-muted: #c7c0c8;
    --text-soft: #958c99;
    --text-dim: #6b6370;
    --accent: #bf667d;
    --accent-soft: #311a24;
    --accent-strong: #704051;
    --accent-text: #ffe7ee;
    --accent-contrast: #14070d;
    --shadow: rgba(0, 0, 0, 0.26);
    --focus-ring: rgba(191, 102, 125, 0.5);
    --range-empty: #2b2731;
  }

  :global(:root[data-theme="royal-gold"]) {
    --bg: #0f0f10;
    --bg-soft: #131313;
    --panel: #151516;
    --panel-soft: #181818;
    --panel-strong: #202020;
    --panel-hover: #272522;
    --border: #2c2b27;
    --border-strong: #3c3931;
    --text: #f7f5ef;
    --text-muted: #c8c2b4;
    --text-soft: #969083;
    --text-dim: #6b665b;
    --accent: #b99a4d;
    --accent-soft: #2e2512;
    --accent-strong: #6a572b;
    --accent-text: #ffefc1;
    --accent-contrast: #130f05;
    --shadow: rgba(0, 0, 0, 0.28);
    --focus-ring: rgba(185, 154, 77, 0.5);
    --range-empty: #2c2b27;
  }

  :global(:root[data-theme="glacier"]) {
    --bg: #071018;
    --bg-soft: #0b1620;
    --panel: #0e1924;
    --panel-soft: #111d28;
    --panel-strong: #172635;
    --panel-hover: #1c3041;
    --border: #24394c;
    --border-strong: #34536a;
    --text: #f2f9ff;
    --text-muted: #b7d2e2;
    --text-soft: #85a6b8;
    --text-dim: #5f7888;
    --accent: #79c7e8;
    --accent-soft: #123044;
    --accent-strong: #326d87;
    --accent-text: #e7f8ff;
    --accent-contrast: #06131b;
    --shadow: rgba(0, 0, 0, 0.26);
    --focus-ring: rgba(121, 199, 232, 0.52);
    --range-empty: #24394c;
  }

  :global(:root[data-theme="light"]) :is(.album-card, .playlist-hero, .artist-detail-header, .genre-detail-header, .album-detail-header) {
    border-color: var(--border);
    box-shadow: 0 18px 44px var(--shadow);
  }

  :global(:root[data-theme="light"]) :is(.playlist-hero, .artist-detail-header, .genre-detail-header, .album-detail-header) {
    background: linear-gradient(115deg, var(--panel-strong), var(--panel) 62%, var(--bg-soft));
  }

  :global(:root[data-theme="light"]) .album-detail-header::after {
    background: none;
  }

  :global(:root[data-theme="light"]) :is(.album-card:hover, .album-card:focus-within, .artist-card:hover, .artist-card:focus-visible, .genre-card:hover, .genre-card:focus-visible) {
    border-color: var(--accent-strong);
    background: var(--panel-hover);
  }

  :global(:root[data-theme="light"]) .album-art .album-art-placeholder {
    background:
      radial-gradient(circle at 50% 42%, color-mix(in srgb, var(--item-color) 28%, transparent), transparent 68%),
      var(--panel-strong);
  }

  :global(:root[data-theme="light"]) .album-art-placeholder small {
    color: var(--text-muted);
  }

  :global(:root[data-theme="light"]) :is(.album-detail-overflow, .album-card-actions button, .album-detail-actions button:not(:first-child)) {
    border-color: var(--border-strong);
    background: var(--panel);
  }

  :global(:root[data-theme="light"]) :is(.album-detail-overflow, .album-card-actions button, .album-detail-actions button:not(:first-child)):is(:hover, :focus-visible):not(:disabled) {
    border-color: var(--accent-strong);
    background: var(--accent-soft);
  }

  :global(:root[data-theme="light"]) :is(.album-card-actions button, .album-detail-actions button):disabled {
    border-color: var(--border);
    background: var(--bg-soft);
  }

  :global(:root[data-theme="light"]) :is(.album-detail-meta span, .playlist-hero-meta span, .album-genre-chip-list button.muted, .album-track-format, .lyrics-offset-controls) {
    border-color: var(--border);
    background: var(--bg-soft);
  }

  :global(:root[data-theme="light"]) :is(.album-genre-chip-list button:not(.muted), .lyrics-genre-links button) {
    border-color: var(--accent-strong);
    background: var(--accent-soft);
  }

  :global(:root[data-theme="light"]) :is(.album-genre-chip-list button:not(.muted), .lyrics-genre-links button):is(:hover, :focus-visible) {
    border-color: var(--accent);
    background: var(--panel-hover);
  }

  :global(:root[data-theme="light"]) .album-track-row:not(.active, :hover, :focus-visible) {
    border-color: var(--border);
    background: var(--panel);
  }

  :global(*) {
    box-sizing: border-box;
  }

  :global(html) {
    height: 100%;
    background: var(--bg);
    color: var(--text);
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
    background: var(--bg);
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
      radial-gradient(circle at top right, color-mix(in srgb, var(--accent) 18%, transparent), transparent 30rem),
      var(--bg);
  }

  .workspace {
    position: relative;
    display: flex;
    min-height: 0;
    overflow: hidden;
  }

  .workspace.lyrics-mode {
    display: block;
  }

  .workspace-navigation {
    position: absolute;
    top: 24px;
    left: 24px;
    z-index: 2;
    display: flex;
    align-items: center;
    gap: 12px;
    height: 42px;
  }

  .brand-mark {
    display: grid;
    width: 42px;
    height: 42px;
    place-items: center;
    border-radius: 8px;
    background: var(--accent);
    color: var(--accent-contrast);
    font-weight: 800;
  }

  .home {
    --player-height: 86px;
    --content-bottom-padding: 56px;
    width: 100%;
    min-width: 0;
    overflow: auto;
    padding: 32px 32px var(--content-bottom-padding);
    scroll-padding-bottom: var(--content-bottom-padding);
  }

  .home.albums-landing-view {
    --content-bottom-padding: 28px;
  }

  .home.album-detail-view {
    --content-bottom-padding: 24px;
  }

  .home.artist-detail-view {
    --content-bottom-padding: 28px;
  }

  .home.genre-detail-view {
    --content-bottom-padding: 28px;
  }

  .home.songs-library-view {
    --content-bottom-padding: 34px;
  }

  .home.lyrics-mode {
    padding: 32px 32px var(--content-bottom-padding);
  }

  .home-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 24px;
    margin-bottom: 32px;
  }

  .home-header > div:first-child {
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .eyebrow {
    margin: 0 0 6px;
    color: var(--accent);
    font-size: 0.78rem;
    font-weight: 800;
    letter-spacing: 0;
    text-transform: uppercase;
  }

  .home-header h2 {
    max-width: 720px;
    margin: 0;
    color: var(--text);
    font-size: clamp(2rem, 4vw, 3.8rem);
    line-height: 1.02;
  }

  .home-header button {
    min-width: 120px;
    min-height: 40px;
    border: 1px solid var(--accent-strong);
    border-radius: 8px;
    background: var(--accent-soft);
    color: var(--accent-text);
    cursor: default;
    font-weight: 800;
    padding: 0 14px;
  }

  .home-header button:disabled {
    border-color: var(--border-strong);
    background: var(--panel-strong);
    color: var(--text-soft);
  }

  .home-header-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 10px;
    padding-top: 6px;
  }

  .home.albums-landing-view .home-header {
    align-items: flex-start;
    gap: 18px;
    margin-bottom: 20px;
  }

  .home.albums-landing-view .home-header h2 {
    font-size: clamp(2.35rem, 3.5vw, 3.35rem);
  }

  .home.albums-landing-view .home-header-actions {
    align-items: center;
    padding-top: 7px;
  }

  .home.albums-landing-view .home-header button {
    min-width: 116px;
    min-height: 38px;
    padding: 0 13px;
  }

  .home.songs-library-view .home-header {
    align-items: flex-start;
    gap: 18px;
    margin-bottom: 18px;
  }

  .home.songs-library-view .home-header h2 {
    font-size: clamp(2.35rem, 3.4vw, 3.35rem);
  }

  .home.songs-library-view .scan-status {
    margin-top: 10px;
  }

  .search-bar {
    display: flex;
    align-items: center;
    gap: 10px;
    max-width: 720px;
    margin: -12px 0 24px;
  }

  .home.albums-landing-view > .search-bar {
    max-width: min(720px, 64vw);
    margin: -4px 0 18px;
  }

  .home.songs-library-view > .search-bar {
    max-width: min(860px, 100%);
    margin: -2px 0 16px;
  }

  .search-bar input {
    width: 100%;
    min-width: 0;
    min-height: 42px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel);
    color: var(--text);
    font: inherit;
    font-weight: 650;
    outline: none;
    padding: 0 14px;
  }

  .search-bar input::placeholder {
    color: var(--text-dim);
  }

  .search-bar input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 22%, transparent);
  }

  .album-track-search {
    max-width: 720px;
    margin-top: 18px;
  }

  .search-bar button {
    min-height: 42px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel-strong);
    color: var(--text);
    cursor: default;
    font: inherit;
    font-size: 0.86rem;
    font-weight: 800;
    padding: 0 13px;
  }

  .search-bar button:hover,
  .search-bar button:focus-visible {
    border-color: var(--accent-strong);
    background: var(--panel-hover);
    outline: none;
  }

  :global(select) {
    appearance: none;
    -webkit-appearance: none;
    min-height: 36px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background-color: var(--panel);
    background-image:
      linear-gradient(45deg, transparent 50%, var(--text-muted) 50%),
      linear-gradient(135deg, var(--text-muted) 50%, transparent 50%);
    background-position:
      calc(100% - 14px) 50%,
      calc(100% - 9px) 50%;
    background-repeat: no-repeat;
    background-size: 5px 5px;
    color: var(--text);
    color-scheme: inherit;
    font: inherit;
    font-size: 0.86rem;
    font-weight: 750;
    outline: none;
    padding: 0 32px 0 10px;
  }

  :global(select:not(:disabled):hover) {
    border-color: var(--accent-strong);
    background-color: var(--panel-hover);
  }

  :global(select:focus-visible),
  :global(select:focus) {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 22%, transparent);
  }

  :global(select:disabled) {
    border-color: var(--border);
    background-color: var(--bg-soft);
    color: var(--text-dim);
  }

  :global(select option) {
    background-color: var(--panel);
    color: var(--text);
  }

  :global(select option:disabled) {
    color: var(--text-dim);
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
    color: var(--text-soft);
    font-size: 0.82rem;
    font-weight: 800;
  }

  .control-bar select {
    min-height: 36px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background-color: var(--panel);
    color: var(--text);
    font: inherit;
    font-size: 0.86rem;
    font-weight: 750;
    outline: none;
    padding: 0 32px 0 10px;
  }

  .control-bar select:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 22%, transparent);
  }

  .direction-toggle {
    min-height: 36px;
    min-width: 58px;
    border: 1px solid var(--accent-strong);
    border-radius: 8px;
    background: var(--accent-soft);
    color: var(--accent-text);
    cursor: default;
    font: inherit;
    font-size: 0.82rem;
    font-weight: 900;
    padding: 0 10px;
  }

  .direction-toggle:hover,
  .direction-toggle:focus-visible {
    border-color: var(--accent);
    background: var(--accent-soft);
    outline: none;
  }

  .song-browser-controls {
    gap: 8px;
    align-items: center;
    width: 100%;
    padding: 2px 0 0;
  }

  .song-browser-controls :global(.compact-dropdown) {
    flex: 0 1 auto;
  }

  .home.songs-library-view :global(.library-section) {
    gap: 12px;
  }

  .home.songs-library-view :global(.section-header) {
    align-items: center;
  }

  .home.songs-library-view :global(.section-label) {
    min-height: 0;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--text-soft);
  }

  .songs-empty-state {
    min-height: 190px;
    background:
      radial-gradient(circle at 50% 0%, color-mix(in srgb, var(--accent) 10%, transparent), transparent 52%),
      color-mix(in srgb, var(--panel) 74%, transparent);
  }

  .scan-status {
    max-width: 720px;
    margin: 14px 0 0;
    overflow-wrap: anywhere;
    color: var(--text-soft);
    font-size: 0.95rem;
    font-weight: 650;
  }

  .home.albums-landing-view .scan-status {
    margin-top: 8px;
    font-size: 0.9rem;
  }

  .scan-error {
    border: 1px solid color-mix(in srgb, var(--danger) 38%, var(--border));
    border-radius: 8px;
    background: var(--danger-soft);
    color: var(--danger);
    font-size: 0.9rem;
    font-weight: 650;
    padding: 12px 14px;
  }

  .scan-error.status-message {
    border-color: var(--accent-strong);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .playlist-warning {
    border: 1px solid color-mix(in srgb, var(--warning) 34%, var(--border));
    border-radius: 8px;
    background: color-mix(in srgb, var(--warning) 14%, var(--panel));
    color: var(--warning);
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
    border: 1px dashed var(--border-strong);
    border-radius: 8px;
    background: color-mix(in srgb, var(--panel) 74%, transparent);
    padding: 20px;
    text-align: center;
  }

  .group-empty h3 {
    margin: 0 0 6px;
  }

  .group-empty p {
    margin: 0;
    color: var(--text-soft);
    font-size: 0.9rem;
    font-weight: 650;
  }

  .album-card-copy small,
  .artist-card p,
  .genre-card p {
    margin: 0;
    color: var(--text-soft);
    font-size: 0.9rem;
    font-weight: 620;
  }

  .album-card,
  .artist-card > div:last-child,
  .genre-card > div:last-child,
  .playlist-card-copy {
    min-width: 0;
  }

  .artist-card h3,
  .artist-card p,
  .genre-card h3,
  .genre-card p {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  h3 {
    margin: 0;
    color: var(--text);
    font-size: 0.98rem;
    line-height: 1.25;
  }

  .album-grid {
    display: grid;
    width: 100%;
    max-width: 1460px;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, 260px), 1fr));
    gap: 24px;
  }

  .albums-landing {
    width: 100%;
  }

  .albums-landing :global(.library-section) {
    gap: 16px;
  }

  .albums-landing :global(.section-header) {
    align-items: center;
    margin-bottom: -2px;
  }

  .albums-landing .control-bar {
    margin-bottom: 4px;
  }

  .albums-landing .album-grid {
    max-width: none;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, max(220px, calc((100% - 72px) / 5))), 1fr));
    gap: 18px;
  }

  .artist-grid,
  .genre-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 14px;
  }

  .artist-card,
  .genre-card {
    width: 100%;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-soft);
    color: inherit;
    cursor: default;
    font: inherit;
    text-align: left;
  }

  .album-card {
    position: relative;
    display: grid;
    min-width: 0;
    border: 1px solid rgba(50, 61, 75, 0.72);
    border-radius: 8px;
    background:
      linear-gradient(180deg, color-mix(in srgb, var(--panel-strong) 88%, transparent), color-mix(in srgb, var(--panel) 92%, transparent)),
      var(--panel-soft);
    box-shadow: 0 18px 42px rgba(0, 0, 0, 0.12);
    transition:
      border-color 160ms ease,
      background 160ms ease,
      box-shadow 160ms ease;
  }

  .album-card-main {
    display: grid;
    width: 100%;
    min-width: 0;
    color: inherit;
    font: inherit;
    padding: 13px;
    text-align: left;
  }

  .album-card:hover,
  .album-card:focus-within,
  .artist-card:hover,
  .artist-card:focus-visible,
  .genre-card:hover,
  .genre-card:focus-visible {
    border-color: var(--accent-strong);
    background:
      linear-gradient(180deg, rgba(29, 37, 46, 0.94), rgba(19, 24, 31, 0.96)),
      var(--panel-hover);
    box-shadow:
      0 24px 60px rgba(0, 0, 0, 0.2),
      0 0 0 1px color-mix(in srgb, var(--accent) 5%, transparent);
    outline: none;
  }

  .album-art {
    position: relative;
    display: grid;
    width: 100%;
    aspect-ratio: 1;
    overflow: hidden;
    place-items: center;
    margin-bottom: 10px;
    border-radius: 8px;
    background:
      radial-gradient(circle at 28% 18%, rgba(255, 255, 255, 0.28), transparent 28%),
      linear-gradient(135deg, rgba(255, 255, 255, 0.18), transparent 42%),
      var(--item-color);
    box-shadow:
      0 14px 30px rgba(0, 0, 0, 0.22),
      inset 0 0 0 1px rgba(255, 255, 255, 0.14);
  }

  .cover-open-button {
    border: 0;
    padding: 0;
    color: inherit;
    cursor: zoom-in;
  }

  .cover-open-button:focus-visible {
    outline: 3px solid var(--focus-ring);
    outline-offset: 4px;
  }

  .album-art img {
    position: absolute;
    inset: 0;
    z-index: 1;
    display: block;
    width: 100%;
    height: 100%;
    aspect-ratio: 1 / 1;
    filter: none;
    object-fit: cover;
    opacity: 1;
    transform: none;
  }

  .album-art-open {
    position: absolute;
    inset: 0;
    z-index: 2;
    border: 0;
    border-radius: inherit;
    background: transparent;
    cursor: default;
    padding: 0;
  }

  .album-art-open:focus-visible {
    outline: 2px solid rgba(216, 255, 250, 0.82);
    outline-offset: -4px;
  }

  .album-art .album-art-disc {
    display: block;
    width: 34%;
    aspect-ratio: 1;
    border: 10px solid color-mix(in srgb, var(--bg) 55%, transparent);
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.6);
  }

  .album-art .album-art-placeholder {
    box-sizing: border-box;
    display: grid;
    width: 100%;
    height: 100%;
    min-width: 0;
    place-content: center;
    gap: 8px;
    border: 0;
    border-radius: 0;
    background:
      radial-gradient(circle at 50% 42%, var(--item-color), transparent 58%),
      linear-gradient(145deg, rgba(33, 39, 49, 0.98), rgba(10, 13, 18, 0.98));
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.08);
    padding: 18px;
    text-align: center;
  }

  .album-art-placeholder strong {
    color: var(--text);
    font-size: clamp(1.7rem, 4vw, 2.4rem);
    font-weight: 950;
    line-height: 1;
  }

  .album-art-placeholder small {
    max-width: 86%;
    justify-self: center;
    overflow: hidden;
    color: rgba(222, 230, 238, 0.74);
    font-size: 0.72rem;
    font-weight: 850;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .album-card-copy {
    display: grid;
    width: 100%;
    gap: 5px;
    min-width: 0;
    border: 0;
    background: transparent;
    color: inherit;
    cursor: default;
    font: inherit;
    padding: 0 2px 0 0;
    text-align: left;
  }

  .album-card-copy:focus-visible {
    border-radius: 6px;
    outline: 2px solid rgba(216, 255, 250, 0.72);
    outline-offset: 4px;
  }

  .album-card-copy strong {
    display: -webkit-box;
    overflow: hidden;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    color: var(--text);
    font-size: 1rem;
    font-weight: 850;
    line-height: 1.24;
    white-space: normal;
  }

  .album-card-copy small {
    display: -webkit-box;
    overflow: hidden;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    color: var(--text-soft);
    font-size: 0.84rem;
    font-weight: 650;
    line-height: 1.35;
    white-space: normal;
  }

  .album-card-actions {
    position: absolute;
    left: 50%;
    bottom: 14px;
    z-index: 3;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    max-width: calc(100% - 28px);
    opacity: 0;
    pointer-events: none;
    transform: translate(-50%, 6px);
    transition:
      opacity 140ms ease,
      transform 140ms ease;
    white-space: nowrap;
  }

  .album-card:hover .album-card-actions,
  .album-card:focus-within .album-card-actions {
    opacity: 1;
    pointer-events: auto;
    transform: translate(-50%, 0);
  }

  .album-card-actions button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-height: 30px;
    border: 1px solid rgba(70, 82, 98, 0.74);
    border-radius: 999px;
    background: rgba(10, 13, 18, 0.76);
    box-shadow: 0 10px 24px rgba(0, 0, 0, 0.24);
    color: var(--text);
    cursor: default;
    font: inherit;
    font-size: 0.74rem;
    font-weight: 850;
    padding: 0 9px;
    backdrop-filter: blur(10px);
    white-space: nowrap;
  }

  .album-card-actions button:hover,
  .album-card-actions button:focus-visible {
    border-color: var(--accent);
    background: rgba(23, 51, 47, 0.92);
    color: var(--accent-text);
    outline: none;
  }

  .album-card-actions button:disabled {
    border-color: rgba(48, 56, 68, 0.72);
    background: color-mix(in srgb, var(--panel) 72%, transparent);
    color: var(--text-dim);
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
    color: var(--bg);
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
    color: var(--bg);
    font-size: 1.15rem;
    font-weight: 900;
  }

  .artist-card p,
  .genre-card p {
    margin-top: 4px;
  }

  .playlists-page,
  .playlist-detail-page {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    width: min(100%, 1320px);
    margin-inline: auto;
    gap: 22px;
    container: playlists / inline-size;
  }

  .playlist-feature-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 14px;
  }

  .playlists-page :global(.library-section) {
    width: 100%;
    gap: 12px;
  }

  .playlists-page :global(.library-section + .library-section) {
    margin-top: 0;
  }

  .playlists-page :global(.section-header) {
    align-items: center;
  }

  .home.playlists-view > .scan-error,
  .playlists-page > .scan-error {
    max-width: 720px;
  }

  .home.playlists-view {
    --content-bottom-padding: 28px;
  }

  .home.playlist-detail-view {
    --content-bottom-padding: 28px;
  }

  .playlist-card-grid {
    display: grid;
    width: 100%;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, 280px), 1fr));
    align-items: stretch;
    justify-items: stretch;
    gap: 12px;
  }

  .playlist-library-card {
    display: grid;
    min-width: 0;
    gap: 12px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--panel-soft);
    padding: 14px;
    transition: border-color 150ms ease, background 150ms ease;
  }

  .playlist-library-card:hover,
  .playlist-library-card:focus-within {
    border-color: var(--accent-strong);
    background: color-mix(in srgb, var(--panel-hover) 88%, transparent);
  }

  .playlist-library-card.smart {
    border-color: color-mix(in srgb, var(--accent) 28%, var(--border));
  }

  .playlist-card-main {
    display: grid;
    grid-template-columns: 68px minmax(0, 1fr);
    gap: 12px;
    align-items: center;
    min-width: 0;
    border: 0;
    background: transparent;
    color: inherit;
    cursor: pointer;
    font: inherit;
    padding: 0;
    text-align: left;
  }

  .playlist-card-main:focus-visible {
    border-radius: 6px;
    outline: 2px solid color-mix(in srgb, var(--accent) 58%, transparent);
    outline-offset: 4px;
  }

  .playlist-card-art,
  .playlist-hero-art {
    display: grid;
    overflow: hidden;
    place-items: center;
    border-radius: 8px;
    background:
      radial-gradient(circle at 30% 18%, rgba(255, 255, 255, 0.24), transparent 30%),
      linear-gradient(145deg, color-mix(in srgb, var(--accent) 82%, var(--panel)), var(--panel-strong));
    color: var(--accent-contrast);
    font-weight: 950;
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.12);
  }

  .playlist-card-art {
    width: 68px;
    height: 68px;
    font-size: 1.4rem;
  }

  .playlist-card-art.liked,
  .playlist-hero-art.liked {
    background:
      radial-gradient(circle at 34% 24%, color-mix(in srgb, var(--warning) 28%, transparent), transparent 38%),
      color-mix(in srgb, var(--warning) 16%, var(--panel));
    color: var(--warning);
  }

  .playlist-card-art.mosaic,
  .playlist-hero-art.mosaic {
    grid-template-columns: repeat(2, 1fr);
    grid-template-rows: repeat(2, 1fr);
    gap: 1px;
    background: var(--panel-strong);
    color: var(--accent-text);
  }

  .playlist-card-art.mosaic span,
  .playlist-hero-art.mosaic span {
    display: grid;
    width: 100%;
    height: 100%;
    place-items: center;
    background: color-mix(in srgb, var(--accent-soft) 72%, var(--panel));
    color: var(--accent-text);
    font-size: 0.88rem;
    font-weight: 950;
  }

  :is(.playlist-card-art, .playlist-hero-art).mosaic span:only-child {
    grid-area: 1 / 1 / -1 / -1;
  }

  :is(.playlist-card-art, .playlist-hero-art).mosaic span:first-child:nth-last-child(2),
  :is(.playlist-card-art, .playlist-hero-art).mosaic span:nth-child(2):last-child {
    grid-row: 1 / -1;
  }

  :is(.playlist-card-art, .playlist-hero-art).mosaic span:nth-child(3):last-child {
    grid-column: 1 / -1;
  }

  .playlist-card-art.mosaic img,
  .playlist-hero-art.mosaic img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .playlist-card-copy {
    display: grid;
    gap: 4px;
  }

  .playlist-card-copy h3 {
    margin: 0;
    font-size: 1.06rem;
    line-height: 1.2;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .playlist-card-copy p,
  .mix-tool-copy small {
    margin: 0;
    color: var(--text-soft);
    font-size: 0.86rem;
    font-weight: 650;
    line-height: 1.35;
  }

  .playlist-card-actions,
  .playlist-hero-actions,
  .playlist-create-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 7px;
  }

  .playlist-hero-action-row {
    display: grid;
    gap: 9px;
  }

  .playlist-management-actions {
    gap: 6px;
  }

  .playlist-card-actions button,
  .playlist-hero-actions button,
  .playlist-create-actions button {
    min-height: 32px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: color-mix(in srgb, var(--panel-strong) 82%, transparent);
    color: var(--text);
    cursor: pointer;
    font: inherit;
    font-size: 0.78rem;
    font-weight: 850;
    padding: 0 10px;
  }

  .playlist-card-actions button:first-child,
  .playlist-hero-actions button:first-child,
  .playlist-create-actions button:first-child {
    border-color: var(--accent-strong);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .playlist-card-actions button:hover:not(:disabled),
  .playlist-card-actions button:focus-visible:not(:disabled),
  .playlist-hero-actions button:hover:not(:disabled),
  .playlist-hero-actions button:focus-visible:not(:disabled),
  .playlist-create-actions button:hover:not(:disabled),
  .playlist-create-actions button:focus-visible:not(:disabled) {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent-soft) 84%, var(--panel-hover));
    color: var(--accent-text);
    outline: none;
  }

  .playlist-card-actions button.secondary,
  .playlist-hero-actions button.secondary,
  .playlist-create-actions button.secondary {
    border-color: var(--border-strong);
    background: color-mix(in srgb, var(--panel-strong) 72%, transparent);
    color: var(--text-muted);
  }

  .playlist-hero-actions button.secondary.active {
    border-color: var(--accent-strong);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .playlist-hero-actions button.more,
  .playlist-card-actions button.playlist-card-menu {
    min-width: 32px;
    font-size: 1.12rem;
    line-height: 1;
  }

  .playlist-card-actions button.playlist-card-menu {
    margin-inline-start: auto;
  }

  .playlist-card-actions button:disabled,
  .playlist-hero-actions button:disabled,
  .playlist-create-actions button:disabled {
    border-color: var(--border);
    background: var(--panel-soft);
    color: var(--text-dim);
  }

  .mix-tool-card {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 14px;
    width: 100%;
    min-width: 0;
    min-height: 112px;
    border: 1px solid color-mix(in srgb, var(--accent) 24%, var(--border));
    border-radius: 10px;
    background:
      linear-gradient(135deg, color-mix(in srgb, var(--accent-soft) 46%, transparent), transparent 58%),
      var(--panel-soft);
    color: inherit;
    cursor: pointer;
    font: inherit;
    padding: 16px;
    text-align: left;
  }

  .mix-tool-card:hover,
  .mix-tool-card:focus-visible {
    border-color: var(--accent-strong);
    background:
      linear-gradient(135deg, color-mix(in srgb, var(--accent-soft) 62%, transparent), transparent 58%),
      var(--panel-hover);
    outline: none;
  }

  .mix-tool-copy {
    display: grid;
    min-width: 0;
    gap: 5px;
  }

  .mix-tool-copy strong {
    font-size: 1.06rem;
  }

  .mix-tool-arrow {
    color: var(--text-soft);
    font-size: 1.5rem;
  }

  .playlist-create-panel {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: 10px;
    align-items: center;
    border: 1px solid color-mix(in srgb, var(--accent) 24%, var(--border));
    border-radius: 10px;
    background: color-mix(in srgb, var(--panel-soft) 84%, transparent);
    padding: 14px;
  }

  .playlist-create-panel label {
    font-weight: 700;
  }

  .playlist-create-panel input,
  .playlist-search input {
    width: 100%;
    min-width: 0;
    min-height: 38px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--bg-soft);
    color: var(--text);
    font: inherit;
    font-size: 0.9rem;
    font-weight: 700;
    outline: none;
    padding: 0 11px;
  }

  .playlist-create-panel input:focus,
  .playlist-search input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 22%, transparent);
  }

  .playlist-create-panel .form-message {
    grid-column: 1 / -1;
  }

  .playlist-search {
    width: min(100%, 480px);
  }

  .playlists-page .playlist-empty-state {
    gap: 8px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--panel-soft);
    padding: clamp(16px, 4cqw, 28px);
  }

  @container playlists (max-width: 560px) {
    .playlist-feature-grid,
    .playlist-create-panel {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  .playlist-status-pill {
    justify-self: start;
    max-width: min(100%, 520px);
    margin: 0;
    border: 1px solid var(--accent-strong);
    border-radius: 999px;
    background: color-mix(in srgb, var(--accent-soft) 72%, transparent);
    color: var(--accent-text);
    font-size: 0.82rem;
    font-weight: 800;
    line-height: 1.25;
    padding: 7px 11px;
  }

  .form-message {
    margin: 8px 0 0;
    color: var(--accent-text);
    font-size: 0.78rem;
    font-weight: 750;
  }

  .form-message.error {
    color: var(--danger);
  }

  .mix-mark,
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

  .mix-mark {
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .health-mark {
    background: var(--panel-strong);
    color: var(--accent-text);
  }

  .detail-view {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 22px;
  }

  .group-songs-intro {
    padding: 8px 0 0;
  }

  .group-songs-intro h3 {
    margin: 6px 0 8px;
    color: var(--text);
    font-size: clamp(1.7rem, 3vw, 2.5rem);
    line-height: 1.15;
  }

  .group-songs-intro > p:last-child {
    margin: 0;
    color: var(--text-soft);
    font-size: 0.9rem;
  }

  .lyrics-view {
    position: relative;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: 10px;
    min-height: calc(100vh - 178px);
  }

  .lyrics-view-top {
    display: grid;
    grid-template-columns: 180px minmax(0, 1fr) auto;
    align-items: start;
    gap: 18px;
  }

  .lyrics-title-block {
    min-width: 0;
    text-align: center;
  }

  .lyrics-title-block h2 {
    margin: 0;
    overflow-wrap: anywhere;
    color: var(--text);
    font-size: 1.05rem;
    font-weight: 850;
    line-height: 1.08;
  }

  .lyrics-sidecar-copy button {
    max-width: 100%;
    overflow: hidden;
    border: 0;
    background: transparent;
    color: inherit;
    cursor: default;
    font: inherit;
    font-weight: inherit;
    padding: 0;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .lyrics-sidecar-copy button:hover,
  .lyrics-sidecar-copy button:focus-visible {
    color: var(--text);
    outline: none;
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  .lyrics-options-button,
  .lyrics-options-header button,
  .lyrics-option-actions button {
    min-height: 34px;
    border: 1px solid rgba(66, 76, 91, 0.48);
    border-radius: 8px;
    background: color-mix(in srgb, var(--panel) 38%, transparent);
    color: var(--text);
    cursor: default;
    font: inherit;
    font-size: 0.84rem;
    font-weight: 780;
    padding: 0 12px;
  }

  .lyrics-options-button:hover,
  .lyrics-options-button:focus-visible,
  .lyrics-options-button.active,
  .lyrics-options-header button:hover,
  .lyrics-options-header button:focus-visible,
  .lyrics-option-actions button:hover,
  .lyrics-option-actions button:focus-visible {
    border-color: var(--accent-strong);
    background: var(--panel-hover);
    color: var(--text);
    outline: none;
  }

  .lyrics-options-button {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    white-space: nowrap;
  }

  .lyrics-options-button span {
    color: var(--accent-text);
  }

  .lyrics-experience {
    --lyrics-cover-size: clamp(320px, min(30vw, 48vh), 360px);

    display: grid;
    grid-template-columns: minmax(var(--lyrics-cover-size), 400px) minmax(0, 1fr) minmax(96px, 0.18fr);
    align-items: center;
    gap: clamp(22px, 2.4vw, 36px);
    width: min(100%, 1380px);
    min-height: min(720px, calc(100vh - 244px));
    margin-inline: auto;
  }

  .lyrics-sidecar {
    position: relative;
    align-self: center;
    justify-self: end;
    width: var(--lyrics-cover-size);
    min-width: 0;
    max-width: none;
    padding: 2px;
  }

  .lyrics-sidecar::before {
    position: absolute;
    inset: -34px -42px auto -42px;
    z-index: 0;
    height: calc(var(--lyrics-cover-size) + 72px);
    border-radius: 28px;
    background: radial-gradient(circle at 50% 44%, color-mix(in srgb, var(--accent) 16%, transparent), color-mix(in srgb, var(--accent) 5%, transparent) 42%, transparent 72%);
    content: "";
    filter: blur(24px);
    opacity: 0.72;
    pointer-events: none;
  }

  .lyrics-cover-ambient {
    position: absolute;
    inset: -44px -54px auto -54px;
    z-index: 0;
    width: calc(var(--lyrics-cover-size) + 108px);
    height: calc(var(--lyrics-cover-size) + 108px);
    border-radius: 30px;
    filter: blur(34px) saturate(0.72);
    opacity: 0.16;
    object-fit: cover;
    pointer-events: none;
    transform: scale(1.02);
  }

  .lyrics-cover {
    position: relative;
    z-index: 1;
    display: grid;
    aspect-ratio: 1;
    width: var(--lyrics-cover-size);
    height: var(--lyrics-cover-size);
    overflow: hidden;
    place-items: center;
    border-radius: 8px;
    background:
      linear-gradient(135deg, rgba(255, 255, 255, 0.18), transparent 46%),
      var(--accent);
    box-shadow:
      0 34px 96px rgba(0, 0, 0, 0.48),
      0 0 54px color-mix(in srgb, var(--accent) 12%, transparent),
      inset 0 0 0 1px rgba(255, 255, 255, 0.12);
  }

  .lyrics-cover::before {
    position: absolute;
    inset: -1px;
    z-index: 2;
    background:
      linear-gradient(135deg, rgba(255, 255, 255, 0.12), transparent 34%),
      linear-gradient(180deg, transparent 62%, rgba(0, 0, 0, 0.18));
    content: "";
    pointer-events: none;
  }

  .lyrics-cover img {
    position: absolute;
    inset: 0;
    z-index: 1;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .lyrics-cover span {
    display: block;
    width: 34%;
    aspect-ratio: 1;
    border: 10px solid color-mix(in srgb, var(--bg) 55%, transparent);
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.6);
  }

  .lyrics-sidecar-copy {
    position: relative;
    z-index: 1;
    margin-top: 14px;
  }

  .lyrics-sidecar-copy h3,
  .lyrics-sidecar-copy p {
    overflow: hidden;
    margin: 0;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .lyrics-sidecar-copy h3 {
    color: var(--text);
    font-size: 1rem;
    line-height: 1.25;
  }

  .lyrics-sidecar-copy p {
    color: var(--text-soft);
    font-size: 0.86rem;
    font-weight: 700;
  }

  .lyrics-sidecar-meta,
  .lyrics-genre-links {
    display: flex;
    flex-wrap: wrap;
    gap: 7px;
    margin-top: 8px;
  }

  .lyrics-sidecar-meta span,
  .lyrics-genre-links button {
    display: inline-flex;
    align-items: center;
    min-height: 26px;
    border: 1px solid rgba(48, 56, 68, 0.58);
    border-radius: 8px;
    background: color-mix(in srgb, var(--panel) 44%, transparent);
    color: var(--text-muted);
    font: inherit;
    font-size: 0.75rem;
    font-weight: 820;
    padding: 0 8px;
  }

  .lyrics-genre-links button {
    border-color: rgba(53, 84, 79, 0.74);
    background: rgba(23, 51, 47, 0.54);
    color: var(--accent-text);
    cursor: default;
  }

  .lyrics-genre-links button:hover,
  .lyrics-genre-links button:focus-visible {
    border-color: var(--accent-strong);
    background: var(--accent-soft);
    outline: none;
  }

  .lyrics-focus {
    display: grid;
    grid-column: 2;
    min-width: 0;
    min-height: min(640px, calc(100vh - 260px));
    align-content: center;
  }

  .lyrics-status-line {
    justify-self: center;
    max-width: min(100%, 720px);
    margin: 0 0 12px;
    color: var(--accent-text);
    font-size: 0.86rem;
    font-weight: 780;
    text-align: center;
  }

  .lyrics-status-line.error {
    color: var(--danger);
  }

  .synced-lyrics,
  .plain-lyrics {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    justify-items: center;
    max-height: min(66vh, 650px);
    overflow-x: hidden;
    overflow-y: auto;
    overscroll-behavior: contain;
    scrollbar-color: transparent transparent;
    scrollbar-gutter: stable both-edges;
    scrollbar-width: thin;
    transition: scrollbar-color 180ms ease;
  }

  .synced-lyrics {
    gap: 4px;
    padding: 104px 0;
  }

  .plain-lyrics {
    align-content: start;
    gap: clamp(1.4rem, 3.6vh, 2.5rem);
    padding-block: clamp(1.5rem, 5vh, 3.5rem);
  }

  .plain-lyrics:focus-visible {
    border-radius: 8px;
    outline: 2px solid var(--focus-ring);
    outline-offset: -2px;
  }

  .synced-lyrics:hover,
  .synced-lyrics:focus-within,
  .plain-lyrics:hover,
  .plain-lyrics:focus {
    scrollbar-color: rgba(124, 139, 156, 0.24) transparent;
  }

  .synced-lyrics::-webkit-scrollbar,
  .plain-lyrics::-webkit-scrollbar {
    width: 8px;
  }

  .synced-lyrics::-webkit-scrollbar-track,
  .plain-lyrics::-webkit-scrollbar-track {
    background: transparent;
  }

  .synced-lyrics::-webkit-scrollbar-thumb,
  .plain-lyrics::-webkit-scrollbar-thumb {
    border: 3px solid transparent;
    border-radius: 999px;
    background: rgba(124, 139, 156, 0);
    background-clip: content-box;
  }

  .synced-lyrics:hover::-webkit-scrollbar-thumb,
  .synced-lyrics:focus-within::-webkit-scrollbar-thumb,
  .plain-lyrics:hover::-webkit-scrollbar-thumb,
  .plain-lyrics:focus::-webkit-scrollbar-thumb {
    background-color: rgba(124, 139, 156, 0.28);
  }

  .synced-lyrics button,
  .plain-lyrics p {
    width: auto;
    min-width: 0;
    max-width: min(100%, 940px);
    margin: 0;
    font: inherit;
    font-size: clamp(1.22rem, 1.9vw, 1.92rem);
    line-height: 1.72;
    padding: 3px 14px;
    text-align: center;
    text-wrap: balance;
    overflow-wrap: anywhere;
  }

  .synced-lyrics button {
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--text-dim);
    cursor: default;
    font-weight: 680;
    transition:
      background 150ms ease,
      color 150ms ease,
      text-shadow 150ms ease;
  }

  .synced-lyrics button:hover,
  .synced-lyrics button:focus-visible {
    background: rgba(23, 29, 36, 0.18);
    color: var(--text);
    outline: none;
  }

  .synced-lyrics button.active {
    background: transparent;
    color: var(--text);
    text-shadow:
      0 0 14px rgba(236, 255, 251, 0.2),
      0 0 28px rgba(158, 227, 217, 0.18),
      0 0 52px color-mix(in srgb, var(--accent) 14%, transparent);
  }

  .synced-lyrics button.lyrics-break-cue {
    display: grid;
    place-items: center;
    min-width: 46px;
    height: 44px;
    min-height: 44px;
    padding: 0 15px;
    color: var(--text-soft);
    cursor: pointer;
    font-size: clamp(0.9rem, 1.15vw, 1.08rem);
    font-weight: 720;
    line-height: 1.4;
    opacity: 0.58;
  }

  .synced-lyrics button.lyrics-break-cue:hover {
    background: transparent;
    color: var(--text);
    opacity: 1;
  }

  .synced-lyrics button.lyrics-break-cue:focus-visible {
    background: transparent;
    color: var(--text);
    outline: none;
    opacity: 1;
  }

  .synced-lyrics button.lyrics-break-cue.active {
    color: var(--text);
    text-shadow: 0 0 18px color-mix(in srgb, var(--accent) 26%, transparent);
    opacity: 1;
  }

  .lyrics-break-cue span {
    font-size: clamp(1.3rem, 1.6vw, 1.55rem);
    line-height: 1;
  }

  .lyrics-break-cue:hover span,
  .lyrics-break-cue:focus-visible span,
  .lyrics-break-cue.active span {
    filter:
      drop-shadow(0 0 4px color-mix(in srgb, var(--text) 68%, transparent))
      drop-shadow(0 0 12px color-mix(in srgb, var(--accent) 42%, transparent));
  }

  .plain-lyrics p {
    color: var(--text);
    font-weight: 740;
    white-space: pre-wrap;
  }

  .lyrics-empty-state {
    display: grid;
    justify-items: center;
    gap: 8px;
    width: min(100%, 620px);
    margin: 0 auto;
    color: var(--text-soft);
    text-align: center;
  }

  .lyrics-empty-state h3 {
    color: var(--text);
    font-size: 1.35rem;
  }

  .lyrics-empty-state p {
    margin: 0;
    font-weight: 680;
  }

  .auto-lyrics-button {
    min-height: 38px;
    margin-top: 12px;
    border: 1px solid var(--accent-strong);
    border-radius: 8px;
    background: var(--accent-soft);
    color: var(--accent-text);
    cursor: default;
    font: inherit;
    font-size: 0.86rem;
    font-weight: 850;
    padding: 0 13px;
  }

  .auto-lyrics-button:hover,
  .auto-lyrics-button:focus-visible {
    border-color: var(--accent-strong);
    background: var(--accent-soft);
    outline: none;
  }

  .auto-lyrics-button:disabled,
  .lyrics-option-actions button:disabled {
    border-color: var(--border-strong);
    background: var(--panel-soft);
    color: var(--text-dim);
  }

  .lyrics-options-drawer {
    position: absolute;
    top: 46px;
    right: 0;
    z-index: 5;
    display: grid;
    gap: 12px;
    width: min(100%, 420px);
    max-height: min(70vh, 620px);
    overflow: auto;
    border: 1px solid rgba(80, 96, 112, 0.54);
    border-radius: 8px;
    background: color-mix(in srgb, var(--panel) 92%, transparent);
    box-shadow: 0 20px 58px rgba(0, 0, 0, 0.36);
    padding: 14px;
  }

  .lyrics-options-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 14px;
  }

  .lyrics-options-header h3 {
    margin: 0;
    font-size: 1rem;
  }

  .lyrics-option-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .lyrics-source-summary {
    display: grid;
    gap: 8px;
    color: var(--text-soft);
    font-size: 0.86rem;
    font-weight: 700;
  }

  .lyrics-source-summary > div {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .lyrics-source-summary span {
    color: var(--text-soft);
  }

  .lyrics-source-summary strong {
    color: var(--text);
  }

  .lyrics-source-summary p {
    margin: 0;
  }

  .lyrics-source-summary .error {
    color: var(--danger);
  }

  .lyrics-result-picker {
    display: grid;
    gap: 10px;
  }

  .lyrics-result-picker-heading {
    display: grid;
    gap: 4px;
  }

  .lyrics-result-picker-heading h4 {
    margin: 0;
    color: var(--text);
    font-size: 0.98rem;
  }

  .lyrics-result-picker-heading p {
    margin: 0;
    color: var(--text-soft);
    font-size: 0.86rem;
    line-height: 1.45;
  }

  .lyrics-result-card {
    display: grid;
    gap: 5px;
    width: 100%;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-soft);
    color: var(--text);
    cursor: default;
    font: inherit;
    padding: 12px;
    text-align: left;
  }

  .lyrics-result-card:hover,
  .lyrics-result-card:focus-visible {
    border-color: var(--accent-strong);
    background: var(--panel-strong);
    outline: none;
  }

  .lyrics-result-card:disabled {
    border-color: var(--border-strong);
    background: var(--panel-soft);
    color: var(--text-dim);
  }

  .lyrics-result-title {
    color: var(--text);
    font-size: 0.96rem;
    font-weight: 850;
  }

  .lyrics-result-artist,
  .lyrics-result-album {
    color: var(--text-muted);
    font-size: 0.84rem;
    font-weight: 700;
  }

  .lyrics-result-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 4px;
  }

  .lyrics-result-tags span {
    display: inline-flex;
    align-items: center;
    min-height: 24px;
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    background: var(--bg-soft);
    color: var(--text-muted);
    font-size: 0.73rem;
    font-weight: 800;
    padding: 0 8px;
  }

  .lyrics-offset-controls {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: rgba(13, 17, 23, 0.68);
    padding: 10px;
  }

  .lyrics-offset-controls > span {
    color: var(--text);
    font-size: 0.82rem;
    font-weight: 850;
    margin-right: 4px;
  }

  .lyrics-offset-controls button {
    min-height: 30px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel-strong);
    color: var(--text);
    cursor: default;
    font: inherit;
    font-size: 0.78rem;
    font-weight: 850;
    padding: 0 10px;
  }

  .lyrics-offset-controls input {
    width: 82px;
    min-height: 30px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 0.82rem;
    font-weight: 750;
    padding: 0 8px;
  }

  .lyrics-offset-controls button:hover,
  .lyrics-offset-controls button:focus-visible,
  .lyrics-offset-controls input:focus-visible {
    border-color: var(--accent-strong);
    outline: none;
  }

  .lyrics-offset-controls button:disabled,
  .lyrics-offset-controls input:disabled {
    border-color: var(--border-strong);
    background: var(--panel-soft);
    color: var(--text-dim);
  }

  .lyrics-empty-shell {
    display: flex;
    align-items: center;
    gap: 18px;
    min-height: 260px;
    border: 1px dashed var(--border-strong);
    border-radius: 8px;
    background: color-mix(in srgb, var(--panel) 54%, transparent);
    padding: 24px;
  }

  .lyrics-empty-art {
    display: grid;
    width: 76px;
    height: 76px;
    flex: 0 0 auto;
    place-items: center;
    border-radius: 8px;
    background: var(--panel-strong);
    color: var(--accent-text);
    font-size: 1.6rem;
    font-weight: 900;
  }

  .lyrics-empty-shell h3 {
    margin: 0 0 6px;
    color: var(--text);
    font-size: 1.35rem;
  }

  .lyrics-empty-shell p:not(.eyebrow) {
    margin: 0;
    color: var(--text-soft);
    font-weight: 650;
  }

  .back-button {
    justify-self: start;
    min-height: 36px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel-strong);
    color: var(--text);
    cursor: default;
    font: inherit;
    font-size: 0.88rem;
    font-weight: 800;
    padding: 0 13px;
  }

  .back-button:hover,
  .back-button:focus-visible {
    border-color: var(--accent-strong);
    background: var(--panel-hover);
    outline: none;
  }

  .back-button:disabled {
    border-color: var(--border-strong);
    background: var(--panel-soft);
    color: var(--text-dim);
  }

  .view-history-controls {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .view-history-controls button {
    width: 36px;
    height: 36px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel-strong);
    color: var(--text);
    font: inherit;
    font-size: 1.5rem;
    line-height: 1;
    cursor: pointer;
  }

  .view-history-controls button:hover:not(:disabled),
  .view-history-controls button:focus-visible {
    border-color: var(--accent-strong);
    background: var(--panel-hover);
    outline: none;
  }

  .view-history-controls button:disabled {
    color: var(--text-dim);
    cursor: default;
    opacity: 0.5;
  }

  .detail-songs-load-more {
    min-height: 38px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel-strong);
    color: var(--text);
    cursor: pointer;
    font: inherit;
    font-size: 0.84rem;
    font-weight: 750;
  }

  .detail-songs-load-more:hover,
  .detail-songs-load-more:focus-visible {
    border-color: var(--accent-strong);
    background: var(--panel-hover);
    outline: none;
  }

  .playlist-detail-back {
    justify-self: start;
  }

  .playlist-hero {
    position: relative;
    isolation: isolate;
    display: grid;
    grid-template-columns: clamp(90px, 16cqw, 156px) minmax(0, 1fr);
    gap: clamp(18px, 2.4vw, 28px);
    align-items: center;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--panel-soft);
    padding: clamp(18px, 2.6vw, 28px);
  }

  .playlist-hero::before {
    position: absolute;
    inset: 0;
    z-index: -1;
    background:
      radial-gradient(circle at 14% 42%, color-mix(in srgb, var(--accent) 28%, transparent), transparent 36%),
      linear-gradient(90deg, rgba(255, 255, 255, 0.04), transparent 44%);
    opacity: 0.82;
    pointer-events: none;
    content: "";
  }

  .playlist-hero.smart::before {
    background:
      radial-gradient(circle at 14% 42%, color-mix(in srgb, var(--warning) 22%, transparent), transparent 36%),
      linear-gradient(90deg, rgba(255, 255, 255, 0.04), transparent 44%);
  }

  .playlist-hero-art {
    width: 100%;
    aspect-ratio: 1;
    min-width: 0;
    font-size: clamp(2.2rem, 6vw, 4.6rem);
    box-shadow:
      0 22px 54px rgba(0, 0, 0, 0.28),
      inset 0 0 0 1px rgba(255, 255, 255, 0.14);
  }

  .playlist-hero-copy {
    display: grid;
    gap: 12px;
    min-width: 0;
  }

  .playlist-hero-copy h2 {
    margin: 0;
    overflow-wrap: anywhere;
    color: var(--text);
    font-size: clamp(1.55rem, 4.4cqw, 3.25rem);
    line-height: 1.05;
    letter-spacing: -0.03em;
  }

  .playlist-hero-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .playlist-hero-meta span {
    min-height: 28px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--panel);
    color: var(--text-muted);
    font-size: 0.78rem;
    font-weight: 800;
    line-height: 1;
    padding: 7px 10px;
  }

  @container playlists (max-width: 425px) {
    .playlist-hero {
      grid-template-columns: minmax(0, 1fr);
      padding: 16px;
    }

    .playlist-hero-art {
      width: min(38%, 120px);
    }
  }

  .artist-detail-header,
  .genre-detail-header,
  .playlist-detail-header {
    display: flex;
    align-items: center;
    gap: 22px;
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-soft);
    padding: 18px;
  }

  .artist-detail-header,
  .genre-detail-header {
    position: relative;
    isolation: isolate;
    overflow: hidden;
    align-items: center;
    border-color: rgba(50, 61, 75, 0.72);
    background:
      linear-gradient(115deg, rgba(22, 27, 35, 0.96), rgba(12, 15, 20, 0.96) 62%, rgba(16, 22, 28, 0.98)),
      var(--panel);
    box-shadow:
      0 22px 58px rgba(0, 0, 0, 0.18),
      inset 0 1px 0 rgba(255, 255, 255, 0.04);
    padding: clamp(20px, 2.6vw, 28px);
  }

  .artist-detail-header::before,
  .genre-detail-header::before {
    position: absolute;
    inset: 0;
    z-index: -1;
    background:
      radial-gradient(circle at 12% 48%, color-mix(in srgb, var(--item-color) 46%, transparent), transparent 30%),
      linear-gradient(90deg, rgba(255, 255, 255, 0.04), transparent 44%);
    opacity: 0.82;
    pointer-events: none;
    content: "";
  }

  .artist-detail-header .detail-copy,
  .genre-detail-header .detail-copy {
    display: grid;
    gap: 12px;
  }

  .artist-detail-header .detail-copy h3,
  .genre-detail-header .detail-copy h3 {
    max-width: 980px;
    margin-bottom: 0;
    font-size: clamp(2rem, 4.6vw, 3.8rem);
  }

  .artist-detail-actions,
  .genre-detail-actions {
    margin-top: 2px;
  }

  .album-detail-header {
    position: relative;
    isolation: isolate;
    display: grid;
    grid-template-columns: minmax(210px, 300px) minmax(0, 1fr);
    gap: clamp(22px, 3vw, 34px);
    align-items: center;
    min-width: 0;
    overflow: hidden;
    border: 1px solid rgba(50, 61, 75, 0.72);
    border-radius: 8px;
    background:
      linear-gradient(115deg, rgba(22, 27, 35, 0.96), rgba(12, 15, 20, 0.96) 58%, rgba(16, 22, 28, 0.98)),
      var(--panel);
    box-shadow:
      0 24px 70px rgba(0, 0, 0, 0.22),
      inset 0 1px 0 rgba(255, 255, 255, 0.04);
    padding: clamp(20px, 2.6vw, 30px);
  }

  .album-detail-header::before,
  .album-detail-header::after {
    position: absolute;
    pointer-events: none;
    content: "";
  }

  .album-detail-header::before {
    inset: 0;
    z-index: -1;
    background:
      radial-gradient(circle at 24% 32%, color-mix(in srgb, var(--item-color) 42%, transparent), transparent 34%),
      linear-gradient(90deg, rgba(255, 255, 255, 0.04), transparent 42%);
    opacity: 0.85;
  }

  .album-detail-header::after {
    inset: auto 0 0;
    z-index: -1;
    height: 42%;
    background: linear-gradient(0deg, rgba(6, 8, 11, 0.36), transparent);
  }

  .album-detail-overflow {
    position: absolute;
    top: 14px;
    right: 14px;
    z-index: 2;
    display: grid;
    width: 40px;
    height: 40px;
    place-items: center;
    border: 1px solid rgba(64, 77, 93, 0.78);
    border-radius: 999px;
    background: rgba(12, 15, 20, 0.66);
    color: var(--text);
    cursor: default;
    font: inherit;
    font-size: 1.55rem;
    font-weight: 850;
    line-height: 1;
    padding: 0 0 8px;
    backdrop-filter: blur(10px);
  }

  .album-detail-overflow:hover,
  .album-detail-overflow:focus-visible {
    border-color: var(--accent-strong);
    background: rgba(27, 32, 39, 0.9);
    color: var(--accent-text);
    outline: none;
  }

  .album-detail-ambient {
    position: absolute;
    inset: -24%;
    z-index: -2;
    width: 72%;
    height: 148%;
    filter: blur(34px) saturate(1.18);
    object-fit: cover;
    opacity: 0.18;
    pointer-events: none;
    transform: translateX(-8%) scale(1.08);
  }

  .album-detail-cover-shell {
    position: relative;
    min-width: 0;
  }

  .detail-cover {
    width: min(28vw, 190px);
    min-width: 136px;
    margin: 0;
  }

  .album-detail-header .detail-cover {
    width: 100%;
    max-width: 300px;
    min-width: 0;
    box-shadow:
      0 28px 70px rgba(0, 0, 0, 0.34),
      inset 0 0 0 1px rgba(255, 255, 255, 0.16);
  }

  .detail-avatar {
    width: 104px;
    height: 104px;
    font-size: 2.1rem;
  }

  .detail-copy {
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .album-detail-copy {
    position: relative;
    display: grid;
    align-content: center;
    gap: 12px;
  }

  .detail-copy h3 {
    margin: 0 0 8px;
    overflow: hidden;
    color: var(--text);
    font-size: clamp(1.7rem, 4vw, 3.2rem);
    line-height: 1.03;
    text-overflow: ellipsis;
  }

  .detail-copy p:not(.eyebrow) {
    margin: 0;
    color: var(--text-soft);
    font-weight: 700;
  }

  .album-detail-copy h3 {
    max-width: 980px;
    margin-bottom: 0;
    font-size: clamp(2rem, 4.8vw, 4rem);
    letter-spacing: 0;
  }

  .album-detail-artist {
    color: var(--text) !important;
    font-size: 1rem;
  }

  .album-detail-artist button {
    border: 0;
    background: transparent;
    color: inherit;
    cursor: pointer;
    font: inherit;
    padding: 0;
  }

  .album-detail-artist button:hover,
  .album-detail-artist button:focus-visible {
    color: var(--accent-text);
    outline: none;
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  .album-detail-meta,
  .album-genre-chip-list {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .album-detail-meta span,
  .album-genre-chip-list button {
    min-height: 28px;
    border: 1px solid rgba(61, 74, 91, 0.72);
    border-radius: 999px;
    background: rgba(12, 15, 20, 0.44);
    color: var(--text-muted);
    font-size: 0.78rem;
    font-weight: 800;
    line-height: 1;
    padding: 7px 10px;
  }

  .album-genre-chip-list button {
    border-color: color-mix(in srgb, var(--accent) 34%, transparent);
    background: rgba(23, 51, 47, 0.34);
    color: var(--accent-text);
    cursor: pointer;
    font-family: inherit;
  }

  .album-genre-chip-list button:hover,
  .album-genre-chip-list button:focus-visible {
    border-color: var(--accent-strong);
    outline: none;
  }

  .album-genre-chip-list button.muted {
    border-color: rgba(61, 74, 91, 0.56);
    background: rgba(12, 15, 20, 0.36);
    color: var(--text-soft);
  }

  .album-detail-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin-top: 4px;
  }

  .album-detail-actions button {
    min-height: 40px;
    border: 1px solid rgba(64, 77, 93, 0.78);
    border-radius: 999px;
    background: rgba(15, 19, 25, 0.72);
    color: var(--text);
    cursor: default;
    font: inherit;
    font-size: 0.86rem;
    font-weight: 850;
    padding: 0 15px;
    backdrop-filter: blur(10px);
  }

  .album-detail-actions button:first-child {
    border-color: var(--accent-strong);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .album-detail-actions button:hover,
  .album-detail-actions button:focus-visible {
    border-color: var(--accent-strong);
    background: rgba(27, 32, 39, 0.9);
    outline: none;
  }

  .album-detail-actions button:first-child:hover,
  .album-detail-actions button:first-child:focus-visible {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .album-detail-actions button:disabled {
    border-color: rgba(48, 56, 68, 0.72);
    background: rgba(21, 26, 33, 0.7);
    color: var(--text-dim);
  }

  .album-tag-move-notice {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    border: 1px solid color-mix(in srgb, var(--warning) 42%, var(--border));
    border-radius: 8px;
    background: color-mix(in srgb, var(--warning) 9%, var(--panel));
    color: var(--text);
    padding: 13px 14px;
  }

  .album-tag-move-notice p {
    margin: 4px 0 0;
    color: var(--text-muted);
    font-size: 0.86rem;
    line-height: 1.45;
  }

  .album-tag-move-actions {
    display: flex;
    flex: 0 0 auto;
    flex-wrap: wrap;
    gap: 8px;
  }

  .album-tag-move-actions button {
    min-height: 34px;
    border: 1px solid var(--border-strong);
    border-radius: 7px;
    background: var(--bg-soft);
    color: var(--text);
    cursor: default;
    font: inherit;
    font-size: 0.8rem;
    font-weight: 800;
    padding: 0 11px;
  }

  .album-tag-move-actions button:hover,
  .album-tag-move-actions button:focus-visible {
    border-color: var(--accent-strong);
    color: var(--accent-text);
    outline: none;
  }

  .album-track-list {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 4px;
  }

  .album-track-list h4 {
    margin: 16px 0 4px;
    color: var(--text-muted);
    font-size: 0.82rem;
    font-weight: 900;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .album-track-row {
    display: grid;
    grid-template-columns: 42px minmax(0, 1fr) minmax(56px, auto) auto auto;
    align-items: center;
    gap: 12px;
    min-height: 54px;
    border: 1px solid rgba(36, 43, 53, 0.72);
    border-radius: 8px;
    background: rgba(17, 21, 27, 0.58);
    color: inherit;
    cursor: default;
    font: inherit;
    padding: 8px 12px;
    outline: none;
    transition:
      border-color 140ms ease,
      background 140ms ease,
      box-shadow 140ms ease;
  }

  .album-track-row.with-play-counts {
    grid-template-columns: 42px minmax(0, 1fr) minmax(74px, auto) minmax(56px, auto) auto auto;
  }

  .album-track-play-count {
    justify-self: end;
    color: var(--text-soft);
    font-size: 0.8rem;
    font-variant-numeric: tabular-nums;
    font-weight: 750;
    white-space: nowrap;
  }

  .album-track-row:hover,
  .album-track-row:focus-visible {
    border-color: var(--accent-strong);
    background: color-mix(in srgb, var(--panel-hover) 86%, transparent);
  }

  .album-track-row.active {
    border-color: color-mix(in srgb, var(--accent) 56%, transparent);
    background: linear-gradient(
      90deg,
      color-mix(in srgb, var(--accent-soft) 58%, transparent),
      color-mix(in srgb, var(--panel-hover) 70%, transparent)
    );
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .album-track-number {
    color: var(--text-soft);
    font-size: 0.86rem;
    font-variant-numeric: tabular-nums;
    font-weight: 850;
    text-align: right;
  }

  .album-track-number.missing {
    color: var(--text-dim);
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
    color: var(--text);
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
    color: var(--text-soft);
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
    color: var(--text);
    outline: none;
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  .album-track-duration,
  .album-track-format {
    color: var(--text-soft);
    font-size: 0.82rem;
    font-variant-numeric: tabular-nums;
    font-weight: 700;
  }

  .album-track-format {
    min-width: 42px;
    border: 1px solid rgba(48, 56, 68, 0.72);
    border-radius: 999px;
    background: rgba(12, 15, 20, 0.36);
    padding: 4px 7px;
    text-align: right;
  }

  .favorite-button {
    display: grid;
    width: 32px;
    height: 32px;
    place-items: center;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel-strong);
    color: var(--text-soft);
    cursor: default;
    font: inherit;
    font-size: 0.95rem;
    font-weight: 900;
    line-height: 1;
  }

  .favorite-button:hover,
  .favorite-button:focus-visible,
  .favorite-button.active {
    border-color: color-mix(in srgb, var(--warning) 48%, var(--border));
    background: color-mix(in srgb, var(--warning) 16%, var(--panel));
    color: var(--warning);
    outline: none;
  }

  .mix-builder-panel {
    display: grid;
    grid-template-columns: minmax(240px, 1fr) auto;
    gap: 14px;
    align-items: center;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel);
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
    color: var(--text-soft);
    font-size: 0.82rem;
    font-weight: 800;
  }

  .mix-toggle input,
  .mix-option-card input {
    accent-color: var(--accent);
  }

  .mix-builder-controls select {
    min-height: 36px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background-color: var(--bg-soft);
    color: var(--text);
    font: inherit;
    font-size: 0.86rem;
    font-weight: 750;
    outline: none;
    padding: 0 32px 0 10px;
  }

  .mix-builder-controls select:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 22%, transparent);
  }

  .mix-builder-actions {
    justify-content: flex-end;
  }

  .mix-builder-actions p {
    margin: 0;
    color: var(--text);
    font-size: 0.9rem;
    font-weight: 800;
  }

  .mix-builder-actions button {
    min-height: 36px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel-strong);
    color: var(--text);
    cursor: default;
    font: inherit;
    font-size: 0.86rem;
    font-weight: 850;
    padding: 0 12px;
  }

  .mix-builder-actions button.accent {
    border-color: var(--accent-strong);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .mix-builder-actions button:disabled {
    border-color: var(--border-strong);
    background: var(--panel-soft);
    color: var(--text-dim);
  }

  .mix-message {
    grid-column: 1 / -1;
    margin: 0;
    color: var(--text-soft);
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
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-soft);
    cursor: default;
    padding: 10px;
  }

  .mix-option-card:hover,
  .mix-option-card:focus-within {
    border-color: var(--accent-strong);
    background: var(--panel-hover);
  }

  .mix-option-card.selected {
    border-color: var(--accent);
    background: var(--accent-soft);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 32%, transparent);
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
    color: var(--bg);
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
    color: var(--text);
    font-size: 0.9rem;
    line-height: 1.25;
  }

  .mix-option-card small {
    margin-top: 3px;
    color: var(--text-soft);
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
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--bg-soft);
  }

  .video-search input {
    width: 100%;
    min-width: 0;
    border: 0;
    background: transparent;
    color: var(--text);
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
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel-strong);
    color: var(--text);
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
    border-color: var(--accent-strong);
    background: var(--panel-hover);
    outline: none;
  }

  .video-edit-actions button.primary {
    border-color: var(--accent-strong);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .videos-toolbar button:disabled,
  .video-actions button:disabled,
  .video-edit-actions button:disabled {
    border-color: var(--border-strong);
    background: var(--panel-soft);
    color: var(--text-dim);
  }

  .videos-toolbar label {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 40px;
    color: var(--text-soft);
    font-size: 0.82rem;
    font-weight: 800;
  }

  .videos-toolbar select {
    min-height: 40px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background-color: var(--bg-soft);
    color: var(--text);
    font: inherit;
    font-size: 0.86rem;
    font-weight: 750;
    outline: none;
    padding: 0 32px 0 10px;
  }

  .video-filter-tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .video-filter-tabs button {
    min-height: 36px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel-strong);
    color: var(--text);
    cursor: default;
    font: inherit;
    font-size: 0.82rem;
    font-weight: 850;
    padding: 0 11px;
  }

  .video-filter-tabs button:hover,
  .video-filter-tabs button:focus-visible,
  .video-filter-tabs button.active {
    border-color: var(--accent-strong);
    background: var(--accent-soft);
    color: var(--accent-text);
    outline: none;
  }

  .video-stat-grid,
  .video-meta-grid,
  .video-codec-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 10px;
  }

  .video-stat-grid > div,
  .video-meta-grid > div,
  .video-codec-grid > div {
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-soft);
    padding: 12px;
  }

  .video-stat-grid span,
  .video-meta-grid span,
  .video-codec-grid span {
    display: block;
    margin-bottom: 6px;
    color: var(--text-soft);
    font-size: 0.74rem;
    font-weight: 850;
    text-transform: uppercase;
  }

  .video-stat-grid strong,
  .video-meta-grid strong,
  .video-codec-grid strong {
    display: block;
    overflow: hidden;
    color: var(--text);
    font-size: 0.98rem;
    font-weight: 900;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dvd-import-panel {
    display: grid;
    gap: 16px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel);
    padding: 16px;
  }

  .dvd-import-panel h3,
  .dvd-import-panel p {
    margin: 0;
  }

  .dvd-import-panel h3 {
    color: var(--text);
    font-size: 1rem;
  }

  .dvd-import-panel p:not(.eyebrow):not(.form-message) {
    margin-top: 4px;
    color: var(--text-soft);
    font-size: 0.9rem;
    font-weight: 700;
  }

  .dvd-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }

  .dvd-actions button {
    min-height: 38px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel-strong);
    color: var(--text);
    cursor: default;
    font: inherit;
    font-size: 0.84rem;
    font-weight: 850;
    padding: 0 12px;
  }

  .dvd-actions button.primary {
    border-color: var(--accent-strong);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .dvd-actions button:hover:not(:disabled),
  .dvd-actions button:focus-visible:not(:disabled) {
    border-color: var(--accent-strong);
    background: var(--panel-hover);
    outline: none;
  }

  .dvd-actions button.primary:hover:not(:disabled),
  .dvd-actions button.primary:focus-visible:not(:disabled) {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .dvd-actions button:disabled {
    border-color: var(--border-strong);
    background: var(--panel-soft);
    color: var(--text-dim);
  }

  .dvd-status-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 10px;
  }

  .dvd-status-grid > div {
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-soft);
    padding: 12px;
  }

  .dvd-status-grid span,
  .dvd-import-form span {
    display: block;
    margin-bottom: 6px;
    color: var(--text-soft);
    font-size: 0.74rem;
    font-weight: 850;
    text-transform: uppercase;
  }

  .dvd-status-grid strong {
    display: block;
    overflow: hidden;
    color: var(--text);
    font-size: 0.9rem;
    font-weight: 850;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dvd-title-table {
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-soft);
  }

  .dvd-title-row {
    display: grid;
    grid-template-columns: 76px 1fr 92px 110px 76px;
    min-height: 42px;
    align-items: center;
    gap: 10px;
    border-top: 1px solid var(--border);
    color: var(--text);
    font-size: 0.86rem;
    font-weight: 750;
    padding: 8px 12px;
  }

  .dvd-title-row:first-child {
    border-top: 0;
  }

  .dvd-title-row.head {
    background: var(--bg-soft);
    color: var(--text-soft);
    font-size: 0.74rem;
    font-weight: 850;
    text-transform: uppercase;
  }

  .dvd-title-row.active {
    background: var(--accent-soft);
  }

  .dvd-title-row input {
    accent-color: var(--accent);
  }

  .dvd-import-form {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 10px;
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
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-soft);
    color: inherit;
    cursor: default;
    font: inherit;
    padding: 10px;
    text-align: left;
  }

  .video-card:hover,
  .video-card:focus-visible {
    border-color: var(--accent-strong);
    background: var(--panel-hover);
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
    background: var(--bg-soft);
    color: var(--accent-text);
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
    background: linear-gradient(90deg, var(--accent) var(--progress), rgba(244, 247, 251, 0.18) var(--progress));
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
    color: var(--text);
    font-size: 0.98rem;
    line-height: 1.25;
  }

  .video-card-copy small {
    color: var(--text-soft);
    font-size: 0.82rem;
    font-weight: 700;
  }

  .video-detail-hero {
    display: grid;
    gap: 14px;
  }

  .video-session-card {
    display: grid;
    grid-template-columns: 180px minmax(0, 1fr) minmax(220px, 0.45fr);
    gap: 16px;
    align-items: center;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-soft);
    padding: 16px;
  }

  .video-session-thumb {
    display: grid;
    overflow: hidden;
    aspect-ratio: 16 / 9;
    min-width: 0;
    place-items: center;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--bg);
    color: var(--accent-text);
    font-size: 2rem;
    font-weight: 900;
  }

  .video-session-thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .video-session-copy {
    display: grid;
    min-width: 0;
    gap: 6px;
  }

  .video-session-copy h3 {
    overflow: hidden;
    margin: 0;
    color: var(--text);
    font-size: clamp(1.35rem, 3vw, 2.25rem);
    line-height: 1.08;
    text-overflow: ellipsis;
  }

  .video-session-copy > p:not(.eyebrow),
  .video-window-note {
    margin: 0;
    color: var(--text-soft);
    font-weight: 750;
  }

  .video-session-progress {
    display: grid;
    gap: 6px;
    margin-top: 6px;
  }

  .video-session-progress > div:first-child {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    color: var(--text);
    font-size: 0.82rem;
    font-variant-numeric: tabular-nums;
    font-weight: 850;
  }

  .video-panel-progress {
    overflow: hidden;
    height: 6px;
    border-radius: 999px;
    background: rgba(244, 247, 251, 0.12);
  }

  .video-panel-progress::before {
    display: block;
    width: var(--progress);
    height: 100%;
    background: var(--accent);
    content: "";
  }

  .video-session-status {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }

  .video-session-status > div {
    display: grid;
    gap: 4px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-soft);
    padding: 10px;
  }

  .video-session-status span {
    color: var(--text-soft);
    font-size: 0.72rem;
    font-weight: 850;
    text-transform: uppercase;
  }

  .video-session-status strong {
    color: var(--text);
    font-size: 0.9rem;
  }

  .video-window-note,
  .video-window-actions {
    grid-column: 2 / -1;
  }

  .video-window-note {
    font-size: 0.88rem;
  }

  .video-player-controls {
    display: grid;
    gap: 12px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-soft);
    padding: 14px;
  }

  .video-detail-copy {
    display: grid;
    min-width: 0;
    gap: 14px;
    border-top: 1px solid var(--border);
    padding-top: 18px;
  }

  .video-detail-copy h3 {
    overflow: hidden;
    margin: 0;
    color: var(--text);
    font-size: 1.35rem;
    line-height: 1.15;
    text-overflow: ellipsis;
  }

  .video-detail-copy > p:not(.eyebrow) {
    margin: 0;
    color: var(--text-soft);
    font-weight: 750;
  }

  .video-meta-grid,
  .video-codec-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .video-codec-grid {
    display: grid;
    gap: 10px;
  }

  .video-location {
    color: var(--text) !important;
  }

  .video-actions,
  .video-edit-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }

  .primary-video-actions button:first-child {
    border-color: var(--accent-strong);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .secondary-video-actions {
    padding-top: 4px;
  }

  .video-native-controls {
    display: grid;
    gap: 10px;
  }

  .video-native-controls label {
    display: grid;
    grid-template-columns: 64px minmax(0, 1fr);
    align-items: center;
    gap: 10px;
    color: var(--text-soft);
    font-size: 0.78rem;
    font-weight: 850;
    text-transform: uppercase;
  }

  .video-native-controls input[type="range"] {
    width: 100%;
    accent-color: var(--accent);
  }

  .video-edit-panel {
    display: grid;
    gap: 14px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel);
    padding: 16px;
  }

  .video-edit-form {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .video-edit-form label,
  .dvd-import-form label {
    display: grid;
    gap: 7px;
    color: var(--text-soft);
    font-size: 0.78rem;
    font-weight: 850;
    text-transform: uppercase;
  }

  .video-edit-form label.wide,
  .dvd-import-form label.wide {
    grid-column: 1 / -1;
  }

  .video-edit-form input,
  .video-edit-form select,
  .video-edit-form textarea,
  .dvd-import-form input,
  .dvd-import-form select,
  .dvd-import-form textarea {
    min-width: 0;
    min-height: 42px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--bg-soft);
    color: var(--text);
    font: inherit;
    font-size: 0.92rem;
    font-weight: 650;
    outline: none;
    padding: 0 12px;
  }

  .video-edit-form textarea,
  .dvd-import-form textarea {
    min-height: 78px;
    padding: 10px 12px;
    resize: vertical;
  }

  .video-edit-form input:focus,
  .video-edit-form select:focus,
  .video-edit-form textarea:focus,
  .dvd-import-form input:focus,
  .dvd-import-form select:focus,
  .dvd-import-form textarea:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 22%, transparent);
  }

  .video-edit-actions {
    grid-column: 1 / -1;
  }

  .detail-view :global(.library-section + .library-section) {
    margin-top: 8px;
  }

  .stats-page {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 24px;
    width: min(100%, 1320px);
    margin-inline: auto;
    container: stats / inline-size;
  }

  .stats-header {
    display: flex;
    flex-wrap: wrap;
    align-items: end;
    justify-content: space-between;
    gap: 14px 24px;
  }

  .stats-header > h2 {
    margin: 0;
    font-size: clamp(2.5rem, 4.8vw, 3.65rem);
    line-height: 1.05;
    letter-spacing: -0.04em;
  }

  .stats-period-toolbar,
  .stats-transfer-actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }

  .stats-period-field {
    display: grid;
    gap: 5px;
    min-width: 0;
  }

  .stats-period-field label {
    color: var(--text-soft);
    font-size: 0.78rem;
    font-weight: 800;
  }

  .stats-period-toolbar select,
  .stats-period-toolbar input,
  .stats-transfer-panel select {
    min-height: 38px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel-soft);
    color: var(--text);
    font: inherit;
    padding: 6px 10px;
    max-width: 100%;
  }

  .stats-period-toolbar select {
    min-width: min(168px, 100%);
  }

  .stats-period-toolbar input[inputmode="numeric"] {
    width: 90px;
  }

  .stats-period-toolbar :is(select, input):focus-visible,
  .stats-transfer-panel :is(select, button):focus-visible,
  .stats-chart-day:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .stats-coverage-note,
  .stats-transfer-panel p,
  .stats-status {
    margin: 0;
    color: var(--text-soft);
    line-height: 1.5;
  }

  .stats-transfer-panel {
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-soft);
    padding: 16px;
  }

  .stats-chart-panel,
  .stats-library-summary,
  .stats-section-grid > :global(.library-section) {
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--panel-soft);
    padding: clamp(16px, 2.3vw, 28px);
  }

  .stats-chart-panel {
    background: linear-gradient(160deg, color-mix(in srgb, var(--accent) 6%, var(--panel-soft)), var(--panel-soft) 58%);
  }

  .stats-chart-heading {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    justify-content: space-between;
    gap: 6px 16px;
    margin: 22px 0 14px;
  }

  .stats-chart-heading h2 {
    margin: 0;
    font-size: 1.12rem;
  }

  .stats-chart-heading span,
  .stats-coverage {
    color: var(--text-soft);
    font-size: 0.86rem;
  }

  .stats-coverage summary {
    width: fit-content;
    cursor: pointer;
    font-weight: 750;
  }

  .stats-coverage {
    margin-top: 18px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }

  .stats-coverage[open] > :not(summary) {
    margin-top: 8px;
  }

  .stats-library-summary h2 {
    margin: 0;
    font-size: 1.08rem;
  }

  .stats-library-summary dl {
    margin: 12px 0 0;
  }

  .stats-library-summary dl > div {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 0;
    border-bottom: 1px solid var(--border);
    color: var(--text-soft);
  }

  .stats-library-summary dl > div:last-child {
    border-bottom: 0;
  }

  .stats-library-summary dd {
    margin: 0;
    color: var(--text);
    font-weight: 750;
    font-variant-numeric: tabular-nums;
  }

  .stats-chart {
    --plot-height: clamp(124px, 22vh, 206px);
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 10px;
    align-items: start;
  }

  .stats-chart-scale {
    display: flex;
    flex-direction: column;
    align-items: end;
    justify-content: space-between;
    height: calc(var(--plot-height) + 8px);
    margin-top: 4px;
    color: var(--text-soft);
    font-size: 0.7rem;
    font-variant-numeric: tabular-nums;
    line-height: 1;
  }

  .stats-chart-scroll {
    overflow-x: auto;
    padding: 8px 3px 4px;
    scrollbar-width: thin;
  }

  .stats-chart-bars {
    display: grid;
    grid-template-columns: repeat(var(--chart-days), minmax(15px, 1fr));
    width: max-content;
    min-width: 100%;
    background: repeating-linear-gradient(to bottom, var(--border) 0 1px, transparent 1px 25%)
      top / 100% var(--plot-height) no-repeat;
  }

  .stats-chart-bars.dense {
    grid-template-columns: repeat(var(--chart-days), minmax(20px, 1fr));
  }

  .stats-chart-bars:has(.stats-chart-day:only-child) {
    grid-template-columns: minmax(72px, 1fr);
  }

  .stats-chart-day {
    position: relative;
    display: grid;
    justify-items: center;
    grid-template-rows: var(--plot-height) 22px;
    min-width: 0;
    color: var(--text-soft);
    font-size: 0.68rem;
    background: transparent;
    border: 0;
    padding: 0;
    cursor: pointer;
  }

  .stats-chart-day:focus-visible {
    z-index: 1;
    outline-offset: 0;
  }

  .stats-chart-plot {
    display: flex;
    align-items: end;
    justify-content: center;
    width: 100%;
    border-bottom: 1px solid var(--border);
  }

  .stats-chart-date {
    position: absolute;
    bottom: 0;
    left: 50%;
    transform: translateX(-50%);
    white-space: nowrap;
    pointer-events: none;
  }

  .stats-chart-day:first-child .stats-chart-date {
    left: 0;
    transform: none;
  }

  .stats-chart-bar {
    display: block;
    width: clamp(5px, 68%, 28px);
    height: var(--bar-height);
    min-height: 2px;
    border-radius: 4px 4px 0 0;
    background: linear-gradient(to top, color-mix(in srgb, var(--accent) 58%, var(--panel-soft)), var(--accent));
  }

  .stats-chart-bar.zero {
    background: var(--border-strong);
  }

  .stats-chart-day:is(:hover, [aria-pressed="true"]) {
    color: var(--text);
    background: var(--panel-hover);
    border-radius: 4px;
  }

  .stats-chart-day[aria-pressed="true"] .stats-chart-bar:not(.zero) {
    background: var(--accent-text);
  }

  .stats-chart-empty,
  .stats-empty {
    margin: 0;
    color: var(--text-soft);
    font-size: 0.9rem;
  }

  .stats-chart-empty {
    display: grid;
    min-height: 105px;
    place-items: center;
    border-block: 1px solid var(--border);
  }

  .stats-transfer-panel,
  .stats-import-preview,
  .stats-import-tracks,
  .stats-import-track {
    display: grid;
    gap: 10px;
  }

  .stats-transfer-actions button,
  .stats-import-preview button {
    min-height: 38px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel-strong);
    color: var(--text);
    cursor: pointer;
    font: inherit;
    font-weight: 750;
    padding: 7px 12px;
  }

  .stats-import-preview {
    border-top: 1px solid var(--border);
    padding-top: 12px;
  }

  .stats-import-preview h5 { margin: 0; font-size: 1rem; }
  .stats-import-tracks { max-height: 280px; overflow-y: auto; }
  .stats-import-track {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 10px;
  }
  .stats-import-track span,
  .stats-import-track small,
  .stats-import-track label { color: var(--text-soft); }

  .stats-overview-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    margin: 0;
    border-bottom: 1px solid var(--border);
  }

  .stats-overview-grid.both-activities {
    grid-template-columns: repeat(5, minmax(0, 1fr));
  }

  .stats-overview-card {
    min-width: 0;
    padding: 4px clamp(10px, 2vw, 24px) 18px;
  }

  .stats-overview-card:first-child {
    padding-left: 0;
  }

  .stats-overview-card + .stats-overview-card {
    border-left: 1px solid var(--border);
  }

  .stats-overview-card dt,
  .stats-overview-card > span {
    color: var(--text-soft);
    font-size: 0.83rem;
    font-weight: 650;
  }

  .stats-overview-card dd {
    margin: 4px 0 0;
    color: var(--text);
    font-size: clamp(1.4rem, 3vw, 2.85rem);
    font-weight: 800;
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.05em;
    line-height: 1.1;
    overflow-wrap: anywhere;
  }

  .stats-overview-card.highlight dd {
    color: var(--accent-text);
  }

  .stats-section-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 18px;
  }

  .stats-section-grid.full-list {
    grid-template-columns: minmax(0, 1fr);
  }

  .stats-back-button {
    justify-self: start;
  }

  .stats-section-grid :global(.library-section) {
    align-content: start;
  }

  .stats-section-grid > :global(.library-section + .library-section) {
    margin-top: 0;
  }

  .stats-section-grid :global(.section-header button) {
    min-height: 28px;
    border-color: transparent;
    background: transparent;
    color: var(--accent-text);
    cursor: pointer;
    padding-inline: 6px;
  }

  .stats-section-grid :global(.section-header button:hover),
  .stats-section-grid :global(.section-header button:focus-visible) {
    background: var(--panel-hover);
  }

  .stats-rank-list,
  .stats-recent-list {
    display: grid;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .stats-rank-card,
  .stats-recent-card {
    display: grid;
    align-items: center;
    width: 100%;
    min-width: 0;
    min-height: 53px;
    border: 1px solid transparent;
    border-radius: 8px;
    background: transparent;
    color: inherit;
    cursor: pointer;
    font: inherit;
    padding: 7px 6px;
    text-align: left;
  }

  .stats-recent-entry {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 6px;
    align-items: center;
  }

  .stats-remove-listen {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel);
    color: var(--text-soft);
    padding: 6px 8px;
    cursor: pointer;
    font: inherit;
    font-size: 0.72rem;
  }

  .stats-remove-listen:hover:not(:disabled),
  .stats-remove-listen:focus-visible {
    border-color: var(--accent-strong);
    background: var(--panel-hover);
    color: var(--text);
  }

  .stats-rank-card {
    grid-template-columns: 20px 36px minmax(0, 1fr) auto;
    gap: 8px;
  }

  .stats-recent-card {
    grid-template-columns: 36px minmax(0, 1fr) auto;
    gap: 8px;
  }

  .stats-rank-card:hover:not(:disabled),
  .stats-rank-card:focus-visible,
  .stats-recent-card:hover:not(:disabled),
  .stats-recent-card:focus-visible {
    border-color: var(--accent-strong);
    background: var(--panel-hover);
    outline: none;
  }

  .stats-rank-card:disabled,
  .stats-recent-card:disabled {
    cursor: default;
    opacity: 0.6;
  }

  .stats-rank-number {
    color: var(--text-soft);
    font-size: 0.78rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    text-align: center;
  }

  .stats-avatar,
  .stats-cover,
  .stats-genre-mark,
  .stats-mini-cover {
    width: 36px;
    height: 36px;
    margin: 0;
    flex: 0 0 auto;
  }

  .stats-cover .album-art-disc {
    border-width: 5px;
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
    color: var(--bg);
    font-size: 1.12rem;
    font-weight: 900;
  }

  .stats-mini-cover {
    background: var(--panel-strong);
    color: var(--text-soft);
    font-size: 1rem;
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
  .stats-played-at,
  .stats-rank-count {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .stats-pagination {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-top: 10px;
    color: var(--text-soft);
    font-size: 0.82rem;
    font-weight: 750;
  }

  .stats-pagination button {
    min-height: 34px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel-strong);
    color: var(--text-muted);
    cursor: default;
    font: inherit;
    font-weight: 800;
    padding: 0 12px;
  }

  .stats-pagination button:hover,
  .stats-pagination button:focus-visible {
    border-color: var(--accent-strong);
    background: var(--panel-hover);
    color: var(--text);
    outline: none;
  }

  .stats-rank-copy strong {
    color: var(--text);
    font-size: 0.95rem;
    font-weight: 820;
    line-height: 1.2;
  }

  .stats-rank-copy small,
  .stats-played-at {
    color: var(--text-soft);
    font-size: 0.8rem;
    font-weight: 750;
  }

  .stats-played-at {
    justify-self: end;
    max-width: 180px;
  }

  .stats-rank-count {
    justify-self: end;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  @container stats (max-width: 870px) {
    .stats-overview-grid.both-activities {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }

    .both-activities .stats-overview-card:nth-child(4) {
      border-left: 0;
      padding-left: 0;
    }

    .both-activities .stats-overview-card:nth-child(n+4) {
      border-top: 1px solid var(--border);
      padding-top: 12px;
    }
  }

  @container stats (max-width: 730px) {
    .stats-section-grid {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  @container stats (max-width: 520px) {
    .stats-overview-grid,
    .stats-overview-grid.both-activities {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .both-activities .stats-overview-card:nth-child(4) {
      border-left: 1px solid var(--border);
      padding-left: clamp(10px, 2vw, 24px);
    }

    .stats-overview-card:nth-child(odd) {
      border-left: 0;
      padding-left: 0;
    }

    .stats-overview-card:nth-child(n+3) {
      border-top: 1px solid var(--border);
      padding-top: 12px;
    }

    .stats-recent-card {
      grid-template-columns: 36px minmax(0, 1fr);
    }

    .stats-played-at {
      grid-column: 2;
      justify-self: start;
      max-width: 100%;
    }
  }

  .organizer-header {
    display: flex;
    flex-wrap: wrap;
    align-items: end;
    justify-content: space-between;
    gap: 14px 24px;
  }

  .organizer-header h2 {
    margin: 0;
    font-size: clamp(2.25rem, 4.8vw, 3.65rem);
    line-height: 1.05;
    letter-spacing: -0.04em;
    overflow-wrap: anywhere;
  }

  .settings-panel {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    width: min(100%, 1320px);
    margin-inline: auto;
    gap: 24px;
    container: settings / inline-size;
  }

  .settings-layout {
    display: grid;
    grid-template-columns: minmax(150px, 182px) minmax(0, 1fr);
    align-items: start;
    gap: 20px;
  }

  .settings-index {
    position: sticky;
    top: 12px;
    display: grid;
    gap: 4px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--panel-soft);
    padding: 8px;
  }

  .settings-index button {
    min-width: 0;
    min-height: 34px;
    border: 1px solid transparent;
    border-radius: 7px;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    font: inherit;
    font-size: 0.88rem;
    font-weight: 700;
    padding: 6px 9px;
    text-align: left;
  }

  .settings-index button:hover,
  .settings-index button:focus-visible {
    border-color: var(--accent-strong);
    background: var(--panel-hover);
    color: var(--text);
  }

  .settings-sections {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 16px;
  }

  @container settings (max-width: 740px) {
    .settings-layout {
      grid-template-columns: minmax(0, 1fr);
      gap: 14px;
    }

    .settings-index {
      position: static;
      display: flex;
      flex-wrap: wrap;
    }
  }

  .cd-ripper-page {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    width: min(100%, 1320px);
    margin-inline: auto;
    gap: 22px;
    container: cd-rip / inline-size;
  }

  .cd-ripper-header {
    display: flex;
    flex-wrap: wrap;
    align-items: end;
    justify-content: space-between;
    gap: 14px;
  }

  .cd-ripper-header h2 {
    margin: 0;
    font-size: clamp(2.5rem, 4.8vw, 3.65rem);
    line-height: 1.05;
    letter-spacing: -0.04em;
  }

  .cd-ripper-header p { margin: 8px 0 0; color: var(--text-soft); }
  .cd-overview {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    margin: 0;
    background: var(--panel-soft);
    border: 1px solid var(--border);
    border-radius: 10px;
  }
  .cd-overview > div { padding: clamp(12px, 3cqw, 22px); min-width: 0; }
  .cd-overview > div + div { border-left: 1px solid var(--border); }
  .cd-overview dt, .cd-overview small, .cd-rip-progress span { color: var(--text-soft); font-size: 0.83rem; }
  .cd-overview dd { margin: 5px 0 0; font-size: clamp(1rem, 3cqw, 1.65rem); font-weight: 750; overflow-wrap: anywhere; }
  .cd-destination { display: flex; justify-content: space-between; align-items: end; gap: 12px; flex-wrap: wrap; }
  .cd-destination > div { min-width: 0; max-width: 100%; }
  .cd-rip-progress { display: grid; gap: 8px; }
  .cd-rip-progress > div { display: flex; gap: 8px; justify-content: space-between; flex-wrap: wrap; }
  .cd-rip-progress progress { width: 100%; accent-color: var(--accent); }

  @container cd-rip (max-width: 380px) {
    .cd-overview > div { padding-inline: 8px; }
  }

  .settings-note {
    margin: 0;
    color: var(--text-soft);
    font-weight: 650;
    line-height: 1.45;
  }

  .settings-section {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    min-width: 0;
    gap: 14px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--panel-soft);
    padding: clamp(14px, 2.3cqw, 24px);
    scroll-margin-top: 12px;
  }

  .settings-subsection {
    display: grid;
    gap: 10px;
    border-top: 1px solid var(--border);
    padding-top: 14px;
  }

  .settings-subsection h4 {
    margin: 0;
    font-size: 0.96rem;
  }

  .cd-status-card,
  .cd-metadata-section,
  .cd-track-section,
  .cd-debug-output {
    display: grid;
    gap: 14px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-soft);
    padding: 18px;
  }

  .settings-section-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
  }

  .settings-section-header :is(h3, h4) {
    margin: 0;
    color: var(--text);
    font-size: 1.08rem;
    line-height: 1.2;
  }

  .settings-section-header h3 {
    font-size: 1.15rem;
  }

  .settings-pill {
    display: inline-flex;
    align-items: center;
    min-height: 28px;
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    background: var(--bg-soft);
    color: var(--text-muted);
    font-size: 0.78rem;
    font-weight: 850;
    padding: 0 10px;
    white-space: nowrap;
  }

  .library-folder-card {
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel);
    padding: 14px;
  }

  .cd-output-folder {
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel);
    padding: 14px;
  }

  .library-folder-card span,
  .cd-output-folder span,
  .settings-status-list span,
  .settings-control-list span,
  .about-grid span {
    display: block;
    margin-bottom: 5px;
    color: var(--text-soft);
    font-size: 0.76rem;
    font-weight: 850;
    text-transform: uppercase;
  }

  .library-folder-card strong,
  .library-folder-card small {
    display: block;
    overflow-wrap: anywhere;
  }

  .library-folder-card strong {
    font-size: 0.95rem;
    font-weight: 760;
  }

  .library-folder-card small {
    margin-top: 6px;
    color: var(--text-soft);
  }

  .cd-output-folder strong {
    display: block;
    overflow-wrap: anywhere;
    color: var(--text);
    font-size: 0.95rem;
    font-weight: 760;
  }

  .settings-stat-grid,
  .settings-status-list {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    margin: 0;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 8px;
  }

  .settings-stat-grid > div,
  .settings-status-list > div {
    min-width: 0;
    padding: clamp(9px, 1.65cqw, 16px);
    overflow-wrap: anywhere;
  }

  .settings-stat-grid > div + div,
  .settings-status-list > div + div {
    border-left: 1px solid var(--border);
  }

  .settings-stat-grid dt {
    color: var(--text-soft);
    font-size: 0.8rem;
  }

  .settings-stat-grid dd {
    margin: 5px 0 0;
    font-size: clamp(1rem, 2.4cqw, 1.5rem);
    font-weight: 750;
  }

  .about-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }

  .settings-control-list,
  .about-grid > div {
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel);
  }

  .about-grid > div,
  .settings-control-list > div,
  .settings-control-list > label {
    padding: 12px 14px;
  }

  .settings-control-list > * + * {
    border-top: 1px solid var(--border);
  }

  .settings-status-list strong,
  .settings-control-list strong,
  .about-grid strong {
    display: block;
    overflow-wrap: anywhere;
    color: var(--text);
    font-size: 0.98rem;
    font-weight: 820;
    line-height: 1.25;
  }

  .settings-control-list small {
    display: block;
    margin-top: 4px;
    color: var(--text-soft);
    font-size: 0.8rem;
    line-height: 1.35;
    overflow-wrap: anywhere;
  }

  .folder-scan-list { display: grid; gap: 8px; list-style: none; margin: 0; padding: 0; }
  .folder-scan-list li { display: flex; flex-wrap: wrap; justify-content: space-between; align-items: center; gap: 10px; padding: 10px 12px; border: 1px solid var(--border); border-radius: 8px; }
  .folder-scan-list li > div { display: grid; gap: 4px; min-width: 0; overflow-wrap: anywhere; }
  .folder-scan-list small { color: var(--text-soft); }
  .folder-scan-progress { display: grid; gap: 8px; overflow-wrap: anywhere; }
  .folder-scan-progress progress { width: 100%; }
  .ignored-tracks summary { cursor: pointer; font-weight: 700; padding: 8px 0; }
  .album-track-row.bulk-selected { outline: 1px solid var(--accent); outline-offset: -1px; background: color-mix(in srgb, var(--accent) 13%, transparent); }
  .album-selection-button { padding: 8px 10px; border: 1px solid var(--border); border-radius: 8px; background: var(--panel); color: inherit; font: inherit; }
  .album-selection-controls { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; }

  .listen-threshold {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    font-weight: 700;
  }

  .listen-threshold select {
    font: inherit;
    color: var(--text);
    background: var(--panel);
    padding: 7px 10px;
    border: 1px solid var(--border);
    border-radius: 8px;
  }

  .settings-toggle-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 3px 12px;
    cursor: pointer;
  }

  .settings-control-list .settings-toggle-row span {
    margin: 0;
    color: var(--text);
    font-size: 0.93rem;
    font-weight: 700;
    text-transform: none;
  }

  .settings-toggle-row input {
    appearance: none;
    grid-area: 1 / 2 / span 2;
    width: 34px;
    height: 20px;
    margin: 0;
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    background: radial-gradient(circle at 9px center, var(--text-soft) 5px, transparent 5.5px), var(--panel-strong);
    cursor: pointer;
  }

  .settings-toggle-row input:checked {
    border-color: var(--accent-strong);
    background: radial-gradient(circle at calc(100% - 9px) center, var(--accent-text) 5px, transparent 5.5px), var(--accent-soft);
  }

  .settings-toggle-row input:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 3px;
  }

  @media (forced-colors: active) {
    .settings-toggle-row input {
      appearance: auto;
      width: 18px;
      height: 18px;
    }
  }

  .settings-toggle-row small {
    grid-column: 1;
    margin-top: 0;
  }

  @container settings (max-width: 470px) {
    .settings-stat-grid,
    .settings-status-list {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    :is(.settings-stat-grid, .settings-status-list) > div:nth-child(3) {
      border-left: 0;
    }

    :is(.settings-stat-grid, .settings-status-list) > div:nth-child(n+3) {
      border-top: 1px solid var(--border);
    }
  }

  .update-available-card {
    display: grid;
    gap: 14px;
    border: 1px solid var(--accent-strong);
    border-radius: 8px;
    background: var(--panel);
    padding: 14px;
  }

  .update-available-card h5,
  .update-release-notes p {
    margin: 0;
  }

  .update-available-card h5 {
    color: var(--text);
    font-size: 1rem;
  }

  .update-release-notes {
    display: grid;
    gap: 6px;
  }

  .update-release-notes strong {
    color: var(--text-muted);
    font-size: 0.78rem;
    text-transform: uppercase;
  }

  .update-release-notes p {
    max-height: 180px;
    overflow: auto;
    color: var(--text-muted);
    font-size: 0.86rem;
    line-height: 1.5;
    white-space: pre-wrap;
  }

  .update-progress {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 12px;
  }

  .update-progress progress {
    width: 100%;
    accent-color: var(--accent);
  }

  .update-progress strong {
    color: var(--text-muted);
    font-size: 0.8rem;
  }

  .update-notice {
    position: fixed;
    right: 24px;
    bottom: 108px;
    z-index: 75;
    display: flex;
    align-items: center;
    gap: 10px;
    max-width: min(520px, calc(100vw - 48px));
    border: 1px solid var(--accent-strong);
    border-radius: 8px;
    background: var(--panel-strong);
    box-shadow: 0 14px 36px var(--shadow);
    color: var(--text);
    padding: 12px;
  }

  .update-notice > div {
    display: grid;
    min-width: 0;
    gap: 3px;
    margin-right: 6px;
  }

  .update-notice strong,
  .update-notice span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .update-notice strong {
    font-size: 0.88rem;
  }

  .update-notice span {
    color: var(--text-soft);
    font-size: 0.76rem;
    font-weight: 720;
  }

  .update-notice button {
    min-height: 34px;
    border: 1px solid var(--border-strong);
    border-radius: 7px;
    background: var(--panel);
    color: var(--text);
    cursor: default;
    font: inherit;
    font-size: 0.78rem;
    font-weight: 850;
    padding: 0 10px;
  }

  .update-notice button:hover,
  .update-notice button:focus-visible {
    border-color: var(--accent-strong);
    background: var(--panel-hover);
    outline: none;
  }

  .theme-preset-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 140px), 1fr));
    gap: 10px;
  }

  .theme-preset-grid button {
    display: grid;
    min-width: 0;
    gap: 8px;
    justify-items: start;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel);
    color: inherit;
    cursor: pointer;
    font: inherit;
    padding: 14px;
    text-align: left;
  }

  .theme-preset-grid button:hover,
  .theme-preset-grid button:focus-visible,
  .theme-preset-grid button.selected {
    border-color: var(--accent-strong);
    background: var(--panel-hover);
    outline: none;
  }

  .theme-preset-grid button.selected {
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 36%, transparent);
  }

  .theme-preset-grid strong {
    color: var(--text);
    font-size: 0.94rem;
    font-weight: 850;
    line-height: 1.2;
  }

  .theme-preset-grid small {
    color: var(--text-soft);
    font-size: 0.78rem;
    font-weight: 720;
    line-height: 1.35;
  }

  .theme-swatch-row {
    display: flex;
    gap: 6px;
  }

  .theme-swatch-row span {
    width: 18px;
    height: 18px;
    border: 1px solid rgba(255, 255, 255, 0.18);
    border-radius: 999px;
    background: var(--swatch-color);
    box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.18);
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
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel-strong);
    color: var(--text);
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
    border-color: var(--accent-strong);
    background: var(--panel-hover);
    outline: none;
  }

  .settings-actions button.primary {
    border-color: var(--accent-strong);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .cd-rip-actions button.primary {
    border-color: var(--accent-strong);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .settings-actions button.danger {
    border-color: color-mix(in srgb, var(--danger) 30%, var(--border));
    color: var(--danger);
  }

  .settings-actions button:disabled,
  .cd-rip-actions button:disabled {
    border-color: var(--border);
    background: var(--bg-soft);
    color: var(--text-dim);
  }

  .settings-tool-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 190px), 1fr));
    gap: 10px;
  }

  .settings-tool-grid button {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    min-width: 0;
    min-height: 66px;
    gap: 2px 12px;
    align-items: center;
    padding: 11px;
    cursor: pointer;
    text-align: left;
  }

  .settings-tool-grid .health-mark,
  .settings-tool-grid .shortcut-mark,
  .mix-tool-mark {
    width: 36px;
    height: 36px;
    font-size: 1rem;
  }

  .settings-tool-copy {
    display: grid;
    min-width: 0;
    gap: 4px;
  }

  .settings-tool-grid strong,
  .settings-tool-grid small {
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .settings-tool-grid strong {
    color: var(--text);
    font-size: 0.98rem;
    line-height: 1.2;
  }

  .settings-tool-grid small {
    color: var(--text-soft);
    font-size: 0.8rem;
    font-weight: 750;
  }

  .cd-track-table {
    display: grid;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel);
  }

  .cd-track-row {
    display: grid;
    grid-template-columns: 60px minmax(150px, 1.25fr) minmax(130px, 1fr) 92px minmax(112px, 0.7fr) minmax(0, 1.4fr);
    min-height: 48px;
    align-items: center;
    gap: 12px;
    border-top: 1px solid var(--border);
    color: var(--text);
    font-size: 0.9rem;
    font-weight: 700;
    padding: 10px 14px;
  }

  .cd-track-row:first-child {
    border-top: 0;
  }

  .cd-track-head {
    min-height: 40px;
    background: var(--bg-soft);
    color: var(--text-soft);
    font-size: 0.76rem;
    font-weight: 850;
    text-transform: uppercase;
  }

  .cd-track-row.active {
    background: var(--accent-soft);
  }

  .cd-track-row.error {
    background: var(--danger-soft);
    color: var(--danger);
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
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--bg-soft);
    color: var(--text);
    font: inherit;
    font-size: 0.86rem;
    font-weight: 650;
    outline: none;
    padding: 0 10px;
  }

  .cd-track-row input:focus,
  .cd-metadata-form input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 22%, transparent);
  }

  .cd-track-row input:disabled,
  .cd-metadata-form input:disabled,
  .cd-release-list button:disabled {
    border-color: var(--border);
    background: var(--bg-soft);
    color: var(--text-soft);
  }

  .cd-track-row strong {
    color: inherit;
    text-transform: capitalize;
  }

  .cd-track-row small {
    display: block;
    overflow: hidden;
    margin-top: 3px;
    color: var(--danger);
    font-size: 0.76rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cd-debug-output summary {
    color: var(--text);
    cursor: default;
    font-weight: 800;
  }

  .cd-debug-output pre {
    overflow: auto;
    max-height: 260px;
    margin: 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-soft);
    color: var(--text-muted);
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
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel);
    color: inherit;
    cursor: default;
    font: inherit;
    padding: 12px;
    text-align: left;
  }

  .cd-release-list button:hover,
  .cd-release-list button:focus-visible,
  .cd-release-list button.active {
    border-color: var(--accent-strong);
    background: var(--accent-soft);
    outline: none;
  }

  .cd-release-list strong,
  .cd-selected-release strong {
    overflow: hidden;
    color: var(--text);
    font-size: 0.95rem;
    font-weight: 850;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cd-release-list small,
  .cd-selected-release small {
    overflow: hidden;
    color: var(--text-soft);
    font-size: 0.8rem;
    font-weight: 700;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cd-selected-release span,
  .cd-metadata-form span {
    color: var(--text-soft);
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
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel);
    padding: 12px;
  }

  .cd-cover-preview {
    display: grid;
    aspect-ratio: 1;
    overflow: hidden;
    place-items: center;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--bg-soft);
    color: var(--text-dim);
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
    color: var(--text-soft);
    font-size: 0.76rem;
    font-weight: 850;
    text-transform: uppercase;
  }

  .cd-cover-copy strong {
    overflow: hidden;
    color: var(--text);
    font-size: 1rem;
    font-weight: 850;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cd-cover-copy small {
    overflow: hidden;
    color: var(--text-soft);
    font-size: 0.82rem;
    font-weight: 700;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cd-cover-copy small.error {
    color: var(--danger);
  }

  .cd-cover-copy button {
    justify-self: start;
    min-height: 36px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel-strong);
    color: var(--text);
    cursor: default;
    font: inherit;
    font-size: 0.84rem;
    font-weight: 850;
    padding: 0 12px;
  }

  .cd-cover-copy button:hover:not(:disabled),
  .cd-cover-copy button:focus-visible:not(:disabled) {
    border-color: var(--accent-strong);
    background: var(--panel-hover);
    outline: none;
  }

  .cd-cover-copy button:disabled {
    border-color: var(--border);
    background: var(--bg-soft);
    color: var(--text-soft);
  }

  .mix-tool-mark {
    display: grid;
    flex: 0 0 auto;
    place-items: center;
    border-radius: 8px;
    background: var(--accent-soft);
    color: var(--accent-text);
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
    background: var(--panel-strong);
    color: var(--text);
    font-size: 1.35rem;
    font-weight: 900;
  }

  .tag-editor-backdrop,
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

  .tag-editor-modal,
  .shortcuts-modal,
  .confirmation-modal {
    width: min(720px, 100%);
    max-height: min(720px, calc(100dvh - 48px));
    overflow: auto;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel);
    box-shadow: 0 24px 70px rgba(0, 0, 0, 0.46);
    outline: none;
    padding: 22px;
  }

  .confirmation-modal {
    width: min(460px, 100%);
  }

  .tag-editor-modal {
    display: grid;
    gap: 16px;
    width: min(820px, 100%);
  }

  .tag-editor-header {
    display: grid;
    grid-template-columns: 78px minmax(0, 1fr) auto;
    align-items: start;
    gap: 14px;
  }

  .tag-editor-art {
    position: relative;
    display: grid;
    width: 78px;
    height: 78px;
    overflow: hidden;
    place-items: center;
    border-radius: 8px;
    background: linear-gradient(145deg, color-mix(in srgb, var(--accent) 42%, var(--panel)), var(--panel-strong));
    color: var(--accent-text);
    font-size: 1.5rem;
    font-weight: 950;
  }

  .tag-editor-art img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .tag-editor-heading {
    min-width: 0;
  }

  .tag-editor-heading h3,
  .tag-editor-heading strong,
  .tag-editor-heading small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tag-editor-heading h3 {
    margin: 0 0 6px;
    color: var(--text);
    font-size: 1.32rem;
    line-height: 1.2;
  }

  .tag-editor-heading strong,
  .tag-editor-heading small {
    display: block;
  }

  .tag-editor-heading strong {
    color: var(--text);
    font-size: 0.98rem;
    font-weight: 850;
  }

  .tag-editor-heading small {
    margin-top: 3px;
    color: var(--text-soft);
    font-size: 0.82rem;
    font-weight: 750;
  }

  .tag-editor-header > button,
  .tag-editor-actions button {
    min-height: 38px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel-soft);
    color: var(--text);
    cursor: default;
    font: inherit;
    font-size: 0.84rem;
    font-weight: 850;
    padding: 0 12px;
  }

  .tag-editor-header > button:hover:not(:disabled),
  .tag-editor-header > button:focus-visible:not(:disabled),
  .tag-editor-actions button:hover:not(:disabled),
  .tag-editor-actions button:focus-visible:not(:disabled) {
    border-color: var(--accent-strong);
    background: var(--panel-hover);
    outline: none;
  }

  .tag-editor-actions button.primary {
    border-color: var(--accent-strong);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .tag-editor-header > button:disabled,
  .tag-editor-actions button:disabled {
    border-color: var(--border);
    background: var(--bg-soft);
    color: var(--text-dim);
  }

  .tag-editor-path {
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-soft);
    padding: 12px;
  }

  .tag-editor-path span,
  .tag-editor-form label span {
    display: block;
    margin-bottom: 5px;
    color: var(--text-soft);
    font-size: 0.74rem;
    font-weight: 850;
    text-transform: uppercase;
  }

  .tag-editor-path code {
    display: block;
    overflow-wrap: anywhere;
    color: var(--text-muted);
    font-family: ui-monospace, SFMono-Regular, "Cascadia Code", "Liberation Mono", monospace;
    font-size: 0.82rem;
    line-height: 1.35;
  }

  .tag-editor-loading,
  .tag-editor-warning {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-soft);
    color: var(--text-muted);
    font-size: 0.9rem;
    font-weight: 760;
    padding: 12px;
  }

  .tag-editor-warning {
    border-color: color-mix(in srgb, var(--warning) 34%, var(--border));
    background: color-mix(in srgb, var(--warning) 12%, var(--panel));
    color: var(--text);
  }

  .tag-editor-form {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .tag-editor-form label {
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-soft);
    padding: 12px;
  }

  .tag-editor-form label.changed {
    border-color: var(--accent-strong);
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .tag-editor-form input {
    width: 100%;
    min-height: 38px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--bg-soft);
    color: var(--text);
    font: inherit;
    font-size: 0.9rem;
    font-weight: 720;
    padding: 0 10px;
  }

  .tag-editor-form input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--focus-ring) 64%, transparent);
    outline: none;
  }

  .tag-editor-form input[aria-invalid="true"] {
    border-color: color-mix(in srgb, var(--danger) 52%, var(--border));
  }

  .tag-editor-form input:disabled {
    color: var(--text-dim);
  }

  .tag-editor-form .form-message,
  .tag-editor-actions {
    grid-column: 1 / -1;
  }

  .tag-editor-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 10px;
  }

  .album-tag-editor-modal {
    width: min(940px, 100%);
  }

  .album-tag-scope-summary {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 8px 16px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-soft);
    color: var(--text-muted);
    padding: 12px;
  }

  .album-tag-scope-summary strong {
    color: var(--text);
  }

  .tag-editor-warning ul,
  .album-tag-confirmation ul {
    margin: 8px 0 0;
    padding-left: 20px;
  }

  .album-tag-field-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .album-tag-field-grid fieldset {
    min-width: 0;
    margin: 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-soft);
    padding: 12px;
  }

  .album-tag-field-grid fieldset.changed {
    border-color: var(--accent-strong);
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .album-tag-field-grid legend {
    color: var(--text);
    font-size: 0.86rem;
    font-weight: 900;
    padding: 0 5px;
  }

  .album-tag-field-grid p {
    margin: 0 0 10px;
    color: var(--text-muted);
    font-size: 0.82rem;
  }

  .album-tag-field-grid .album-tag-field-help {
    min-height: 2.8em;
    color: var(--text-soft);
    font-size: 0.76rem;
    line-height: 1.4;
  }

  .album-tag-field-grid label {
    display: grid;
    gap: 5px;
    margin-top: 8px;
  }

  .album-tag-field-grid label span {
    color: var(--text-soft);
    font-size: 0.72rem;
    font-weight: 850;
    text-transform: uppercase;
  }

  .album-tag-field-grid select,
  .album-tag-field-grid input {
    width: 100%;
    min-height: 38px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--bg-soft);
    color: var(--text);
    font: inherit;
    font-size: 0.88rem;
    font-weight: 720;
    padding: 0 10px;
  }

  .album-tag-field-grid select:focus,
  .album-tag-field-grid input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--focus-ring) 64%, transparent);
    outline: none;
  }

  .album-tag-clear-note {
    margin-top: 10px !important;
    color: var(--warning) !important;
    font-weight: 780;
  }

  .album-tag-track-scope,
  .album-tag-confirmation section {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-soft);
    padding: 12px;
  }

  .album-tag-track-scope h4,
  .album-tag-confirmation h4 {
    margin: 0 0 10px;
    color: var(--text);
    font-size: 0.9rem;
  }

  .album-tag-track-list,
  .album-tag-result-list {
    display: grid;
    gap: 7px;
    max-height: 230px;
    overflow: auto;
  }

  .album-tag-track-list > div,
  .album-tag-result-list > div {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--panel-soft);
    padding: 9px 10px;
  }

  .album-tag-track-list > div.excluded {
    opacity: 0.72;
  }

  .album-tag-track-list span,
  .album-tag-track-list strong,
  .album-tag-track-list small,
  .album-tag-result-list span,
  .album-tag-result-list small,
  .album-tag-result-list code {
    display: block;
  }

  .album-tag-track-list small,
  .album-tag-result-list small,
  .album-tag-result-list code {
    margin-top: 2px;
    overflow-wrap: anywhere;
    color: var(--text-soft);
    font-size: 0.76rem;
  }

  .album-tag-track-status {
    max-width: 42%;
    color: var(--text-muted);
    font-size: 0.78rem;
    font-weight: 760;
    text-align: right;
  }

  .album-tag-confirmation {
    display: grid;
    gap: 10px;
  }

  .album-tag-confirmation li {
    color: var(--text-muted);
    font-size: 0.86rem;
    line-height: 1.5;
  }

  .album-tag-confirmation li small {
    color: var(--text-soft);
  }

  .album-tag-progress {
    display: grid;
    gap: 8px;
    color: var(--text);
    font-size: 0.86rem;
  }

  .album-tag-progress progress {
    width: 100%;
    accent-color: var(--accent);
  }

  .album-tag-result {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-soft);
    padding: 12px;
  }

  .album-tag-result.success {
    border-color: color-mix(in srgb, var(--accent) 50%, var(--border));
  }

  .album-tag-result.error {
    border-color: color-mix(in srgb, var(--danger) 55%, var(--border));
  }

  .album-tag-result p,
  .album-tag-result-list p {
    margin: 5px 0 0;
    color: var(--text-muted);
    font-size: 0.84rem;
    line-height: 1.45;
  }

  .album-tag-result-list > div {
    display: grid;
    grid-template-columns: minmax(150px, 0.7fr) minmax(0, 1.3fr);
  }

  .album-tag-result-list code {
    grid-column: 1 / -1;
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
    color: var(--text);
    font-size: 1.35rem;
    line-height: 1.2;
  }

  .confirmation-modal > p {
    margin: 0 0 18px;
    color: var(--text-muted);
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
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel-soft);
    color: var(--text);
    cursor: default;
    font: inherit;
    font-size: 0.86rem;
    font-weight: 850;
    padding: 0 13px;
  }

  .confirmation-actions button:hover,
  .confirmation-actions button:focus-visible {
    border-color: var(--accent-strong);
    background: var(--panel-hover);
    color: var(--text);
    outline: none;
  }

  .confirmation-actions button.destructive {
    border-color: color-mix(in srgb, var(--danger) 34%, var(--border));
    background: var(--danger-soft);
    color: var(--danger);
  }

  .confirmation-actions button.destructive:hover,
  .confirmation-actions button.destructive:focus-visible {
    border-color: color-mix(in srgb, var(--danger) 44%, var(--border));
    background: color-mix(in srgb, var(--danger) 18%, var(--danger-soft));
  }

  .confirmation-actions button:disabled {
    border-color: var(--border-strong);
    background: var(--panel-soft);
    color: var(--text-dim);
  }

  .shortcuts-header button {
    min-height: 34px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel-soft);
    color: var(--text);
    cursor: default;
    font: inherit;
    font-size: 0.84rem;
    font-weight: 800;
    padding: 0 12px;
  }

  .shortcuts-header button:hover,
  .shortcuts-header button:focus-visible {
    border-color: var(--accent-strong);
    background: var(--panel-hover);
    color: var(--text);
    outline: none;
  }

  .shortcut-group-list {
    display: grid;
    gap: 14px;
  }

  .shortcut-group {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-soft);
    padding: 14px;
  }

  .shortcut-group h4 {
    margin: 0 0 10px;
    color: var(--text);
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
    color: var(--text-dim);
    font-size: 0.78rem;
    font-weight: 900;
  }

  kbd {
    min-width: 28px;
    border: 1px solid var(--border-strong);
    border-bottom-color: var(--border);
    border-radius: 6px;
    background: var(--bg);
    color: var(--text);
    font-family: inherit;
    font-size: 0.78rem;
    font-weight: 850;
    line-height: 1;
    padding: 7px 9px;
    text-align: center;
  }

  .shortcut-row p {
    margin: 0;
    color: var(--text-muted);
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
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel);
    padding: 12px;
  }

  .health-summary-grid div.issue {
    border-color: color-mix(in srgb, var(--warning) 34%, var(--border));
    background: color-mix(in srgb, var(--warning) 14%, var(--panel));
  }

  .health-summary-grid span {
    display: block;
    overflow: hidden;
    margin-bottom: 5px;
    color: var(--text-soft);
    font-size: 0.75rem;
    font-weight: 850;
    text-overflow: ellipsis;
    text-transform: uppercase;
    white-space: nowrap;
  }

  .health-summary-grid strong {
    color: var(--text);
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
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-soft);
    color: inherit;
    cursor: default;
    font: inherit;
    padding: 10px;
    text-align: left;
  }

  .diagnostic-album-card:hover,
  .diagnostic-album-card:focus-visible {
    border-color: var(--accent-strong);
    background: var(--panel-hover);
    outline: none;
  }

  .diagnostic-album-card > span:first-child {
    display: grid;
    width: 42px;
    height: 42px;
    place-items: center;
    border-radius: 8px;
    background: var(--accent-soft);
    color: var(--accent-text);
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
    color: var(--text);
    font-size: 0.9rem;
    line-height: 1.25;
  }

  .diagnostic-album-card small {
    margin-top: 3px;
    color: var(--text-soft);
    font-size: 0.78rem;
    font-weight: 700;
  }

  .duplicate-album-card {
    display: grid;
    gap: 12px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-soft);
    padding: 14px;
  }

  .duplicate-album-card h3 {
    margin-bottom: 5px;
  }

  .duplicate-album-card p:not(.eyebrow) {
    margin: 0;
    color: var(--text-soft);
    font-weight: 700;
  }

  .duplicate-album-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .duplicate-album-actions button {
    min-height: 34px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel);
    color: var(--text);
    cursor: default;
    font: inherit;
    font-size: 0.82rem;
    font-weight: 800;
    padding: 0 11px;
  }

  .duplicate-album-actions button:hover,
  .duplicate-album-actions button:focus-visible {
    border-color: var(--accent-strong);
    background: var(--panel-hover);
    outline: none;
  }

  .duplicate-folders {
    overflow-wrap: anywhere;
    font-size: 0.82rem;
  }

  .queue-backdrop {
    position: fixed;
    inset: 0 0 86px;
    z-index: 3;
    border: 0;
    background: transparent;
    cursor: default;
  }

  .queue-panel {
    position: fixed;
    right: 22px;
    bottom: 104px;
    z-index: 4;
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: min(480px, calc(100vw - 32px));
    max-height: min(650px, calc(100dvh - 136px));
    border: 1px solid var(--border-strong);
    border-radius: 14px;
    background: var(--panel-soft);
    box-shadow: 0 22px 60px rgba(0, 0, 0, 0.28);
    padding: 16px;
  }

  .queue-panel-header,
  .queue-panel-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .queue-panel-header {
    justify-content: space-between;
  }

  .queue-panel-title {
    min-width: 0;
  }

  .queue-panel-title h3 {
    margin: 0 0 3px;
    font-size: 1.35rem;
    line-height: 1.1;
  }

  .queue-panel-title span,
  .queue-current-label {
    color: var(--text-soft);
    font-size: 0.78rem;
    font-weight: 700;
  }

  .queue-panel-actions button,
  .queue-remove {
    min-height: 32px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel);
    color: var(--text);
    font: inherit;
    font-size: 0.8rem;
    font-weight: 750;
    padding: 0 10px;
    cursor: pointer;
  }

  .queue-panel-actions button.icon,
  .queue-remove {
    width: 32px;
    padding: 0;
    font-size: 1.15rem;
    line-height: 1;
  }

  .queue-panel-actions button:hover:not(:disabled),
  .queue-remove:hover {
    border-color: var(--accent-strong);
    background: var(--panel-hover);
  }

  .queue-panel-actions button:disabled {
    color: var(--text-dim);
    cursor: default;
  }

  .queue-list {
    display: grid;
    align-content: start;
    gap: 3px;
    min-height: 0;
    overflow: auto;
    overscroll-behavior: contain;
    scrollbar-color: var(--border-strong) transparent;
    scrollbar-width: thin;
  }

  .queue-empty-state {
    border: 1px dashed var(--border);
    border-radius: 10px;
    padding: 24px 14px;
    text-align: center;
  }

  .queue-empty-state p {
    margin: 5px 0 0;
    color: var(--text-soft);
    font-size: 0.84rem;
    line-height: 1.45;
  }

  .queue-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    align-items: center;
    gap: 8px;
    min-width: 0;
    border: 1px solid transparent;
    border-radius: 9px;
    padding: 4px;
  }

  .queue-row:hover,
  .queue-row:focus-within {
    background: var(--panel-hover);
  }

  .queue-row.active {
    grid-template-columns: minmax(0, 1fr) auto;
    border-color: var(--accent-strong);
    background: var(--accent-soft);
    padding: 6px 9px 6px 6px;
  }

  .queue-row-main {
    display: grid;
    grid-template-columns: 25px 38px minmax(0, 1fr);
    align-items: center;
    gap: 8px;
    min-width: 0;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: inherit;
    cursor: pointer;
    font: inherit;
    padding: 3px;
    text-align: left;
  }

  .queue-row.active .queue-row-main {
    grid-template-columns: 46px minmax(0, 1fr);
  }

  .queue-row-position {
    color: var(--text-soft);
    font-size: 0.75rem;
    font-weight: 700;
    text-align: center;
  }

  .queue-row-art {
    display: grid;
    width: 100%;
    aspect-ratio: 1;
    place-items: center;
    overflow: hidden;
    border-radius: 7px;
    background: color-mix(in srgb, var(--accent) 22%, var(--panel-strong));
    color: var(--text);
    font-weight: 800;
  }

  .queue-row-art > * {
    grid-area: 1 / 1;
  }

  .queue-row-art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .queue-row-copy {
    display: grid;
    min-width: 0;
    gap: 3px;
  }

  .queue-row-copy strong,
  .queue-row-copy small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .queue-row-copy strong {
    font-size: 0.9rem;
    line-height: 1.2;
  }

  .queue-row-copy small,
  .queue-row-duration {
    color: var(--text-soft);
    font-size: 0.76rem;
  }

  .queue-current-label {
    color: var(--accent-text);
  }

  .queue-row-duration {
    font-variant-numeric: tabular-nums;
  }

  .queue-remove {
    border-color: transparent;
    background: transparent;
    color: var(--text-muted);
  }

  @media (max-width: 1020px) {
    .artist-grid,
    .genre-grid,
    .video-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .videos-toolbar,
    .video-detail-hero {
      grid-template-columns: 1fr;
    }

    .video-session-card {
      grid-template-columns: 160px minmax(0, 1fr);
    }

    .video-session-status,
    .video-window-note,
    .video-window-actions {
      grid-column: 1 / -1;
    }

    .video-stat-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .dvd-status-grid,
    .dvd-import-form {
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

    .cd-metadata-form {
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

    .workspace-navigation {
      top: 16px;
    }

    .home {
      --content-bottom-padding: 58px;
      padding: 22px 16px var(--content-bottom-padding);
    }

    .home.albums-landing-view {
      --content-bottom-padding: 24px;
      padding-top: 22px;
    }

    .home.album-detail-view {
      --content-bottom-padding: 22px;
    }

    .home.artist-detail-view {
      --content-bottom-padding: 24px;
    }

    .home.genre-detail-view {
      --content-bottom-padding: 24px;
    }

    .home.songs-library-view {
      --content-bottom-padding: 34px;
      padding-top: 22px;
    }

    .home.playlists-view {
      --content-bottom-padding: 28px;
    }

    .home.playlist-detail-view {
      --content-bottom-padding: 28px;
    }

    .home.lyrics-mode {
      padding: 82px 16px var(--content-bottom-padding);
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

    .home-header-actions {
      justify-content: flex-start;
      padding-top: 0;
    }

    .home.albums-landing-view .home-header {
      gap: 14px;
      margin-bottom: 20px;
    }

    .home.albums-landing-view .home-header h2 {
      font-size: 2.4rem;
    }

    .home.albums-landing-view > .search-bar {
      max-width: none;
      margin: -2px 0 16px;
    }

    .home.songs-library-view .home-header {
      gap: 14px;
      margin-bottom: 18px;
    }

    .home.songs-library-view .home-header h2 {
      font-size: 2.4rem;
    }

    .home.songs-library-view > .search-bar {
      max-width: none;
      margin: -2px 0 14px;
    }

    .song-browser-controls {
      align-items: stretch;
    }

    .song-browser-controls :global(.compact-dropdown) {
      flex: 1 1 180px;
    }

    .albums-landing .control-bar {
      justify-content: flex-start;
    }

    .albums-landing .album-grid {
      gap: 16px;
    }

    .lyrics-view-top,
    .lyrics-experience {
      grid-template-columns: minmax(0, 1fr);
    }

    .lyrics-view-top {
      gap: 12px;
    }

    .lyrics-header-spacer {
      display: none;
    }

    .lyrics-options-button {
      justify-self: start;
    }

    .lyrics-experience {
      min-height: auto;
    }

    .lyrics-sidecar {
      display: grid;
      width: 100%;
      grid-template-columns: clamp(156px, 32vw, 204px) minmax(0, 1fr);
      align-items: end;
      gap: 14px;
      justify-self: stretch;
      order: 2;
    }

    .lyrics-sidecar::before {
      inset-inline: 0;
    }

    .lyrics-cover-ambient {
      inset-inline: 0 auto;
      width: clamp(156px, 32vw, 204px);
      height: clamp(156px, 32vw, 204px);
      transform: none;
    }

    .lyrics-cover {
      width: clamp(156px, 32vw, 204px);
      height: clamp(156px, 32vw, 204px);
    }

    .lyrics-sidecar-copy {
      margin-top: 0;
    }

    .lyrics-focus {
      grid-column: auto;
      min-height: 420px;
      order: 1;
    }

    .synced-lyrics,
    .plain-lyrics {
      max-height: min(54vh, 480px);
    }

    .artist-detail-header,
    .genre-detail-header,
    .playlist-detail-header {
      align-items: flex-start;
      flex-direction: column;
    }

    .album-detail-header {
      grid-template-columns: 1fr;
      align-items: start;
    }

    .album-tag-move-notice {
      align-items: stretch;
      flex-direction: column;
    }

    .album-detail-cover-shell {
      width: min(100%, 280px);
    }

    .detail-cover {
      width: min(100%, 220px);
    }

    .album-detail-header .detail-cover {
      width: 100%;
      max-width: 280px;
    }

    .album-track-row {
      grid-template-columns: 36px minmax(0, 1fr) auto auto;
    }

    .album-track-row.with-play-counts {
      grid-template-columns: 36px minmax(0, 1fr) minmax(70px, auto) auto auto;
    }

    .album-track-format {
      display: none;
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

    .about-grid {
      grid-template-columns: 1fr;
    }

    .cd-track-row {
      grid-template-columns: 52px minmax(130px, 1.1fr) minmax(110px, 0.9fr) 80px minmax(100px, 0.8fr) minmax(0, 1fr);
    }

    .queue-panel {
      left: 16px;
      right: 16px;
      bottom: 150px;
      width: auto;
      max-height: calc(100dvh - 178px);
    }

    .tag-editor-backdrop,
    .shortcuts-backdrop,
    .confirmation-backdrop {
      align-items: start;
      padding: 16px;
    }

    .tag-editor-header {
      grid-template-columns: 64px minmax(0, 1fr);
    }

    .tag-editor-header > button {
      grid-column: 1 / -1;
      justify-self: start;
    }

    .tag-editor-art {
      width: 64px;
      height: 64px;
    }

    .tag-editor-form {
      grid-template-columns: 1fr;
    }

    .album-tag-field-grid,
    .album-tag-result-list > div {
      grid-template-columns: 1fr;
    }

    .album-tag-track-list > div {
      display: grid;
    }

    .album-tag-track-status {
      max-width: none;
      text-align: left;
    }
  }

  @media (max-width: 520px) {
    .album-grid,
    .artist-grid,
    .genre-grid,
    .video-grid,
    .mix-option-grid,
    .health-summary-grid,
    .diagnostic-album-list,
    .video-stat-grid,
    .video-meta-grid,
    .video-codec-grid,
    .video-edit-form,
    .dvd-status-grid,
    .dvd-import-form,
    .cd-metadata-form {
      grid-template-columns: minmax(0, 1fr);
    }

    .home.albums-landing-view {
      --player-height: 146px;
      --content-bottom-padding: 24px;
    }

    .home.album-detail-view {
      --player-height: 146px;
      --content-bottom-padding: 22px;
    }

    .home.artist-detail-view {
      --player-height: 146px;
      --content-bottom-padding: 22px;
    }

    .home.genre-detail-view {
      --player-height: 146px;
      --content-bottom-padding: 22px;
    }

    .home.songs-library-view {
      --player-height: 146px;
      --content-bottom-padding: 24px;
    }

    .home.playlists-view,
    .home.playlist-detail-view {
      --player-height: 146px;
      --content-bottom-padding: 24px;
    }

    .queue-backdrop {
      bottom: 146px;
    }

    .home.albums-landing-view .home-header-actions {
      width: 100%;
    }

    .home.albums-landing-view .home-header button {
      flex: 1 1 136px;
    }

    .video-session-card,
    .video-session-status {
      grid-template-columns: 1fr;
    }

    .video-session-thumb {
      width: 100%;
    }

    .cd-track-head {
      display: none;
    }

    .cd-track-row {
      grid-template-columns: 48px minmax(0, 1fr);
      gap: 8px 12px;
      align-items: start;
    }

    .dvd-title-row {
      grid-template-columns: 52px minmax(0, 1fr);
      gap: 6px 10px;
      align-items: start;
    }

    .dvd-title-row.head {
      display: none;
    }

    .dvd-title-row span:nth-child(n + 2),
    .dvd-title-row label {
      grid-column: 2;
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

    .album-detail-header {
      padding: 16px;
    }

    .playlist-card-main {
      grid-template-columns: 58px minmax(0, 1fr);
    }

    .album-detail-actions button {
      flex: 1 1 auto;
    }

    .lyrics-empty-shell {
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

    .album-track-row.with-play-counts {
      grid-template-columns: 32px minmax(0, 1fr) minmax(64px, auto) auto;
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

    .update-notice {
      right: 16px;
      bottom: 150px;
      left: 16px;
      display: grid;
      max-width: none;
    }

    .update-notice button {
      width: 100%;
    }

    .update-progress {
      grid-template-columns: 1fr;
    }
  }
</style>
