export function installWallboardAutoHideListeners(args: {
  showControls: () => void;
  onPointerDown?: (target: HTMLElement | null) => void;
  onEscape?: () => boolean;
  onToggleFocusMode?: () => void;
}) {
  if (typeof window === 'undefined') return null;

  const onAny = () => args.showControls();
  const onPointerDown = (e: PointerEvent) => {
    args.onPointerDown?.(e.target as HTMLElement | null);
    args.showControls();
  };
  const onKey = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      const handled = args.onEscape?.() ?? false;
      if (handled) return;
    }

    if (e.key.toLowerCase() === 'f' && !e.metaKey && !e.ctrlKey && !e.altKey) {
      const tag = (e.target as HTMLElement | null)?.tagName?.toLowerCase() || '';
      const editing = tag === 'input' || tag === 'textarea' || tag === 'select';
      if (!editing) {
        args.onToggleFocusMode?.();
        e.preventDefault();
      }
    }

    if (e.key === 'Escape') return;
    args.showControls();
  };

  window.addEventListener('mousemove', onAny, { passive: true });
  window.addEventListener('pointermove', onAny, { passive: true });
  window.addEventListener('pointerdown', onPointerDown, { passive: true });
  window.addEventListener('wheel', onAny, { passive: true });
  window.addEventListener('touchstart', onAny, { passive: true });
  window.addEventListener('keydown', onKey);

  return () => {
    window.removeEventListener('mousemove', onAny as any);
    window.removeEventListener('pointermove', onAny as any);
    window.removeEventListener('pointerdown', onPointerDown as any);
    window.removeEventListener('wheel', onAny as any);
    window.removeEventListener('touchstart', onAny as any);
    window.removeEventListener('keydown', onKey as any);
  };
}
