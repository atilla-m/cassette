<script lang="ts">
  type IconName = "albums" | "artists" | "genres" | "songs" | "playlists" | "disc" | "stats" | "settings" | "shuffle" | "previous" | "play" | "pause" | "next" | "repeat" | "repeat-one";
  let { name, size = 18 }: { name: IconName; size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">
  {#if name === "albums"}<rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="12" cy="12" r="5"/><circle cx="12" cy="12" r="1"/>
  {:else if name === "artists"}<circle cx="12" cy="8" r="4"/><path d="M4 21a8 8 0 0 1 16 0"/>
  {:else if name === "genres"}<path d="M4 4h9l7 7-9 9-7-7V4Z"/><circle cx="8" cy="8" r="1"/>
  {:else if name === "songs"}<path d="M9 18V5l11-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="17" cy="16" r="3"/>
  {:else if name === "playlists"}<path d="M4 6h12M4 11h12M4 16h8M17 15v6M17 15l5-1v6"/><circle cx="15" cy="21" r="2"/><circle cx="20" cy="20" r="2"/>
  {:else if name === "disc"}<circle cx="12" cy="12" r="9"/><circle cx="12" cy="12" r="2"/>
  {:else if name === "stats"}<path d="M4 20V11M10 20V5M16 20v-8M22 20v-5"/>
  {:else if name === "settings"}<circle cx="12" cy="12" r="3"/><path d="M12 2v2M12 20v2M2 12h2M20 12h2M5 5l1.5 1.5M17.5 17.5 19 19M19 5l-1.5 1.5M6.5 17.5 5 19"/>
  {:else if name === "shuffle"}<path d="M3 6h3c5 0 7 12 12 12h3M18 15l3 3-3 3M3 18h3c2 0 3-2 4-4M14 9c1-2 2-3 4-3h3M18 3l3 3-3 3"/>
  {:else if name === "previous"}<path d="M5 5v14M19 5l-11 7 11 7V5Z"/>
  {:else if name === "play"}<path d="M7 4l13 8-13 8V4Z"/>
  {:else if name === "pause"}<path d="M8 5v14M16 5v14"/>
  {:else if name === "next"}<path d="M19 5v14M5 5l11 7-11 7V5Z"/>
  {:else if name === "repeat-one"}<path d="M4 8V6a2 2 0 0 1 2-2h12l3 3-3 3M20 16v2a2 2 0 0 1-2 2H6l-3-3 3-3M12 9v6M10 11l2-2"/>
  {:else}<path d="M4 8V6a2 2 0 0 1 2-2h12l3 3-3 3M20 16v2a2 2 0 0 1-2 2H6l-3-3 3-3"/>{/if}
</svg>
