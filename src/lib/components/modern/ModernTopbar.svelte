<script lang="ts">
  type Props = {
    eyebrow: string;
    title: string;
    status: string;
    searchValue: string;
    searchPlaceholder?: string;
    showSearch?: boolean;
    isAlbumsLanding?: boolean;
    hasTracks?: boolean;
    hasAlbums?: boolean;
    isScanning?: boolean;
    scanLabel: string;
    onSearchInput: (value: string) => void;
    onSearchKeydown: (event: KeyboardEvent) => void;
    onClearSearch: () => void;
    onShuffleLibrary: () => void;
    onRandomAlbum: () => void;
    onScanLibrary: () => void;
  };

  let {
    eyebrow,
    title,
    status,
    searchValue,
    searchPlaceholder = "Search...",
    showSearch = false,
    isAlbumsLanding = false,
    hasTracks = false,
    hasAlbums = false,
    isScanning = false,
    scanLabel,
    onSearchInput,
    onSearchKeydown,
    onClearSearch,
    onShuffleLibrary,
    onRandomAlbum,
    onScanLibrary,
  }: Props = $props();

  function inputValue(event: Event) {
    return event.currentTarget instanceof HTMLInputElement ? event.currentTarget.value : "";
  }
</script>

<header class="modern-topbar">
  <div class="modern-title-block">
    <p>{eyebrow}</p>
    <h2>{title}</h2>
    <span>{status}</span>
  </div>

  <div class="modern-command-area">
    {#if showSearch}
      <label class="modern-search">
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
          <button type="button" aria-label="Clear search" onclick={onClearSearch}>×</button>
        {/if}
      </label>
    {/if}

    <div class="modern-topbar-actions">
      {#if isAlbumsLanding}
        <button type="button" disabled={!hasTracks} onclick={onShuffleLibrary}>Shuffle</button>
        <button type="button" disabled={!hasTracks || !hasAlbums} onclick={onRandomAlbum}>Random</button>
      {/if}
      <button class="primary" type="button" disabled={isScanning} onclick={onScanLibrary}>{scanLabel}</button>
    </div>
  </div>
</header>

<style>
  .modern-topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    min-height: 88px;
    margin-bottom: 22px;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 64%, transparent);
    padding: 0 0 18px;
  }

  .modern-title-block {
    min-width: 190px;
  }

  .modern-title-block p,
  .modern-title-block h2,
  .modern-title-block span {
    margin: 0;
  }

  .modern-title-block p {
    margin-bottom: 3px;
    color: var(--accent);
    font-size: 0.68rem;
    font-weight: 900;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }

  .modern-title-block h2 {
    color: var(--text);
    font-size: clamp(1.65rem, 2.2vw, 2.25rem);
    font-weight: 820;
    letter-spacing: -0.04em;
    line-height: 1.08;
  }

  .modern-title-block span {
    display: block;
    max-width: 660px;
    margin-top: 5px;
    overflow: hidden;
    color: var(--text-soft);
    font-size: 0.76rem;
    font-weight: 630;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .modern-command-area,
  .modern-topbar-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .modern-command-area {
    justify-content: flex-end;
    min-width: 0;
  }

  .modern-search {
    display: flex;
    width: clamp(210px, 23vw, 340px);
    min-height: 40px;
    align-items: center;
    gap: 8px;
    border: 1px solid var(--modern-input-border, color-mix(in srgb, var(--border-strong) 74%, transparent));
    border-radius: 8px;
    background: var(--modern-input-background, color-mix(in srgb, var(--modern-elevated, var(--panel-soft)) 88%, transparent));
    color: var(--text-soft);
    padding: 0 10px;
  }

  .modern-search:focus-within {
    border-color: var(--accent-strong);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--focus-ring) 55%, transparent);
  }

  .modern-search input {
    min-width: 0;
    flex: 1;
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 0.84rem;
    font-weight: 650;
  }

  .modern-search input::placeholder {
    color: var(--text-dim);
  }

  .modern-search button {
    display: grid;
    width: 24px;
    height: 24px;
    place-items: center;
    border: 0;
    border-radius: 5px;
    background: transparent;
    color: var(--text-soft);
    font: inherit;
    font-size: 1rem;
  }

  .modern-topbar-actions button {
    min-height: 40px;
    border: 1px solid color-mix(in srgb, var(--border-strong) 72%, transparent);
    border-radius: 8px;
    background: var(--modern-control-background, color-mix(in srgb, var(--panel-strong) 78%, transparent));
    color: var(--text-muted);
    cursor: default;
    font: inherit;
    font-size: 0.78rem;
    font-weight: 780;
    padding: 0 12px;
  }

  .modern-topbar-actions button:hover:not(:disabled),
  .modern-topbar-actions button:focus-visible:not(:disabled) {
    border-color: var(--accent-strong);
    background: var(--modern-control-hover, var(--panel-hover));
    color: var(--text);
    outline: none;
  }

  .modern-topbar-actions button.primary {
    border-color: color-mix(in srgb, var(--accent) 58%, var(--border));
    background: var(--accent);
    color: var(--accent-contrast);
  }

  .modern-topbar-actions button:disabled {
    color: var(--text-dim);
    opacity: 0.62;
  }

  @media (max-width: 1280px) {
    .modern-topbar {
      align-items: flex-start;
      flex-direction: column;
      gap: 14px;
    }

    .modern-command-area {
      width: 100%;
      justify-content: space-between;
    }

    .modern-search {
      width: min(360px, 48%);
    }
  }

  @media (max-width: 820px) {
    .modern-command-area {
      align-items: stretch;
      flex-direction: column;
    }

    .modern-search {
      width: 100%;
    }

    .modern-topbar-actions {
      flex-wrap: wrap;
    }
  }
</style>
