export type ModernAppearance = "default";

export const MODERN_APPEARANCE_SETTING_KEY = "cassette:modern-appearance";
export const DEFAULT_MODERN_APPEARANCE: ModernAppearance = "default";

const MODERN_APPEARANCES: readonly ModernAppearance[] = [DEFAULT_MODERN_APPEARANCE];

export function isModernAppearance(value: string | null): value is ModernAppearance {
  return MODERN_APPEARANCES.some((appearance) => appearance === value);
}

export function resolveModernAppearance(value: string | null): ModernAppearance {
  return isModernAppearance(value) ? value : DEFAULT_MODERN_APPEARANCE;
}
