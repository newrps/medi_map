export type FacilityKind = 'Hospital' | 'Pharmacy' | 'AED';

export interface Facility {
  kind: FacilityKind;
  hpid: string;
  name: string;
  address: string | null;
  phone: string | null;
  lat: number | null;
  lon: number | null;
  hours: Record<string, string>; // {Mon:"09:00-18:00", ...}
  etc: string | null;
}

const BASE = '/api';

export async function fetchHospitals(sido?: string): Promise<Facility[]> {
  const url = sido ? `${BASE}/hospitals?sido=${encodeURIComponent(sido)}` : `${BASE}/hospitals`;
  const r = await fetch(url);
  if (!r.ok) throw new Error(`hospitals: ${r.status}`);
  return r.json();
}

export async function fetchPharmacies(sido?: string): Promise<Facility[]> {
  const url = sido ? `${BASE}/pharmacies?sido=${encodeURIComponent(sido)}` : `${BASE}/pharmacies`;
  const r = await fetch(url);
  if (!r.ok) throw new Error(`pharmacies: ${r.status}`);
  return r.json();
}

export async function fetchAEDs(sido?: string): Promise<Facility[]> {
  const url = sido ? `${BASE}/aeds?sido=${encodeURIComponent(sido)}` : `${BASE}/aeds`;
  const r = await fetch(url);
  if (!r.ok) throw new Error(`aeds: ${r.status}`);
  return r.json();
}

export interface HolidayClinic {
  hpid: string;
  name: string;
  kind_name: string;
  address: string | null;
  phone: string | null;
  days: string[];   // ["2026-02-14", ...]
  time: string | null;
}

export async function fetchHolidayClinics(): Promise<HolidayClinic[]> {
  const r = await fetch(`${BASE}/holiday-clinics`);
  if (!r.ok) throw new Error(`holiday: ${r.status}`);
  return r.json();
}

export async function fetchRealtimeBeds(hpid: string): Promise<unknown> {
  const r = await fetch(`${BASE}/realtime/${encodeURIComponent(hpid)}`);
  if (!r.ok) throw new Error(`realtime: ${r.status}`);
  return r.json();
}

export function kindColor(k: FacilityKind, hasER: boolean = false): string {
  if (hasER) return '#D32F2F';
  if (k === 'Pharmacy') return '#1565C0';
  if (k === 'AED') return '#FBC02D';
  return '#2E7D32';
}

// 운영시간 파싱 — Mon..Sun, Hol
const DOW: Record<number, string> = {
  0: 'Sun', 1: 'Mon', 2: 'Tue', 3: 'Wed', 4: 'Thu', 5: 'Fri', 6: 'Sat'
};

/** 지금 진료 중인지 — Date 기준. 운영시간 정보 없으면 24시간 운영(응급실)으로 간주. */
export function isOpenNow(f: Facility, now: Date = new Date(), holiday = false): boolean {
  if (!f.hours || Object.keys(f.hours).length === 0) return true; // 응급의료기관 등 24시간
  const key = holiday ? 'Hol' : DOW[now.getDay()];
  const range = f.hours[key];
  if (!range) return false;
  const [a, b] = range.split('-').map(s => s.trim());
  if (!a || !b) return false;
  const nowMin = now.getHours() * 60 + now.getMinutes();
  const sa = toMin(a);
  const sb = toMin(b);
  if (sa == null || sb == null) return false;
  // 종료 시각이 시작보다 작으면 익일까지 (예: 21:00-08:00)
  if (sb < sa) return nowMin >= sa || nowMin < sb;
  return nowMin >= sa && nowMin < sb;
}

function toMin(hhmm: string): number | null {
  const m = hhmm.match(/^(\d{1,2}):?(\d{2})$/);
  if (!m) return null;
  return parseInt(m[1], 10) * 60 + parseInt(m[2], 10);
}

/** 카카오맵 길찾기 URL — 도착지 좌표 기반. 내 위치 있으면 출발지도 포함. */
export function kakaoDirectionsUrl(
  dest: { name: string; lat: number; lon: number },
  origin?: { lat: number; lon: number } | null
): string {
  const eName = encodeURIComponent(dest.name);
  if (origin) {
    return `https://map.kakao.com/link/from/내위치,${origin.lat},${origin.lon}/to/${eName},${dest.lat},${dest.lon}`;
  }
  return `https://map.kakao.com/link/to/${eName},${dest.lat},${dest.lon}`;
}

/** 네이버맵 길찾기 URL — 도착지 좌표 기반 */
export function naverDirectionsUrl(
  dest: { name: string; lat: number; lon: number },
  origin?: { lat: number; lon: number } | null
): string {
  const eName = encodeURIComponent(dest.name);
  // /p/directions/{slng},{slat},{sname},,/{dlng},{dlat},{dname},,/{mode}
  if (origin) {
    return `https://map.naver.com/p/directions/${origin.lon},${origin.lat},%EB%82%B4%EC%9C%84%EC%B9%98,,/${dest.lon},${dest.lat},${eName},,/-/transit`;
  }
  return `https://map.naver.com/p/directions/-/-/${dest.lon},${dest.lat},${eName},,/-/transit`;
}

export function distanceKm(a: { lat: number; lon: number }, b: { lat: number; lon: number }): number {
  const R = 6371;
  const dLat = ((b.lat - a.lat) * Math.PI) / 180;
  const dLon = ((b.lon - a.lon) * Math.PI) / 180;
  const sa = Math.sin(dLat / 2);
  const sb = Math.sin(dLon / 2);
  const h = sa * sa + Math.cos((a.lat * Math.PI) / 180) * Math.cos((b.lat * Math.PI) / 180) * sb * sb;
  return 2 * R * Math.asin(Math.sqrt(h));
}
