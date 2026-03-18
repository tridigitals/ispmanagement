type RouterLike = {
  id: string;
  name: string;
  host: string;
  port: number;
  identity?: string | null;
  is_online: boolean;
};

type InterfaceLike = {
  name: string;
  interface_type?: string | null;
  running: boolean;
  disabled: boolean;
};

type PickerSlotLike = {
  routerId: string;
};

export function openPickerState<T extends PickerSlotLike>(
  slots: (T | null)[],
  idx: number,
) {
  const current = slots[idx];
  return {
    pickerIndex: idx,
    pickerRouterSearch: '',
    pickerIfaceSearch: '',
    pickerRouterId: current?.routerId ?? null,
  };
}

export function closePickerState() {
  return {
    pickerIndex: null as number | null,
    pickerRouterId: null as string | null,
  };
}

export function filterPickerRouters(
  rows: RouterLike[],
  status: 'all' | 'offline' | 'online',
  query: string,
) {
  const q = query.trim().toLowerCase();
  return rows
    .filter((r) => {
      if (status === 'online') return !!r.is_online;
      if (status === 'offline') return !r.is_online;
      return true;
    })
    .filter((r) => {
      if (!q) return true;
      const hay = `${r.name} ${r.identity || ''} ${r.host}`.toLowerCase();
      return hay.includes(q);
    });
}

export function filterPickerInterfaces(
  ifaceCatalog: Record<string, InterfaceLike[]>,
  routerId: string | null,
  query: string,
) {
  if (!routerId) return [];
  const q = query.trim().toLowerCase();
  return (ifaceCatalog[routerId] || []).filter((i) => {
    if (!q) return true;
    return i.name.toLowerCase().includes(q) || (i.interface_type || '').toLowerCase().includes(q);
  });
}

export function findRouterById<T extends { id: string }>(rows: T[], id: string) {
  return rows.find((r) => r.id === id) || null;
}
