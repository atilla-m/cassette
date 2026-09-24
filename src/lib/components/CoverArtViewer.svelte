<script lang="ts">
  import { onMount } from "svelte";
  import { fade, scale } from "svelte/transition";

  type Props = {
    source: string;
    title: string;
    opener: HTMLElement;
    fallbackFocus: HTMLElement | undefined;
    onClose: () => void;
  };

  let { source, title, opener, fallbackFocus, onClose }: Props = $props();
  let dialog: HTMLElement | undefined;
  let closeButton: HTMLButtonElement | undefined;
  let imageState = $state<"loading" | "loaded" | "error">("loading");
  const reducedMotion = typeof window !== "undefined"
    && window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  onMount(() => {
    closeButton?.focus({ preventScroll: true });
    return () => {
      const focusTarget = opener.isConnected ? opener : fallbackFocus;
      focusTarget?.focus({ preventScroll: true });
    };
  });

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      onClose();
    } else if (event.key === "Tab") {
      // The close button is the viewer's only focusable control.
      event.preventDefault();
      closeButton?.focus({ preventScroll: true });
    }
  }

  function handleFocusIn(event: FocusEvent) {
    if (event.target instanceof Node && !dialog?.contains(event.target)) {
      closeButton?.focus({ preventScroll: true });
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} onfocusin={handleFocusIn} />

<div
  class="cover-viewer-backdrop"
  role="presentation"
  onclick={(event) => { if (event.target === event.currentTarget) onClose(); }}
  transition:fade={{ duration: reducedMotion ? 0 : 170 }}
>
  <div
    bind:this={dialog}
    class="cover-viewer"
    role="dialog"
    aria-modal="true"
    aria-labelledby="cover-viewer-title"
    transition:scale={{ duration: reducedMotion ? 0 : 170, start: 0.97, opacity: 0 }}
  >
    <header>
      <h2 id="cover-viewer-title">{title} cover art</h2>
      <button bind:this={closeButton} type="button" aria-label="Close cover art viewer" onclick={onClose}>×</button>
    </header>
    <div class="cover-viewer-content" aria-live="polite">
      <img
        class:loaded={imageState === "loaded"}
        src={source}
        alt={`Full cover art for ${title}`}
        onload={() => { imageState = "loaded"; }}
        onerror={() => { imageState = "error"; }}
      />
      {#if imageState === "loading"}
        <p role="status">Loading cover art…</p>
      {:else if imageState === "error"}
        <p role="status">This cover art could not be opened.</p>
      {/if}
    </div>
  </div>
</div>

<style>
  .cover-viewer-backdrop {
    position: fixed;
    inset: 0;
    z-index: 110;
    display: grid;
    place-items: center;
    padding: 20px;
    background: rgba(0, 0, 0, 0.82);
  }

  .cover-viewer {
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: max-content;
    max-width: 100%;
    max-height: 100%;
    border: 1px solid var(--border-strong);
    border-radius: 12px;
    padding: 12px;
    background: var(--panel);
    color: var(--text);
    box-shadow: 0 24px 72px rgba(0, 0, 0, 0.5);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    min-height: 36px;
  }

  h2 {
    overflow: hidden;
    min-width: 0;
    margin: 0;
    color: var(--text);
    font-size: 0.95rem;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  button {
    flex: none;
    display: grid;
    width: 36px;
    height: 36px;
    place-items: center;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-soft);
    color: var(--text);
    cursor: pointer;
    font-size: 1.35rem;
    line-height: 1;
  }

  button:hover,
  button:focus-visible {
    border-color: var(--accent);
    color: var(--accent-text);
  }

  button:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }

  .cover-viewer-content {
    display: grid;
    max-width: calc(100vw - 66px);
    max-height: calc(100vh - 116px);
    min-width: min(240px, calc(100vw - 66px));
    min-height: 160px;
    place-items: center;
    overflow: hidden;
  }

  img {
    display: none;
    max-width: 100%;
    max-height: calc(100vh - 116px);
    width: auto;
    height: auto;
    object-fit: contain;
  }

  img.loaded {
    display: block;
  }

  p {
    margin: 0;
    color: var(--text-muted);
    text-align: center;
  }
</style>
