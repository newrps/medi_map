<script lang="ts">
  import type { Facility, HolidayClinic } from '$lib/api';
  import { kindColor, isOpenNow, distanceKm, fetchRealtimeBeds } from '$lib/api';
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

  function naverUrl(): string {
    const q = encodeURIComponent(facility.address ?? facility.name);
    return `https://map.naver.com/p/search/${q}`;
  }
  function kakaoUrl(): string {
    const q = encodeURIComponent(facility.address ?? facility.name);
    return `https://map.kakao.com/?q=${q}`;
  }
  function telUrl(): string | null {
    return facility.phone ? `tel:${facility.phone.replace(/[^\d+]/g, '')}` : null;
  }

  const DOW_LABEL = { Mon:'월', Tue:'화', Wed:'수', Thu:'목', Fri:'금', Sat:'토', Sun:'일', Hol:'공휴일' };
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
      {:else if realtime}
        <pre class="rt">{JSON.stringify(realtime, null, 2)}</pre>
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
      <a class="btn" href={naverUrl()} target="_blank" rel="noopener">네이버 지도</a>
      <a class="btn" href={kakaoUrl()} target="_blank" rel="noopener">카카오맵</a>
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
  .rt { background: #f6f6f6; padding: 8px; border-radius: 6px; font-size: 10px; max-height: 200px; overflow: auto; }
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
