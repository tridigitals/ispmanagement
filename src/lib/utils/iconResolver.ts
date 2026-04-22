const iconAliases: Record<string, string> = {
  alert: 'alert-circle',
  app: 'rocket',
  building: 'building-2',
  ban: 'circle-off',
  check: 'check-circle',
  chevron: 'chevron-right',
  clock: 'clock-3',
  dashboard: 'layout-dashboard',
  edit: 'square-pen',
  global: 'globe',
  key: 'key-round',
  layers: 'layers-3',
  link: 'link-2',
  logout: 'log-out',
  profile: 'user',
  'sidebar-toggle': 'panel-left',
  trash: 'trash-2',
  'x-circle': 'circle-x',
};

export function getLucideIconImportPath(name: string | null | undefined): string {
  if (!name) return 'help-circle';
  return iconAliases[name] || name;
}
