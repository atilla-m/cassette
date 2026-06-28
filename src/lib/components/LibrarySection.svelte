<script lang="ts">
  import type { Snippet } from "svelte";

  type Props = {
    title: string;
    viewAllLabel?: string;
    children: Snippet;
  };

  let { title, viewAllLabel = "View all", children }: Props = $props();
</script>

<section class="library-section" aria-labelledby={title.toLowerCase().replaceAll(" ", "-")}>
  <div class="section-header">
    <h2 id={title.toLowerCase().replaceAll(" ", "-")}>{title}</h2>
    <button type="button">{viewAllLabel}</button>
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

  button {
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
</style>
