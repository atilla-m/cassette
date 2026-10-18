<script lang="ts">
  type IconName = "albums" | "artists" | "genres" | "songs" | "playlists" | "disc" | "stats" | "settings";
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
  {/if}
</svg>
