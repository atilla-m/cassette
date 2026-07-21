export type InterfaceMode = "modern" | "legacy";

export const INTERFACE_MODE_SETTING_KEY = "cassette:interface-mode";
export const DEFAULT_INTERFACE_MODE: InterfaceMode = "legacy";

export function isInterfaceMode(value: string | null): value is InterfaceMode {
  return value === "modern" || value === "legacy";
}

export function resolveInterfaceMode(
  storedValue: string | null,
  search: string,
): { mode: InterfaceMode; hasOverride: boolean } {
  const overrideValue = new URLSearchParams(search).get("interface");

  if (overrideValue !== null) {
    return {
      mode: isInterfaceMode(overrideValue) ? overrideValue : DEFAULT_INTERFACE_MODE,
      hasOverride: true,
    };
  }

  return {
    mode: isInterfaceMode(storedValue) ? storedValue : DEFAULT_INTERFACE_MODE,
    hasOverride: false,
  };
}
