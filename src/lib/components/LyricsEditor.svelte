<script lang="ts">
  import { onMount } from "svelte";
  import { parseLrcLine } from "$lib/utils/syncedLyrics";
  import type { TrackLyrics } from "$lib/types/library";

  let { title, initial, position, onSave, onClose }: {
    title: string;
    initial: Pick<TrackLyrics, "kind" | "text">;
    position: number | null;
    onSave: (kind: TrackLyrics["kind"], text: string) => Promise<void>;
    onClose: () => void;
  } = $props();

  let kind = $state<TrackLyrics["kind"]>("plain");
  let text = $state("");
  let error = $state("");
  let saving = $state(false);
  let dialog: HTMLDialogElement;
  let editor: HTMLTextAreaElement;

  onMount(() => {
    ({ kind, text } = initial);
    dialog.showModal();
    editor.focus();
  });

  function insertTimestamp() {
    if (position === null || position < 0 || position >= 6000) return;
    const centiseconds = Math.floor(position * 100);
    const minutes = String(Math.floor(centiseconds / 6000)).padStart(2, "0");
    const seconds = String(Math.floor(centiseconds / 100) % 60).padStart(2, "0");
    const fraction = String(centiseconds % 100).padStart(2, "0");
    const lineStart = editor.selectionStart === 0 ? 0 : text.lastIndexOf("\n", editor.selectionStart - 1) + 1;
    const previousStamp = text.slice(lineStart).match(/^\[\d{1,2}:\d{2}(?:\.\d{1,3})?\]/)?.[0];
    editor.setRangeText(`[${minutes}:${seconds}.${fraction}]`, lineStart, lineStart + (previousStamp?.length ?? 0), "end");
    text = editor.value;
    editor.focus();
  }

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (saving || !text.trim()) return;
    if (kind === "synced" && !text.split(/\r?\n/).some((line) => parseLrcLine(line).length > 0)) {
      error = "Add at least one LRC timestamp, such as [00:12.50].";
      return;
    }
    error = "";
    saving = true;
    try {
      await onSave(kind, text);
      dialog.close();
    } catch (problem) {
      error = problem instanceof Error ? problem.message : String(problem);
    } finally {
      saving = false;
    }
  }
</script>

<dialog bind:this={dialog} aria-labelledby="lyrics-editor-title" onclose={onClose}
  oncancel={(event) => { if (saving) event.preventDefault(); }}>
  <form onsubmit={submit}>
    <header>
      <h2 id="lyrics-editor-title">Edit lyrics</h2>
      <p>{title}</p>
    </header>
    <div class="toolbar">
      <label>Format
        <select bind:value={kind} disabled={saving}>
          <option value="plain">Plain lyrics</option>
          <option value="synced">Synced lyrics (LRC)</option>
        </select>
      </label>
      {#if kind === "synced"}
        <button type="button" onclick={insertTimestamp} disabled={saving || position === null || position >= 6000}>
          Stamp current line
        </button>
      {/if}
    </div>
    <textarea bind:this={editor} bind:value={text} aria-label="Lyrics text" spellcheck="false" disabled={saving}
      placeholder={kind === "synced" ? "[00:12.50] First line" : "Enter or paste lyrics"}></textarea>
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    <footer>
      <p>Edits are saved in Cassette’s library.</p>
      <button type="button" onclick={() => dialog.close()} disabled={saving}>Cancel</button>
      <button class="primary" type="submit" disabled={saving || !text.trim()}>{saving ? "Saving…" : "Save lyrics"}</button>
    </footer>
  </form>
</dialog>

<style>
  dialog {
    width: min(44rem, calc(100% - 2rem));
    max-height: calc(100% - 2rem);
    box-sizing: border-box;
    padding: clamp(1rem, 3vw, 1.75rem);
    border: 1px solid var(--border, #666);
    border-radius: 14px;
    background: Canvas;
    color: CanvasText;
  }
  dialog::backdrop { background: rgb(0 0 0 / 60%); }
  form { display: grid; gap: 1rem; min-width: 0; }
  header h2, p { margin: 0; }
  p { opacity: .8; overflow-wrap: anywhere; }
  .toolbar, footer { display: flex; flex-wrap: wrap; align-items: center; gap: .65rem; }
  footer p { margin-right: auto; font-size: .87rem; }
  label { display: flex; align-items: center; flex-wrap: wrap; gap: .65rem; }
  select, button, textarea {
    font: inherit;
    color: inherit;
    background: color-mix(in srgb, CanvasText 7%, Canvas);
    border: 1px solid color-mix(in srgb, CanvasText 22%, Canvas);
    border-radius: 8px;
    padding: .6rem .8rem;
  }
  button.primary { background: CanvasText; color: Canvas; font-weight: 700; }
  button:disabled { opacity: .5; }
  .error { color: var(--error, #e26363); }
  textarea { resize: vertical; min-height: min(14rem, 42vh); line-height: 1.55; }
</style>
