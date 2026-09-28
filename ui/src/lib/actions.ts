/**
 * Closes a popover on outside click or Escape. Clicks on the element that
 * controls it (`aria-controls="<node id>"`) are ignored: that toggle handles
 * itself, otherwise it would close then immediately reopen the panel.
 */
export function dismissable(node: HTMLElement, onDismiss: () => void) {
  let handler = onDismiss;
  const onPointer = (e: PointerEvent) => {
    const target = e.target as Element;
    if (node.contains(target)) return;
    if (node.id && target.closest?.(`[aria-controls="${node.id}"]`)) return;
    handler();
  };
  const onKey = (e: KeyboardEvent) => {
    if (e.key === 'Escape') handler();
  };
  document.addEventListener('pointerdown', onPointer, true);
  document.addEventListener('keydown', onKey);
  return {
    update(next: () => void) {
      handler = next;
    },
    destroy() {
      document.removeEventListener('pointerdown', onPointer, true);
      document.removeEventListener('keydown', onKey);
    },
  };
}
