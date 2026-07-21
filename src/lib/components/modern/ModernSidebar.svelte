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
    {#each items as item}
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
    width: 224px;
    min-width: 224px;
    flex-direction: column;
    border-right: 1px solid color-mix(in srgb, var(--border) 72%, transparent);
    background: var(--modern-sidebar, var(--bg-soft));
    padding: 22px 14px 18px;
  }

  .modern-brand {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 48px;
    margin: 0 5px 26px;
  }

  .modern-brand-mark {
    position: relative;
    display: grid;
    width: 40px;
    height: 40px;
    flex: 0 0 auto;
    place-items: center;
    border-radius: 10px;
    background: var(--accent);
    box-shadow: 0 8px 24px color-mix(in srgb, var(--accent) 22%, transparent);
  }

  .modern-brand-mark::before {
    width: 21px;
    height: 14px;
    border: 2px solid var(--accent-contrast);
    border-radius: 4px;
    content: "";
  }

  .modern-brand-mark span {
    position: absolute;
    bottom: 10px;
    width: 5px;
    height: 5px;
    border: 1.5px solid var(--accent-contrast);
    border-radius: 50%;
  }

  .modern-brand-mark span:first-child {
    left: 11px;
  }

  .modern-brand-mark span:last-child {
    right: 11px;
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
    font-size: 1.08rem;
    font-weight: 850;
    letter-spacing: -0.02em;
  }

  .modern-brand-copy small {
    margin-top: 1px;
    color: var(--text-soft);
    font-size: 0.72rem;
    font-weight: 650;
  }

  nav {
    display: grid;
    gap: 3px;
  }

  nav button {
    position: relative;
    display: flex;
    align-items: center;
    gap: 11px;
    width: 100%;
    min-height: 42px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--text-soft);
    cursor: default;
    font: inherit;
    font-size: 0.88rem;
    font-weight: 720;
    padding: 0 10px;
    text-align: left;
  }

  nav button::before {
    position: absolute;
    left: 0;
    width: 3px;
    height: 18px;
    border-radius: 0 3px 3px 0;
    background: transparent;
    content: "";
  }

  nav button:hover,
  nav button:focus-visible {
    background: color-mix(in srgb, var(--modern-selected, var(--panel-hover)) 62%, transparent);
    color: var(--text);
    outline: none;
  }

  nav button:focus-visible {
    box-shadow: inset 0 0 0 2px var(--focus-ring);
  }

  nav button.active {
    background: var(--modern-selected, var(--panel-hover));
    color: var(--text);
  }

  nav button.active::before {
    background: var(--accent);
  }

  .modern-nav-mark {
    display: grid;
    width: 27px;
    height: 27px;
    flex: 0 0 auto;
    place-items: center;
    border-radius: 7px;
    background: color-mix(in srgb, var(--panel-strong) 74%, transparent);
    color: var(--text-muted);
    font-size: 0.7rem;
    font-weight: 900;
  }

  button.active .modern-nav-mark {
    background: color-mix(in srgb, var(--accent) 20%, var(--panel-strong));
    color: var(--accent-text);
  }

  .modern-sidebar-footer {
    display: grid;
    gap: 2px;
    margin-top: auto;
    padding: 14px 10px 0;
    color: var(--text-dim);
  }

  .modern-sidebar-footer span {
    color: var(--text-soft);
    font-size: 0.65rem;
    font-weight: 900;
    letter-spacing: 0.12em;
  }

  .modern-sidebar-footer small {
    font-size: 0.68rem;
    font-weight: 620;
  }

  @media (max-width: 1120px) {
    .modern-sidebar {
      width: 78px;
      min-width: 78px;
      padding-inline: 10px;
    }

    .modern-brand {
      justify-content: center;
      margin-inline: 0;
    }

    .modern-brand-copy,
    .modern-nav-label,
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
  }
</style>
