export function shouldSuppressPopupOnTargetPick(
  linkPickMode: boolean,
  targetKind: 'node' | 'router' | 'topology_asset' | 'link',
) {
  if (!linkPickMode) return false;
  return targetKind === 'node' || targetKind === 'router' || targetKind === 'topology_asset';
}
