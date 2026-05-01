<script lang="ts">
  import type { Facility, HolidayClinic } from '$lib/api';
  import { kindColor, isOpenNow, distanceKm, fetchRealtimeBeds, kakaoDirectionsUrl, naverDirectionsUrl } from '$lib/api';
  import { createEventDispatcher } from 'svelte';

  export let facility: Facility;
  export let holiday: HolidayClinic | null = null;
  export let userPos: { lat: number; lon: number } | null = null;
  export let now: Date = new Date();

  const dispatch = createEventDispatcher<{ close: undefined }>();

  let realtime: any = null;
  let realtimeLoading = false;

  $: if (facility.kind === 'Hospital') loadRealtime(facility.hpid);

  async function loadRealtime(hpid: string) {
    if (!hpid) return;
    realtimeLoading = true;
    try {
      const r = await fetchRealtimeBeds(hpid);
      if (facility.hpid === hpid) realtime = r;
    } catch {/* silent */}
    finally { realtimeLoading = false; }
  }

  $: open = isOpenNow(facility, now);
  $: dist = userPos && facility.lat != null && facility.lon != null
    ? distanceKm(userPos, { lat: facility.lat, lon: facility.lon }) : null;

  $: dest = facility.lat != null && facility.lon != null
    ? { name: facility.name, lat: facility.lat, lon: facility.lon } : null;

  function naverUrl(): string {
    if (!dest) return `https://map.naver.com/p/search/${encodeURIComponent(facility.name)}`;
    return naverDirectionsUrl(dest, userPos);
  }
  function kakaoUrl(): string {
    if (!dest) return `https://map.kakao.com/?q=${encodeURIComponent(facility.name)}`;
    return kakaoDirectionsUrl(dest, userPos);
  }
  function telUrl(): string | null {
    return facility.phone ? `tel:${facility.phone.replace(/[^\d+]/g, '')}` : null;
  }

  const DOW_LABEL = { Mon:'월', Tue:'화', Wed:'수', Thu:'목', Fri:'금', Sat:'토', Sun:'일', Hol:'공휴일' };

  function extractRealtime(rt: any): any | null {
    const item = rt?.response?.body?.items?.item;
    if (!item) return null;
    return Array.isArray(item) ? item[0] : item;
  }
  function fmtHvidate(s: string): string {
    // 20260501072815 → 2026-05-01 07:28
    if (!/^\d{8}/.test(s)) return s;
    const y = s.slice(0,4), m = s.slice(4,6), d = s.slice(6,8);
    const hh = s.slice(8,10) || '', mm = s.slice(10,12) || '';
    return `${y}-${m}-${d}` + (hh && mm ? ` ${hh}:${mm}` : '');
  }
</script>

<div class="backdrop" on:click={() => dispatch('close')} role="presentation"></div>
<div class="card" role="dialog">
  <header style="border-left-color: {kindColor(facility.kind)}">
    <div class="title-row">
      <h2>{facility.name}</h2>
      <button class="close" on:click={() => dispatch('close')}>✕</button>
    </div>
    <div class="meta">
      <span class="tag" style="background: {kindColor(facility.kind)}">
        {facility.kind === 'Pharmacy' ? '약국' : '병원/응급'}
      </span>
      <span class="status" class:open class:closed={!open}>
        {open ? '🟢 진료 중' : '🔴 진료 종료'}
      </span>
      {#if dist != null}
        <span class="dist">📍 {dist < 1 ? `${Math.round(dist*1000)}m` : `${dist.toFixed(1)}km`}</span>
      {/if}
    </div>
  </header>

  <div class="body">
    {#if facility.address}
      <p class="addr">📍 {facility.address}</p>
    {/if}

    <h3>운영시간</h3>
    <table class="hours">
      {#each ['Mon','Tue','Wed','Thu','Fri','Sat','Sun','Hol'] as d}
        {@const r = facility.hours[d]}
        {#if r}
          <tr><th>{DOW_LABEL[d]}</th><td>{r}</td></tr>
        {/if}
      {/each}
    </table>

    {#if facility.kind === 'Hospital' && (realtimeLoading || realtime)}
      <h3>실시간 응급실 가용 병상</h3>
      {#if realtimeLoading}
        <p class="state">불러오는 중…</p>
      {:else}
        {@const r = extractRealtime(realtime)}
        {#if r}
          <div class="rt-grid">
            {#if r.hvec != null}<div class="rt-cell"><span class="lbl">응급실</span><b class="val">{r.hvec}</b></div>{/if}
            {#if r.hvoc != null}<div class="rt-cell"><span class="lbl">수술실</span><b class="val">{r.hvoc}</b></div>{/if}
            {#if r.hvgc != null}<div class="rt-cell"><span class="lbl">입원실</span><b class="val">{r.hvgc}</b></div>{/if}
            {#if r.hvncc != null}<div class="rt-cell"><span class="lbl">신생아 중환자</span><b class="val">{r.hvncc}</b></div>{/if}
            {#if r.hv2 != null}<div class="rt-cell"><span class="lbl">약물 중환자</span><b class="val">{r.hv2}</b></div>{/if}
            {#if r.hv3 != null}<div class="rt-cell"><span class="lbl">외과 중환자</span><b class="val">{r.hv3}</b></div>{/if}
          </div>
          <div class="rt-equip">
            {#if r.hvecmoayn === 'Y'}<span>ECMO</span>{/if}
            {#if r.hvctayn === 'Y'}<span>CT</span>{/if}
            {#if r.hvmriayn === 'Y'}<span>MRI</span>{/if}
            {#if r.hvangioayn === 'Y'}<span>혈관조영</span>{/if}
            {#if r.hvincuayn === 'Y'}<span>인큐베이터</span>{/if}
            {#if r.hvventiayn === 'Y'}<span>인공호흡기</span>{/if}
            {#if r.hvoxyayn === 'Y'}<span>산소</span>{/if}
            {#if r.hvcrrtayn === 'Y'}<span>CRRT</span>{/if}
          </div>
          {#if r.hvidate}<p class="rt-time">📡 {fmtHvidate(String(r.hvidate))} 기준</p>{/if}
        {:else}
          <p class="state empty">현재 정보 없음</p>
        {/if}
      {/if}
    {/if}

    {#if holiday && holiday.days.length > 0}
      <div class="holiday">
        🌕 <strong>명절 비상진료</strong> ({holiday.kind_name})
        <div class="hd-days">{holiday.days.join(', ')}</div>
        {#if holiday.time}<div class="hd-time">{holiday.time}</div>{/if}
      </div>
    {/if}

    {#if facility.etc}
      <p class="note">📝 {facility.etc}</p>
    {/if}

    <div class="actions">
      {#if telUrl()}<a class="btn primary" href={telUrl()}>☎ 전화</a>{/if}
      <a class="btn" href={kakaoUrl()} target="_blank" rel="noopener">🚗 카카오 길찾기</a>
      <a class="btn" href={naverUrl()} target="_blank" rel="noopener">🚗 네이버 길찾기</a>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed; inset: 0;
    background: rgba(0,30,50,0.45);
    z-index: 1000; backdrop-filter: blur(2px);
  }
  .card {
    position: fixed; z-index: 1001;
    left: 50%; top: 50%; transform: translate(-50%,-50%);
    background: #fff; border-radius: 14px;
    box-shadow: 0 12px 48px rgba(0,0,0,0.32);
    width: min(380px, 92vw); max-height: 88vh;
    display: flex; flex-direction: column; overflow: hidden;
    font-size: 13px; line-height: 1.5;
  }
  header {
    border-left: 4px solid #D32F2F;
    padding: 14px 16px 12px;
    background: #fafafa; border-bottom: 1px solid #f0f0f0;
  }
  .title-row { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
  h2 { margin: 0 0 4px; font-size: 17px; color: #1a1a1a; }
  .close {
    background: rgba(0,0,0,0.06); border: none;
    width: 28px; height: 28px; border-radius: 50%;
    cursor: pointer; font-size: 14px; color: #555;
  }
  .close:hover { background: #d32f2f; color: #fff; }
  .meta { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; font-size: 11px; color: #555; }
  .tag { color: #fff; padding: 2px 8px; border-radius: 10px; font-weight: 600; }
  .status.open { color: #2E7D32; font-weight: 600; }
  .status.closed { color: #888; }
  .dist { color: #666; }
  .body { overflow-y: auto; padding: 0 16px 14px; }
  .addr { margin: 10px 0 4px; color: #333; }
  h3 { margin: 14px 0 6px; font-size: 12px; color: #555; }
  .hours { width: 100%; border-collapse: collapse; font-size: 12px; }
  .hours th {
    text-align: left; font-weight: 600; color: #666;
    width: 60px; padding: 4px 0;
  }
  .hours td { font-family: ui-monospace, monospace; color: #333; }
  .rt-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 4px;
    margin-bottom: 8px;
  }
  .rt-cell {
    background: #fff5f5; border: 1px solid #ffe5e5;
    border-radius: 6px; padding: 5px 6px;
    display: flex; flex-direction: column;
    align-items: center; gap: 1px;
  }
  .rt-cell .lbl { font-size: 10px; color: #777; }
  .rt-cell .val { font-size: 16px; color: #B71C1C; font-weight: 700; }
  .rt-equip {
    display: flex; flex-wrap: wrap; gap: 4px;
    margin-bottom: 6px;
  }
  .rt-equip span {
    background: #E8F5E9; color: #2E7D32;
    border-radius: 6px; padding: 2px 7px;
    font-size: 10px; font-weight: 600;
  }
  .rt-time { font-size: 10px; color: #999; margin: 4px 0 0; text-align: right; }
  .state { color: #888; font-size: 12px; }
  .note {
    margin-top: 10px; padding: 8px;
    background: #FFF8E1; border-left: 3px solid #FFB800;
    font-size: 12px; color: #555; border-radius: 4px;
  }
  .holiday {
    margin-top: 10px; padding: 10px 12px;
    background: #FFF8E1; border-left: 3px solid #FFB300;
    border-radius: 4px;
    font-size: 12px; color: #5D4037;
  }
  .holiday strong { color: #E65100; margin-left: 4px; }
  .hd-days {
    font-family: ui-monospace, monospace;
    font-size: 11px; color: #6D4C41; margin-top: 4px;
  }
  .hd-time { font-size: 11px; color: #2E7D32; margin-top: 2px; }
  .actions { display: flex; gap: 6px; margin-top: 14px; }
  .btn {
    flex: 1; padding: 8px;
    background: #f4f4f4; color: #333; text-align: center;
    text-decoration: none; border-radius: 8px;
    font-size: 12px; font-weight: 600;
  }
  .btn.primary { background: #D32F2F; color: #fff; }
  .btn:hover { filter: brightness(1.05); }
</style>
