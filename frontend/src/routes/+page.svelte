<script lang="ts">
  import { onMount } from 'svelte';
  import MapView from '$lib/components/Map.svelte';
  import FacilityDetail from '$lib/components/FacilityDetail.svelte';
  import NearbyList from '$lib/components/NearbyList.svelte';
  import { fetchHospitals, fetchPharmacies, fetchAEDs, fetchHolidayClinics, isOpenNow, distanceKm } from '$lib/api';
  import type { Facility, HolidayClinic } from '$lib/api';

  let hospitals: Facility[] = [];
  let pharmacies: Facility[] = [];
  let aeds: Facility[] = [];
  let holidayClinics: HolidayClinic[] = [];
  let holidayMap: Map<string, HolidayClinic> = new Map();
  let loading = true;
  let loadError: string | null = null;
  let selected: Facility | null = null;

  let now = new Date();
  let userPos: { lat: number; lon: number } | null = null;
  let userAccuracy: number | null = null;
  let userPosError: string | null = null;
  let pickMode = false; // 지도 클릭으로 위치 수정 모드
  let showHospitals = true;
  let showPharmacies = true;
  let showAEDs = false;
  let mapZoom = 7;
  const AED_MIN_ZOOM = 14; // 양 많아 가까이 줌해야 보임
  // 내 위치 기반 반경 (km). 위치 없을 때만 전국 모두 표시.
  let radiusKm = 2;
  // 마커 너무 많으면 가까운 N개만
  const MAX_MARKERS = 800;

  // 화면 영역 (Map에서 dispatch)
  let bounds: { south: number; west: number; north: number; east: number; center: { lat: number; lon: number }; zoom: number } | null = null;
  $: if (bounds) mapZoom = bounds.zoom;
  let listCollapsed = true; // 기본 접힘 (지도 가리지 않게)
  let focusTarget: { lat: number; lon: number; ts: number } | null = null;

  // 1분마다 갱신
  setInterval(() => (now = new Date()), 60_000);

  onMount(async () => {
    try {
      const [h, p, a, hd] = await Promise.allSettled([
        fetchHospitals(), fetchPharmacies(), fetchAEDs(), fetchHolidayClinics()
      ]);
      if (h.status === 'fulfilled') hospitals = h.value;
      if (p.status === 'fulfilled') pharmacies = p.value;
      if (a.status === 'fulfilled') aeds = a.value;
      if (hd.status === 'fulfilled') {
        holidayClinics = hd.value;
        holidayMap = new Map(holidayClinics.map(x => [x.hpid, x]));
      }
      if (h.status === 'rejected' && p.status === 'rejected') {
        loadError = String(h.reason);
      }
    } finally {
      loading = false;
    }
    // 위치 권한 요청
    requestLocation();
  });

  function requestLocation() {
    userPosError = null;
    if (!navigator.geolocation) {
      userPosError = '브라우저가 위치 미지원';
      return;
    }
    navigator.geolocation.getCurrentPosition(
      (pos) => {
        userPos = { lat: pos.coords.latitude, lon: pos.coords.longitude };
        userAccuracy = pos.coords.accuracy ?? null;
      },
      (err) => {
        userPosError = err.code === 1 ? '권한 거부됨' : err.code === 2 ? '위치 알 수 없음' : '시간 초과';
      },
      { timeout: 10000, enableHighAccuracy: true, maximumAge: 0 }
    );
  }

  function setManualPos(p: { lat: number; lon: number }) {
    userPos = p;
    userAccuracy = null; // 수동 설정 = 정확도 원 안 보임
    pickMode = false;
  }

  $: visible = (() => {
    let list: Facility[] = [];
    if (showHospitals) list = list.concat(hospitals);
    if (showPharmacies) list = list.concat(pharmacies);
    // AED는 줌 14 이상일 때만 (전국 단위면 6만 개라 마비)
    if (showAEDs && mapZoom >= AED_MIN_ZOOM) list = list.concat(aeds);
    if (userPos) {
      const u = userPos;
      // 거리 계산 + 반경 필터
      const withDist = list
        .filter((f) => f.lat != null && f.lon != null)
        .map((f) => ({ f, d: distanceKm(u, { lat: f.lat!, lon: f.lon! }) }));
      const within = withDist.filter((x) => x.d <= radiusKm);
      // 반경 내가 너무 적으면 가까운 50개만 (반경 fallback)
      const items = within.length >= 5
        ? within.sort((a, b) => a.d - b.d)
        : withDist.sort((a, b) => a.d - b.d).slice(0, 50);
      list = items.slice(0, MAX_MARKERS).map((x) => x.f);
    } else {
      // 위치 없으면 전국 → 마커 수 제한 (응급실 우선)
      const er = list.filter((f) => f.kind === 'Hospital');
      const ph = list.filter((f) => f.kind === 'Pharmacy');
      list = [...er, ...ph.slice(0, Math.max(0, MAX_MARKERS - er.length))];
    }
    return list;
  })();

  function isOpen(f: Facility) { return isOpenNow(f, now); }

  // 명절 시즌 감지 — 오늘 또는 ±5일 안에 명절 진료 날짜가 있나
  $: holidaySeason = (() => {
    if (holidayClinics.length === 0) return null;
    const todayStr = new Date().toISOString().slice(0, 10);
    const allDays = new Set<string>();
    for (const h of holidayClinics) for (const d of h.days) allDays.add(d);
    if (allDays.has(todayStr)) return { active: true, days: [...allDays].sort() };
    // 미래 5일 안
    const todayMs = new Date(todayStr).getTime();
    for (const d of allDays) {
      const diff = (new Date(d).getTime() - todayMs) / (24 * 3600 * 1000);
      if (diff > 0 && diff <= 5) return { active: false, days: [...allDays].sort() };
    }
    return null;
  })();

  // 화면(bounds) 안에 들어오는 시설만
  $: visibleInView = (() => {
    if (!bounds) return [];
    const { south, west, north, east } = bounds;
    return visible.filter((f) => {
      if (f.lat == null || f.lon == null) return false;
      return f.lat >= south && f.lat <= north && f.lon >= west && f.lon <= east;
    });
  })();

  function pickFacility(f: Facility) {
    selected = f;
    if (f.lat != null && f.lon != null) {
      focusTarget = { lat: f.lat, lon: f.lon, ts: Date.now() };
    }
  }
</script>

<main class="full-map">
  <MapView
    facilities={visible}
    {userPos}
    {userAccuracy}
    {pickMode}
    {focusTarget}
    on:select={(e) => (selected = e.detail)}
    on:bounds={(e) => (bounds = e.detail)}
    on:setUserPos={(e) => setManualPos(e.detail)}
  />

  <NearbyList
    facilities={visibleInView}
    {userPos}
    mapCenter={bounds?.center ?? null}
    {now}
    collapsed={listCollapsed}
    on:pick={(e) => pickFacility(e.detail)}
    on:toggle={() => (listCollapsed = !listCollapsed)}
  />

  <div class="overlay top-left">
    <div class="brand">
      <svg viewBox="0 0 40 40" class="logo" aria-hidden="true">
        <circle cx="20" cy="20" r="19" fill="#D32F2F" />
        <rect x="17" y="9" width="6" height="22" fill="#fff" />
        <rect x="9" y="17" width="22" height="6" fill="#fff" />
      </svg>
      <h1>Ps의료지도</h1>
    </div>

    <div class="filter-row">
      <button class="chip" class:active={showHospitals} on:click={() => (showHospitals = !showHospitals)}>
        🏥 병원·응급실
      </button>
      <button class="chip" class:active={showPharmacies} on:click={() => (showPharmacies = !showPharmacies)}>
        💊 약국
      </button>
      <button class="chip" class:active={showAEDs} on:click={() => (showAEDs = !showAEDs)}
        title={mapZoom < AED_MIN_ZOOM ? `AED는 줌 ${AED_MIN_ZOOM} 이상에서만 표시` : ''}>
        ⚡ AED {#if showAEDs && mapZoom < AED_MIN_ZOOM}<span class="zoom-hint">(줌하세요)</span>{/if}
      </button>
    </div>

    {#if !userPos}
      <button class="loc-cta" on:click={requestLocation}>
        📍 내 위치 사용
        {#if userPosError}<span class="err">({userPosError})</span>{/if}
      </button>
    {/if}
  </div>

  {#if loading}
    <div class="overlay center loading">의료기관 데이터 불러오는 중…</div>
  {/if}

  {#if loadError}
    <div class="overlay bottom-center error">⚠️ {loadError}</div>
  {/if}

  {#if holidaySeason}
    <div class="overlay bottom-center holiday-banner">
      🌕 {holidaySeason.active ? '오늘 명절 비상진료 운영 중' : '곧 명절 비상진료 시작'}
      ({holidaySeason.days.join(', ')})
    </div>
  {/if}

  {#if selected}
    <FacilityDetail
      facility={selected}
      holiday={holidayMap.get(selected.hpid) ?? null}
      {userPos} {now}
      on:close={() => (selected = null)}
    />
  {/if}
</main>

<style>
  .full-map { position: fixed; inset: 0; overflow: hidden; }
  :global(.full-map > .map) { position: absolute; inset: 0; width: 100%; height: 100%; }

  .overlay { position: absolute; z-index: 500; pointer-events: none; }
  .overlay > * { pointer-events: auto; }

  .top-left {
    top: 16px; left: 16px;
    display: flex; flex-direction: column; gap: 10px;
    max-width: calc(100vw - 32px);
  }

  .brand {
    background: rgba(255,255,255,0.95); backdrop-filter: blur(10px);
    border-radius: 12px; padding: 8px 14px;
    display: inline-flex; align-items: center; gap: 10px;
    box-shadow: 0 2px 12px rgba(0,0,0,0.12);
    width: fit-content;
  }
  .logo { width: 30px; height: 30px; }
  .brand h1 { margin: 0; font-size: 18px; color: #B71C1C; font-weight: 700; }

  .filter-row {
    display: flex; flex-wrap: wrap; gap: 6px;
    background: rgba(255,255,255,0.95); backdrop-filter: blur(10px);
    border-radius: 14px; padding: 6px;
    box-shadow: 0 2px 12px rgba(0,0,0,0.1);
  }
  .chip {
    background: transparent; border: 1.5px solid transparent;
    border-radius: 12px; padding: 5px 11px;
    font-size: 12px; color: #333; cursor: pointer;
    font-family: inherit; font-weight: 500;
    white-space: nowrap;
  }
  .chip:hover { background: rgba(0,0,0,0.05); }
  .chip.active {
    background: rgba(211,47,47,0.1);
    border-color: #D32F2F; color: #B71C1C; font-weight: 600;
  }
  .zoom-hint { font-size: 10px; color: #FBC02D; margin-left: 4px; }

  .loc-row {
    display: inline-flex; align-items: center; gap: 8px;
    background: rgba(255,255,255,0.95); backdrop-filter: blur(10px);
    border-radius: 12px; padding: 6px 12px;
    box-shadow: 0 2px 12px rgba(0,0,0,0.08);
    font-size: 12px; color: #444;
    width: fit-content;
  }
  .loc-tag { color: #B71C1C; font-weight: 600; }
  .loc-row select {
    border: 1px solid #ddd; border-radius: 6px;
    padding: 2px 4px; font-size: 12px; font-family: inherit;
  }
  .ct { color: #888; }
  .loc-cta {
    background: rgba(255,179,0,0.95); color: #5d4037;
    border: none; border-radius: 12px;
    padding: 8px 14px; font-size: 12px; font-weight: 600;
    cursor: pointer; font-family: inherit;
    box-shadow: 0 2px 12px rgba(0,0,0,0.1);
    width: fit-content;
  }
  .loc-cta:hover { filter: brightness(1.05); }
  .loc-cta .err { color: #b71c1c; margin-left: 6px; font-size: 11px; }

  .loc-tools {
    display: inline-flex; align-items: center; gap: 4px;
    background: rgba(255,255,255,0.95); backdrop-filter: blur(10px);
    border-radius: 12px; padding: 4px 8px;
    box-shadow: 0 2px 12px rgba(0,0,0,0.06);
    font-size: 11px;
    width: fit-content;
  }
  .mini {
    background: transparent; border: 1px solid #ddd;
    border-radius: 8px; padding: 3px 8px;
    font-size: 11px; color: #555; cursor: pointer; font-family: inherit;
  }
  .mini:hover { background: #f4f4f4; }
  .mini.active {
    background: #FFF3E0; border-color: #FFB300; color: #E65100; font-weight: 600;
  }
  .acc-warn { color: #888; margin-left: 4px; font-size: 10px; }

  .center { top: 50%; left: 50%; transform: translate(-50%,-50%); }
  .loading {
    background: rgba(255,255,255,0.95); padding: 12px 18px;
    border-radius: 10px; color: #B71C1C;
    box-shadow: 0 2px 12px rgba(0,0,0,0.1);
  }
  .bottom-center { bottom: 16px; left: 50%; transform: translateX(-50%); }
  .error {
    background: #ffebee; color: #b71c1c;
    border-left: 3px solid #d32f2f;
    padding: 10px 16px; border-radius: 8px; font-size: 13px;
  }
  .holiday-banner {
    background: #FFF8E1; color: #5D4037;
    border-left: 3px solid #FFB300;
    padding: 8px 14px; border-radius: 8px;
    font-size: 12px; font-weight: 600;
    box-shadow: 0 2px 12px rgba(0,0,0,0.1);
  }

  @media (max-width: 480px) {
    .top-left { top: 8px; left: 8px; right: 8px; }
    .brand h1 { font-size: 16px; }
  }
</style>
