// Follows the Windows app mode (light / dark), like the Mac app follows its appearance.

const query = typeof window !== "undefined" ? window.matchMedia("(prefers-color-scheme: dark)") : null;

export const appearance = $state({ dark: query?.matches ?? false });

query?.addEventListener("change", (e) => (appearance.dark = e.matches));
