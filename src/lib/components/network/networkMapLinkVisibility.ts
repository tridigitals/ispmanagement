export function buildLinkLayerVisibilityState(args: {
  linksVisible: boolean;
  topologyAssetsVisible: boolean;
  linkPickMode: boolean;
  activeAssetConnectSourceId: string | null;
}) {
  const connectFocusActive =
    args.linkPickMode && String(args.activeAssetConnectSourceId || '').trim().length > 0;

  return {
    mainLinksVisible: args.linksVisible && !connectFocusActive,
    topologyAssetLinksVisible:
      args.linksVisible && args.topologyAssetsVisible && !connectFocusActive,
  };
}
