<script lang="ts">
  import type { NavItem } from "$lib/types/library";

  type Props = {
    items: NavItem[];
    active?: string;
    onNavigate?: (label: string) => void;
  };

  let { items, active = "Home", onNavigate }: Props = $props();
</script>

<aside class="sidebar" aria-label="Primary navigation">
  <div class="brand">
    <div class="brand-mark" aria-hidden="true">C</div>
    <div>
      <p class="eyebrow">Local Library</p>
      <h1>Cassette</h1>
    </div>
  </div>

  <nav>
    {#each items as item}
      <button
        class:active={item.label === active}
        type="button"
        aria-current={item.label === active ? "page" : undefined}
        onclick={() => onNavigate?.(item.label)}
      >
        <span class="nav-icon" aria-hidden="true">{item.icon}</span>
        <span>{item.label}</span>
      </button>
    {/each}
  </nav>
</aside>

<style>
  .sidebar {
    width: 244px;
    min-width: 244px;
    border-right: 1px solid rgba(255, 255, 255, 0.08);
    background: var(--bg-soft);
    padding: 24px 16px;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 32px;
    padding: 0 8px;
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

  .eyebrow {
    margin: 0 0 2px;
    color: var(--text-soft);
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0;
    text-transform: uppercase;
  }

  h1 {
    margin: 0;
    color: var(--text);
    font-size: 1.35rem;
    line-height: 1.1;
  }

  nav {
    display: grid;
    gap: 6px;
  }

  button {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    min-height: 44px;
    border: 0;
    padding: 0 12px;
    border-radius: 8px;
    background: transparent;
    color: var(--text-muted);
    cursor: default;
    font: inherit;
    font-weight: 650;
  }

  button:hover,
  button.active {
    background: var(--panel-hover);
    color: var(--text);
  }

  button.active .nav-icon {
    background: var(--accent);
    color: var(--accent-contrast);
  }

  .nav-icon {
    display: grid;
    width: 28px;
    height: 28px;
    place-items: center;
    border-radius: 7px;
    background: var(--panel-strong);
    color: var(--text);
    font-size: 0.78rem;
    font-weight: 800;
  }

  @media (max-width: 760px) {
    .sidebar {
      width: 100%;
      min-width: 0;
      border-right: 0;
      border-bottom: 1px solid rgba(255, 255, 255, 0.08);
      padding: 16px;
    }

    .brand {
      margin-bottom: 16px;
    }

    nav {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }

    button {
      justify-content: center;
      min-height: 40px;
      padding: 0 8px;
      font-size: 0.9rem;
    }

    .nav-icon {
      display: none;
    }
  }
</style>
