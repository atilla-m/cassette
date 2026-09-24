export type ViewHistory<T> = { entries: T[]; index: number };

export function visitView<T>(history: ViewHistory<T>, location: T, equals: (left: T, right: T) => boolean): ViewHistory<T> {
  if (equals(history.entries[history.index], location)) return history;

  return {
    entries: [...history.entries.slice(0, history.index + 1), location],
    index: history.index + 1,
  };
}

export function stepView<T>(history: ViewHistory<T>, direction: -1 | 1): ViewHistory<T> {
  const index = history.index + direction;
  return index >= 0 && index < history.entries.length ? { entries: history.entries, index } : history;
}
