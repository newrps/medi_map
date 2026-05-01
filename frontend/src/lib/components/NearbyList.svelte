<script lang="ts">
  import type { Facility } from '$lib/api';
  import { kindColor, distanceKm, isOpenNow, kakaoDirectionsUrl, naverDirectionsUrl } from '$lib/api';
  import { createEventDispatcher } from 'svelte';

  export let facilities: Facility[] = [];
  export let userPos: { lat: number; lon: number } | null = null;
  export let mapCenter: { lat: number; lon: number } | null = null;
  export let now: Date = new Date();
  export let collapsed = false;

  const dispatch = createEventDispatcher<{ pick: Facility; toggle: undefined }>();

  // 거리 기준점: 내 위치 우선, 없으면 지도 중심
  $: anchor = userPos ?? mapCenter;

  $: sorted = (() => {
    if (!anchor) return facilities;
    const a = anchor;
    return [...facilities]
      .filter((f) => f.lat != null && f.lon != null)
      .map((f) => ({ f, d: distanceKm(a, { lat: f.lat!, lon: f.lon! }) }))
      .sort((x, y) => x.d - y.d)
      .map((x) => ({ ...x.f, _dist: x.d }) as Facility & { _dist: number });
  })();

  function fmtDist(km: number): string {
    return km < 1 ? `${Math.round(km * 1000)}m` : `${km.toFixed(1)}km`;
  }
  function kindLabel(f: Facility): string {
    return f.kind === 'Pharmacy' ? '약국' : '병원·응급';
  }
  function todayHours(f: Facility): string | null {
    if (!f.hours || Object.keys(f.hours).length === 0) return '24시간';
    const dow = ['Sun','Mon','Tue','Wed','Thu','Fri','Sat'];
    return f.hours[dow[now.getDay()]] ?? null;
  }
</script>

<button class="floating-toggle" class:open={!collapsed}
  on:click={() => dispatch('toggle')} aria-label="목록 토글">
  {collapsed ? '◀' : '▶'}
</button>

{#if !collapsed}
<aside class="panel">
    <header>
      <h2>📋 화면 안 의료기관</h2>
      <span class="ct">{sorted.length}곳</span>
    </header>
    {#if sorted.length === 0}
      <p class="empty">화면을 이동하거나 확대해보세요.</p>
    {:else}
      <ul>
        {#each sorted as f}
          {@const open = isOpenNow(f, now)}
          {@const hr = todayHours(f)}
          <li>
            <div class="row-wrap">
              <button class="row" on:click={() => dispatch('pick', f)}>
                <span class="dot" style="background: {kindColor(f.kind)}"></span>
                <span class="name">{f.name}</span>
                <span class="kind">{kindLabel(f)}</span>
                {#if anchor && f._dist != null}
                  <span class="dist">{fmtDist(f._dist)}</span>
                {/if}
                {#if hr}<span class="hr">{hr}</span>{/if}
                {#if f.address}<span class="addr">{f.address}</span>{/if}
              </button>
              {#if f.lat != null && f.lon != null}
                <div class="dir-btns">
                  <a href={kakaoDirectionsUrl({ name: f.name, lat: f.lat, lon: f.lon }, userPos)}
                    target="_blank" rel="noopener" class="dir kakao" title="카카오 길찾기">🚗K</a>
                  <a href={naverDirectionsUrl({ name: f.name, lat: f.lat, lon: f.lon }, userPos)}
                    target="_blank" rel="noopener" class="dir naver" title="네이버 길찾기">🚗N</a>
                </div>
              {/if}
            </div>
          </li>
        {/each}
      </ul>
    {/if}
</aside>
{/if}

<style>
  .panel {
    position: absolute;
    top: 0; right: 0; bottom: 0;
    width: min(340px, 88vw);
    background: rgba(255, 255, 255, 0.97);
    backdrop-filter: blur(10px);
    box-shadow: -2px 0 12px rgba(0, 0, 0, 0.1);
    display: flex; flex-direction: column;
    z-index: 600;
    pointer-events: auto;
  }

  .floating-toggle {
    position: fixed;
    right: 0;
    top: 50%;
    transform: translateY(-50%);
    width: 28px; height: 44px;
    background: rgba(255,255,255,0.95);
    border: none;
    border-radius: 8px 0 0 8px;
    box-shadow: -2px 2px 8px rgba(0,0,0,0.18);
    cursor: pointer;
    font-size: 14px; color: #555;
    font-family: inherit;
    z-index: 700;
    pointer-events: auto;
    transition: right 0.18s ease;
  }
  .floating-toggle.open { right: min(340px, 88vw); }
  .floating-toggle:hover { background: #fff; color: #B71C1C; }

  @media (max-width: 480px) {
    .panel { width: 80vw; }
    .floating-toggle.open { right: 80vw; }
  }

  header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 14px 16px 8px;
    border-bottom: 1px solid #f0f0f0;
  }
  h2 { margin: 0; font-size: 14px; color: #B71C1C; font-weight: 700; }
  .ct {
    background: #f4f4f4; padding: 2px 8px;
    border-radius: 10px; font-size: 11px; color: #666;
  }

  .empty {
    padding: 30px 16px; text-align: center;
    color: #888; font-size: 12px;
  }

  ul { list-style: none; margin: 0; padding: 6px 0; overflow-y: auto; flex: 1; }
  li { padding: 0; border-bottom: 1px solid #f5f5f5; }

  .row-wrap {
    display: flex; align-items: stretch;
  }

  .row {
    flex: 1;
    text-align: left;
    background: transparent; border: none; cursor: pointer;
    padding: 8px 8px 8px 14px;
    display: grid;
    grid-template-columns: 8px 1fr auto;
    grid-template-rows: auto auto auto;
    column-gap: 6px;
    row-gap: 2px;
    font-family: inherit; font-size: 12px;
    color: #333;
  }
  .row:hover { background: rgba(211, 47, 47, 0.04); }

  .dir-btns {
    display: flex; flex-direction: column; gap: 2px;
    padding: 6px;
  }
  .dir {
    display: inline-flex; align-items: center; justify-content: center;
    width: 38px; height: 22px;
    text-decoration: none;
    border-radius: 6px;
    font-size: 10px; font-weight: 700;
    border: 1px solid #ddd;
    color: #333;
  }
  .dir.kakao { background: #FEE500; border-color: #E4CC00; color: #3A2E00; }
  .dir.naver { background: #03C75A; border-color: #028A3F; color: #fff; }
  .dir:hover { filter: brightness(1.05); }

  .dot {
    grid-row: 1 / 4;
    width: 8px; height: 8px; border-radius: 50%;
    margin-top: 6px;
    align-self: start;
  }
  .name {
    grid-column: 2 / 3; grid-row: 1;
    font-weight: 600; color: #1a1a1a;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .kind {
    grid-column: 3 / 4; grid-row: 1;
    font-size: 10px; color: #888;
    background: #f4f4f4; padding: 1px 6px;
    border-radius: 6px;
  }
  .status { grid-column: 4 / 5; grid-row: 1; font-size: 12px; }
  .dist {
    grid-column: 2 / 5; grid-row: 2;
    font-size: 11px; color: #B71C1C; font-weight: 600;
  }
  .hr {
    grid-column: 2 / 5; grid-row: 2;
    justify-self: end;
    font-size: 11px; color: #2E7D32;
    font-family: ui-monospace, monospace;
  }
  .addr {
    grid-column: 2 / 5; grid-row: 3;
    font-size: 11px; color: #777;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .status.closed { opacity: 0.45; }
</style>
