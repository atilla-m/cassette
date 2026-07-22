<script lang="ts">
  import type { NavItem } from "$lib/types/library";

  type Props = {
    items: NavItem[];
    active: string;
    onNavigate: (label: string) => void;
  };

  let { items, active, onNavigate }: Props = $props();
</script>

<aside class="modern-sidebar" aria-label="Primary navigation">
  <div class="modern-brand">
    <span class="modern-brand-mark" aria-hidden="true">
      <span></span>
      <span></span>
    </span>
    <span class="modern-brand-copy">
      <strong>Cassette</strong>
      <small>Local music library</small>
    </span>
  </div>

  <nav aria-label="Library and tools">
    <span class="modern-nav-section">Library</span>
    {#each items as item}
      {#if item.label === "CD Rip"}
        <span class="modern-nav-section tools">Tools</span>
      {/if}
      <button
        class:active={item.label === active}
        type="button"
        aria-current={item.label === active ? "page" : undefined}
        aria-label={item.label}
        title={item.label}
        onclick={() => onNavigate(item.label)}
      >
        <span class="modern-nav-mark" aria-hidden="true">{item.icon}</span>
        <span class="modern-nav-label">{item.label}</span>
      </button>
    {/each}
  </nav>

  <div class="modern-sidebar-footer">
    <span>LOCAL</span>
    <small>Nothing leaves this device</small>
  </div>
</aside>

<style>
  .modern-sidebar {
    display: flex;
    width: 202px;
    min-width: 202px;
    flex-direction: column;
    border-right: 1px solid color-mix(in srgb, var(--border) 52%, transparent);
    background: var(--modern-sidebar, var(--bg-soft));
    padding: 26px 18px 20px;
  }

  .modern-brand {
    display: flex;
    align-items: center;
    gap: 11px;
    min-height: 44px;
    margin: 0 1px 38px;
  }

  .modern-brand-mark {
    position: relative;
    display: grid;
    width: 34px;
    height: 34px;
    flex: 0 0 auto;
    place-items: center;
    border: 1px solid color-mix(in srgb, var(--accent) 58%, var(--border));
    border-radius: 50%;
    background: color-mix(in srgb, var(--accent-soft) 44%, transparent);
  }

  .modern-brand-mark::before {
    width: 18px;
    height: 12px;
    border: 1.5px solid var(--accent-text);
    border-radius: 3px;
    content: "";
  }

  .modern-brand-mark span {
    position: absolute;
    bottom: 9px;
    width: 4px;
    height: 4px;
    border: 1px solid var(--accent-text);
    border-radius: 50%;
  }

  .modern-brand-mark span:first-child {
    left: 9px;
  }

  .modern-brand-mark span:last-child {
    right: 9px;
  }

  .modern-brand-copy {
    min-width: 0;
  }

  .modern-brand-copy strong,
  .modern-brand-copy small {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .modern-brand-copy strong {
    color: var(--text);
    font-size: 1.03rem;
    font-weight: 760;
    letter-spacing: -0.025em;
  }

  .modern-brand-copy small {
    margin-top: 2px;
    color: var(--text-dim);
    font-size: 0.68rem;
    font-weight: 560;
  }

  nav {
    display: grid;
    gap: 1px;
  }

  .modern-nav-section {
    margin: 0 0 9px 16px;
    color: var(--text-dim);
    font-size: 0.62rem;
    font-weight: 680;
    letter-spacing: 0.13em;
    text-transform: uppercase;
  }

  .modern-nav-section.tools {
    margin-top: 22px;
  }

  nav button {
    position: relative;
    display: flex;
    align-items: center;
    gap: 13px;
    width: 100%;
    min-height: 39px;
    border: 0;
    border-radius: 0;
    background: transparent;
    color: var(--text-soft);
    cursor: default;
    font: inherit;
    font-size: 0.84rem;
    font-weight: 590;
    padding: 0 7px 0 15px;
    text-align: left;
  }

  nav button::before {
    position: absolute;
    left: 0;
    width: 2px;
    height: 22px;
    border-radius: 2px;
    background: transparent;
    content: "";
  }

  nav button:hover,
  nav button:focus-visible {
    background: linear-gradient(90deg, color-mix(in srgb, var(--modern-selected, var(--panel-hover)) 52%, transparent), transparent 88%);
    color: var(--text);
    outline: none;
  }

  nav button:focus-visible {
    box-shadow: inset 0 0 0 2px color-mix(in srgb, var(--focus-ring) 72%, transparent);
  }

  nav button.active {
    background: linear-gradient(90deg, color-mix(in srgb, var(--modern-selected, var(--panel-hover)) 72%, transparent), transparent 92%);
    color: var(--accent-text);
    font-weight: 720;
  }

  nav button.active::before {
    background: var(--accent);
  }

  .modern-nav-mark {
    display: grid;
    width: 8px;
    height: 8px;
    flex: 0 0 auto;
    place-items: center;
    overflow: hidden;
    border: 1px solid var(--text-dim);
    border-radius: 50%;
    background: transparent;
    color: transparent;
    font-size: 0;
  }

  button.active .modern-nav-mark {
    border-color: var(--accent);
    background: var(--accent);
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent) 11%, transparent);
  }

  .modern-sidebar-footer {
    display: grid;
    gap: 2px;
    margin-top: auto;
    padding: 14px 1px 0;
    color: var(--text-dim);
  }

  .modern-sidebar-footer span {
    color: var(--text-muted);
    font-size: 0.65rem;
    font-weight: 720;
    letter-spacing: 0.12em;
  }

  .modern-sidebar-footer small {
    font-size: 0.68rem;
    font-weight: 620;
  }

  @media (max-width: 1120px) {
    .modern-sidebar {
    width: 70px;
    min-width: 70px;
    padding-inline: 11px;
    }

    .modern-brand {
      justify-content: center;
      margin-inline: 0;
    }

    .modern-brand-copy,
    .modern-nav-label,
    .modern-nav-section,
    .modern-sidebar-footer {
      position: absolute;
      width: 1px;
      height: 1px;
      overflow: hidden;
      clip: rect(0 0 0 0);
      clip-path: inset(50%);
      white-space: nowrap;
    }

    nav button {
      justify-content: center;
      padding: 0;
    }

    .modern-nav-mark {
      width: 28px;
      height: 28px;
      border: 0;
      background: transparent;
      color: var(--text-soft);
      font-size: 0.66rem;
      font-weight: 720;
    }

    button.active .modern-nav-mark {
      border: 0;
      background: transparent;
      box-shadow: none;
      color: var(--accent-text);
    }
  }
</style>
