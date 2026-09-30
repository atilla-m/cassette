export function sortMenuPlacement(top: number, bottom: number, contentTop: number, contentBottom: number, optionCount: number) {
  const below = contentBottom - bottom - 12;
  const above = top - contentTop - 12;
  const opensUp = below < Math.min(370, (optionCount + 2) * 34 + 16) && above > below;
  return { opensUp, availableHeight: Math.max(90, Math.min(370, opensUp ? above : below)) };
}
