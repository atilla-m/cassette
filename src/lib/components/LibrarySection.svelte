<script lang="ts">
  import type { Snippet } from "svelte";

  type Props = {
    title: string;
    viewAllLabel?: string;
    onViewAll?: () => void;
    children: Snippet;
  };

  let { title, viewAllLabel = "View all", onViewAll, children }: Props = $props();
</script>

<section class="library-section" aria-labelledby={title.toLowerCase().replaceAll(" ", "-")}>
  <div class="section-header">
    <h2 id={title.toLowerCase().replaceAll(" ", "-")}>{title}</h2>
    {#if onViewAll}
      <button type="button" onclick={onViewAll}>{viewAllLabel}</button>
    {:else}
      <span class="section-label">{viewAllLabel}</span>
    {/if}
  </div>

  {@render children()}
</section>

<style>
  .library-section {
    display: grid;
    gap: 14px;
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  h2 {
    margin: 0;
    color: #f4f7fb;
    font-size: 1.08rem;
    line-height: 1.25;
  }

  button,
  .section-label {
    min-height: 32px;
    border: 1px solid #303844;
    border-radius: 8px;
    background: #161a20;
    color: #aeb7c4;
    cursor: default;
    font: inherit;
    font-size: 0.84rem;
    font-weight: 700;
    padding: 0 12px;
  }

  .section-label {
    display: inline-flex;
    align-items: center;
  }

  button:hover,
  button:focus-visible {
    border-color: #35544f;
    background: #1b2027;
    color: #d5dce5;
    outline: none;
  }
</style>
