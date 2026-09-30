import type { ListeningImportResult } from "$lib/types/listening";

export function importCompletionHeading(result: ListeningImportResult): string {
  if (result.importedEvents === 0 && result.importedUndatedPlays === 0) {
    return "Import completed — no new history added.";
  }
  return `Import completed — ${result.importedEvents} events added.`;
}
