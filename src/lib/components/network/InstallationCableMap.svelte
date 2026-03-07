<script lang="ts">
  import { createEventDispatcher, onDestroy, onMount } from 'svelte';
  import { api } from '$lib/api/client';
  import Icon from '$lib/components/ui/Icon.svelte';
  import { toast } from '$lib/stores/toast';
  import 'maplibre-gl/dist/maplibre-gl.css';

  type NMNode = {
    id: string;
    name: string;
    node_type: string;
    status: string;
    lat: number;
    lng: number;
  };

  type NMLink = {
    id: string;
    name: string;
    link_type: string;
    status: string;
    from_node_id?: string;
    to_node_id?: string;
    geometry: GeoJSON.Geometry;
  };

  const dispatch = createEventDispatcher<{
    saved: { link_id?: string; from_node_id: string; to_node_id: string };
    closed: {};
  }>();

  export let workOrderId: string;
  export let customerId: string;
  export let locationId: string;
  export let initialFromNodeId: string | null = null;

  let hostEl: HTMLDivElement | null = null;
  let map: import('maplibre-gl').Map | null = null;
  let maplibre: typeof import('maplibre-gl') | null = null;
  let loading = true;
  let saving = false;
  let drawing = false;
  let fromNodeId = '';
  let toNodeId = '';
  let linkName = '';
  let bendPoints: Array<[number, number]> = [];
  let nodes: NMNode[] = [];
  let links: NMLink[] = [];
  let customerCoord: [number, number] | null = null;
  let customerMarker: import('maplibre-gl').Marker | null = null;

  const SOURCE_NODES = 'icm-nodes';
  const SOURCE_LINKS = 'icm-links';
  const SOURCE_DRAFT = 'icm-draft';
  const SOURCE_DRAFT_POINTS = 'icm-draft-points';

  onMount(() => {
    void init();
  });

  onDestroy(() => {
    customerMarker?.remove();
    map?.remove();
  });

  async function init() {
    try {
      const mod = await import('maplibre-gl');
      maplibre = mod;
      if (!hostEl) return;

      map = new mod.Map({
        container: hostEl,
        style: {
          version: 8,
          glyphs: 'https://demotiles.maplibre.org/font/{fontstack}/{range}.pbf',
          sources: {
            osm: {
              type: 'raster',
              tiles: ['https://tile.openstreetmap.org/{z}/{x}/{y}.png'],
              tileSize: 256,
            },
          },
          layers: [{ id: 'base', type: 'raster', source: 'osm' }],
        },
        center: [106.8456, -6.2088],
        zoom: 11,
      });

      map.addControl(new mod.NavigationControl({ visualizePitch: true }), 'top-right');

      map.on('load', async () => {
        if (!map) return;
        map.addSource(SOURCE_LINKS, { type: 'geojson', data: emptyFC() });
        map.addSource(SOURCE_NODES, { type: 'geojson', data: emptyFC() });
        map.addSource(SOURCE_DRAFT, { type: 'geojson', data: emptyFC() });
        map.addSource(SOURCE_DRAFT_POINTS, { type: 'geojson', data: emptyFC() });

        map.addLayer({
          id: 'icm-links',
          type: 'line',
          source: SOURCE_LINKS,
          paint: { 'line-color': '#3f8cff', 'line-width': 2.5, 'line-opacity': 0.9 },
        });

        map.addLayer({
          id: 'icm-nodes',
          type: 'circle',
          source: SOURCE_NODES,
          paint: {
            'circle-radius': ['interpolate', ['linear'], ['zoom'], 9, 6, 14, 9],
            'circle-color': [
              'match',
              ['get', 'node_type'],
              'customer_endpoint',
              '#06b6d4',
              '#16a34a',
            ],
            'circle-stroke-color': '#e2e8f0',
            'circle-stroke-width': 1.4,
          },
        });

        map.addLayer({
          id: 'icm-draft',
          type: 'line',
          source: SOURCE_DRAFT,
          paint: {
            'line-color': '#38bdf8',
            'line-width': 3,
            'line-opacity': 0.95,
            'line-dasharray': [1.4, 1.2],
          },
        });

        map.addLayer({
          id: 'icm-draft-points',
          type: 'circle',
          source: SOURCE_DRAFT_POINTS,
          paint: {
            'circle-radius': 4.5,
            'circle-color': '#38bdf8',
            'circle-stroke-color': '#0b1020',
            'circle-stroke-width': 1.2,
          },
        });

        map.on('click', 'icm-nodes', (e) => {
          if (!drawing || !e.features?.[0]) return;
          const nodeId = String(e.features[0].properties?.id || '');
          if (!nodeId) return;
          if (!fromNodeId) {
            fromNodeId = nodeId;
            toast.info('Source node selected. Click map for path points, then destination node.');
            refreshDraft();
            return;
          }
          if (nodeId === fromNodeId) {
            toast.error('Destination node must be different.');
            return;
          }
          toNodeId = nodeId;
          const fromName = nodes.find((n) => n.id === fromNodeId)?.name || 'Source';
          const toName = nodes.find((n) => n.id === toNodeId)?.name || 'Destination';
          if (!linkName.trim()) linkName = `Cable ${fromName} → ${toName}`;
          refreshDraft();
          toast.success('Destination node selected. Click Save Link.');
        });

        map.on('click', (e) => {
          if (!drawing || !fromNodeId) return;
          const hitNode =
            map?.queryRenderedFeatures(e.point, {
              layers: ['icm-nodes'],
            }).length || 0;
          if (hitNode) return;
          bendPoints = [...bendPoints, [e.lngLat.lng, e.lngLat.lat]];
          refreshDraft();
        });

        await loadData();
      });
    } catch (e: any) {
      toast.error(e?.message || 'Failed to initialize cable map');
    } finally {
      loading = false;
    }
  }

  async function loadData() {
    if (!map || !maplibre) return;
    try {
      const [nodesRes, linksRes] = await Promise.all([
        api.networkMapping.nodes.list({ page: 1, per_page: 1000 }),
        api.networkMapping.links.list({ page: 1, per_page: 1000 }),
      ]);
      nodes = (nodesRes.data || []) as NMNode[];
      links = (linksRes.data || []) as NMLink[];

      if (!fromNodeId) {
        fromNodeId = initialFromNodeId || nodes[0]?.id || '';
      }

      setSource(SOURCE_NODES, {
        type: 'FeatureCollection',
        features: nodes
          .filter((n) => Number.isFinite(n.lng) && Number.isFinite(n.lat))
          .map((n) => ({
            type: 'Feature',
            geometry: { type: 'Point', coordinates: [n.lng, n.lat] },
            properties: { id: n.id, name: n.name, node_type: n.node_type, status: n.status },
          })),
      });
      setSource(SOURCE_LINKS, {
        type: 'FeatureCollection',
        features: links.map((l) => ({
          type: 'Feature',
          geometry: l.geometry,
          properties: { id: l.id, name: l.name, status: l.status, link_type: l.link_type },
        })),
      });

      await placeCustomerMarker();
      refreshDraft();
      fitToCustomerOrNetwork();
    } catch (e: any) {
      toast.error(e?.message || 'Failed to load map data');
    }
  }

  async function placeCustomerMarker() {
    if (!map || !maplibre || !customerId || !locationId) return;
    try {
      const locations = await api.customers.locations.list(customerId);
      const loc = (locations || []).find((x) => x.id === locationId);
      if (!loc || !Number.isFinite(loc.longitude) || !Number.isFinite(loc.latitude)) return;
      customerCoord = [Number(loc.longitude), Number(loc.latitude)];
      customerMarker?.remove();
      customerMarker = new maplibre.Marker({ color: '#06b6d4', scale: 1.12 })
        .setLngLat(customerCoord)
        .setPopup(
          new maplibre.Popup({ offset: 10 }).setHTML(
            `<div class="icm-popup"><strong>${escapeHtml(loc.label || 'Customer')}</strong></div>`,
          ),
        )
        .addTo(map);
    } catch {
      // ignore location fetch failures
    }
  }

  function fitToCustomerOrNetwork() {
    if (!map || !maplibre) return;
    if (customerCoord) {
      map.easeTo({ center: customerCoord, zoom: 16, duration: 420 });
      return;
    }
    const pts = nodes
      .filter((n) => Number.isFinite(n.lng) && Number.isFinite(n.lat))
      .map((n) => [n.lng, n.lat] as [number, number]);
    if (pts.length === 0) return;
    const bounds = pts.reduce(
      (acc, p) => acc.extend(p),
      new maplibre.LngLatBounds(pts[0], pts[0]),
    );
    map.fitBounds(bounds, {
      padding: { top: 48, right: 48, bottom: 48, left: 48 },
      maxZoom: 15,
      duration: 460,
    });
  }

  function toggleDraw() {
    drawing = !drawing;
    if (drawing) {
      toNodeId = '';
      bendPoints = [];
      refreshDraft();
      toast.info('Draw mode enabled. Select source node.');
    }
  }

  function cancelDraw() {
    drawing = false;
    toNodeId = '';
    bendPoints = [];
    refreshDraft();
  }

  function undoPoint() {
    if (bendPoints.length === 0) return;
    bendPoints = bendPoints.slice(0, -1);
    refreshDraft();
  }

  function getNodeCoord(nodeId: string): [number, number] | null {
    const n = nodes.find((x) => x.id === nodeId);
    if (!n) return null;
    if (!Number.isFinite(n.lng) || !Number.isFinite(n.lat)) return null;
    return [n.lng, n.lat];
  }

  function buildDraftLineCoords(): Array<[number, number]> {
    const from = getNodeCoord(fromNodeId);
    if (!from) return [];
    const coords: Array<[number, number]> = [from, ...bendPoints];
    if (toNodeId) {
      const to = getNodeCoord(toNodeId);
      if (to) coords.push(to);
    }
    return coords;
  }

  function refreshDraft() {
    const coords = buildDraftLineCoords();
    setSource(SOURCE_DRAFT, {
      type: 'FeatureCollection',
      features:
        coords.length >= 2
          ? [{ type: 'Feature', geometry: { type: 'LineString', coordinates: coords }, properties: {} }]
          : [],
    });
    setSource(SOURCE_DRAFT_POINTS, {
      type: 'FeatureCollection',
      features: bendPoints.map((p) => ({
        type: 'Feature',
        geometry: { type: 'Point', coordinates: p },
        properties: {},
      })),
    });
  }

  async function saveLink() {
    if (!fromNodeId || !toNodeId) {
      toast.error('Select source and destination node first.');
      return;
    }
    const coords = buildDraftLineCoords();
    if (coords.length < 2) {
      toast.error('Draw valid cable path first.');
      return;
    }
    saving = true;
    try {
      const payload = {
        name:
          linkName.trim() ||
          `Cable ${workOrderId ? `WO-${workOrderId.slice(0, 8)} ` : ''}${fromNodeId} → ${toNodeId}`,
        link_type: 'fiber',
        status: 'up',
        from_node_id: fromNodeId,
        to_node_id: toNodeId,
        priority: 100,
        capacity_mbps: null,
        utilization_pct: null,
        loss_db: null,
        latency_ms: null,
        geometry: { type: 'LineString', coordinates: coords },
      };
      const saved = await api.networkMapping.links.create(payload);
      toast.success('Cable route saved');
      dispatch('saved', { link_id: saved?.id, from_node_id: fromNodeId, to_node_id: toNodeId });
      await loadData();
      cancelDraw();
    } catch (e: any) {
      toast.error(e?.message || 'Failed to save cable route');
    } finally {
      saving = false;
    }
  }

  function setSource(sourceId: string, data: GeoJSON.FeatureCollection) {
    if (!map?.getSource(sourceId)) return;
    const src = map.getSource(sourceId) as import('maplibre-gl').GeoJSONSource | undefined;
    src?.setData(data as any);
  }

  function emptyFC(): GeoJSON.FeatureCollection {
    return { type: 'FeatureCollection', features: [] };
  }

  function escapeHtml(input: unknown): string {
    return String(input ?? '-')
      .replaceAll('&', '&amp;')
      .replaceAll('<', '&lt;')
      .replaceAll('>', '&gt;')
      .replaceAll('"', '&quot;')
      .replaceAll("'", '&#039;');
  }
</script>

<div class="icm-wrap">
  <div class="icm-toolbar">
    <button class="btn ghost mini" type="button" onclick={toggleDraw} disabled={loading || saving}>
      <Icon name="map-pin" size={14} />
      {drawing ? 'Drawing active' : 'Draw Cable'}
    </button>
    <button class="btn ghost mini" type="button" onclick={undoPoint} disabled={!drawing || bendPoints.length === 0 || saving}>
      <Icon name="arrow-left" size={14} />
      Undo
    </button>
    <button class="btn ghost mini" type="button" onclick={cancelDraw} disabled={!drawing || saving}>
      <Icon name="x-circle" size={14} />
      Cancel
    </button>
    <button class="btn mini" type="button" onclick={saveLink} disabled={saving || !drawing || !fromNodeId || !toNodeId}>
      <Icon name="save" size={14} />
      {saving ? 'Saving...' : 'Save Link'}
    </button>
  </div>

  <div class="icm-hints">
    {#if drawing}
      <span>Click source node → click map to add bends → click destination node → save.</span>
    {:else}
      <span>Start Draw Cable to create route.</span>
    {/if}
    {#if customerCoord}
      <span class="tag">Customer marker focused</span>
    {/if}
  </div>

  <div class="icm-map" bind:this={hostEl}></div>
</div>

<style>
  .icm-wrap {
    display: grid;
    gap: 8px;
  }
  .icm-toolbar {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: center;
  }
  .icm-hints {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    flex-wrap: wrap;
    font-size: 0.82rem;
    color: #9fb0cc;
  }
  .tag {
    border: 1px solid #245d79;
    color: #8fe3ff;
    background: rgba(6, 182, 212, 0.14);
    border-radius: 999px;
    padding: 2px 8px;
    font-weight: 700;
  }
  .icm-map {
    width: 100%;
    height: clamp(420px, 62vh, 720px);
    border: 1px solid #2d3f61;
    border-radius: 12px;
    overflow: hidden;
    background: #08101d;
  }
  :global(.icm-map .maplibregl-ctrl-group button) {
    width: 28px;
    height: 28px;
  }
</style>
