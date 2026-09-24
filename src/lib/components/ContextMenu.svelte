<script lang="ts">
  import { tick } from "svelte";

  type ContextMenuItem = {
    label: string;
    disabled?: boolean;
    action?: () => void | Promise<void>;
    items?: ContextMenuItem[];
    checked?: boolean;
    toggle?: boolean;
    radio?: boolean;
  };

  type Props = {
    x: number;
    y: number;
    items: ContextMenuItem[];
    onClose: () => void;
  };

  let { x, y, items, onClose }: Props = $props();
  let menuElement: HTMLDivElement | undefined = $state();
  let submenuElement: HTMLDivElement | undefined = $state();
  let submenuIndex = $state<number | null>(null);
  let submenuPosition = $state({ x: 0, y: 0 });
  let submenuTrigger: HTMLButtonElement | undefined;
  let submenuItems = $derived(submenuIndex === null ? [] : (items[submenuIndex]?.items ?? []));

  function handleWindowPointerDown(event: PointerEvent) {
    if (menuElement?.contains(event.target as Node) || submenuElement?.contains(event.target as Node)) {
      return;
    }

    onClose();
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      if (submenuIndex !== null) {
        submenuIndex = null;
        submenuTrigger?.focus();
        return;
      }
      onClose();
    }
  }

  function showSubmenu(index: number, button: HTMLButtonElement) {
    const childItems = items[index]?.items;
    if (!childItems) {
      submenuIndex = null;
      return;
    }

    const bounds = button.getBoundingClientRect();
    const width = 210;
    const margin = 8;
    const x = bounds.right + width + margin <= window.innerWidth
      ? bounds.right
      : bounds.left - width >= margin
        ? bounds.left - width
        : Math.max(margin, window.innerWidth - width - margin);
    const height = Math.min(window.innerHeight - margin * 2, childItems.length * 38 + 12);
    submenuPosition = {
      x,
      y: Math.max(margin, Math.min(bounds.top, window.innerHeight - height - margin)),
    };
    submenuTrigger = button;
    submenuIndex = index;
  }

  async function focusSubmenu() {
    await tick();
    submenuElement?.querySelector<HTMLButtonElement>("button:not(:disabled)")?.focus();
  }

  async function handleAction(item: ContextMenuItem) {
    if (item.disabled || !item.action) {
      return;
    }

    await item.action();
    if (item.toggle) {
      item.checked = !item.checked;
      return;
    }
    onClose();
  }
</script>

<svelte:window onpointerdown={handleWindowPointerDown} onkeydown={handleWindowKeydown} />

<div
  bind:this={menuElement}
  class="context-menu"
  style={`--menu-x: ${x}px; --menu-y: ${y}px;`}
  role="menu"
  tabindex="-1"
>
  {#each items as item, index}
    <button
      type="button"
      role="menuitem"
      disabled={item.disabled}
      aria-haspopup={item.items ? "menu" : undefined}
      aria-expanded={item.items ? submenuIndex === index : undefined}
      onpointerenter={(event) => showSubmenu(index, event.currentTarget)}
      onkeydown={(event) => {
        if (item.items && event.key === "ArrowRight") {
          event.preventDefault();
          showSubmenu(index, event.currentTarget);
          void focusSubmenu();
        }
      }}
      onclick={(event) => {
        if (item.items) {
          showSubmenu(index, event.currentTarget);
          void focusSubmenu();
        } else {
          void handleAction(item);
        }
      }}
    >
      <span>{item.label}</span>
      {#if item.items}<span aria-hidden="true">›</span>{/if}
    </button>
  {/each}
</div>

{#if submenuIndex !== null}
  <div
    bind:this={submenuElement}
    class="context-menu submenu"
    style={`--submenu-x: ${submenuPosition.x}px; --submenu-y: ${submenuPosition.y}px;`}
    role="menu"
    aria-label={items[submenuIndex].label}
    tabindex="-1"
    onkeydown={(event) => {
      if (event.key === "ArrowLeft") {
        event.preventDefault();
        submenuIndex = null;
        submenuTrigger?.focus();
      }
    }}
  >
    {#each submenuItems as item}
      <button
        type="button"
        role={item.toggle ? "menuitemcheckbox" : item.radio ? "menuitemradio" : "menuitem"}
        aria-checked={item.toggle || item.radio ? !!item.checked : undefined}
        disabled={item.disabled}
        onclick={() => void handleAction(item)}
      >
        <span>{item.label}</span>
        {#if item.toggle}
          <span class:checked={item.checked} class="menu-switch" aria-hidden="true"><span></span></span>
        {:else if item.checked}
          <span aria-hidden="true">✓</span>
        {/if}
      </button>
    {/each}
  </div>
{/if}

<style>
  .context-menu {
    position: fixed;
    z-index: 30;
    top: max(8px, min(var(--menu-y), calc(100vh - 280px)));
    left: max(8px, min(var(--menu-x), calc(100vw - 220px)));
    display: grid;
    width: 210px;
    overflow: hidden;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel);
    box-shadow: 0 18px 42px rgba(0, 0, 0, 0.36);
    padding: 6px;
  }

  .submenu {
    top: var(--submenu-y);
    left: var(--submenu-x);
    max-height: calc(100vh - 16px);
    overflow-y: auto;
  }

  button {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    min-height: 34px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--text);
    cursor: default;
    font: inherit;
    font-size: 0.88rem;
    font-weight: 750;
    padding: 0 10px;
    text-align: left;
  }

  button:hover,
  button:focus-visible {
    background: var(--panel-hover);
    color: var(--text);
    outline: none;
  }

  button:disabled {
    color: var(--text-dim);
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

  .menu-switch.checked span {
    background: var(--accent-text);
    transform: translateX(12px);
  }

  @media (prefers-reduced-motion: reduce) {
    .menu-switch span { transition: none; }
  }
</style>
