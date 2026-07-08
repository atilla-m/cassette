<script lang="ts">
  export type DropdownOption = {
    value: string;
    label: string;
  };

  type Props = {
    label: string;
    value: string;
    options: DropdownOption[];
    onChange: (value: string) => void;
  };

  let { label, value, options, onChange }: Props = $props();

  let isOpen = $state(false);
  let activeIndex = $state(0);
  let dropdownElement: HTMLDivElement | undefined = $state();

  let selectedOption = $derived(options.find((option) => option.value === value) ?? options[0]);

  function selectedIndex() {
    return Math.max(0, options.findIndex((option) => option.value === value));
  }

  function openDropdown() {
    activeIndex = selectedIndex();
    isOpen = true;
  }

  function closeDropdown() {
    isOpen = false;
  }

  function toggleDropdown() {
    if (isOpen) {
      closeDropdown();
      return;
    }

    openDropdown();
  }

  function chooseOption(option: DropdownOption) {
    onChange(option.value);
    closeDropdown();
  }

  function moveActiveOption(offset: number) {
    if (options.length === 0) {
      return;
    }

    activeIndex = (activeIndex + offset + options.length) % options.length;
  }

  function handleButtonKeydown(event: KeyboardEvent) {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();

      if (!isOpen) {
        openDropdown();
        return;
      }

      moveActiveOption(event.key === "ArrowDown" ? 1 : -1);
      return;
    }

    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();

      if (isOpen && options[activeIndex]) {
        chooseOption(options[activeIndex]);
        return;
      }

      openDropdown();
      return;
    }

    if (event.key === "Escape") {
      event.preventDefault();
      closeDropdown();
    }
  }

  function handleOptionKeydown(event: KeyboardEvent) {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      moveActiveOption(event.key === "ArrowDown" ? 1 : -1);
      return;
    }

    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();

      if (options[activeIndex]) {
        chooseOption(options[activeIndex]);
      }

      return;
    }

    if (event.key === "Escape") {
      event.preventDefault();
      closeDropdown();
    }
  }

  function handleWindowPointerDown(event: PointerEvent) {
    if (!isOpen || dropdownElement?.contains(event.target as Node)) {
      return;
    }

    closeDropdown();
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      closeDropdown();
    }
  }
</script>

<svelte:window onpointerdown={handleWindowPointerDown} onkeydown={handleWindowKeydown} />

<div bind:this={dropdownElement} class="compact-dropdown">
  <span class="compact-dropdown-label">{label}</span>
  <button
    class:open={isOpen}
    type="button"
    aria-haspopup="listbox"
    aria-expanded={isOpen}
    aria-label={`${label}: ${selectedOption?.label ?? ""}`}
    onclick={toggleDropdown}
    onkeydown={handleButtonKeydown}
  >
    <span>{selectedOption?.label ?? ""}</span>
    <span class="compact-dropdown-chevron" aria-hidden="true">v</span>
  </button>

  {#if isOpen}
    <div class="compact-dropdown-menu" role="listbox" aria-label={label}>
      {#each options as option, index}
        <button
          class:active={option.value === value}
          class:focused={index === activeIndex}
          type="button"
          role="option"
          aria-selected={option.value === value}
          tabindex={index === activeIndex ? 0 : -1}
          onclick={() => chooseOption(option)}
          onmouseenter={() => activeIndex = index}
          onkeydown={handleOptionKeydown}
        >
          {option.label}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .compact-dropdown {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
  }

  .compact-dropdown-label {
    color: var(--text-soft);
    font-size: 0.78rem;
    font-weight: 850;
  }

  .compact-dropdown > button {
    display: inline-grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 10px;
    min-width: 128px;
    min-height: 36px;
    border: 1px solid color-mix(in srgb, var(--border-strong) 86%, transparent);
    border-radius: 8px;
    background: color-mix(in srgb, var(--panel) 84%, transparent);
    color: var(--text);
    cursor: default;
    font: inherit;
    font-size: 0.84rem;
    font-weight: 800;
    padding: 0 10px 0 11px;
    text-align: left;
  }

  .compact-dropdown > button span:first-child {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .compact-dropdown > button:hover,
  .compact-dropdown > button:focus-visible,
  .compact-dropdown > button.open {
    border-color: var(--accent-strong);
    background: color-mix(in srgb, var(--panel-hover) 88%, transparent);
    outline: none;
  }

  .compact-dropdown > button.open {
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 16%, transparent);
  }

  .compact-dropdown-chevron {
    color: var(--text-soft);
    font-size: 0.66rem;
    font-weight: 950;
  }

  .compact-dropdown-menu {
    position: absolute;
    z-index: 45;
    top: calc(100% + 7px);
    left: 0;
    display: grid;
    width: max(100%, 190px);
    overflow: hidden;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel);
    box-shadow:
      0 18px 42px rgba(0, 0, 0, 0.34),
      inset 0 1px 0 rgba(255, 255, 255, 0.04);
    padding: 6px;
  }

  .compact-dropdown-menu button {
    min-height: 34px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--text-muted);
    cursor: default;
    font: inherit;
    font-size: 0.84rem;
    font-weight: 760;
    padding: 0 10px;
    text-align: left;
  }

  .compact-dropdown-menu button:hover,
  .compact-dropdown-menu button:focus-visible,
  .compact-dropdown-menu button.focused {
    background: color-mix(in srgb, var(--accent-soft) 62%, var(--panel-hover));
    color: var(--accent-text);
    outline: none;
  }

  .compact-dropdown-menu button.active {
    background: var(--accent-soft);
    color: var(--accent-text);
    box-shadow: inset 3px 0 0 var(--accent);
  }

  @media (max-width: 760px) {
    .compact-dropdown {
      width: 100%;
      justify-content: space-between;
    }

    .compact-dropdown > button {
      width: 100%;
      min-width: 0;
    }

    .compact-dropdown-menu {
      right: 0;
      width: 100%;
    }
  }
</style>
