// Hover tips drawn by the panel itself (one bubble at the root, never clipped by a block),
// like the Mac app's HoverTip. Use as `use:tip={"text"}`.

export const tipState = $state<{ text: string | null; rect: DOMRect | null }>({ text: null, rect: null });

const DELAY_MS = 450;

export function tip(node: HTMLElement, text: string | null | undefined) {
  let current = text;
  let timer: ReturnType<typeof setTimeout> | undefined;
  const show = () => {
    if (!current) return;
    timer = setTimeout(() => {
      tipState.text = current ?? null;
      tipState.rect = node.getBoundingClientRect();
    }, DELAY_MS);
  };
  const hide = () => {
    clearTimeout(timer);
    if (tipState.rect && tipState.text === current) {
      tipState.text = null;
      tipState.rect = null;
    }
  };
  node.addEventListener("pointerenter", show);
  node.addEventListener("pointerleave", hide);
  node.addEventListener("pointerdown", hide);
  return {
    update(next: string | null | undefined) {
      if (tipState.text === current && tipState.rect) tipState.text = next ?? null;
      current = next;
    },
    destroy() {
      hide();
      node.removeEventListener("pointerenter", show);
      node.removeEventListener("pointerleave", hide);
      node.removeEventListener("pointerdown", hide);
    },
  };
}

/** Shows a tip right away for a resolved target, e.g. the hovered bar of a chart. */
export function showTip(text: string | null, rect: DOMRect | null) {
  tipState.text = text;
  tipState.rect = text ? rect : null;
}
