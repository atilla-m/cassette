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
    align-items: flex-end;
    justify-content: space-between;
    gap: clamp(28px, 4vw, 72px);
    min-height: 96px;
    margin-bottom: 26px;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 42%, transparent);
    padding: 4px 0 22px;
  }

  .modern-title-block {
    min-width: 210px;
    flex: 1 1 auto;
  }

  .modern-title-block p,
  .modern-title-block h2,
  .modern-title-block span {
    margin: 0;
  }

  .modern-title-block p {
    margin-bottom: 7px;
    color: var(--text-dim);
    font-size: 0.64rem;
    font-weight: 680;
    letter-spacing: 0.15em;
    text-transform: uppercase;
  }

  .modern-title-block h2 {
    color: var(--text);
    font-size: clamp(2rem, 2.8vw, 2.8rem);
    font-weight: 720;
    letter-spacing: -0.055em;
    line-height: 0.98;
  }

  .modern-title-block span {
    display: block;
    max-width: 660px;
    margin-top: 9px;
    overflow: hidden;
    color: var(--text-soft);
    font-size: 0.74rem;
    font-weight: 540;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .modern-command-area,
  .modern-topbar-actions {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .modern-command-area {
    justify-content: flex-end;
    flex: 0 1 720px;
    min-width: 0;
  }

  .modern-search {
    display: flex;
    position: relative;
    width: clamp(220px, 25vw, 380px);
    min-height: 42px;
    align-items: center;
    gap: 8px;
    border: 0;
    border-bottom: 1px solid var(--modern-input-border, var(--border));
    border-radius: 0;
    background: transparent;
    color: var(--text-soft);
    padding: 0 2px;
    transition: border-color 130ms ease;
  }

  .modern-search:focus-within {
    border-color: var(--accent-hover);
    box-shadow: 0 2px 0 color-mix(in srgb, var(--focus-ring) 72%, transparent);
  }

  .modern-search input {
    min-width: 0;
    flex: 1;
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 0.86rem;
    font-weight: 560;
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
    border-radius: 50%;
    background: transparent;
    color: var(--text-soft);
    font: inherit;
    font-size: 1rem;
  }

  .modern-topbar-actions button {
    min-height: 38px;
    border: 0;
    border-radius: 0;
    background: transparent;
    color: var(--text-muted);
    cursor: default;
    font: inherit;
    font-size: 0.78rem;
    font-weight: 650;
    padding: 0 6px;
  }

  .modern-topbar-actions button:hover:not(:disabled),
  .modern-topbar-actions button:focus-visible:not(:disabled) {
    background: transparent;
    color: var(--accent-text);
    outline: 2px solid var(--focus-ring);
    outline-offset: 3px;
  }

  .modern-topbar-actions button.primary {
    min-height: 42px;
    border-radius: 999px;
    background: var(--accent);
    color: var(--accent-contrast);
    font-weight: 760;
    padding: 0 17px;
  }

  .modern-topbar-actions button.primary:hover:not(:disabled),
  .modern-topbar-actions button.primary:focus-visible:not(:disabled) {
    background: var(--accent-hover);
    color: var(--accent-contrast);
  }

  .modern-topbar-actions button:disabled {
    color: var(--text-dim);
    opacity: 0.52;
  }

  @media (max-width: 1260px) {
    .modern-topbar {
      align-items: flex-start;
      flex-direction: column;
      gap: 18px;
      justify-content: flex-start;
    }

    .modern-command-area {
      width: 100%;
      flex: none;
      justify-content: space-between;
    }

    .modern-search {
      width: min(390px, 50%);
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

  @media (prefers-reduced-motion: reduce) {
    .modern-search {
      transition: none;
    }
  }
</style>
