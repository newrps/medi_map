<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from 'svelte';
  import type { Facility } from '$lib/api';
  import { kindColor } from '$lib/api';

  export let facilities: Facility[] = [];
  export let openOnly = false;
  export let userPos: { lat: number; lon: number } | null = null;
  export let userAccuracy: number | null = null; // meters
  export let isOpenFn: (f: Facility) => boolean = () => true;

  const dispatch = createEventDispatcher<{
    select: Facility;
    bounds: { south: number; west: number; north: number; east: number; center: { lat: number; lon: number }; zoom: number };
    setUserPos: { lat: number; lon: number };
  }>();

  let mapEl: HTMLDivElement;
  let map: any;
  let L: any;
  let markersLayer: any;
  let userMarker: any = null;
  let userAccuracyCircle: any = null;
  export let pickMode = false; // true면 지도 클릭 시 내 위치로 설정

  onMount(async () => {
    L = (await import('leaflet')).default;
    await import('leaflet/dist/leaflet.css');

    map = L.map(mapEl, {
      center: [37.5665, 126.9780], // 서울 시청 기본
      zoom: 12,
      zoomControl: true
    });
    L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
      attribution: '© OpenStreetMap',
      subdomains: 'abc',
      maxZoom: 19
    }).addTo(map);

    markersLayer = L.layerGroup().addTo(map);

    // 화면 이동/줌 시 bounds 이벤트 발신 (debounce 200ms)
    let boundsTimer: any = null;
    const emitBounds = () => {
      const b = map.getBounds();
      const c = map.getCenter();
      dispatch('bounds', {
        south: b.getSouth(), west: b.getWest(),
        north: b.getNorth(), east: b.getEast(),
        center: { lat: c.lat, lon: c.lng },
        zoom: map.getZoom()
      });
    };
    map.on('moveend zoomend', () => {
      if (boundsTimer) clearTimeout(boundsTimer);
      boundsTimer = setTimeout(emitBounds, 200);
    });
    setTimeout(emitBounds, 50);

    // 길게 누르기(터치) / 우클릭 / 픽모드 클릭 — 위치 수동 설정
    map.on('contextmenu', (e: any) => {
      dispatch('setUserPos', { lat: e.latlng.lat, lon: e.latlng.lng });
    });
    map.on('click', (e: any) => {
      if (pickMode) {
        dispatch('setUserPos', { lat: e.latlng.lat, lon: e.latlng.lng });
      }
    });

    renderMarkers();
  });

  /** 외부에서 마커 위치로 이동/팝업 */
  export function focusFacility(f: Facility) {
    if (!map || f.lat == null || f.lon == null) return;
    map.flyTo([f.lat, f.lon], Math.max(map.getZoom(), 16), { duration: 0.6 });
  }

  onDestroy(() => { if (map) map.remove(); });

  function renderMarkers() {
    if (!map || !L) return;
    markersLayer.clearLayers();
    for (const f of facilities) {
      if (f.lat == null || f.lon == null) continue;
      if (openOnly && !isOpenFn(f)) continue;
      const open = isOpenFn(f);
      const m = L.circleMarker([f.lat, f.lon], {
        radius: 6,
        fillColor: kindColor(f.kind),
        color: open ? '#fff' : '#bbb',
        weight: 1.5,
        fillOpacity: open ? 0.95 : 0.45
      });
      m.bindTooltip(f.name, { direction: 'top', offset: [0, -6] });
      m.on('click', () => dispatch('select', f));
      m.addTo(markersLayer);
    }
    if (userPos) {
      if (userMarker) userMarker.remove();
      if (userAccuracyCircle) userAccuracyCircle.remove();
      userMarker = L.circleMarker([userPos.lat, userPos.lon], {
        radius: 8, fillColor: '#FFB300', color: '#fff', weight: 3, fillOpacity: 1
      }).addTo(map).bindTooltip('내 위치 (우클릭/길게눌러서 수정)', { direction: 'top' });
      // 정확도 원 (브라우저가 정확도 알려준 경우)
      if (userAccuracy && userAccuracy > 20) {
        userAccuracyCircle = L.circle([userPos.lat, userPos.lon], {
          radius: userAccuracy,
          color: '#FFB300', fillColor: '#FFB300', fillOpacity: 0.08,
          weight: 1, dashArray: '4 4'
        }).addTo(map);
      }
    }
  }

  let lastFlyTo: string = '';
  $: if (map && userPos) {
    const sig = `${userPos.lat.toFixed(4)},${userPos.lon.toFixed(4)}`;
    if (sig !== lastFlyTo) {
      lastFlyTo = sig;
      map.flyTo([userPos.lat, userPos.lon], 15, { duration: 0.8 });
    }
  }
  $: if (map && facilities) renderMarkers();
  $: if (map && userPos) renderMarkers();
  $: if (map) renderMarkers();
</script>

<div class="map" class:pick={pickMode} bind:this={mapEl}></div>

<style>
  .map { width: 100%; height: 100%; background: #cfe5f3; }
  .map.pick { cursor: crosshair; }
</style>
