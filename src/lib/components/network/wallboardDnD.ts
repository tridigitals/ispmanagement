export type WallboardDragState = {
  dragFrom: number | null;
  dragOver: number | null;
  dragging: boolean;
};

export function swapWallboardSlots<T>(slotsAll: (T | null)[], from: number, to: number): (T | null)[] {
  if (from === to) return slotsAll;
  const next = [...slotsAll];
  const need = Math.max(from, to);
  if (next.length <= need) next.push(...Array.from({ length: need + 1 - next.length }, () => null));
  const a = next[from] ?? null;
  const b = next[to] ?? null;
  next[from] = b;
  next[to] = a;
  return next;
}

export function getWallboardHoverIndexFromPoint(doc: Document, x: number, y: number) {
  const target = doc.elementFromPoint(x, y) as HTMLElement | null;
  const bar = target?.closest?.('.bar') as HTMLElement | null;
  if (!bar) return null;
  const raw = bar.dataset?.idx;
  if (!raw) return null;
  const idx = Number.parseInt(raw, 10);
  return Number.isFinite(idx) && idx >= 0 ? idx : null;
}

export function getWallboardSlotIndexFromPoint(doc: Document, x: number, y: number) {
  const el = doc.elementFromPoint(x, y) as HTMLElement | null;
  const host = el?.closest?.('[data-wall-slot]') as HTMLElement | null;
  const raw = host?.dataset?.wallSlot;
  if (!raw) return null;
  const idx = Number.parseInt(raw, 10);
  return Number.isFinite(idx) && idx >= 0 ? idx : null;
}

function canStartDragFromTarget(target: HTMLElement | null) {
  if (!target) return false;
  return !target.closest('button, a, input, select, textarea, [role="menu"], .tile-menu');
}

export function createWallboardDnDController(args: {
  getDragState: () => WallboardDragState;
  setDragState: (state: WallboardDragState) => void;
  onSwapSlots: (from: number, to: number) => void;
}) {
  const onDragMove = (e: PointerEvent) => {
    const current = args.getDragState();
    if (!current.dragging || typeof document === 'undefined') return;
    const idx = getWallboardSlotIndexFromPoint(document, e.clientX, e.clientY);
    if (idx == null) return;
    args.setDragState({
      ...current,
      dragOver: idx,
    });
  };

  const onDragUp = () => {
    endDrag(true);
  };

  const onDragCancel = () => {
    endDrag(false);
  };

  const removeListeners = () => {
    if (typeof window === 'undefined') return;
    window.removeEventListener('pointermove', onDragMove as any);
    window.removeEventListener('pointerup', onDragUp as any);
    window.removeEventListener('pointercancel', onDragCancel as any);
  };

  const endDrag = (apply: boolean) => {
    const current = args.getDragState();
    if (apply && current.dragFrom != null && current.dragOver != null) {
      args.onSwapSlots(current.dragFrom, current.dragOver);
    }
    args.setDragState({
      dragFrom: null,
      dragOver: null,
      dragging: false,
    });
    if (typeof document !== 'undefined') document.body.classList.remove('wall-dragging');
    removeListeners();
  };

  const startDrag = (e: PointerEvent, idx: number) => {
    e.preventDefault();
    e.stopPropagation();
    args.setDragState({
      dragging: true,
      dragFrom: idx,
      dragOver: idx,
    });
    if (typeof document !== 'undefined') document.body.classList.add('wall-dragging');
    if (typeof window !== 'undefined') {
      window.addEventListener('pointermove', onDragMove as any);
      window.addEventListener('pointerup', onDragUp as any);
      window.addEventListener('pointercancel', onDragCancel as any);
    }
  };

  const startDragFromTile = (e: PointerEvent, idx: number) => {
    const target = e.target as HTMLElement | null;
    if (!canStartDragFromTarget(target)) return;
    startDrag(e, idx);
  };

  const getHoverIndexFromPoint = (x: number, y: number) => {
    if (typeof document === 'undefined') return null;
    return getWallboardHoverIndexFromPoint(document, x, y);
  };

  const dispose = () => {
    endDrag(false);
  };

  return {
    startDrag,
    startDragFromTile,
    endDrag,
    getHoverIndexFromPoint,
    dispose,
  };
}
