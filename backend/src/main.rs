use anyhow::Context;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

const BASE_URL: &str = "http://apis.data.go.kr/B552657";

// 데이터 종류 식별
#[derive(Serialize, Deserialize, Clone, Debug, Copy, PartialEq, Eq)]
pub enum FacilityKind {
    Hospital,   // 응급의료기관
    Pharmacy,   // 약국
    AED,        // 자동심장충격기
}

#[derive(Serialize, Clone, Debug)]
pub struct Facility {
    pub kind: FacilityKind,
    pub hpid: String,         // 기관 고유 코드
    pub name: String,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    /// 요일별 운영시간 (Mon..Sun + 공휴일/명절). HH:MM-HH:MM 형태.
    pub hours: HashMap<String, String>,
    pub etc: Option<String>,  // 비고
}

// ===== API 응답 deserialization =====
// data.go.kr B552657 표준 응답 구조 (JSON)
#[derive(Deserialize, Debug)]
struct ApiHeader {
    #[serde(rename = "resultCode", default)]
    result_code: String,
    #[serde(rename = "resultMsg", default)]
    result_msg: String,
}

#[derive(Deserialize, Debug)]
struct ApiBody {
    #[serde(default)]
    items: Option<serde_json::Value>,
    #[serde(rename = "totalCount", default)]
    total_count: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug)]
struct ApiResponse {
    header: ApiHeader,
    #[serde(default)]
    body: Option<ApiBody>,
}

#[derive(Deserialize, Debug)]
struct ApiEnvelope {
    response: ApiResponse,
}

#[derive(Clone)]
struct AppState {
    http: reqwest::Client,
    service_key: String,
    cache: Arc<RwLock<Cache>>,
}

#[derive(Default)]
struct Cache {
    hospitals: Option<Vec<Facility>>,
    pharmacies: Option<Vec<Facility>>,
    aeds: Option<Vec<Facility>>,
    holiday: Option<Vec<HolidayClinic>>,
}

#[derive(Serialize, Clone, Debug)]
pub struct HolidayClinic {
    pub hpid: String,
    pub name: String,
    pub kind_name: String,    // "약국" | "병원" | ...
    pub address: Option<String>,
    pub phone: Option<String>,
    pub days: Vec<String>,    // ["2026-02-14", "2026-02-15", ...]
    pub time: Option<String>, // "09:00~14:00"
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,medi_map_backend=debug,tower_http=info".into()),
        )
        .init();

    let service_key = std::env::var("MEDI_SERVICE_KEY")
        .context("MEDI_SERVICE_KEY missing in .env")?;
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8771);

    let state = Arc::new(AppState {
        http: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .user_agent("medi-map/0.1")
            .build()?,
        service_key,
        cache: Arc::new(RwLock::new(Cache::default())),
    });

    // 백그라운드 초기 로드 + 자동 재시도
    {
        let st = state.clone();
        tokio::spawn(async move {
            for attempt in 1..=5 {
                match load_hospitals(&st).await {
                    Ok(n) => { tracing::info!("응급의료기관 로드 완료: {n}건"); break; }
                    Err(e) => {
                        let wait = 10u64.saturating_mul(attempt);
                        tracing::warn!("응급의료기관 로드 실패 ({attempt}/5, {wait}s 후 재시도): {e}");
                        tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
                    }
                }
            }
        });
    }
    {
        let st = state.clone();
        tokio::spawn(async move {
            for attempt in 1..=5 {
                match load_pharmacies(&st).await {
                    Ok(n) => { tracing::info!("약국 로드 완료: {n}건"); break; }
                    Err(e) => {
                        let wait = 10u64.saturating_mul(attempt);
                        tracing::warn!("약국 로드 실패 ({attempt}/5, {wait}s 후 재시도): {e}");
                        tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
                    }
                }
            }
        });
    }
    {
        let st = state.clone();
        tokio::spawn(async move {
            for attempt in 1..=5 {
                match load_aeds(&st).await {
                    Ok(n) => { tracing::info!("AED 로드 완료: {n}건"); break; }
                    Err(e) => {
                        let wait = 15u64.saturating_mul(attempt);
                        tracing::warn!("AED 로드 실패 ({attempt}/5, {wait}s 후 재시도): {e}");
                        tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
                    }
                }
            }
        });
    }
    {
        let st = state.clone();
        tokio::spawn(async move {
            for attempt in 1..=3 {
                match load_holiday(&st).await {
                    Ok(n) => { tracing::info!("명절 비상진료 로드 완료: {n}건"); break; }
                    Err(e) => {
                        let wait = 15u64.saturating_mul(attempt);
                        tracing::warn!("명절 로드 실패 ({attempt}/3, {wait}s 후 재시도): {e}");
                        tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
                    }
                }
            }
        });
    }

    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);

    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/hospitals", get(list_hospitals))
        .route("/api/pharmacies", get(list_pharmacies))
        .route("/api/aeds", get(list_aeds))
        .route("/api/holiday-clinics", get(list_holiday))
        .route("/api/realtime/:hpid", get(realtime_beds))
        .route("/api/refresh", get(refresh_all))
        .with_state(state)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let addr = format!("0.0.0.0:{port}");
    tracing::info!("listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> &'static str { "ok" }

#[derive(Deserialize)]
struct ListQuery {
    /// 시도 필터 (예: "서울")
    #[serde(default)]
    sido: Option<String>,
    /// 시군구 필터
    #[serde(default)]
    sgg: Option<String>,
}

async fn list_hospitals(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<Facility>>, ApiError> {
    let r = state.cache.read().await;
    if let Some(list) = &r.hospitals {
        return Ok(Json(filter(list, q.sido.as_deref(), q.sgg.as_deref())));
    }
    drop(r);
    load_hospitals(&state).await.map_err(|e| ApiError::upstream(format!("{e}")))?;
    let r = state.cache.read().await;
    let list = r.hospitals.as_deref().unwrap_or(&[]);
    Ok(Json(filter(list, q.sido.as_deref(), q.sgg.as_deref())))
}

async fn list_pharmacies(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<Facility>>, ApiError> {
    let r = state.cache.read().await;
    if let Some(list) = &r.pharmacies {
        return Ok(Json(filter(list, q.sido.as_deref(), q.sgg.as_deref())));
    }
    drop(r);
    load_pharmacies(&state).await.map_err(|e| ApiError::upstream(format!("{e}")))?;
    let r = state.cache.read().await;
    let list = r.pharmacies.as_deref().unwrap_or(&[]);
    Ok(Json(filter(list, q.sido.as_deref(), q.sgg.as_deref())))
}

fn filter(all: &[Facility], sido: Option<&str>, sgg: Option<&str>) -> Vec<Facility> {
    all.iter()
        .filter(|f| {
            let addr = f.address.as_deref().unwrap_or("");
            sido.map_or(true, |s| addr.contains(s))
                && sgg.map_or(true, |s| addr.contains(s))
        })
        .cloned()
        .collect()
}

/// 응급실 실시간 가용병상 정보
async fn realtime_beds(
    State(state): State<Arc<AppState>>,
    Path(hpid): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let key = urlencoding::encode(&state.service_key);
    let url = format!(
        "{BASE_URL}/ErmctInfoInqireService/getEmrrmRltmUsefulSckbdInfoInqire\
         ?serviceKey={key}&HPID={hpid}&pageNo=1&numOfRows=1&_type=json"
    );
    let body = state.http.get(&url).send().await
        .map_err(|e| ApiError::upstream(format!("realtime: {e}")))?
        .text().await
        .map_err(|e| ApiError::upstream(format!("realtime body: {e}")))?;
    let parsed: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| ApiError::upstream(format!("realtime parse: {e}")))?;
    Ok(Json(parsed))
}

async fn refresh_all(State(state): State<Arc<AppState>>) -> Result<Json<serde_json::Value>, ApiError> {
    let h = load_hospitals(&state).await.map_err(|e| ApiError::upstream(format!("{e}")))?;
    let p = load_pharmacies(&state).await.map_err(|e| ApiError::upstream(format!("{e}")))?;
    Ok(Json(serde_json::json!({ "hospitals": h, "pharmacies": p })))
}

async fn load_hospitals(state: &Arc<AppState>) -> anyhow::Result<usize> {
    // 응급의료기관 목록 (위·경도 포함)
    let list = fetch_paged(state, "ErmctInfoInqireService/getEgytListInfoInqire").await?;
    let facilities: Vec<Facility> = list.into_iter()
        .map(|item| parse_facility(item, FacilityKind::Hospital))
        .collect();
    let n = facilities.len();
    state.cache.write().await.hospitals = Some(facilities);
    Ok(n)
}

async fn load_pharmacies(state: &Arc<AppState>) -> anyhow::Result<usize> {
    let list = fetch_paged(state, "ErmctInsttInfoInqireService/getParmacyListInfoInqire").await?;
    let facilities: Vec<Facility> = list.into_iter()
        .map(|item| parse_facility(item, FacilityKind::Pharmacy))
        .collect();
    let n = facilities.len();
    state.cache.write().await.pharmacies = Some(facilities);
    Ok(n)
}

async fn load_aeds(state: &Arc<AppState>) -> anyhow::Result<usize> {
    let list = fetch_paged(state, "AEDInfoInqireService/getAedLcinfoInqire").await?;
    let facilities: Vec<Facility> = list.into_iter()
        .map(parse_aed)
        .filter(|f| f.lat.is_some() && f.lon.is_some())
        .collect();
    let n = facilities.len();
    state.cache.write().await.aeds = Some(facilities);
    Ok(n)
}

async fn list_aeds(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<Facility>>, ApiError> {
    let r = state.cache.read().await;
    if let Some(list) = &r.aeds {
        return Ok(Json(filter(list, q.sido.as_deref(), q.sgg.as_deref())));
    }
    drop(r);
    load_aeds(&state).await.map_err(|e| ApiError::upstream(format!("{e}")))?;
    let r = state.cache.read().await;
    let list = r.aeds.as_deref().unwrap_or(&[]);
    Ok(Json(filter(list, q.sido.as_deref(), q.sgg.as_deref())))
}

async fn load_holiday(state: &Arc<AppState>) -> anyhow::Result<usize> {
    let list = fetch_paged(state, "HolidyEmgncClnicInsttInfoInqireService/getHolidyClnicPosblEgytInfoInqire").await?;
    let clinics: Vec<HolidayClinic> = list.into_iter().map(parse_holiday).collect();
    let n = clinics.len();
    state.cache.write().await.holiday = Some(clinics);
    Ok(n)
}

async fn list_holiday(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<HolidayClinic>>, ApiError> {
    let r = state.cache.read().await;
    if let Some(list) = &r.holiday {
        return Ok(Json(list.clone()));
    }
    drop(r);
    load_holiday(&state).await.map_err(|e| ApiError::upstream(format!("{e}")))?;
    let r = state.cache.read().await;
    Ok(Json(r.holiday.clone().unwrap_or_default()))
}

fn parse_holiday(v: serde_json::Value) -> HolidayClinic {
    let s = |k: &str| v.get(k).and_then(|x| match x {
        serde_json::Value::String(s) => Some(s.trim().to_string()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }).filter(|s| !s.is_empty());
    let mut days: Vec<String> = Vec::new();
    for i in 1..=10 {
        if let Some(d) = s(&format!("dutyDay{i}")) {
            days.push(d);
        }
    }
    HolidayClinic {
        hpid: s("hpid").unwrap_or_default(),
        name: s("dutyName").unwrap_or_default(),
        kind_name: s("dutyDivName").unwrap_or_default(),
        address: s("dutyAddr"),
        phone: s("dutyTel1"),
        days,
        time: s("dutyDaytime1"),
    }
}

fn parse_aed(v: serde_json::Value) -> Facility {
    let s = |k: &str| v.get(k).and_then(|x| match x {
        serde_json::Value::String(s) => Some(s.trim().to_string()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }).filter(|s| !s.is_empty());
    let f = |k: &str| s(k).and_then(|x| x.parse::<f64>().ok());

    // AED 운영시간: monSttTme/monEndTme 등 영문 약어
    let mut hours: HashMap<String, String> = HashMap::new();
    let dow_map: [(&str, &str); 7] = [
        ("mon", "Mon"), ("tue", "Tue"), ("wed", "Wed"),
        ("thu", "Thu"), ("fri", "Fri"), ("sat", "Sat"), ("sun", "Sun"),
    ];
    for (k, label) in dow_map {
        let start = s(&format!("{k}SttTme"));
        let close = s(&format!("{k}EndTme"));
        if let (Some(a), Some(b)) = (start, close) {
            hours.insert(label.to_string(), format!("{}-{}", fmt_hhmm(&a), fmt_hhmm(&b)));
        }
    }

    // org(설치기관) + buildPlace(설치장소)를 이름으로 결합
    let org = s("org").unwrap_or_default();
    let place = s("buildPlace");
    let name = match place {
        Some(p) if !p.is_empty() => format!("{} ({})", org, p),
        _ => org,
    };
    let etc = match (s("manager"), s("managerTel"), s("model")) {
        (Some(m), Some(t), Some(mo)) => Some(format!("관리자: {m} ({t}) · 모델: {mo}")),
        (Some(m), Some(t), None) => Some(format!("관리자: {m} ({t})")),
        _ => None,
    };

    Facility {
        kind: FacilityKind::AED,
        hpid: s("cnt").unwrap_or_else(|| s("serialSeq").unwrap_or_default()),
        name,
        address: s("buildAddress"),
        phone: s("clerkTel"),
        lat: f("wgs84Lat"),
        lon: f("wgs84Lon"),
        hours,
        etc,
    }
}

/// data.go.kr B552657 페이지네이션 fetch (numOfRows=1000)
async fn fetch_paged(state: &Arc<AppState>, path: &str) -> anyhow::Result<Vec<serde_json::Value>> {
    let mut all: Vec<serde_json::Value> = Vec::new();
    let mut page = 1u32;
    let per = 1000u32;
    let key = urlencoding::encode(&state.service_key);
    loop {
        let url = format!(
            "{BASE_URL}/{path}?serviceKey={key}&pageNo={page}&numOfRows={per}&_type=json"
        );
        tracing::debug!("fetch page={page} {path}");
        let resp = state.http.get(&url).send().await?;
        let text = resp.text().await?;
        let env: ApiEnvelope = serde_json::from_str(&text)
            .with_context(|| format!("parse failed. body head: {}",
                text.chars().take(300).collect::<String>()))?;
        if env.response.header.result_code != "00" {
            anyhow::bail!("API error {} {}",
                env.response.header.result_code, env.response.header.result_msg);
        }
        let body = env.response.body.unwrap_or(ApiBody { items: None, total_count: None });
        // items.item 은 단일 객체 또는 배열일 수 있음
        let page_items: Vec<serde_json::Value> = match body.items {
            Some(serde_json::Value::Object(map)) => {
                if let Some(item) = map.get("item") {
                    match item {
                        serde_json::Value::Array(a) => a.clone(),
                        v => vec![v.clone()],
                    }
                } else { vec![] }
            }
            Some(serde_json::Value::String(_)) => vec![],
            _ => vec![],
        };
        let count = page_items.len();
        all.extend(page_items);
        if count < per as usize { break; }
        page += 1;
        if page > 200 { tracing::warn!("page 한도(200), 중단"); break; }
    }
    Ok(all)
}

fn parse_facility(v: serde_json::Value, kind: FacilityKind) -> Facility {
    let s = |k: &str| v.get(k).and_then(|x| match x {
        serde_json::Value::String(s) => Some(s.trim().to_string()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }).filter(|s| !s.is_empty());
    let f = |k: &str| s(k).and_then(|x| x.parse::<f64>().ok());

    let mut hours: HashMap<String, String> = HashMap::new();
    // 운영시간 필드명: dutyTime{1..8}{s|c} — 1=월, 2=화, ..., 6=토, 7=일, 8=공휴일
    let dow_map: [(u8, &str); 8] = [
        (1, "Mon"), (2, "Tue"), (3, "Wed"), (4, "Thu"),
        (5, "Fri"), (6, "Sat"), (7, "Sun"), (8, "Hol")
    ];
    for (n, label) in dow_map {
        let start = s(&format!("dutyTime{n}s"));
        let close = s(&format!("dutyTime{n}c"));
        if let (Some(a), Some(b)) = (start, close) {
            hours.insert(label.to_string(), format!("{}-{}", fmt_hhmm(&a), fmt_hhmm(&b)));
        }
    }

    Facility {
        kind,
        hpid: s("hpid").or_else(|| s("HPID")).unwrap_or_default(),
        name: s("dutyName").unwrap_or_else(|| s("name").unwrap_or_default()),
        address: s("dutyAddr").or_else(|| s("addr")),
        phone: s("dutyTel1").or_else(|| s("tel")),
        lat: f("wgs84Lat"),
        lon: f("wgs84Lon"),
        hours,
        etc: s("dutyEtc"),
    }
}

/// "0900" 또는 "900" 또는 "09:00" 같은 입력을 "09:00"으로 정규화
fn fmt_hhmm(s: &str) -> String {
    let cleaned: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if cleaned.is_empty() { return s.to_string(); }
    let padded = format!("{:0>4}", cleaned);
    format!("{}:{}", &padded[0..2], &padded[2..4])
}

// --- error handling ---

struct ApiError { status: StatusCode, message: String }

impl ApiError {
    fn upstream(msg: impl Into<String>) -> Self {
        Self { status: StatusCode::BAD_GATEWAY, message: msg.into() }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        tracing::warn!("api error: {}", self.message);
        (self.status, Json(serde_json::json!({ "error": self.message }))).into_response()
    }
}
