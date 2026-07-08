<script lang="ts">
  type ContextMenuItem = {
    label: string;
    disabled?: boolean;
    action: () => void | Promise<void>;
  };

  type Props = {
    x: number;
    y: number;
    items: ContextMenuItem[];
    onClose: () => void;
  };

  let { x, y, items, onClose }: Props = $props();
  let menuElement: HTMLDivElement | undefined = $state();

  function handleWindowPointerDown(event: PointerEvent) {
    if (menuElement?.contains(event.target as Node)) {
      return;
    }

    onClose();
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      onClose();
    }
  }

  async function handleAction(item: ContextMenuItem) {
    if (item.disabled) {
      return;
    }

    await item.action();
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
  {#each items as item}
    <button
      type="button"
      role="menuitem"
      disabled={item.disabled}
      onclick={() => void handleAction(item)}
    >
      {item.label}
    </button>
  {/each}
</div>

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

  button {
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
</style>
