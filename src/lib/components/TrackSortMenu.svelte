<script lang="ts">
  import type { AlbumTrackSortDirection, AlbumTrackSortKey } from "$lib/utils/albumTrackSort";
  import { tick } from "svelte";

  type Option = { value: AlbumTrackSortKey; label: string };
  type Props = {
    value: AlbumTrackSortKey;
    direction: AlbumTrackSortDirection;
    options: Option[];
    onSortChange: (value: AlbumTrackSortKey) => void;
    onDirectionChange: (direction: AlbumTrackSortDirection) => void;
  };

  let { value, direction, options, onSortChange, onDirectionChange }: Props = $props();
  let isOpen = $state(false);
  let rootElement: HTMLDivElement | undefined = $state();
  let triggerElement: HTMLButtonElement | undefined = $state();
  let panelElement: HTMLDivElement | undefined = $state();
  let directionEnabled = $derived(value !== "mostPlayed" && value !== "leastPlayed");

  function closeMenu(restoreFocus = false) {
    isOpen = false;
    if (restoreFocus) triggerElement?.focus();
  }

  function handleOutsidePointer(event: PointerEvent) {
    if (isOpen && !rootElement?.contains(event.target as Node)) closeMenu();
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (isOpen && event.key === "Escape") {
      event.preventDefault();
      closeMenu(true);
    }
  }

  async function handleTriggerKeydown(event: KeyboardEvent) {
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    event.preventDefault();
    isOpen = true;
    await tick();
    const buttons = panelElement?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)");
    (event.key === "ArrowDown" ? buttons?.[0] : buttons?.[buttons.length - 1])?.focus();
  }

  function handlePanelKeydown(event: KeyboardEvent) {
    const buttons = Array.from(panelElement?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? []);
    const current = buttons.indexOf(document.activeElement as HTMLButtonElement);
    if (buttons.length === 0 || current < 0) return;
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      buttons[(current + (event.key === "ArrowDown" ? 1 : -1) + buttons.length) % buttons.length].focus();
    } else if (event.key === "Home" || event.key === "End") {
      event.preventDefault();
      (event.key === "Home" ? buttons[0] : buttons[buttons.length - 1]).focus();
    }
  }

  function handleFocusOut(event: FocusEvent) {
    if (isOpen && event.relatedTarget && !rootElement?.contains(event.relatedTarget as Node)) closeMenu();
  }
</script>

<svelte:window onpointerdown={handleOutsidePointer} onkeydown={handleWindowKeydown} />

<div bind:this={rootElement} class="track-sort-menu" onfocusout={handleFocusOut}>
  <button
    bind:this={triggerElement}
    type="button"
    class="sort-trigger"
    aria-haspopup="menu"
    aria-expanded={isOpen}
    onclick={() => isOpen = !isOpen}
    onkeydown={(event) => void handleTriggerKeydown(event)}
  >Sort songs <span aria-hidden="true">⌄</span></button>

  {#if isOpen}
    <div bind:this={panelElement} class="sort-panel" role="menu" aria-label="Sort displayed songs" tabindex="-1" onkeydown={handlePanelKeydown}>
      {#each options as option}
        <button
          type="button"
          role="menuitemradio"
          aria-checked={value === option.value}
          onclick={() => onSortChange(option.value)}
        >
          <span>{option.label}</span>
          {#if value === option.value}<span aria-hidden="true">✓</span>{/if}
        </button>
      {/each}
      <button
        type="button"
        role="menuitemcheckbox"
        aria-checked={direction === "desc"}
        disabled={!directionEnabled}
        onclick={() => onDirectionChange(direction === "asc" ? "desc" : "asc")}
      >
        <span>Descending order</span>
        <span class:checked={direction === "desc"} class="menu-switch" aria-hidden="true"><span></span></span>
      </button>
      <button class="close-menu" type="button" aria-label="Close sort menu" onclick={() => closeMenu(true)}>Close</button>
    </div>
  {/if}
</div>

<style>
  .track-sort-menu {
    position: relative;
    z-index: 5;
  }

  button {
    color: var(--text);
    font: inherit;
    cursor: pointer;
  }

  .sort-trigger {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    min-height: 34px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel-strong);
    font-size: 0.84rem;
    font-weight: 750;
    padding: 0 12px;
  }

  .sort-trigger:hover,
  .sort-trigger:focus-visible,
  .sort-trigger[aria-expanded="true"] {
    border-color: var(--accent-strong);
    background: var(--panel-hover);
    outline: none;
  }

  .sort-panel {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    display: grid;
    min-width: 216px;
    max-height: min(370px, 70vh);
    overflow-y: auto;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel);
    box-shadow: 0 18px 42px rgba(0, 0, 0, 0.36);
    padding: 6px;
  }

  .sort-panel > button {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    min-height: 34px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    font-size: 0.84rem;
    font-weight: 750;
    padding: 0 10px;
    text-align: left;
  }

  .sort-panel > button:hover:not(:disabled),
  .sort-panel > button:focus-visible {
    background: var(--panel-hover);
    outline: none;
  }

  .sort-panel > button:disabled {
    color: var(--text-dim);
    cursor: default;
  }

  .sort-panel .close-menu {
    justify-content: center;
    margin-top: 4px;
    border-top: 1px solid var(--border-strong);
    color: var(--text-muted);
  }

  .menu-switch {
    display: flex;
    width: 30px;
    height: 18px;
    flex: 0 0 auto;
    align-items: center;
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    background: var(--panel-strong);
    padding: 2px;
  }

  .menu-switch span {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--text-muted);
    transition: transform 140ms ease;
  }

  .menu-switch.checked {
    border-color: var(--accent-strong);
    background: var(--accent-soft);
  }

  .menu-switch.checked span { transform: translateX(11px); background: var(--accent-text); }

  @media (prefers-reduced-motion: reduce) {
    .menu-switch span { transition: none; }
  }
</style>
