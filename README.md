# 🏥 Ps의료지도 (medi-map)

전국 약국·병원·응급실을 지도에서 검색. **현재 시간 기준 진료 중인 곳** 자동 필터.
보건복지부 / 국립중앙의료원(E-Gen) OpenAPI 사용.

## 구성

```
backend/   Rust (axum) — 응급의료 OpenAPI 프록시 + 메모리 캐시
frontend/  SvelteKit + Leaflet
```

## 데이터 출처

- **응급의료기관 정보** (data.go.kr B552657/ErmctInfoInqireService)
- **응급의료기관 기본/상세 정보** (B552657/ErmctInsttInfoInqireService)
- **응급실 실시간 가용병상** (B552657/HsptlAsembySearchService) — 5분 polling
- **약국 정보**

## 핵심 기능

- 전국 응급실/병원/약국 마커 (색상·아이콘 구분)
- 현재 시간 기준 **"지금 진료 중"** 자동 필터
- **위치 기반** 가까운 순 정렬 (브라우저 Geolocation)
- **야간 / 주말 / 공휴일** 빠른 필터
- 마커 클릭 → 진료시간 표, 주소, 전화, 응급실 가용 병상

## 실행

### 개발

```bash
cd backend && cp .env.example .env  # MEDI_SERVICE_KEY 채우기
cargo run                            # → http://127.0.0.1:8771

cd frontend && npm install
npm run dev                          # → http://localhost:5176
```

### 배포 (NAS)

```bash
docker compose up -d --build
# → http://localhost:18082
```

## 환경변수

| 키 | 설명 |
|----|------|
| `MEDI_SERVICE_KEY` | data.go.kr 인증키 (Encoding) |
| `PORT` | 백엔드 포트 (기본 8771) |
| `HOST_PORT` | 프론트 호스트 포트 (기본 18082) |
| `RUST_LOG` | 로그 레벨 (info) |
