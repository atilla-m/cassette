<script lang="ts">
  import type { NavItem } from "$lib/types/library";

  type Props = {
    items: NavItem[];
    active?: string;
  };

  let { items, active = "Home" }: Props = $props();
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
      <a class:active={item.label === active} href="/" aria-current={item.label === active ? "page" : undefined}>
        <span class="nav-icon" aria-hidden="true">{item.icon}</span>
        <span>{item.label}</span>
      </a>
    {/each}
  </nav>
</aside>

<style>
  .sidebar {
    width: 244px;
    min-width: 244px;
    border-right: 1px solid rgba(255, 255, 255, 0.08);
    background: #111317;
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
    background: #2f8f83;
    color: #07110f;
    font-weight: 800;
  }

  .eyebrow {
    margin: 0 0 2px;
    color: #8d96a3;
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0;
    text-transform: uppercase;
  }

  h1 {
    margin: 0;
    color: #f4f7fb;
    font-size: 1.35rem;
    line-height: 1.1;
  }

  nav {
    display: grid;
    gap: 6px;
  }

  a {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 44px;
    padding: 0 12px;
    border-radius: 8px;
    color: #b4bdc8;
    font-weight: 650;
    text-decoration: none;
  }

  a:hover,
  a.active {
    background: #1b2027;
    color: #ffffff;
  }

  a.active .nav-icon {
    background: #2f8f83;
    color: #07110f;
  }

  .nav-icon {
    display: grid;
    width: 28px;
    height: 28px;
    place-items: center;
    border-radius: 7px;
    background: #232a33;
    color: #d1d7df;
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

    a {
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
