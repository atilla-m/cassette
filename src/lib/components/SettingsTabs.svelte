<script module lang="ts">
  export type SettingsSection = "appearance" | "playback" | "library" | "history" | "updates" | "about";
</script>

<script lang="ts">
  let { active, onSelect }: { active: SettingsSection; onSelect: (section: SettingsSection) => void } = $props();

  const sections: { id: SettingsSection; label: string }[] = [
    { id: "appearance", label: "Appearance" },
    { id: "playback", label: "Playback" },
    { id: "library", label: "Library" },
    { id: "history", label: "Listening history" },
    { id: "updates", label: "Updates" },
    { id: "about", label: "About" },
  ];

  function handleKeydown(event: KeyboardEvent, index: number) {
    if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
    let next: number;
    switch (event.key) {
      case "ArrowRight": next = (index + 1) % sections.length; break;
      case "ArrowLeft": next = (index + sections.length - 1) % sections.length; break;
      case "Home": next = 0; break;
      case "End": next = sections.length - 1; break;
      default: return;
    }
    event.preventDefault();
    onSelect(sections[next].id);
    const tab = event.currentTarget as HTMLButtonElement;
    tab.parentElement?.querySelector<HTMLButtonElement>(`#settings-tab-${sections[next].id}`)?.focus();
  }
</script>

<div class="settings-tabs" role="tablist" aria-label="Settings sections" aria-orientation="horizontal">
  {#each sections as section, index (section.id)}
    <button
      id={`settings-tab-${section.id}`}
      type="button"
      role="tab"
      class:active={active === section.id}
      aria-selected={active === section.id}
      aria-controls={`settings-panel-${section.id}`}
      tabindex={active === section.id ? 0 : -1}
      onclick={() => onSelect(section.id)}
      onkeydown={(event) => handleKeydown(event, index)}
    >{section.label}</button>
  {/each}
</div>

<style>
  .settings-tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    border-bottom: 1px solid var(--border);
    padding-bottom: 10px;
  }

  button {
    min-height: 36px;
    border: 1px solid transparent;
    border-radius: 8px;
    background: transparent;
    color: var(--text-muted);
    font: inherit;
    font-size: 0.86rem;
    font-weight: 750;
    padding: 5px 12px;
    cursor: pointer;
  }

  button:hover,
  button.active {
    border-color: var(--border-strong);
    background: var(--panel-strong);
    color: var(--text);
  }

  button:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }
</style>
