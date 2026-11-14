// Holding a block or right clicking it starts editing the panel (TileInteraction `startsEditing`).
// Pass `null` while already editing to switch it off. Use as `use:holdToEdit={fn}`.

const HOLD_MS = 450;
const SLOP = 6;

export function holdToEdit(node: HTMLElement, onHold: (() => void) | null) {
  let cb = onHold;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let sx = 0;
  let sy = 0;
  const cancel = () => clearTimeout(timer);
  const down = (e: PointerEvent) => {
    if (!cb || e.button !== 0 || (e.target as Element).closest("button, select, a")) return;
    sx = e.clientX;
    sy = e.clientY;
    timer = setTimeout(() => cb?.(), HOLD_MS);
  };
  const move = (e: PointerEvent) => {
    if (Math.hypot(e.clientX - sx, e.clientY - sy) > SLOP) cancel();
  };
  const menu = (e: MouseEvent) => {
    if (!cb) return;
    e.preventDefault();
    cb();
  };
  node.addEventListener("pointerdown", down);
  node.addEventListener("pointermove", move);
  node.addEventListener("pointerup", cancel);
  node.addEventListener("pointercancel", cancel);
  node.addEventListener("contextmenu", menu);
  return {
    update(next: (() => void) | null) {
      cb = next;
      if (!next) cancel();
    },
    destroy() {
      cancel();
      node.removeEventListener("pointerdown", down);
      node.removeEventListener("pointermove", move);
      node.removeEventListener("pointerup", cancel);
      node.removeEventListener("pointercancel", cancel);
      node.removeEventListener("contextmenu", menu);
    },
  };
}
