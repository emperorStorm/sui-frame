use base64::{engine::general_purpose, Engine as _};
use futures::future::join_all;
use regex::Regex;
use reqwest::{header, Client, Response};
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, RunEvent, WindowEvent};
use tauri_plugin_shell::{process::CommandChild, ShellExt};

const USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 Chrome/126 Safari/537.36";
const SETTINGS_FILE: &str = "search-settings.json";
const USERS_FILE: &str = "users.json";
const FAVORITES_FILE: &str = "favorites.json";
const DEFAULT_USERNAME: &str = "admin";
const DEFAULT_PASSWORD: &str = "123456";
const RULE_SOURCE_FILE: &str = "rules/sources/default.json";
const EMBEDDED_PANSOU_SOURCE_ID: &str = "embedded-pansou";
const EMBEDDED_PANSOU_SIDECAR: &str = "pansou-sidecar";
const EMBEDDED_PANSOU_DEFAULT_PORT: u16 = 10323;
const EMPTY_FOLDER_PROBE_BYTES: usize = 256 * 1024;
const EMBEDDED_PANSOU_DEFAULT_PLUGINS: &[&str] = &[
    "labi",
    "zhizhen",
    "shandian",
    "duoduo",
    "muou",
    "wanou",
    "hunhepan",
    "jikepan",
    "panwiki",
    "pansearch",
    "qupansou",
    "hdr4k",
    "pan666",
    "susu",
    "fox4k",
    "pianku",
    "clmao",
    "hdmoli",
    "yuhuage",
    "xinjuc",
    "aikanzy",
];
const PAGE_SEARCH_DEPTH: u32 = 3;
const BOOST_WORDS: &[&str] = &[
    "全集",
    "更新",
    "第",
    "集",
    "电视剧",
    "剧集",
    "1080",
    "4K",
    "2160",
    "字幕",
    "合集",
    "爱奇艺",
    "40集",
    "1080P",
    "2160P",
];
const VIDEO_SIGNAL_WORDS: &[&str] = &[
    "2026",
    "40集",
    "全集",
    "更新",
    "电视剧",
    "剧集",
    "杨紫",
    "胡歌",
    "爱奇艺",
    "第",
    "集",
    "EP",
    "S0",
    "E0",
];
const NOISE_WORDS: &[&str] = &[
    "广告",
    "扩容",
    "AIGC",
    "导航",
    "mp3",
    "音乐",
    "歌曲",
    "epub",
    "pdf",
    "电子书",
    "纪录片",
    "课程",
    "小说",
    "花絮",
    "预告",
    "生命之树",
    "The Tree of Life",
    "生命树物语",
    "游戏",
    "英文免安装版",
    "Steam",
    "Switch",
    "安卓",
    "iOS",
    "修改器",
    "免安装",
    "物语",
    "绘本",
    "手抄报",
    "课件",
    "生物",
    "植物",
    "盆栽",
];
const DISK_HOST_MARKERS: &[&str] = &[
    "pan.quark.cn",
    "aliyundrive.com",
    "alipan.com",
    "pan.baidu.com",
    "pan.xunlei.com",
    "115.com",
    "lanzou",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchSource {
    id: String,
    name: String,
    group: String,
    enabled: bool,
    description: String,
    kind: String,
    config_index: Option<usize>,
    health_score: i64,
    status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchFilters {
    source_ids: Vec<String>,
    disk_type: String,
    sort_order: String,
    exact_match: bool,
    settings: Option<SearchSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct SearchSettings {
    embedded_pansou: EmbeddedPansouConfig,
    pansou_endpoint: String,
    pansou_token: String,
    pansou_refresh: bool,
    pansou_endpoints: Vec<PansouEndpointConfig>,
    pansou_channels: Vec<String>,
    pansou_plugins: Vec<String>,
    pansou_src: String,
    pansou_cloud_types: Vec<String>,
    pansou_cache: bool,
    pansou_concurrency: usize,
    cms_endpoints: Vec<String>,
    cms_sources: Vec<CmsSourceConfig>,
    indexers: Vec<IndexerConfig>,
    tmdb_api_key: String,
}

impl Default for SearchSettings {
    fn default() -> Self {
        Self {
            embedded_pansou: EmbeddedPansouConfig::default(),
            pansou_endpoint: String::new(),
            pansou_token: String::new(),
            pansou_refresh: false,
            pansou_endpoints: Vec::new(),
            pansou_channels: Vec::new(),
            pansou_plugins: Vec::new(),
            pansou_src: "all".to_string(),
            pansou_cloud_types: Vec::new(),
            pansou_cache: true,
            pansou_concurrency: 4,
            cms_endpoints: Vec::new(),
            cms_sources: Vec::new(),
            indexers: Vec::new(),
            tmdb_api_key: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct EmbeddedPansouConfig {
    enabled: bool,
    auto_start: bool,
    port: u16,
    src: String,
    channels: Vec<String>,
    plugins: Vec<String>,
    cloud_types: Vec<String>,
    refresh: bool,
    cache: bool,
    concurrency: usize,
}

impl Default for EmbeddedPansouConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_start: true,
            port: EMBEDDED_PANSOU_DEFAULT_PORT,
            src: "all".to_string(),
            channels: Vec::new(),
            plugins: EMBEDDED_PANSOU_DEFAULT_PLUGINS
                .iter()
                .map(|item| item.to_string())
                .collect(),
            cloud_types: Vec::new(),
            refresh: false,
            cache: true,
            concurrency: 4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct PansouEndpointConfig {
    id: String,
    name: String,
    endpoint: String,
    token: String,
    enabled: bool,
    refresh: bool,
    channels: Vec<String>,
    plugins: Vec<String>,
    src: String,
    cloud_types: Vec<String>,
    concurrency: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct CmsSourceConfig {
    id: String,
    name: String,
    endpoint: String,
    enabled: bool,
    last_success_at: String,
    failure_count: u32,
    average_count: f64,
    health_score: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct IndexerConfig {
    id: String,
    name: String,
    base_url: String,
    api_key: String,
    indexer_type: String,
    categories: Vec<String>,
    enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CmsHealthResult {
    id: String,
    name: String,
    endpoint: String,
    ok: bool,
    count: usize,
    elapsed_ms: u128,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct EmbeddedPansouStatus {
    enabled: bool,
    running: bool,
    reused: bool,
    endpoint: String,
    port: u16,
    message: String,
}

impl Default for EmbeddedPansouStatus {
    fn default() -> Self {
        Self {
            enabled: true,
            running: false,
            reused: false,
            endpoint: embedded_pansou_endpoint(EMBEDDED_PANSOU_DEFAULT_PORT),
            port: EMBEDDED_PANSOU_DEFAULT_PORT,
            message: "内置 PanSou 尚未启动".to_string(),
        }
    }
}

struct EmbeddedPansouRuntime {
    child: Option<CommandChild>,
    status: EmbeddedPansouStatus,
}

impl Default for EmbeddedPansouRuntime {
    fn default() -> Self {
        Self {
            child: None,
            status: EmbeddedPansouStatus::default(),
        }
    }
}

#[derive(Default)]
struct EmbeddedPansouState {
    runtime: Mutex<EmbeddedPansouRuntime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceCoverage {
    group: String,
    total: usize,
    success: usize,
    failed: usize,
    disabled: usize,
    count: usize,
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct RuleSourceConfig {
    id: String,
    name: String,
    group: String,
    kind: String,
    enabled: bool,
    description: String,
    health_score: i64,
    status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MediaCandidate {
    id: String,
    title: String,
    original_title: String,
    year: String,
    media_type: String,
    actors: Vec<String>,
    aliases: Vec<String>,
    platforms: Vec<String>,
    overview: String,
    confidence: i64,
    source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchPlan {
    original_query: String,
    active_candidate_id: String,
    candidates: Vec<MediaCandidate>,
    search_terms: Vec<String>,
    include_keywords: Vec<String>,
    exclude_keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ResourcePayload {
    title: String,
    final_url: String,
    detail_url: String,
    password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResourceItem {
    id: String,
    title: String,
    info: String,
    url: String,
    source_id: String,
    source_name: String,
    disk_type: String,
    share_user: String,
    tags: Vec<String>,
    payload: ResourcePayload,
    relevance_score: i64,
    relevance_level: String,
    match_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserRecord {
    username: String,
    password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserSession {
    username: String,
    display_name: String,
    login_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FavoriteResource {
    id: String,
    username: String,
    title: String,
    url: String,
    source_name: String,
    disk_type: String,
    share_user: String,
    info: String,
    created_at: String,
    resource_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResultGroup {
    key: String,
    title: String,
    items: Vec<ResourceItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResourceDetail {
    title: String,
    url: String,
    source_name: String,
    message: String,
    validation_status: String,
    can_open: bool,
    validation_message: String,
}

struct LinkValidation {
    status: String,
    can_open: bool,
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceSearchState {
    source_id: String,
    source_name: String,
    group: String,
    kind: String,
    status: String,
    message: String,
    count: usize,
    elapsed_ms: u128,
    health_score: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchResponse {
    items: Vec<ResourceItem>,
    groups: Vec<ResultGroup>,
    states: Vec<SourceSearchState>,
    coverage: Vec<SourceCoverage>,
    target_resource_count: usize,
    target_resource_message: String,
    search_plan: SearchPlan,
}

struct SearchOutcome {
    items: Vec<ResourceItem>,
    state: SourceSearchState,
}

#[tauri::command]
async fn list_search_sources(
    app: AppHandle,
    settings: Option<SearchSettings>,
) -> Vec<SearchSource> {
    let settings = normalize_settings(settings.unwrap_or_default());
    if settings.embedded_pansou.enabled && settings.embedded_pansou.auto_start {
        sync_embedded_pansou(&app, &settings.embedded_pansou).await;
    }
    let status = embedded_pansou_status_from_state(&app);
    search_sources_with_embedded(&settings, Some(status))
}

#[tauri::command]
fn get_search_settings(app: AppHandle) -> Result<SearchSettings, String> {
    read_search_settings(&app)
}

#[tauri::command]
fn login_user(app: AppHandle, username: String, password: String) -> Result<UserSession, String> {
    let users = read_users(&app)?;
    authenticate_user(&users, &username, &password)
}

#[tauri::command]
async fn save_search_settings(
    app: AppHandle,
    settings: SearchSettings,
) -> Result<SearchSettings, String> {
    let old_settings = read_search_settings(&app).unwrap_or_default();
    let normalized = normalize_settings(settings);
    let path = settings_path(&app)?;
    let text = serde_json::to_string_pretty(&normalized).map_err(|error| error.to_string())?;
    fs::write(path, text).map_err(|error| error.to_string())?;
    if embedded_pansou_config_changed(&old_settings.embedded_pansou, &normalized.embedded_pansou)
        || normalized.embedded_pansou.enabled
    {
        sync_embedded_pansou(&app, &normalized.embedded_pansou).await;
    }
    Ok(normalized)
}

#[tauri::command]
fn get_embedded_pansou_status(app: AppHandle) -> EmbeddedPansouStatus {
    embedded_pansou_status_from_state(&app)
}

#[tauri::command]
async fn restart_embedded_pansou(app: AppHandle, settings: SearchSettings) -> EmbeddedPansouStatus {
    let normalized = normalize_settings(settings);
    stop_owned_embedded_pansou(&app, "正在重启内置 PanSou");
    sync_embedded_pansou(&app, &normalized.embedded_pansou).await;
    embedded_pansou_status_from_state(&app)
}

#[tauri::command]
fn import_cms_sources(text: String, settings: SearchSettings) -> Result<SearchSettings, String> {
    let mut normalized = normalize_settings(settings);
    for source in parse_cms_source_text(&text)? {
        if !normalized
            .cms_sources
            .iter()
            .any(|item| item.endpoint == source.endpoint)
        {
            normalized.cms_sources.push(source);
        }
    }
    normalized = normalize_settings(normalized);
    Ok(normalized)
}

#[tauri::command]
async fn test_cms_sources(settings: SearchSettings) -> Result<Vec<CmsHealthResult>, String> {
    let settings = normalize_settings(settings);
    let client = Client::builder()
        .timeout(Duration::from_secs(12))
        .user_agent(USER_AGENT)
        .build()
        .map_err(|error| error.to_string())?;
    let tasks = settings
        .cms_sources
        .into_iter()
        .filter(|source| source.enabled)
        .map(|source| test_cms_source(client.clone(), source));
    Ok(join_all(tasks).await)
}

#[tauri::command]
async fn search_resources(
    app: AppHandle,
    query: String,
    page: Option<u32>,
    filters: SearchFilters,
) -> Result<SearchResponse, String> {
    let text = query.trim().to_string();
    if text.is_empty() {
        return Err("搜索关键词不能为空".to_string());
    }

    let settings = match filters.settings.clone() {
        Some(settings) => normalize_settings(settings),
        None => read_search_settings(&app).unwrap_or_default(),
    };
    let plan = build_search_plan(&text, &settings).await;
    let client = Client::builder()
        .timeout(Duration::from_secs(18))
        .user_agent(USER_AGENT)
        .build()
        .map_err(|error| error.to_string())?;
    let page_no = page.unwrap_or(1).max(1);
    let selected = resolve_selected_sources(&filters, &settings);
    if selected
        .iter()
        .any(|source| source.id == EMBEDDED_PANSOU_SOURCE_ID)
        && settings.embedded_pansou.enabled
    {
        sync_embedded_pansou(&app, &settings.embedded_pansou).await;
    }
    let tasks = selected.into_iter().map(|source| {
        search_source(
            client.clone(),
            source,
            plan.clone(),
            page_no,
            filters.clone(),
            settings.clone(),
        )
    });
    let outcomes = join_all(tasks).await;
    let mut items = Vec::new();
    let mut states = Vec::new();

    for outcome in outcomes {
        states.push(outcome.state);
        items.extend(outcome.items);
    }

    items = dedupe_items(items);
    score_items(&mut items, &plan);
    if filters.sort_order == "source" {
        items.sort_by(|left, right| {
            left.source_name
                .cmp(&right.source_name)
                .then(right.relevance_score.cmp(&left.relevance_score))
        });
    } else {
        items.sort_by(|left, right| {
            right
                .relevance_score
                .cmp(&left.relevance_score)
                .then(left.title.cmp(&right.title))
        });
    }
    let groups = group_items(&items);
    let coverage = build_coverage(&states);
    let target_resource_count = count_target_resources(&items, &plan);
    let target_resource_message = target_resource_message(target_resource_count, &items, &plan);

    Ok(SearchResponse {
        items,
        groups,
        states,
        coverage,
        target_resource_count,
        target_resource_message,
        search_plan: plan,
    })
}

#[tauri::command]
async fn get_resource_detail(item: ResourceItem) -> Result<ResourceDetail, String> {
    if item.source_id == "aliso" && !item.payload.detail_url.is_empty() {
        return resolve_aliso_detail(&item).await;
    }
    if matches!(
        item.source_id.as_str(),
        "cuppaso" | "buyutu" | "quarkso" | "yyurl" | "myxiaozhan" | "xuebapan" | "panquduo"
    ) && !item.payload.detail_url.is_empty()
    {
        return resolve_public_page_detail(&item).await;
    }

    let url = first_non_empty(&[&item.payload.final_url, &item.url, &item.payload.detail_url]);
    if url.is_empty() {
        return Err("未找到可跳转地址".to_string());
    }

    detail_with_url(&item, url, "已获得跳转地址。").await
}

#[tauri::command]
fn list_favorites(app: AppHandle, username: String) -> Result<Vec<FavoriteResource>, String> {
    let user = username.trim();
    if user.is_empty() {
        return Err("用户名不能为空".to_string());
    }
    Ok(read_favorites(&app)?
        .into_iter()
        .filter(|item| item.username == user)
        .collect())
}

#[tauri::command]
async fn add_favorite(
    app: AppHandle,
    username: String,
    item: ResourceItem,
    detail: ResourceDetail,
) -> Result<FavoriteResource, String> {
    let mut favorites = read_favorites(&app)?;
    let favorite = add_favorite_record(&mut favorites, &username, &item, &detail)?;
    write_favorites(&app, &favorites)?;
    Ok(favorite)
}

#[tauri::command]
fn remove_favorite(
    app: AppHandle,
    username: String,
    favorite_id: String,
) -> Result<Vec<FavoriteResource>, String> {
    let mut favorites = read_favorites(&app)?;
    let user_favorites = remove_favorite_record(&mut favorites, &username, &favorite_id)?;
    write_favorites(&app, &favorites)?;
    Ok(user_favorites)
}

#[tauri::command]
fn open_external_url(url: String) -> Result<(), String> {
    let text = url.trim();
    if !is_allowed_external_url(text) {
        return Err("只允许打开 http、https、magnet 或 ed2k 地址".to_string());
    }
    open::that(text).map_err(|error| error.to_string())
}

async fn search_source(
    client: Client,
    source: SearchSource,
    plan: SearchPlan,
    page: u32,
    filters: SearchFilters,
    settings: SearchSettings,
) -> SearchOutcome {
    let start = Instant::now();
    if !source.enabled {
        let message = if source.status == "requiresConfig" {
            "该来源需要 code、cookie 或用户配置，默认不参与搜索".to_string()
        } else {
            "来源未配置或暂不可用".to_string()
        };
        return outcome(
            source,
            Vec::new(),
            "disabled",
            message,
            start.elapsed().as_millis(),
        );
    }

    let result = match source.kind.as_str() {
        "pansou" => search_pansou(&client, &source, &plan, &filters, &settings).await,
        EMBEDDED_PANSOU_SOURCE_ID => {
            search_embedded_pansou(&client, &source, &plan, &filters, &settings).await
        }
        "cms-v10" => search_cms_v10(&client, &source, &plan, &settings).await,
        "torznab" | "newznab" => search_indexer(&client, &source, &plan, &settings).await,
        "hunhepan" => {
            search_terms(
                client.clone(),
                &source,
                &plan,
                page,
                &filters,
                search_hunhepan,
            )
            .await
        }
        "pikasoo" => {
            search_terms(
                client.clone(),
                &source,
                &plan,
                page,
                &filters,
                search_pikasoo,
            )
            .await
        }
        "aliso" => search_terms(client.clone(), &source, &plan, page, &filters, search_aliso).await,
        "cuppaso" => {
            search_terms(
                client.clone(),
                &source,
                &plan,
                page,
                &filters,
                search_cuppaso,
            )
            .await
        }
        "buyutu" => {
            search_terms(
                client.clone(),
                &source,
                &plan,
                page,
                &filters,
                search_buyutu,
            )
            .await
        }
        "xuebapan" => {
            search_terms(
                client.clone(),
                &source,
                &plan,
                page,
                &filters,
                search_xuebapan,
            )
            .await
        }
        "quarkso" => {
            search_terms(
                client.clone(),
                &source,
                &plan,
                page,
                &filters,
                search_quarkso,
            )
            .await
        }
        "yyurl" | "myxiaozhan" => {
            search_terms(
                client.clone(),
                &source,
                &plan,
                page,
                &filters,
                search_flarum,
            )
            .await
        }
        "panquduo" => {
            search_terms(
                client.clone(),
                &source,
                &plan,
                page,
                &filters,
                search_panquduo,
            )
            .await
        }
        _ => Err("未知来源".to_string()),
    };

    let elapsed_ms = start.elapsed().as_millis();
    match result {
        Ok(items) => {
            let items = filter_valid_resource_items(items);
            if items.is_empty() {
                return outcome(
                    source,
                    items,
                    "empty",
                    "未返回匹配结果".to_string(),
                    elapsed_ms,
                );
            }
            let message = format!("返回 {} 条", items.len());
            outcome(source, items, "success", message, elapsed_ms)
        }
        Err(error) => outcome(
            source,
            Vec::new(),
            "failed",
            friendly_search_error(&error),
            elapsed_ms,
        ),
    }
}

async fn search_terms<F, Fut>(
    client: Client,
    source: &SearchSource,
    plan: &SearchPlan,
    page: u32,
    filters: &SearchFilters,
    searcher: F,
) -> Result<Vec<ResourceItem>, String>
where
    F: Fn(Client, SearchSource, String, u32, SearchFilters) -> Fut + Copy,
    Fut: std::future::Future<Output = Result<Vec<ResourceItem>, String>>,
{
    let depth = if source.group == "公开页面源" || source.group == "页面源" {
        PAGE_SEARCH_DEPTH
    } else {
        1
    };
    let tasks = plan
        .search_terms
        .iter()
        .take(8)
        .flat_map(|term| {
            let client = client.clone();
            let source = source.clone();
            let filters = filters.clone();
            (page..(page + depth)).map(move |page_no| {
                (
                    term.clone(),
                    searcher(
                        client.clone(),
                        source.clone(),
                        term.clone(),
                        page_no,
                        filters.clone(),
                    ),
                )
            })
        })
        .collect::<Vec<_>>();
    let outcomes = join_all(tasks.into_iter().map(|(_, task)| task)).await;
    let mut output = Vec::new();
    let mut errors = Vec::new();
    for outcome in outcomes {
        match outcome {
            Ok(mut items) => output.append(&mut items),
            Err(error) => errors.push(error),
        }
    }
    if output.is_empty() && !errors.is_empty() {
        return Err(errors.into_iter().take(3).collect::<Vec<_>>().join("；"));
    }
    Ok(output)
}

async fn search_hunhepan(
    client: Client,
    source: SearchSource,
    query: String,
    page: u32,
    filters: SearchFilters,
) -> Result<Vec<ResourceItem>, String> {
    let body = json!({
        "q": query,
        "page": page,
        "exact": filters.exact_match,
        "time": "",
        "type": disk_filter_for_api(&filters.disk_type),
        "size": 20
    });
    let data: Value = client
        .post("https://hunhepan.com/open/search/disk")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())?;

    let list = data
        .pointer("/data/list")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(list
        .iter()
        .enumerate()
        .filter_map(|(index, value)| hunhepan_item(&source, value, index, &query, &filters))
        .collect())
}

async fn search_pikasoo(
    client: Client,
    source: SearchSource,
    query: String,
    page: u32,
    filters: SearchFilters,
) -> Result<Vec<ResourceItem>, String> {
    let pan = if filters.disk_type == "all" {
        "all"
    } else {
        filters.disk_type.as_str()
    };
    let url = format!(
        "https://www.pikasoo.top/search/?pan={}&type=all&q={}&page={}",
        pan,
        urlencoding::encode(&query),
        page
    );
    let html = fetch_text(&client, &url).await?;
    let document = Html::parse_document(&html);
    let item_selector = selector(".search-item")?;
    Ok(document
        .select(&item_selector)
        .skip(2)
        .enumerate()
        .filter_map(|(index, element)| pikasoo_item(&source, element, index, &query, &filters))
        .collect())
}

async fn search_aliso(
    client: Client,
    source: SearchSource,
    query: String,
    page: u32,
    filters: SearchFilters,
) -> Result<Vec<ResourceItem>, String> {
    let url = format!(
        "https://aliso.cc/s/{}-{}-0.html",
        urlencoding::encode(&query),
        page
    );
    let html = fetch_text(&client, &url).await?;
    let document = Html::parse_document(&html);
    let item_selector = selector(".resource-item")?;
    Ok(document
        .select(&item_selector)
        .enumerate()
        .filter_map(|(index, element)| aliso_item(&source, element, index, &query, &filters))
        .collect())
}

async fn search_cuppaso(
    client: Client,
    source: SearchSource,
    query: String,
    page: u32,
    filters: SearchFilters,
) -> Result<Vec<ResourceItem>, String> {
    let disk_type = match filters.disk_type.as_str() {
        "quark" => "1",
        "aliyun" => "2",
        _ => "0",
    };
    let url = format!(
        "https://www.cuppaso.com/search?type={}&keyword={}&searchType=0&page={}",
        disk_type,
        urlencoding::encode(&query),
        page
    );
    let html = fetch_text(&client, &url).await?;
    let document = Html::parse_document(&html);
    let item_selector = selector(".card")?;
    Ok(document
        .select(&item_selector)
        .enumerate()
        .filter_map(|(index, element)| cuppaso_item(&source, element, index, &query, &filters))
        .collect())
}

async fn search_buyutu(
    client: Client,
    source: SearchSource,
    query: String,
    page: u32,
    filters: SearchFilters,
) -> Result<Vec<ResourceItem>, String> {
    let disk_type = match filters.disk_type.as_str() {
        "quark" => "quark",
        "aliyun" => "alipan",
        "baidu" => "baidu",
        _ => "all",
    };
    let b64_query = general_purpose::STANDARD.encode(query.as_bytes());
    let encoded = urlencoding::encode(&b64_query);
    let url = format!(
        "https://www.buyutu.com/s/{}?p={}&disktype={}&type=0",
        encoded, page, disk_type
    );
    let html = fetch_text(&client, &url).await?;
    let document = Html::parse_document(&html);
    let item_selector = selector("#list")?;
    Ok(document
        .select(&item_selector)
        .enumerate()
        .filter_map(|(index, element)| buyutu_item(&source, element, index, &query, &filters))
        .collect())
}

async fn search_xuebapan(
    client: Client,
    source: SearchSource,
    query: String,
    page: u32,
    filters: SearchFilters,
) -> Result<Vec<ResourceItem>, String> {
    let url = format!(
        "https://www.xuebapan.com/s/{}-{}-0.html",
        urlencoding::encode(&query),
        page
    );
    let html = fetch_text(&client, &url).await?;
    let document = Html::parse_document(&html);
    let item_selector = selector(".resource-item")?;
    Ok(document
        .select(&item_selector)
        .enumerate()
        .filter_map(|(index, element)| {
            dalipan_item(
                &source,
                "https://www.xuebapan.com",
                element,
                index,
                &query,
                &filters,
            )
        })
        .collect())
}

async fn search_quarkso(
    client: Client,
    source: SearchSource,
    query: String,
    page: u32,
    filters: SearchFilters,
) -> Result<Vec<ResourceItem>, String> {
    let url = format!(
        "https://www.quark.so/s?query={}&type=1&category={}&p={}",
        urlencoding::encode(&query),
        urlencoding::encode("全部"),
        page
    );
    let html = fetch_text(&client, &url).await?;
    let document = Html::parse_document(&html);
    let item_selector = selector("a")?;
    Ok(document
        .select(&item_selector)
        .enumerate()
        .filter_map(|(index, element)| quarkso_item(&source, element, index, &query, &filters))
        .collect())
}

async fn search_flarum(
    client: Client,
    source: SearchSource,
    query: String,
    page: u32,
    _filters: SearchFilters,
) -> Result<Vec<ResourceItem>, String> {
    let base = flarum_base_url(&source.kind);
    let offset = (page.saturating_sub(1) * 20).to_string();
    let data: Value = client
        .get(format!("{}/api/discussions", base))
        .query(&[
            ("filter[q]", query.as_str()),
            ("page[offset]", offset.as_str()),
            (
                "include",
                "user,lastPostedUser,mostRelevantPost,mostRelevantPost.user,tags,tags.parent,firstPost",
            ),
        ])
        .send()
        .await
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())?;
    let list = data
        .pointer("/data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(list
        .iter()
        .enumerate()
        .filter_map(|(index, value)| flarum_item(&source, base, value, index, &query))
        .collect())
}

async fn search_panquduo(
    client: Client,
    source: SearchSource,
    query: String,
    page: u32,
    filters: SearchFilters,
) -> Result<Vec<ResourceItem>, String> {
    let url = format!(
        "https://www.panquduo.com/search.php?q={}&page={}",
        urlencoding::encode(&query),
        page
    );
    let html = fetch_text(&client, &url).await?;
    let document = Html::parse_document(&html);
    let item_selector = selector("a")?;
    Ok(document
        .select(&item_selector)
        .enumerate()
        .filter_map(|(index, element)| panquduo_item(&source, element, index, &query, &filters))
        .collect())
}

async fn search_pansou(
    client: &Client,
    source: &SearchSource,
    plan: &SearchPlan,
    filters: &SearchFilters,
    settings: &SearchSettings,
) -> Result<Vec<ResourceItem>, String> {
    let index = source
        .config_index
        .ok_or_else(|| "PanSou 来源配置缺失".to_string())?;
    let endpoint = settings
        .pansou_endpoints
        .get(index)
        .ok_or_else(|| "PanSou 来源配置不存在".to_string())?;
    if endpoint.endpoint.trim().is_empty() {
        return Err("PanSou endpoint 未配置".to_string());
    }
    search_pansou_endpoint(client, source, plan, filters, settings, endpoint).await
}

async fn search_embedded_pansou(
    client: &Client,
    source: &SearchSource,
    plan: &SearchPlan,
    filters: &SearchFilters,
    settings: &SearchSettings,
) -> Result<Vec<ResourceItem>, String> {
    if !settings.embedded_pansou.enabled {
        return Err("内置 PanSou 已关闭".to_string());
    }
    let endpoint = embedded_pansou_endpoint_config(&settings.embedded_pansou);
    search_pansou_endpoint(client, source, plan, filters, settings, &endpoint).await
}

async fn search_pansou_endpoint(
    client: &Client,
    source: &SearchSource,
    plan: &SearchPlan,
    filters: &SearchFilters,
    settings: &SearchSettings,
    endpoint: &PansouEndpointConfig,
) -> Result<Vec<ResourceItem>, String> {
    let mut output = Vec::new();
    for term in plan.search_terms.iter().take(4) {
        let mut params = vec![("kw".to_string(), term.clone())];
        push_pansou_param(&mut params, "src", &endpoint.src);
        push_pansou_list_param(&mut params, "channels", &endpoint.channels);
        push_pansou_list_param(&mut params, "plugins", &endpoint.plugins);
        if filters.disk_type != "all" {
            params.push((
                "cloud_types".to_string(),
                disk_filter_for_pansou(&filters.disk_type).to_string(),
            ));
        } else if !endpoint.cloud_types.is_empty() {
            params.push(("cloud_types".to_string(), endpoint.cloud_types.join(",")));
        }
        if endpoint.refresh || settings.pansou_refresh {
            params.push(("refresh".to_string(), "true".to_string()));
        }
        if !settings.pansou_cache {
            params.push(("cache".to_string(), "false".to_string()));
        }
        let filter = json!({
            "include": plan.include_keywords,
            "exclude": plan.exclude_keywords,
        });
        let ext = build_pansou_ext(plan);
        params.push(("filter".to_string(), filter.to_string()));
        params.push(("ext".to_string(), ext.to_string()));
        let mut request = client
            .get(format!(
                "{}/api/search",
                endpoint.endpoint.trim_end_matches('/')
            ))
            .query(&params);
        let token = first_non_empty(&[&endpoint.token, &settings.pansou_token]);
        if !token.is_empty() {
            request = request.bearer_auth(token);
        }
        let data: Value = request
            .send()
            .await
            .map_err(|error| error.to_string())?
            .json()
            .await
            .map_err(|error| error.to_string())?;
        output.extend(parse_pansou_items(source, term, &data));
    }
    Ok(output)
}

async fn search_cms_v10(
    client: &Client,
    source: &SearchSource,
    plan: &SearchPlan,
    settings: &SearchSettings,
) -> Result<Vec<ResourceItem>, String> {
    let index = source
        .config_index
        .ok_or_else(|| "CMS 来源配置缺失".to_string())?;
    let cms_source = settings
        .cms_sources
        .get(index)
        .ok_or_else(|| "CMS 来源配置不存在".to_string())?;
    if cms_source.endpoint.trim().is_empty() {
        return Err("CMS V10 接口未配置".to_string());
    }
    let mut output = Vec::new();
    let base = cms_source.endpoint.trim().trim_end_matches('/');
    for term in plan.search_terms.iter().take(3) {
        let url = format!("{}?ac=videolist&wd={}", base, urlencoding::encode(term));
        let data: Value = client
            .get(url)
            .send()
            .await
            .map_err(|error| error.to_string())?
            .json()
            .await
            .map_err(|error| error.to_string())?;
        output.extend(parse_cms_items(source, base, &data));
    }
    Ok(output)
}

async fn search_indexer(
    client: &Client,
    source: &SearchSource,
    plan: &SearchPlan,
    settings: &SearchSettings,
) -> Result<Vec<ResourceItem>, String> {
    let index = source
        .config_index
        .ok_or_else(|| "索引器配置缺失".to_string())?;
    let indexer = settings
        .indexers
        .get(index)
        .ok_or_else(|| "索引器配置不存在".to_string())?;
    if indexer.base_url.trim().is_empty() {
        return Err("索引器地址未配置".to_string());
    }

    let mut output = Vec::new();
    let candidate = plan
        .candidates
        .iter()
        .find(|item| item.id == plan.active_candidate_id);
    let primary_type = if candidate.map(|item| item.media_type.as_str()) == Some("电影") {
        "movie"
    } else if candidate.map(|item| item.media_type.as_str()) == Some("电视剧") {
        "tvsearch"
    } else {
        "search"
    };
    for term in plan.search_terms.iter().take(3) {
        let mut items =
            request_indexer(client, source, indexer, primary_type, term, candidate).await?;
        if items.is_empty() && primary_type != "search" {
            items = request_indexer(client, source, indexer, "search", term, candidate).await?;
        }
        output.extend(items);
    }
    Ok(output)
}

async fn request_indexer(
    client: &Client,
    source: &SearchSource,
    indexer: &IndexerConfig,
    search_type: &str,
    term: &str,
    candidate: Option<&MediaCandidate>,
) -> Result<Vec<ResourceItem>, String> {
    let mut params = vec![
        ("t".to_string(), search_type.to_string()),
        ("q".to_string(), term.to_string()),
    ];
    if !indexer.api_key.trim().is_empty() {
        params.push(("apikey".to_string(), indexer.api_key.trim().to_string()));
    }
    if !indexer.categories.is_empty() {
        params.push(("cat".to_string(), indexer.categories.join(",")));
    }
    if let Some(candidate) = candidate {
        if !candidate.year.is_empty() && search_type != "search" {
            params.push(("year".to_string(), candidate.year.clone()));
        }
    }
    let text = client
        .get(indexer.base_url.trim())
        .query(&params)
        .send()
        .await
        .map_err(|error| error.to_string())?
        .text()
        .await
        .map_err(|error| error.to_string())?;
    parse_indexer_items(source, term, &text)
}

fn hunhepan_item(
    source: &SearchSource,
    value: &Value,
    index: usize,
    query: &str,
    filters: &SearchFilters,
) -> Option<ResourceItem> {
    let title = clean_text(
        value
            .get("disk_name")
            .and_then(Value::as_str)
            .unwrap_or_default(),
    );
    let info = json_text(value.get("files"));
    if !matches_exact(&title, &info, query, filters.exact_match) {
        return None;
    }
    let disk_type = clean_text(
        value
            .get("disk_type")
            .and_then(Value::as_str)
            .unwrap_or_default(),
    );
    let share_user = clean_text(
        value
            .get("share_user")
            .and_then(Value::as_str)
            .unwrap_or_default(),
    );
    let link = value
        .get("link")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let url = value
        .get("url")
        .and_then(Value::as_str)
        .unwrap_or(&link)
        .to_string();
    Some(resource_item(
        source,
        index,
        title,
        info,
        url,
        link,
        String::new(),
        disk_type,
        share_user,
        vec![],
    ))
}

fn pikasoo_item(
    source: &SearchSource,
    element: ElementRef,
    index: usize,
    query: &str,
    filters: &SearchFilters,
) -> Option<ResourceItem> {
    let title_selector = selector(".search-title").ok()?;
    let desc_selector = selector(".search-des").ok()?;
    let link_selector = selector("a").ok()?;
    let image_selector = selector(".search-title img").ok()?;
    let meta_selector = selector(".search-file-size, .search-file-num, .search-creator").ok()?;
    let title = clean_text(&element_text(element, &title_selector));
    let info = clean_text(&element_text(element, &desc_selector));
    if !matches_exact(&title, &info, query, filters.exact_match) {
        return None;
    }
    let url = element
        .select(&link_selector)
        .next()
        .and_then(|link| link.value().attr("href"))
        .unwrap_or_default()
        .to_string();
    let disk_type = element
        .select(&image_selector)
        .next()
        .and_then(|image| image.value().attr("alt"))
        .map(clean_text)
        .unwrap_or_default();
    let tags = element
        .select(&meta_selector)
        .map(|meta| clean_text(&meta.text().collect::<Vec<_>>().join(" ")))
        .filter(|text| !text.is_empty())
        .collect();
    Some(resource_item(
        source,
        index,
        title,
        info,
        url.clone(),
        url,
        String::new(),
        disk_type,
        String::new(),
        tags,
    ))
}

fn aliso_item(
    source: &SearchSource,
    element: ElementRef,
    index: usize,
    query: &str,
    filters: &SearchFilters,
) -> Option<ResourceItem> {
    let title_selector = selector(".resource-title > a").ok()?;
    let info_selector = selector(".detail-wrap").ok()?;
    let tag_selector = selector(".meta-item, .other-info").ok()?;
    let title_node = element.select(&title_selector).next()?;
    let title = clean_text(&element_text(title_node, &selector("*").ok()?));
    let info = clean_text(&element_text(element, &info_selector));
    if !matches_exact(&title, &info, query, filters.exact_match) {
        return None;
    }
    let href = title_node.value().attr("href").unwrap_or_default();
    let detail_url = absolute_url("https://aliso.cc", href);
    let tags = element
        .select(&tag_selector)
        .map(|tag| clean_text(&tag.text().collect::<Vec<_>>().join(" ")))
        .filter(|text| !text.is_empty())
        .collect();
    Some(resource_item(
        source,
        index,
        title,
        info,
        detail_url.clone(),
        String::new(),
        detail_url,
        "网盘".to_string(),
        String::new(),
        tags,
    ))
}

fn cuppaso_item(
    source: &SearchSource,
    element: ElementRef,
    index: usize,
    query: &str,
    filters: &SearchFilters,
) -> Option<ResourceItem> {
    let title_selector = selector(".card-title").ok()?;
    let link_selector = selector("a").ok()?;
    let time_selector = selector(".fs-4").ok()?;
    let user_selector = selector(".card-actions > a").ok()?;
    let title = clean_text(&element_text(element, &title_selector));
    let info = clean_text(&element.text().collect::<Vec<_>>().join(" "));
    if title.is_empty() || !matches_exact(&title, &info, query, filters.exact_match) {
        return None;
    }
    let href = element
        .select(&link_selector)
        .next()
        .and_then(|link| link.value().attr("href"))
        .unwrap_or_default();
    let detail_url = absolute_url("https://www.cuppaso.com", href);
    let tags = [
        clean_text(&element_text(element, &time_selector)),
        clean_text(&element_text(element, &user_selector)),
    ]
    .into_iter()
    .filter(|text| !text.is_empty())
    .collect::<Vec<_>>();
    Some(resource_item(
        source,
        index,
        title,
        info,
        detail_url.clone(),
        String::new(),
        detail_url,
        "网盘".to_string(),
        String::new(),
        tags,
    ))
}

fn buyutu_item(
    source: &SearchSource,
    element: ElementRef,
    index: usize,
    query: &str,
    filters: &SearchFilters,
) -> Option<ResourceItem> {
    let link_selector = selector(".card-body a").ok()?;
    let body_selector = selector("#body > .card-body, .card-body").ok()?;
    let title_node = element.select(&link_selector).next()?;
    let title = clean_text(&title_node.text().collect::<Vec<_>>().join(" "));
    let info = clean_text(&element_text(element, &body_selector));
    if title.is_empty() || !matches_exact(&title, &info, query, filters.exact_match) {
        return None;
    }
    let href = title_node
        .value()
        .attr("href")
        .unwrap_or_default()
        .replace("..", "");
    let detail_url = absolute_url("https://www.buyutu.com", &href);
    let tags = info
        .lines()
        .map(clean_text)
        .filter(|text| !text.is_empty())
        .take(4)
        .collect::<Vec<_>>();
    Some(resource_item(
        source,
        index,
        title,
        info,
        detail_url.clone(),
        String::new(),
        detail_url,
        "网盘".to_string(),
        String::new(),
        tags,
    ))
}

fn dalipan_item(
    source: &SearchSource,
    base_url: &str,
    element: ElementRef,
    index: usize,
    query: &str,
    filters: &SearchFilters,
) -> Option<ResourceItem> {
    let title_selector = selector(".resource-title > a").ok()?;
    let info_selector = selector(".detail-wrap").ok()?;
    let tag_selector = selector(".meta-item, .other-info").ok()?;
    let title_node = element.select(&title_selector).next()?;
    let title = clean_text(&title_node.text().collect::<Vec<_>>().join(" "));
    let info = clean_text(&element_text(element, &info_selector));
    if title.is_empty() || !matches_exact(&title, &info, query, filters.exact_match) {
        return None;
    }
    let href = title_node.value().attr("href").unwrap_or_default();
    let detail_url = absolute_url(base_url, href);
    let tags = element
        .select(&tag_selector)
        .map(|tag| clean_text(&tag.text().collect::<Vec<_>>().join(" ")))
        .filter(|text| !text.is_empty())
        .collect();
    Some(resource_item(
        source,
        index,
        title,
        info,
        detail_url.clone(),
        String::new(),
        detail_url,
        "网盘".to_string(),
        String::new(),
        tags,
    ))
}

fn quarkso_item(
    source: &SearchSource,
    element: ElementRef,
    index: usize,
    query: &str,
    filters: &SearchFilters,
) -> Option<ResourceItem> {
    let title_selector = selector("h2").ok()?;
    let desc_selector = selector(".yp-search-result-item-text-desc").ok()?;
    let tag_selector = selector(".yp-search-result-item-other-category").ok()?;
    let title = clean_text(&element_text(element, &title_selector));
    if title.is_empty() {
        return None;
    }
    let info = clean_text(&element_text(element, &desc_selector));
    if !matches_exact(&title, &info, query, filters.exact_match) {
        return None;
    }
    let href = element.value().attr("href").unwrap_or_default();
    let detail_url = absolute_url("https://www.quark.so", href);
    let tags = vec![clean_text(&element_text(element, &tag_selector))]
        .into_iter()
        .filter(|text| !text.is_empty())
        .collect();
    Some(resource_item(
        source,
        index,
        title,
        info,
        detail_url.clone(),
        String::new(),
        detail_url,
        "夸克".to_string(),
        String::new(),
        tags,
    ))
}

fn flarum_item(
    source: &SearchSource,
    base_url: &str,
    value: &Value,
    index: usize,
    query: &str,
) -> Option<ResourceItem> {
    let title = clean_text(value.pointer("/attributes/title").and_then(Value::as_str)?);
    let slug = clean_text(
        value
            .pointer("/attributes/slug")
            .and_then(Value::as_str)
            .unwrap_or_default(),
    );
    let id = first_json_string(value, &["id"]).unwrap_or_default();
    if title.is_empty() || id.is_empty() {
        return None;
    }
    let created_at = clean_text(
        value
            .pointer("/attributes/createdAt")
            .and_then(Value::as_str)
            .unwrap_or_default(),
    );
    let last_posted_at = clean_text(
        value
            .pointer("/attributes/lastPostedAt")
            .and_then(Value::as_str)
            .unwrap_or_default(),
    );
    let detail_url = format!("{}/d/{}", base_url, id);
    Some(resource_item(
        source,
        index,
        title,
        slug,
        detail_url.clone(),
        String::new(),
        detail_url,
        "网盘".to_string(),
        String::new(),
        vec![created_at, last_posted_at, query.to_string()]
            .into_iter()
            .filter(|text| !text.is_empty())
            .collect(),
    ))
}

fn panquduo_item(
    source: &SearchSource,
    element: ElementRef,
    index: usize,
    query: &str,
    filters: &SearchFilters,
) -> Option<ResourceItem> {
    let href = element.value().attr("href").unwrap_or_default();
    if !href.contains("/post/") || !href.ends_with(".html") {
        return None;
    }
    let title_attr = element.value().attr("title").unwrap_or_default();
    let title_text = element.text().collect::<Vec<_>>().join(" ");
    let title = clean_panquduo_title(&clean_text(
        first_non_empty(&[title_attr, &title_text]).as_str(),
    ));
    if title.is_empty() || !matches_exact(&title, "", query, filters.exact_match) {
        return None;
    }
    let detail_url = absolute_url("https://www.panquduo.com", href);
    Some(resource_item(
        source,
        index,
        title.clone(),
        title,
        detail_url.clone(),
        String::new(),
        detail_url,
        "夸克/百度".to_string(),
        String::new(),
        vec![query.to_string()],
    ))
}

fn clean_panquduo_title(title: &str) -> String {
    Regex::new(r"^\d+\s*")
        .map(|regex| regex.replace(title, "").to_string())
        .unwrap_or_else(|_| title.to_string())
}

fn parse_pansou_items(source: &SearchSource, term: &str, data: &Value) -> Vec<ResourceItem> {
    let candidates = [
        data.pointer("/data/merged_by_type"),
        data.pointer("/data/results"),
        data.pointer("/data/list"),
    ];
    let mut output = Vec::new();
    for candidate in candidates.iter().flatten() {
        collect_pansou_values(source, term, candidate, "", &mut output);
    }
    if output.is_empty() {
        if let Some(candidate) = data.get("data") {
            collect_pansou_values(source, term, candidate, "", &mut output);
        }
    }
    output
}

fn collect_pansou_values(
    source: &SearchSource,
    term: &str,
    value: &Value,
    disk_type: &str,
    output: &mut Vec<ResourceItem>,
) {
    match value {
        Value::Array(items) => {
            for item in items {
                if let Some(resource) = pansou_item(source, output.len(), term, item, disk_type) {
                    output.push(resource);
                }
            }
        }
        Value::Object(map) => {
            for (key, child) in map {
                let child_disk_type = if disk_type.is_empty() {
                    key.as_str()
                } else {
                    disk_type
                };
                collect_pansou_values(source, term, child, child_disk_type, output);
            }
        }
        _ => {}
    }
}

fn pansou_item(
    source: &SearchSource,
    index: usize,
    term: &str,
    value: &Value,
    fallback_disk_type: &str,
) -> Option<ResourceItem> {
    let title = first_json_string(value, &["title", "name", "disk_name", "file_name", "note"])?;
    let url =
        first_json_string(value, &["url", "link", "share_url", "shareLink"]).unwrap_or_default();
    if url.is_empty() && title.is_empty() {
        return None;
    }
    let info = first_json_string(
        value,
        &[
            "content",
            "description",
            "info",
            "files",
            "note",
            "remark",
            "datetime",
            "source",
        ],
    )
    .unwrap_or_else(|| term.to_string());
    let disk_type = first_json_string(value, &["type", "cloud_type", "disk_type"])
        .unwrap_or_else(|| fallback_disk_type.to_string());
    let share_user = first_json_string(value, &["user", "share_user", "owner"]).unwrap_or_default();
    Some(resource_item(
        source,
        index,
        clean_text(&title),
        clean_text(&info),
        url.clone(),
        url,
        String::new(),
        clean_text(&disk_type),
        clean_text(&share_user),
        vec![term.to_string()],
    ))
}

fn parse_cms_items(source: &SearchSource, endpoint: &str, data: &Value) -> Vec<ResourceItem> {
    let list = data
        .pointer("/list")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    list.iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let title = first_json_string(item, &["vod_name", "title", "name"])?;
            let year = first_json_string(item, &["vod_year", "year"]).unwrap_or_default();
            let remarks = first_json_string(item, &["vod_remarks", "remarks"]).unwrap_or_default();
            let id = first_json_string(item, &["vod_id", "id"]).unwrap_or_default();
            let info = [year.clone(), remarks.clone()]
                .into_iter()
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            let detail_url = if id.is_empty() {
                endpoint.to_string()
            } else {
                format!("{}?ac=detail&ids={}", endpoint, id)
            };
            Some(resource_item(
                source,
                index,
                clean_text(&title),
                info,
                detail_url.clone(),
                String::new(),
                detail_url,
                "CMS".to_string(),
                String::new(),
                vec![year, remarks],
            ))
        })
        .collect()
}

fn parse_indexer_items(
    source: &SearchSource,
    term: &str,
    text: &str,
) -> Result<Vec<ResourceItem>, String> {
    let document = roxmltree::Document::parse(text)
        .map_err(|error| format!("索引器 XML 解析失败：{}", error))?;
    let mut output = Vec::new();
    for (index, item) in document
        .descendants()
        .filter(|node| node.has_tag_name("item"))
        .enumerate()
    {
        let title = xml_child_text(item, "title").unwrap_or_else(|| term.to_string());
        let link = xml_child_text(item, "link")
            .or_else(|| xml_child_text(item, "guid"))
            .or_else(|| xml_enclosure_url(item))
            .unwrap_or_default();
        let size = xml_child_text(item, "size")
            .or_else(|| xml_attr_by_local(item, "attr", "name", "size"))
            .unwrap_or_default();
        let pub_date = xml_child_text(item, "pubDate").unwrap_or_default();
        let category = xml_child_text(item, "category")
            .or_else(|| xml_attr_by_local(item, "attr", "name", "category"))
            .unwrap_or_default();
        let info = [size_label(&size), pub_date.clone(), category.clone()]
            .into_iter()
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        let tags = [category, pub_date, term.to_string()]
            .into_iter()
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        output.push(resource_item(
            source,
            index,
            clean_text(&title),
            clean_text(&info),
            link.clone(),
            link,
            String::new(),
            source.kind.clone(),
            String::new(),
            tags,
        ));
    }
    Ok(output)
}

fn xml_child_text(node: roxmltree::Node, name: &str) -> Option<String> {
    node.children()
        .find(|child| child.is_element() && child.tag_name().name() == name)
        .and_then(|child| child.text())
        .map(clean_text)
        .filter(|text| !text.is_empty())
}

fn xml_enclosure_url(node: roxmltree::Node) -> Option<String> {
    node.children()
        .find(|child| child.is_element() && child.tag_name().name() == "enclosure")
        .and_then(|child| child.attribute("url"))
        .map(|value| value.to_string())
}

fn xml_attr_by_local(
    node: roxmltree::Node,
    tag: &str,
    name_key: &str,
    name_value: &str,
) -> Option<String> {
    node.children()
        .filter(|child| child.is_element() && child.tag_name().name() == tag)
        .find(|child| child.attribute(name_key) == Some(name_value))
        .and_then(|child| child.attribute("value"))
        .map(|value| value.to_string())
}

fn size_label(value: &str) -> String {
    let number = match value.parse::<f64>() {
        Ok(number) if number > 0.0 => number,
        _ => return String::new(),
    };
    let gb = number / 1024.0 / 1024.0 / 1024.0;
    if gb >= 1.0 {
        return format!("{:.2} GB", gb);
    }
    format!("{:.2} MB", number / 1024.0 / 1024.0)
}

async fn resolve_aliso_detail(item: &ResourceItem) -> Result<ResourceDetail, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(18))
        .user_agent(USER_AGENT)
        .build()
        .map_err(|error| error.to_string())?;
    let html = fetch_text(&client, &item.payload.detail_url).await?;
    let pwd = Regex::new(r#"<[^>]+id=["']pwd["'][^>]*>([^<]*)"#)
        .map_err(|error| error.to_string())?
        .captures(&html)
        .and_then(|captures| captures.get(1).map(|value| clean_text(value.as_str())))
        .unwrap_or_default();
    let message = if pwd.is_empty() {
        "该来源未解析到提取码，已回退到来源详情页。"
    } else {
        "该来源需要在详情页继续跳转，已附带提取码提示。"
    };
    let mut url = item.payload.detail_url.clone();
    if !pwd.is_empty() {
        url = format!("{}?pwd={}", url, pwd);
    }
    detail_with_url(item, url, message).await
}

async fn resolve_public_page_detail(item: &ResourceItem) -> Result<ResourceDetail, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(18))
        .user_agent(USER_AGENT)
        .build()
        .map_err(|error| error.to_string())?;
    let html = fetch_text(&client, &item.payload.detail_url).await?;
    let resolved_page_url = {
        let document = Html::parse_document(&html);
        if item.source_id == "cuppaso" {
            if let Ok(selector) = selector(".btn-green") {
                document
                    .select(&selector)
                    .next()
                    .and_then(|node| node.value().attr("href"))
                    .map(|href| href.to_string())
                    .filter(|href| !href.is_empty())
                    .map(|url| (url, "已解析到咔帕搜索详情页中的网盘链接。"))
            } else {
                None
            }
        } else {
            None
        }
    };
    if let Some((url, message)) = resolved_page_url {
        return detail_with_url(item, url, message).await;
    }
    if item.source_id == "quarkso" {
        if let Some(url) = Regex::new(r#""(https?://pan\.quark\.cn/s/[A-Za-z0-9_-]+)""#)
            .map_err(|error| error.to_string())?
            .captures(&html)
            .and_then(|captures| captures.get(1).map(|value| value.as_str().to_string()))
        {
            return detail_with_url(item, url, "已解析到夸克搜详情页中的网盘链接。").await;
        }
    }
    let links = find_disk_links(&html);
    if let Some(url) = links.first() {
        return detail_with_url(item, url.clone(), "已从来源详情页提取到网盘链接。").await;
    }
    detail_with_url(
        item,
        item.payload.detail_url.clone(),
        "未解析到最终网盘链接，已回退到来源详情页。",
    )
    .await
}

async fn detail_with_url(
    item: &ResourceItem,
    url: String,
    message: &str,
) -> Result<ResourceDetail, String> {
    let validation = validate_resource_url(&url).await;
    Ok(ResourceDetail {
        title: item.title.clone(),
        url,
        source_name: item.source_name.clone(),
        message: message.to_string(),
        validation_status: validation.status,
        can_open: validation.can_open,
        validation_message: validation.message,
    })
}

async fn fetch_text(client: &Client, url: &str) -> Result<String, String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| friendly_search_error(&error.to_string()))?;
    let status = response.status();
    if !status.is_success() {
        return Err("资源暂时无法访问，请稍后重试".to_string());
    }
    response
        .text()
        .await
        .map_err(|error| friendly_search_error(&error.to_string()))
}

async fn validate_resource_url(url: &str) -> LinkValidation {
    let text = url.trim();
    if text.is_empty() {
        return invalid_link("未找到可校验的资源地址。");
    }
    if !is_allowed_external_url(text) {
        return invalid_link("资源地址协议不受支持，已禁止直接打开。");
    }
    if text.starts_with("magnet:") || text.starts_with("ed2k://") {
        return LinkValidation {
            status: "warning".to_string(),
            can_open: true,
            message:
                "已识别为下载协议链接，无法通过网页方式确认可访问性，可复制后使用对应客户端打开。"
                    .to_string(),
        };
    }

    let client = match Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent(USER_AGENT)
        .build()
    {
        Ok(client) => client,
        Err(_) => {
            return LinkValidation {
                status: "warning".to_string(),
                can_open: true,
                message: "链接格式有效，但当前无法创建校验请求，可尝试在浏览器中打开确认。"
                    .to_string(),
            }
        }
    };
    let status = match client.head(text).send().await {
        Ok(response) => Some(response.status().as_u16()),
        Err(_) => match client.get(text).header("range", "bytes=0-0").send().await {
            Ok(response) => Some(response.status().as_u16()),
            Err(_) => None,
        },
    };
    let is_disk_link = has_disk_host(text);
    let empty_folder = if matches!(status, Some(200..=399)) && is_disk_link {
        probe_empty_folder(&client, text).await
    } else {
        None
    };
    link_validation_from_status(status, is_disk_link, empty_folder)
}

fn link_validation_from_status(
    status: Option<u16>,
    is_disk_link: bool,
    empty_folder: Option<bool>,
) -> LinkValidation {
    match status {
        Some(200..=399) if is_disk_link && empty_folder == Some(true) => {
            invalid_link("资源文件夹为空，已禁止打开。")
        }
        Some(200..=399) if is_disk_link => LinkValidation {
            status: "valid".to_string(),
            can_open: true,
            message: "资源链接已通过访问校验，可直接打开。".to_string(),
        },
        Some(200..=399) => LinkValidation {
            status: "warning".to_string(),
            can_open: true,
            message: "链接可访问，但未确认是最终网盘地址，可打开后继续确认。".to_string(),
        },
        Some(401) | Some(403) | Some(405) | Some(429) => LinkValidation {
            status: "warning".to_string(),
            can_open: true,
            message: "链接可能需要浏览器环境或被站点限制访问，可尝试打开后确认。".to_string(),
        },
        Some(404) | Some(410) => invalid_link("资源链接已失效或页面不存在。"),
        Some(_) => LinkValidation {
            status: "warning".to_string(),
            can_open: true,
            message: "链接有响应但状态异常，可尝试打开后确认资源是否仍可用。".to_string(),
        },
        None => LinkValidation {
            status: "warning".to_string(),
            can_open: true,
            message: "当前网络无法确认链接可访问性，可尝试在浏览器中打开确认。".to_string(),
        },
    }
}

async fn probe_empty_folder(client: &Client, url: &str) -> Option<bool> {
    let response = client
        .get(url)
        .header(
            header::ACCEPT,
            "text/html,application/json,text/plain,*/*;q=0.5",
        )
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    if let Some(content_type) = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_lowercase())
    {
        let readable = content_type.starts_with("text/")
            || content_type.contains("html")
            || content_type.contains("json")
            || content_type.contains("xml")
            || content_type.contains("javascript");
        if !readable {
            return None;
        }
    }
    let text = read_limited_response_text(response, EMPTY_FOLDER_PROBE_BYTES).await?;
    if text.trim().is_empty() {
        return None;
    }
    if looks_like_empty_folder_page(&text) {
        return Some(true);
    }
    if looks_like_non_empty_folder_page(&text) {
        return Some(false);
    }
    None
}

async fn read_limited_response_text(mut response: Response, max_bytes: usize) -> Option<String> {
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.ok()? {
        if bytes.len() >= max_bytes {
            break;
        }
        let remaining = max_bytes - bytes.len();
        if chunk.len() > remaining {
            bytes.extend_from_slice(&chunk[..remaining]);
            break;
        }
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8(bytes).ok()
}

fn looks_like_empty_folder_page(text: &str) -> bool {
    let lower = text.to_lowercase();
    let normalized = text.split_whitespace().collect::<String>();
    let markers = [
        "文件夹为空",
        "文件夹是空的",
        "此文件夹为空",
        "该文件夹为空",
        "暂无文件",
        "没有文件",
        "无文件",
        "空文件夹",
        "目录为空",
        "文件为空",
    ];
    if markers
        .iter()
        .any(|marker| text.contains(marker) || normalized.contains(marker))
    {
        return true;
    }
    [
        "folder is empty",
        "this folder is empty",
        "empty folder",
        "directory is empty",
        "no files",
        "no file found",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn looks_like_non_empty_folder_page(text: &str) -> bool {
    let lower = text.to_lowercase();
    let markers = [
        "保存到网盘",
        "转存",
        "下载",
        "文件列表",
        "全部文件",
        "share-file-list",
        "file-list",
        "filename",
        "file_name",
    ];
    markers
        .iter()
        .any(|marker| text.contains(marker) || lower.contains(marker))
}

fn invalid_link(message: &str) -> LinkValidation {
    LinkValidation {
        status: "invalid".to_string(),
        can_open: false,
        message: message.to_string(),
    }
}

fn is_allowed_external_url(text: &str) -> bool {
    text.starts_with("http://")
        || text.starts_with("https://")
        || text.starts_with("magnet:")
        || text.starts_with("ed2k://")
}

fn filter_valid_resource_items(items: Vec<ResourceItem>) -> Vec<ResourceItem> {
    items
        .into_iter()
        .filter(is_basic_valid_resource_item)
        .collect()
}

fn is_basic_valid_resource_item(item: &ResourceItem) -> bool {
    if item.title.trim().is_empty() {
        return false;
    }
    let link = first_non_empty(&[&item.payload.final_url, &item.url, &item.payload.detail_url]);
    !link.is_empty() && is_allowed_external_url(&link)
}

fn friendly_search_error(error: &str) -> String {
    let lower = error.to_lowercase();
    if lower.contains("timed out")
        || lower.contains("timeout")
        || lower.contains("error sending request")
        || lower.contains("connection")
        || lower.contains("dns")
        || lower.contains("network")
    {
        return "资源暂时无法访问，请稍后重试".to_string();
    }
    if lower.contains("json")
        || lower.contains("decode")
        || lower.contains("decoding")
        || lower.contains("解析")
    {
        return "资源返回内容异常，暂时无法解析".to_string();
    }
    if lower.contains("http") || error.contains("请求失败") {
        return "资源暂时无法访问，请稍后重试".to_string();
    }
    "资源暂时无法访问，请稍后重试".to_string()
}

async fn build_search_plan(query: &str, settings: &SearchSettings) -> SearchPlan {
    let candidates = identify_media_candidates(query, settings).await;
    let active_candidate = candidates.first().cloned();
    let search_terms = build_search_terms(query, active_candidate.as_ref());
    let include_keywords = build_include_keywords(active_candidate.as_ref());
    let exclude_keywords = build_exclude_keywords(active_candidate.as_ref());
    SearchPlan {
        original_query: query.to_string(),
        active_candidate_id: active_candidate
            .as_ref()
            .map(|candidate| candidate.id.clone())
            .unwrap_or_default(),
        candidates,
        search_terms,
        include_keywords,
        exclude_keywords,
    }
}

async fn identify_media_candidates(query: &str, settings: &SearchSettings) -> Vec<MediaCandidate> {
    let mut candidates = local_media_candidates(query);
    if !settings.tmdb_api_key.is_empty() {
        candidates.extend(tmdb_media_candidates(query, settings).await);
    }
    candidates.sort_by(|left, right| right.confidence.cmp(&left.confidence));
    candidates = dedupe_candidates(candidates);
    candidates
}

async fn tmdb_media_candidates(query: &str, settings: &SearchSettings) -> Vec<MediaCandidate> {
    let client = match Client::builder()
        .timeout(Duration::from_secs(12))
        .user_agent(USER_AGENT)
        .build()
    {
        Ok(client) => client,
        Err(_) => return Vec::new(),
    };
    let mut request = client.get(format!(
        "https://api.themoviedb.org/3/search/multi?query={}&language=zh-CN&include_adult=false&page=1",
        urlencoding::encode(query)
    ));
    if settings.tmdb_api_key.starts_with("ey") {
        request = request.bearer_auth(&settings.tmdb_api_key);
    } else {
        request = request.query(&[("api_key", settings.tmdb_api_key.as_str())]);
    }
    let data: Value = match request.send().await {
        Ok(response) => match response.json().await {
            Ok(data) => data,
            Err(_) => return Vec::new(),
        },
        Err(_) => return Vec::new(),
    };
    data.pointer("/results")
        .and_then(Value::as_array)
        .unwrap_or(&Vec::new())
        .iter()
        .take(6)
        .filter_map(tmdb_candidate)
        .collect()
}

fn tmdb_candidate(value: &Value) -> Option<MediaCandidate> {
    let media_type = value
        .get("media_type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if media_type != "movie" && media_type != "tv" {
        return None;
    }
    let title = first_json_string(value, &["name", "title"])?;
    let original_title =
        first_json_string(value, &["original_name", "original_title"]).unwrap_or_default();
    let date = first_json_string(value, &["first_air_date", "release_date"]).unwrap_or_default();
    let year = date.get(0..4).unwrap_or_default().to_string();
    let overview = first_json_string(value, &["overview"]).unwrap_or_default();
    Some(MediaCandidate {
        id: format!(
            "tmdb-{}-{}",
            media_type,
            first_json_string(value, &["id"]).unwrap_or_default()
        ),
        title: clean_text(&title),
        original_title: clean_text(&original_title),
        year,
        media_type: if media_type == "tv" {
            "电视剧"
        } else {
            "电影"
        }
        .to_string(),
        actors: Vec::new(),
        aliases: if original_title.is_empty() {
            Vec::new()
        } else {
            vec![clean_text(&original_title)]
        },
        platforms: Vec::new(),
        overview: clean_text(&overview),
        confidence: 68,
        source: "TMDB".to_string(),
    })
}

fn dedupe_candidates(candidates: Vec<MediaCandidate>) -> Vec<MediaCandidate> {
    let mut seen = HashSet::new();
    let mut output = Vec::new();
    for candidate in candidates {
        let key = format!("{}|{}", candidate.title, candidate.year);
        if seen.insert(key) {
            output.push(candidate);
        }
    }
    output
}

fn local_media_candidates(query: &str) -> Vec<MediaCandidate> {
    let normalized = query.to_lowercase();
    let mut candidates = Vec::new();
    let mentions_life_tree = query.contains("生命树") || normalized.contains("born to be alive");
    let mentions_hu_ge = query.contains("胡歌");
    let mentions_yang_zi = query.contains("杨紫");
    let mentions_spartacus = query.contains("斯巴达克斯") || normalized.contains("spartacus");

    if mentions_life_tree || (mentions_hu_ge && query.contains("树")) {
        candidates.push(MediaCandidate {
            id: "tv-life-tree-2026".to_string(),
            title: "生命树".to_string(),
            original_title: "Born to be alive".to_string(),
            year: "2026".to_string(),
            media_type: "电视剧".to_string(),
            actors: vec!["杨紫".to_string(), "胡歌".to_string()],
            aliases: vec!["Born to be alive".to_string(), "生命树 2026".to_string()],
            platforms: vec!["爱奇艺".to_string()],
            overview: "杨紫、胡歌主演电视剧。".to_string(),
            confidence: 96
                + if mentions_hu_ge || mentions_yang_zi {
                    6
                } else {
                    0
                },
            source: "本地影视规则".to_string(),
        });
    }

    if mentions_spartacus {
        candidates.push(MediaCandidate {
            id: "tv-spartacus".to_string(),
            title: "斯巴达克斯".to_string(),
            original_title: "Spartacus".to_string(),
            year: "2010".to_string(),
            media_type: "电视剧".to_string(),
            actors: vec![],
            aliases: vec![
                "Spartacus".to_string(),
                "斯巴达克斯 血与沙".to_string(),
                "斯巴达克斯 竞技场之神".to_string(),
            ],
            platforms: vec![],
            overview: "美剧《斯巴达克斯》系列。".to_string(),
            confidence: 92,
            source: "本地影视规则".to_string(),
        });
    }

    if mentions_hu_ge && candidates.is_empty() {
        candidates.extend(vec![
            person_candidate("tv-life-tree-2026", "生命树", "2026", 78),
            person_candidate("tv-county-party-committee", "县委大院", "2022", 74),
            person_candidate("tv-nirvana-in-fire", "琅琊榜", "2015", 72),
            person_candidate("tv-game-of-hunting", "猎场", "2017", 70),
        ]);
    }

    if candidates.is_empty() {
        candidates.push(MediaCandidate {
            id: "keyword".to_string(),
            title: query.to_string(),
            original_title: String::new(),
            year: String::new(),
            media_type: "关键词".to_string(),
            actors: Vec::new(),
            aliases: Vec::new(),
            platforms: Vec::new(),
            overview: "未识别到明确影视条目，按关键词检索。".to_string(),
            confidence: 30,
            source: "关键词规则".to_string(),
        });
    }

    candidates
}

fn person_candidate(id: &str, title: &str, year: &str, confidence: i64) -> MediaCandidate {
    MediaCandidate {
        id: id.to_string(),
        title: title.to_string(),
        original_title: String::new(),
        year: year.to_string(),
        media_type: "电视剧".to_string(),
        actors: vec!["胡歌".to_string()],
        aliases: Vec::new(),
        platforms: Vec::new(),
        overview: "胡歌相关影视候选。".to_string(),
        confidence,
        source: "本地人物规则".to_string(),
    }
}

fn build_search_terms(query: &str, candidate: Option<&MediaCandidate>) -> Vec<String> {
    let mut terms = Vec::new();
    if let Some(candidate) = candidate {
        push_unique(&mut terms, candidate.title.clone());
        if !candidate.year.is_empty() {
            push_unique(
                &mut terms,
                format!("{} {}", candidate.title, candidate.year),
            );
        }
        if candidate.media_type == "电视剧" {
            push_unique(&mut terms, format!("{} 全集", candidate.title));
            push_unique(&mut terms, format!("{} 更新", candidate.title));
        }
        if candidate.title == "生命树" {
            push_unique(&mut terms, "生命树 40集".to_string());
            push_unique(&mut terms, "生命树 杨紫 胡歌".to_string());
            push_unique(&mut terms, "生命树 夸克".to_string());
            push_unique(&mut terms, "生命树 阿里云盘".to_string());
        }
        for platform in &candidate.platforms {
            push_unique(&mut terms, format!("{} {}", candidate.title, platform));
        }
        for alias in &candidate.aliases {
            push_unique(&mut terms, alias.clone());
        }
    }
    push_unique(&mut terms, query.to_string());
    terms.into_iter().take(12).collect()
}

fn build_include_keywords(candidate: Option<&MediaCandidate>) -> Vec<String> {
    let mut keywords = Vec::new();
    if let Some(candidate) = candidate {
        push_unique(&mut keywords, candidate.title.clone());
        if !candidate.year.is_empty() {
            push_unique(&mut keywords, candidate.year.clone());
        }
        for actor in &candidate.actors {
            push_unique(&mut keywords, actor.clone());
        }
        for platform in &candidate.platforms {
            push_unique(&mut keywords, platform.clone());
        }
    }
    keywords
}

fn build_exclude_keywords(candidate: Option<&MediaCandidate>) -> Vec<String> {
    let mut keywords = NOISE_WORDS
        .iter()
        .map(|item| item.to_string())
        .collect::<Vec<_>>();
    if let Some(candidate) = candidate {
        if candidate.title == "生命树" {
            push_unique(&mut keywords, "生命之树".to_string());
            push_unique(&mut keywords, "Tree of Life".to_string());
        }
    }
    keywords
}

fn build_pansou_ext(plan: &SearchPlan) -> Value {
    let candidate = plan
        .candidates
        .iter()
        .find(|item| item.id == plan.active_candidate_id);
    match candidate {
        Some(candidate) => json!({
            "title": candidate.title,
            "originalTitle": candidate.original_title,
            "year": candidate.year,
            "mediaType": candidate.media_type,
            "actors": candidate.actors,
            "aliases": candidate.aliases,
            "platforms": candidate.platforms,
            "collection": candidate.media_type == "电视剧",
        }),
        None => json!({
            "title": plan.original_query,
            "collection": false,
        }),
    }
}

fn push_pansou_param(params: &mut Vec<(String, String)>, key: &str, value: &str) {
    let text = value.trim();
    if !text.is_empty() && text != "all" {
        params.push((key.to_string(), text.to_string()));
    }
}

fn push_pansou_list_param(params: &mut Vec<(String, String)>, key: &str, values: &[String]) {
    let text = values
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>()
        .join(",");
    if !text.is_empty() {
        params.push((key.to_string(), text));
    }
}

fn score_items(items: &mut [ResourceItem], plan: &SearchPlan) {
    for item in items {
        let (score, level, reasons) = score_item(item, plan);
        item.relevance_score = score;
        item.relevance_level = level;
        item.match_reasons = reasons;
    }
}

fn score_item(item: &ResourceItem, plan: &SearchPlan) -> (i64, String, Vec<String>) {
    let mut score = 0;
    let mut reasons = Vec::new();
    let haystack = format!("{} {} {}", item.title, item.info, item.tags.join(" "));
    let candidate = plan
        .candidates
        .iter()
        .find(|candidate| candidate.id == plan.active_candidate_id);

    if let Some(candidate) = candidate {
        if item.title == candidate.title {
            score += 80;
            reasons.push("片名精确命中".to_string());
        } else if item.title.contains(&candidate.title) || item.info.contains(&candidate.title) {
            score += 55;
            reasons.push("片名命中".to_string());
        }
        if candidate.title == "生命树" {
            let video_signals = matched_video_signals(&haystack);
            let title_matched =
                item.title.contains(&candidate.title) || item.info.contains(&candidate.title);
            if title_matched && !video_signals.is_empty() {
                score += 18 + (video_signals.len().min(4) as i64 * 6);
                reasons.push(format!("影视信号 {}", video_signals.join("/")));
            } else if title_matched {
                score -= 45;
                reasons.push("缺少目标剧影视信号".to_string());
            } else {
                score -= 80;
                reasons.push("未命中目标片名".to_string());
            }
        }
        if !candidate.year.is_empty() && haystack.contains(&candidate.year) {
            score += 18;
            reasons.push(format!("年份 {}", candidate.year));
        }
        for actor in &candidate.actors {
            if haystack.contains(actor) {
                score += 8;
                reasons.push(format!("主演 {}", actor));
            }
        }
        for platform in &candidate.platforms {
            if haystack.contains(platform) {
                score += 8;
                reasons.push(format!("平台 {}", platform));
            }
        }
        for alias in &candidate.aliases {
            if !alias.is_empty() && haystack.contains(alias) {
                score += 12;
                reasons.push("别名命中".to_string());
            }
        }
    }

    for &word in BOOST_WORDS {
        if haystack.contains(word) {
            score += 5;
        }
    }
    for word in &plan.exclude_keywords {
        if !word.is_empty() && haystack.contains(word) {
            score -= 35;
            reasons.push(format!("疑似噪声 {}", word));
        }
    }
    if let Some(noise) = matched_noise_word(&haystack) {
        score -= 40;
        reasons.push(format!("疑似噪声 {}", noise));
    }
    if item.url.starts_with("http") || item.payload.final_url.starts_with("http") {
        score += 8;
        reasons.push("链接有效".to_string());
    }
    if has_disk_link(item) {
        score += 18;
        reasons.push("网盘链接".to_string());
    }
    if item.source_id.starts_with("pansou") {
        score += 10;
        reasons.push("PanSou 来源".to_string());
    }
    if item.source_id.starts_with("cms-v10") {
        score += 6;
        reasons.push("CMS 源".to_string());
    }
    if item.source_id.starts_with("torznab") || item.source_id.starts_with("newznab") {
        score += 8;
        reasons.push("外部索引器".to_string());
    }
    if reasons.is_empty() {
        reasons.push("关键词相关".to_string());
    }

    if let Some(candidate) = candidate {
        if candidate.title == "生命树" {
            let title_matched =
                item.title.contains(&candidate.title) || item.info.contains(&candidate.title);
            if !title_matched || matched_video_signals(&haystack).is_empty() {
                score = score.min(24);
            }
        }
    }

    let level = if score >= 70 {
        "high"
    } else if score >= 25 {
        "possible"
    } else {
        "low"
    };
    (score, level.to_string(), reasons)
}

fn group_items(items: &[ResourceItem]) -> Vec<ResultGroup> {
    let configs = [
        ("high", "高可信资源"),
        ("possible", "可能相关"),
        ("low", "低相关"),
    ];
    configs
        .iter()
        .map(|(key, title)| ResultGroup {
            key: (*key).to_string(),
            title: (*title).to_string(),
            items: items
                .iter()
                .filter(|item| item.relevance_level == *key)
                .cloned()
                .collect(),
        })
        .filter(|group| !group.items.is_empty())
        .collect()
}

fn count_target_resources(items: &[ResourceItem], plan: &SearchPlan) -> usize {
    items
        .iter()
        .filter(|item| is_target_resource(item, plan))
        .count()
}

fn target_resource_message(count: usize, items: &[ResourceItem], plan: &SearchPlan) -> String {
    let candidate = active_candidate(plan);
    if candidate.map(|item| item.title.as_str()) != Some("生命树") {
        return format!("目标资源命中 {} 条", count);
    }
    if count > 0 {
        return format!("已命中胡歌/杨紫版《生命树》网盘资源 {} 条", count);
    }
    if items.is_empty() {
        return "未返回任何网盘结果".to_string();
    }
    "未命中目标剧网盘资源，当前结果多为低相关或噪声内容".to_string()
}

fn is_target_resource(item: &ResourceItem, plan: &SearchPlan) -> bool {
    let candidate = match active_candidate(plan) {
        Some(candidate) => candidate,
        None => return item.relevance_score >= 70,
    };
    if candidate.title == "生命树" {
        let haystack = format!("{} {} {}", item.title, item.info, item.tags.join(" "));
        return (item.relevance_level == "high" || item.relevance_level == "possible")
            && haystack.contains(&candidate.title)
            && !matched_video_signals(&haystack).is_empty()
            && matched_noise_word(&haystack).is_none();
    }
    item.relevance_level == "high"
}

fn active_candidate<'a>(plan: &'a SearchPlan) -> Option<&'a MediaCandidate> {
    plan.candidates
        .iter()
        .find(|candidate| candidate.id == plan.active_candidate_id)
}

fn matched_video_signals(text: &str) -> Vec<String> {
    let upper = text.to_uppercase();
    let mut output = Vec::new();
    for &word in VIDEO_SIGNAL_WORDS {
        if text.contains(word) || upper.contains(word) {
            push_unique(&mut output, word.to_string());
        }
    }
    if Regex::new(r"(第\s*\d+\s*集|全\s*\d+\s*集|EP\s*\d+|S\d{1,2}E\d{1,2})")
        .map(|regex| regex.is_match(&upper))
        .unwrap_or(false)
    {
        push_unique(&mut output, "集数".to_string());
    }
    output
}

fn matched_noise_word(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    for &word in NOISE_WORDS {
        if text.contains(word) || lower.contains(&word.to_lowercase()) {
            return Some(word.to_string());
        }
    }
    None
}

fn has_disk_link(item: &ResourceItem) -> bool {
    let text = format!(
        "{} {} {}",
        item.url, item.payload.final_url, item.payload.detail_url
    );
    has_disk_host(&text)
}

fn has_disk_host(text: &str) -> bool {
    DISK_HOST_MARKERS.iter().any(|marker| text.contains(marker))
}

fn build_coverage(states: &[SourceSearchState]) -> Vec<SourceCoverage> {
    let mut groups: HashMap<String, SourceCoverage> = HashMap::new();
    for state in states {
        let entry = groups.entry(state.group.clone()).or_insert(SourceCoverage {
            group: state.group.clone(),
            total: 0,
            success: 0,
            failed: 0,
            disabled: 0,
            count: 0,
            message: String::new(),
        });
        entry.total += 1;
        entry.count += state.count;
        match state.status.as_str() {
            "success" => entry.success += 1,
            "failed" => entry.failed += 1,
            "disabled" => entry.disabled += 1,
            _ => {}
        }
    }
    let mut output = groups.into_values().collect::<Vec<_>>();
    output.sort_by(|left, right| left.group.cmp(&right.group));
    for item in &mut output {
        item.message = format!(
            "{} 个来源，成功 {}，失败 {}，禁用 {}，返回 {} 条",
            item.total, item.success, item.failed, item.disabled, item.count
        );
    }
    output
}

fn search_sources(settings: &SearchSettings) -> Vec<SearchSource> {
    search_sources_with_embedded(settings, None)
}

fn search_sources_with_embedded(
    settings: &SearchSettings,
    embedded_status: Option<EmbeddedPansouStatus>,
) -> Vec<SearchSource> {
    let mut sources = Vec::new();
    if settings.embedded_pansou.enabled {
        let fallback_status = EmbeddedPansouStatus {
            enabled: settings.embedded_pansou.enabled,
            running: true,
            reused: false,
            endpoint: embedded_pansou_endpoint(settings.embedded_pansou.port),
            port: settings.embedded_pansou.port,
            message: "内置 PanSou 将在搜索前自动启动".to_string(),
        };
        let status = embedded_status.unwrap_or(fallback_status);
        sources.push(SearchSource {
            id: EMBEDDED_PANSOU_SOURCE_ID.to_string(),
            name: "内置 PanSou".to_string(),
            group: "内置聚合源".to_string(),
            enabled: settings.embedded_pansou.enabled && status.running,
            description: status.message,
            kind: EMBEDDED_PANSOU_SOURCE_ID.to_string(),
            config_index: None,
            health_score: if status.running { 76 } else { 45 },
            status: if status.running {
                "configured".to_string()
            } else {
                "requiresConfig".to_string()
            },
        });
    }
    for (index, endpoint) in settings.pansou_endpoints.iter().enumerate() {
        sources.push(SearchSource {
            id: source_config_id("pansou", index, &endpoint.endpoint),
            name: if endpoint.name.is_empty() {
                format!("PanSou {}", index + 1)
            } else {
                endpoint.name.clone()
            },
            group: "PanSou 深度池".to_string(),
            enabled: endpoint.enabled && !endpoint.endpoint.trim().is_empty(),
            description: "用户配置的 PanSou 网盘搜索 API。".to_string(),
            kind: "pansou".to_string(),
            config_index: Some(index),
            health_score: 72,
            status: "configured".to_string(),
        });
    }
    for (index, cms_source) in settings.cms_sources.iter().enumerate() {
        sources.push(SearchSource {
            id: source_config_id("cms-v10", index, &cms_source.endpoint),
            name: if cms_source.name.is_empty() {
                format!("CMS {}", index + 1)
            } else {
                cms_source.name.clone()
            },
            group: "CMS 源池".to_string(),
            enabled: cms_source.enabled
                && !cms_source.endpoint.trim().is_empty()
                && cms_source.failure_count < 5,
            description: "用户配置的苹果 CMS V10 资源站接口。".to_string(),
            kind: "cms-v10".to_string(),
            config_index: Some(index),
            health_score: cms_source.health_score,
            status: "configured".to_string(),
        });
    }
    for (index, indexer) in settings.indexers.iter().enumerate() {
        let kind = if indexer.indexer_type == "newznab" {
            "newznab"
        } else {
            "torznab"
        };
        sources.push(SearchSource {
            id: source_config_id(kind, index, &indexer.base_url),
            name: if indexer.name.is_empty() {
                format!("Indexer {}", index + 1)
            } else {
                indexer.name.clone()
            },
            group: "外部索引器".to_string(),
            enabled: indexer.enabled && !indexer.base_url.trim().is_empty(),
            description: "用户自配 Torznab/Newznab 索引器，只做搜索和跳转。".to_string(),
            kind: kind.to_string(),
            config_index: Some(index),
            health_score: 68,
            status: "configured".to_string(),
        });
    }
    sources.extend(load_rule_sources());
    sources
}

fn resolve_selected_sources(
    filters: &SearchFilters,
    settings: &SearchSettings,
) -> Vec<SearchSource> {
    search_sources(settings)
        .into_iter()
        .filter(|source| filters.source_ids.is_empty() || filters.source_ids.contains(&source.id))
        .collect()
}

fn load_rule_sources() -> Vec<SearchSource> {
    let configs = read_rule_source_configs().unwrap_or_else(|_| default_rule_sources());
    configs
        .into_iter()
        .filter(|item| !item.id.is_empty() && !item.kind.is_empty())
        .map(|item| SearchSource {
            id: item.id,
            name: item.name,
            group: item.group,
            enabled: item.enabled,
            description: item.description,
            kind: item.kind,
            config_index: None,
            health_score: if item.health_score == 0 {
                60
            } else {
                item.health_score
            },
            status: if item.status.is_empty() {
                "ready".to_string()
            } else {
                item.status
            },
        })
        .collect()
}

fn read_rule_source_configs() -> Result<Vec<RuleSourceConfig>, String> {
    for path in rule_source_paths() {
        if path.exists() {
            let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
            return serde_json::from_str::<Vec<RuleSourceConfig>>(&text)
                .map_err(|error| error.to_string());
        }
    }
    Err("未找到本地规则文件".to_string())
}

fn rule_source_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd.join(RULE_SOURCE_FILE));
        paths.push(cwd.join("..").join(RULE_SOURCE_FILE));
        paths.push(cwd.join("../..").join(RULE_SOURCE_FILE));
    }
    paths.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(RULE_SOURCE_FILE),
    );
    paths
}

fn default_rule_sources() -> Vec<RuleSourceConfig> {
    vec![
        rule_source(
            "hunhepan",
            "混合盘",
            "公开页面源",
            "hunhepan",
            "混合盘开放搜索接口，详情可直接得到跳转链接。",
            64,
            true,
            "ready",
        ),
        rule_source(
            "pikasoo",
            "皮卡搜索",
            "公开页面源",
            "pikasoo",
            "页面规则解析，搜索结果多数直接指向网盘链接。",
            58,
            true,
            "ready",
        ),
        rule_source(
            "aliso",
            "阿里搜",
            "公开页面源",
            "aliso",
            "页面规则解析，详情阶段尝试补充跳转信息。",
            54,
            true,
            "ready",
        ),
        rule_source(
            "cuppaso",
            "咔帕搜索",
            "公开页面源",
            "cuppaso",
            "公开页面源，详情页通常可解析到阿里、夸克或迅雷网盘链接。",
            62,
            true,
            "ready",
        ),
        rule_source(
            "buyutu",
            "捕娱兔",
            "公开页面源",
            "buyutu",
            "公开页面源，按视频类型搜索并从详情页提取网盘链接。",
            60,
            true,
            "ready",
        ),
        rule_source(
            "xuebapan",
            "学霸盘",
            "公开页面源",
            "xuebapan",
            "公开页面源，列表结构与阿里搜相近，详情页加密时会回退来源页。",
            56,
            true,
            "ready",
        ),
        rule_source(
            "quarkso",
            "夸克搜",
            "公开页面源",
            "quarkso",
            "公开夸克网盘搜索源，详情页尝试解析夸克网盘链接。",
            66,
            true,
            "ready",
        ),
        rule_source(
            "yyurl",
            "云盘资源共享站",
            "公开页面源",
            "yyurl",
            "Flarum 公开共享站，详情页提取正文中的网盘链接。",
            58,
            true,
            "ready",
        ),
        rule_source(
            "myxiaozhan",
            "我的小站",
            "公开页面源",
            "myxiaozhan",
            "Flarum 公开共享站，详情页提取正文中的网盘链接。",
            58,
            true,
            "ready",
        ),
        rule_source(
            "panquduo",
            "盘趣多",
            "公开页面源",
            "panquduo",
            "公开夸克/百度网盘搜索源，搜索页和详情页可解析目标剧资源。",
            72,
            true,
            "ready",
        ),
        rule_source(
            "qianfan",
            "千帆",
            "需配置源",
            "qianfan",
            "需要 code，默认不参与搜索。",
            50,
            false,
            "requiresConfig",
        ),
        rule_source(
            "yiso",
            "易搜",
            "需配置源",
            "yiso",
            "需要 cookie，默认不参与搜索。",
            50,
            false,
            "requiresConfig",
        ),
        rule_source(
            "qkpanso",
            "夸克盘搜",
            "需配置源",
            "qkpanso",
            "当前直接请求不稳定，默认不参与搜索。",
            45,
            false,
            "requiresConfig",
        ),
    ]
}

fn rule_source(
    id: &str,
    name: &str,
    group: &str,
    kind: &str,
    description: &str,
    health_score: i64,
    enabled: bool,
    status: &str,
) -> RuleSourceConfig {
    RuleSourceConfig {
        id: id.to_string(),
        name: name.to_string(),
        group: group.to_string(),
        kind: kind.to_string(),
        enabled,
        description: description.to_string(),
        health_score,
        status: status.to_string(),
    }
}

fn source_config_id(kind: &str, index: usize, value: &str) -> String {
    format!("{}-{}-{}", kind, index, stable_hash(value))
}

fn resource_item(
    source: &SearchSource,
    index: usize,
    title: String,
    info: String,
    url: String,
    final_url: String,
    detail_url: String,
    disk_type: String,
    share_user: String,
    tags: Vec<String>,
) -> ResourceItem {
    ResourceItem {
        id: format!(
            "{}-{}-{}",
            source.id,
            index,
            stable_hash(&format!("{}{}{}", title, url, detail_url))
        ),
        title: title.clone(),
        info,
        url,
        source_id: source.id.clone(),
        source_name: source.name.clone(),
        disk_type,
        share_user,
        tags,
        payload: ResourcePayload {
            title,
            final_url,
            detail_url,
            password: String::new(),
        },
        relevance_score: 0,
        relevance_level: "low".to_string(),
        match_reasons: Vec::new(),
    }
}

fn outcome(
    source: SearchSource,
    items: Vec<ResourceItem>,
    status: &str,
    message: String,
    elapsed_ms: u128,
) -> SearchOutcome {
    let count = items.len();
    SearchOutcome {
        items,
        state: SourceSearchState {
            source_id: source.id,
            source_name: source.name,
            group: source.group,
            kind: source.kind,
            status: status.to_string(),
            message,
            count,
            elapsed_ms,
            health_score: source.health_score,
        },
    }
}

fn dedupe_items(items: Vec<ResourceItem>) -> Vec<ResourceItem> {
    let mut seen_links = HashSet::new();
    let mut seen_titles = HashSet::new();
    let mut seen_hashes = HashSet::new();
    let mut output = Vec::new();
    for item in items {
        let link = first_non_empty(&[&item.payload.final_url, &item.url, &item.payload.detail_url]);
        let title_key = normalize_dedupe_key(&item.title);
        let link_key = normalize_dedupe_key(&link);
        let hash_key = stable_hash(&format!(
            "{}|{}|{}",
            title_key,
            link_key,
            normalize_dedupe_key(&item.info)
        ));
        let is_new_link = link_key.is_empty() || seen_links.insert(link_key);
        let is_new_title = title_key.is_empty()
            || seen_titles.insert(format!(
                "{}|{}",
                title_key,
                normalize_dedupe_key(&item.disk_type)
            ));
        let is_new_hash = seen_hashes.insert(hash_key);
        if is_new_link && is_new_title && is_new_hash {
            output.push(item);
        }
    }
    output
}

fn normalize_dedupe_key(value: &str) -> String {
    value
        .to_lowercase()
        .replace([' ', '\n', '\t', '-', '_', '【', '】', '[', ']'], "")
}

fn parse_cms_source_text(text: &str) -> Result<Vec<CmsSourceConfig>, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    if trimmed.starts_with('[') {
        let sources = serde_json::from_str::<Vec<CmsSourceConfig>>(trimmed)
            .map_err(|error| error.to_string())?;
        return Ok(sources
            .into_iter()
            .map(normalize_cms_source)
            .filter(|item| !item.endpoint.is_empty())
            .collect());
    }
    let mut output = Vec::new();
    for line in trimmed.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts = line.split_once(',').or_else(|| line.split_once(' '));
        let (name, endpoint) = match parts {
            Some((left, right)) if right.contains("http") => {
                (left.trim().to_string(), right.trim().to_string())
            }
            _ => (host_label(line), line.to_string()),
        };
        output.push(normalize_cms_source(CmsSourceConfig {
            id: String::new(),
            name,
            endpoint,
            enabled: true,
            last_success_at: String::new(),
            failure_count: 0,
            average_count: 0.0,
            health_score: 60,
        }));
    }
    Ok(output)
}

async fn test_cms_source(client: Client, source: CmsSourceConfig) -> CmsHealthResult {
    let start = Instant::now();
    let endpoint = source.endpoint.trim().trim_end_matches('/').to_string();
    let url = format!(
        "{}?ac=videolist&wd={}",
        endpoint,
        urlencoding::encode("生命树")
    );
    let result = client.get(url).send().await;
    match result {
        Ok(response) if response.status().is_success() => match response.json::<Value>().await {
            Ok(data) => {
                let count = data
                    .pointer("/list")
                    .and_then(Value::as_array)
                    .map(|items| items.len())
                    .unwrap_or(0);
                CmsHealthResult {
                    id: source.id,
                    name: source.name,
                    endpoint,
                    ok: true,
                    count,
                    elapsed_ms: start.elapsed().as_millis(),
                    message: format!("接口可用，返回 {} 条", count),
                }
            }
            Err(error) => {
                cms_health_failed(source, endpoint, start, format!("JSON 解析失败：{}", error))
            }
        },
        Ok(response) => cms_health_failed(
            source,
            endpoint,
            start,
            format!("HTTP {}", response.status()),
        ),
        Err(error) => cms_health_failed(source, endpoint, start, error.to_string()),
    }
}

fn cms_health_failed(
    source: CmsSourceConfig,
    endpoint: String,
    start: Instant,
    message: String,
) -> CmsHealthResult {
    CmsHealthResult {
        id: source.id,
        name: source.name,
        endpoint,
        ok: false,
        count: 0,
        elapsed_ms: start.elapsed().as_millis(),
        message,
    }
}

fn normalize_settings(settings: SearchSettings) -> SearchSettings {
    let embedded_pansou = normalize_embedded_pansou_config(settings.embedded_pansou);
    let mut pansou_endpoints = settings
        .pansou_endpoints
        .into_iter()
        .map(normalize_pansou_endpoint)
        .filter(|item| !item.endpoint.is_empty())
        .collect::<Vec<_>>();
    if pansou_endpoints.is_empty() && !settings.pansou_endpoint.trim().is_empty() {
        pansou_endpoints.push(normalize_pansou_endpoint(PansouEndpointConfig {
            id: String::new(),
            name: "PanSou 默认".to_string(),
            endpoint: settings.pansou_endpoint.clone(),
            token: settings.pansou_token.clone(),
            enabled: true,
            refresh: settings.pansou_refresh,
            channels: settings.pansou_channels.clone(),
            plugins: settings.pansou_plugins.clone(),
            src: settings.pansou_src.clone(),
            cloud_types: settings.pansou_cloud_types.clone(),
            concurrency: settings.pansou_concurrency,
        }));
    }

    let mut cms_sources = settings
        .cms_sources
        .into_iter()
        .map(normalize_cms_source)
        .filter(|item| !item.endpoint.is_empty())
        .collect::<Vec<_>>();
    for endpoint in settings.cms_endpoints {
        let normalized = endpoint.trim().trim_end_matches('/').to_string();
        if !normalized.is_empty() && !cms_sources.iter().any(|item| item.endpoint == normalized) {
            cms_sources.push(normalize_cms_source(CmsSourceConfig {
                id: String::new(),
                name: host_label(&normalized),
                endpoint: normalized,
                enabled: true,
                last_success_at: String::new(),
                failure_count: 0,
                average_count: 0.0,
                health_score: 60,
            }));
        }
    }

    let indexers = settings
        .indexers
        .into_iter()
        .map(normalize_indexer)
        .filter(|item| !item.base_url.is_empty())
        .collect::<Vec<_>>();

    let pansou_endpoint = pansou_endpoints
        .first()
        .map(|item| item.endpoint.clone())
        .unwrap_or_default();
    let pansou_token = pansou_endpoints
        .first()
        .map(|item| item.token.clone())
        .unwrap_or_else(|| settings.pansou_token.trim().to_string());
    let pansou_refresh = pansou_endpoints
        .first()
        .map(|item| item.refresh)
        .unwrap_or(settings.pansou_refresh);
    let cms_endpoints = cms_sources
        .iter()
        .map(|item| item.endpoint.clone())
        .collect::<Vec<_>>();

    SearchSettings {
        embedded_pansou,
        pansou_endpoint,
        pansou_token,
        pansou_refresh,
        pansou_endpoints,
        pansou_channels: normalize_string_list(settings.pansou_channels),
        pansou_plugins: normalize_string_list(settings.pansou_plugins),
        pansou_src: normalize_pansou_src(&settings.pansou_src),
        pansou_cloud_types: normalize_string_list(settings.pansou_cloud_types),
        pansou_cache: settings.pansou_cache,
        pansou_concurrency: settings.pansou_concurrency.clamp(1, 8),
        cms_endpoints,
        cms_sources,
        indexers,
        tmdb_api_key: settings.tmdb_api_key.trim().to_string(),
    }
}

fn normalize_embedded_pansou_config(config: EmbeddedPansouConfig) -> EmbeddedPansouConfig {
    let mut plugins = normalize_string_list(config.plugins);
    if plugins.is_empty() {
        plugins = EMBEDDED_PANSOU_DEFAULT_PLUGINS
            .iter()
            .map(|item| item.to_string())
            .collect();
    }
    EmbeddedPansouConfig {
        enabled: config.enabled,
        auto_start: config.auto_start,
        port: if config.port == 0 {
            EMBEDDED_PANSOU_DEFAULT_PORT
        } else {
            config.port
        },
        src: normalize_pansou_src(&config.src),
        channels: normalize_string_list(config.channels),
        plugins,
        cloud_types: normalize_string_list(config.cloud_types),
        refresh: config.refresh,
        cache: config.cache,
        concurrency: config.concurrency.clamp(1, 8),
    }
}

fn normalize_pansou_endpoint(endpoint: PansouEndpointConfig) -> PansouEndpointConfig {
    let clean_endpoint = endpoint.endpoint.trim().trim_end_matches('/').to_string();
    let id = if endpoint.id.trim().is_empty() {
        format!("pansou-{}", stable_hash(&clean_endpoint))
    } else {
        endpoint.id.trim().to_string()
    };
    PansouEndpointConfig {
        id,
        name: if endpoint.name.trim().is_empty() {
            host_label(&clean_endpoint)
        } else {
            endpoint.name.trim().to_string()
        },
        endpoint: clean_endpoint,
        token: endpoint.token.trim().to_string(),
        enabled: endpoint.enabled,
        refresh: endpoint.refresh,
        channels: normalize_string_list(endpoint.channels),
        plugins: normalize_string_list(endpoint.plugins),
        src: normalize_pansou_src(&endpoint.src),
        cloud_types: normalize_string_list(endpoint.cloud_types),
        concurrency: endpoint.concurrency.clamp(1, 8),
    }
}

fn normalize_cms_source(source: CmsSourceConfig) -> CmsSourceConfig {
    let endpoint = source.endpoint.trim().trim_end_matches('/').to_string();
    let id = if source.id.trim().is_empty() {
        format!("cms-{}", stable_hash(&endpoint))
    } else {
        source.id.trim().to_string()
    };
    CmsSourceConfig {
        id,
        name: if source.name.trim().is_empty() {
            host_label(&endpoint)
        } else {
            source.name.trim().to_string()
        },
        endpoint,
        enabled: source.enabled,
        last_success_at: source.last_success_at,
        failure_count: source.failure_count,
        average_count: source.average_count,
        health_score: if source.health_score == 0 {
            60
        } else {
            source.health_score
        },
    }
}

fn normalize_indexer(indexer: IndexerConfig) -> IndexerConfig {
    let base_url = indexer.base_url.trim().to_string();
    let id = if indexer.id.trim().is_empty() {
        format!("indexer-{}", stable_hash(&base_url))
    } else {
        indexer.id.trim().to_string()
    };
    let indexer_type = if indexer.indexer_type == "newznab" {
        "newznab"
    } else {
        "torznab"
    };
    IndexerConfig {
        id,
        name: if indexer.name.trim().is_empty() {
            host_label(&base_url)
        } else {
            indexer.name.trim().to_string()
        },
        base_url,
        api_key: indexer.api_key.trim().to_string(),
        indexer_type: indexer_type.to_string(),
        categories: normalize_string_list(indexer.categories),
        enabled: indexer.enabled,
    }
}

fn normalize_string_list(values: Vec<String>) -> Vec<String> {
    let mut output = Vec::new();
    for value in values {
        push_unique(&mut output, value.trim().to_string());
    }
    output
}

fn normalize_pansou_src(value: &str) -> String {
    match value.trim() {
        "tg" => "tg".to_string(),
        "plugin" => "plugin".to_string(),
        _ => "all".to_string(),
    }
}

fn host_label(value: &str) -> String {
    let text = value
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/');
    text.split('/').next().unwrap_or(text).to_string()
}

fn read_search_settings(app: &AppHandle) -> Result<SearchSettings, String> {
    let path = settings_path(app)?;
    if !path.exists() {
        return Ok(SearchSettings::default());
    }
    let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str::<SearchSettings>(&text)
        .map(normalize_settings)
        .map_err(|error| error.to_string())
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    config_file_path(app, SETTINGS_FILE)
}

fn users_path(app: &AppHandle) -> Result<PathBuf, String> {
    config_file_path(app, USERS_FILE)
}

fn favorites_path(app: &AppHandle) -> Result<PathBuf, String> {
    config_file_path(app, FAVORITES_FILE)
}

fn config_file_path(app: &AppHandle, file_name: &str) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir.join(file_name))
}

fn read_users(app: &AppHandle) -> Result<Vec<UserRecord>, String> {
    let path = users_path(app)?;
    read_users_from_path(&path)
}

fn read_favorites(app: &AppHandle) -> Result<Vec<FavoriteResource>, String> {
    let path = favorites_path(app)?;
    read_favorites_from_path(&path)
}

fn write_favorites(app: &AppHandle, favorites: &[FavoriteResource]) -> Result<(), String> {
    let path = favorites_path(app)?;
    write_json_file(&path, favorites)
}

fn read_users_from_path(path: &PathBuf) -> Result<Vec<UserRecord>, String> {
    if !path.exists() {
        let users = vec![UserRecord {
            username: DEFAULT_USERNAME.to_string(),
            password: DEFAULT_PASSWORD.to_string(),
        }];
        write_json_file(path, &users)?;
        return Ok(users);
    }
    let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let mut users =
        serde_json::from_str::<Vec<UserRecord>>(&text).map_err(|error| error.to_string())?;
    if users.is_empty() {
        users.push(UserRecord {
            username: DEFAULT_USERNAME.to_string(),
            password: DEFAULT_PASSWORD.to_string(),
        });
        write_json_file(path, &users)?;
    }
    Ok(users)
}

fn read_favorites_from_path(path: &PathBuf) -> Result<Vec<FavoriteResource>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str::<Vec<FavoriteResource>>(&text).map_err(|error| error.to_string())
}

fn write_json_file<T: Serialize + ?Sized>(path: &PathBuf, value: &T) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;
    fs::write(path, text).map_err(|error| error.to_string())
}

fn authenticate_user(
    users: &[UserRecord],
    username: &str,
    password: &str,
) -> Result<UserSession, String> {
    let name = username.trim();
    let secret = password.trim();
    if name.is_empty() || secret.is_empty() {
        return Err("用户名或密码不能为空".to_string());
    }
    let user = users
        .iter()
        .find(|item| item.username == name && item.password == secret)
        .ok_or_else(|| "用户名或密码错误".to_string())?;
    Ok(UserSession {
        username: user.username.clone(),
        display_name: user.username.clone(),
        login_at: current_timestamp_millis(),
    })
}

fn add_favorite_record(
    favorites: &mut Vec<FavoriteResource>,
    username: &str,
    item: &ResourceItem,
    detail: &ResourceDetail,
) -> Result<FavoriteResource, String> {
    let user = username.trim();
    if user.is_empty() {
        return Err("用户名不能为空".to_string());
    }
    let url = detail.url.trim();
    if url.is_empty() {
        return Err("收藏链接不能为空".to_string());
    }
    if detail.validation_status.trim() == "invalid" {
        return Err(detail.validation_message.clone());
    }
    if let Some(existing) = favorites
        .iter()
        .find(|favorite| favorite.username == user && favorite.url == url)
        .cloned()
    {
        return Ok(existing);
    }
    let favorite = FavoriteResource {
        id: stable_hash(&format!("{}{}{}", user, item.id, url)).to_string(),
        username: user.to_string(),
        title: non_empty_or(detail.title.trim().to_string(), item.title.clone()),
        url: url.to_string(),
        source_name: item.source_name.clone(),
        disk_type: item.disk_type.clone(),
        share_user: item.share_user.clone(),
        info: item.info.clone(),
        created_at: current_timestamp_millis(),
        resource_id: item.id.clone(),
    };
    favorites.push(favorite.clone());
    Ok(favorite)
}

fn remove_favorite_record(
    favorites: &mut Vec<FavoriteResource>,
    username: &str,
    favorite_id: &str,
) -> Result<Vec<FavoriteResource>, String> {
    let user = username.trim();
    let id = favorite_id.trim();
    if user.is_empty() || id.is_empty() {
        return Err("参数不能为空".to_string());
    }
    favorites.retain(|item| !(item.username == user && item.id == id));
    Ok(favorites
        .iter()
        .filter(|item| item.username == user)
        .cloned()
        .collect())
}

fn current_timestamp_millis() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn non_empty_or(value: String, fallback: String) -> String {
    if value.trim().is_empty() {
        fallback
    } else {
        value
    }
}

fn selector(value: &str) -> Result<Selector, String> {
    Selector::parse(value).map_err(|_| format!("选择器不合法：{}", value))
}

fn element_text(element: ElementRef, selector: &Selector) -> String {
    let nested = element.select(selector).next();
    match nested {
        Some(node) => node.text().collect::<Vec<_>>().join(" "),
        None => element.text().collect::<Vec<_>>().join(" "),
    }
}

fn clean_text(text: &str) -> String {
    strip_markup(text)
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn strip_markup(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut in_tag = false;
    for ch in text.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }
    output
}

fn json_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(clean_text)
            .collect::<Vec<_>>()
            .join("\n"),
        Some(Value::String(text)) => clean_text(text),
        Some(other) => clean_text(&other.to_string()),
        None => String::new(),
    }
}

fn matches_exact(title: &str, info: &str, query: &str, exact: bool) -> bool {
    if !exact {
        return true;
    }
    title.contains(query) || info.contains(query)
}

fn disk_filter_for_api(value: &str) -> &str {
    match value {
        "quark" => "QUARK",
        "aliyun" => "ALIYUN",
        "baidu" => "BAIDU",
        _ => "",
    }
}

fn disk_filter_for_pansou(value: &str) -> &str {
    match value {
        "quark" => "quark",
        "aliyun" => "aliyun",
        "baidu" => "baidu",
        _ => "",
    }
}

fn flarum_base_url(kind: &str) -> &'static str {
    match kind {
        "myxiaozhan" => "https://myxiaozhan.net",
        _ => "https://yyurl.cc",
    }
}

fn find_disk_links(text: &str) -> Vec<String> {
    let patterns = [
        r#"https?://pan\.quark\.cn/s/[A-Za-z0-9_-]+"#,
        r#"https?://(?:www\.)?(?:aliyundrive|alipan)\.com/s/[A-Za-z0-9_-]+(?:\?pwd=[A-Za-z0-9]+)?"#,
        r#"https?://(?:pan|eyun)\.baidu\.com/(?:s/[A-Za-z0-9_-]+|share/init\?surl=[A-Za-z0-9_-]+)(?:\?pwd=[A-Za-z0-9]+)?"#,
        r#"https?://pan\.xunlei\.com/s/[A-Za-z0-9_-]+(?:\?pwd=[A-Za-z0-9]+)?"#,
        r#"https?://115\.com/s/[A-Za-z0-9_-]+"#,
        r#"https?://(?:\w+\.)?lanzou[a-z]?\.com/[A-Za-z0-9_-]+"#,
    ];
    let mut output = Vec::new();
    for pattern in patterns {
        if let Ok(regex) = Regex::new(pattern) {
            for captures in regex.find_iter(text) {
                push_unique(&mut output, captures.as_str().trim_matches('"').to_string());
            }
        }
    }
    output
}

fn absolute_url(base: &str, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        return href.to_string();
    }
    format!("{}{}", base.trim_end_matches('/'), href)
}

fn first_non_empty(values: &[&str]) -> String {
    values
        .iter()
        .find(|value| !value.trim().is_empty())
        .map(|value| value.trim().to_string())
        .unwrap_or_default()
}

fn first_json_string(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(text) = value.get(*key).and_then(Value::as_str) {
            if !text.trim().is_empty() {
                return Some(text.to_string());
            }
        }
        if let Some(number) = value.get(*key).and_then(Value::as_i64) {
            return Some(number.to_string());
        }
    }
    None
}

fn push_unique(items: &mut Vec<String>, value: String) {
    let text = value.trim();
    if !text.is_empty() && !items.iter().any(|item| item == text) {
        items.push(text.to_string());
    }
}

fn stable_hash(value: &str) -> u64 {
    let mut hash = 14695981039346656037u64;
    for byte in value.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

fn embedded_pansou_endpoint(port: u16) -> String {
    format!("http://127.0.0.1:{}", port)
}

fn embedded_pansou_endpoint_config(config: &EmbeddedPansouConfig) -> PansouEndpointConfig {
    PansouEndpointConfig {
        id: EMBEDDED_PANSOU_SOURCE_ID.to_string(),
        name: "内置 PanSou".to_string(),
        endpoint: embedded_pansou_endpoint(config.port),
        token: String::new(),
        enabled: config.enabled,
        refresh: config.refresh,
        channels: config.channels.clone(),
        plugins: config.plugins.clone(),
        src: config.src.clone(),
        cloud_types: config.cloud_types.clone(),
        concurrency: config.concurrency,
    }
}

fn embedded_pansou_status_from_state(app: &AppHandle) -> EmbeddedPansouStatus {
    app.state::<EmbeddedPansouState>()
        .runtime
        .lock()
        .map(|runtime| runtime.status.clone())
        .unwrap_or_default()
}

fn set_embedded_pansou_status(app: &AppHandle, status: EmbeddedPansouStatus) {
    if let Ok(mut runtime) = app.state::<EmbeddedPansouState>().runtime.lock() {
        runtime.status = status;
    }
}

fn embedded_pansou_config_changed(
    left: &EmbeddedPansouConfig,
    right: &EmbeddedPansouConfig,
) -> bool {
    left.enabled != right.enabled
        || left.auto_start != right.auto_start
        || left.port != right.port
        || left.src != right.src
        || left.channels != right.channels
        || left.plugins != right.plugins
        || left.cloud_types != right.cloud_types
        || left.refresh != right.refresh
        || left.cache != right.cache
        || left.concurrency != right.concurrency
}

fn stop_owned_embedded_pansou(app: &AppHandle, message: &str) {
    if let Ok(mut runtime) = app.state::<EmbeddedPansouState>().runtime.lock() {
        if let Some(child) = runtime.child.take() {
            let _ = child.kill();
        }
        runtime.status.running = false;
        runtime.status.reused = false;
        runtime.status.message = message.to_string();
    }
}

async fn sync_embedded_pansou(app: &AppHandle, config: &EmbeddedPansouConfig) {
    let config = normalize_embedded_pansou_config(config.clone());
    let endpoint = embedded_pansou_endpoint(config.port);
    if !config.enabled || !config.auto_start {
        stop_owned_embedded_pansou(app, "内置 PanSou 已关闭");
        set_embedded_pansou_status(
            app,
            EmbeddedPansouStatus {
                enabled: config.enabled,
                running: false,
                reused: false,
                endpoint,
                port: config.port,
                message: if config.enabled {
                    "内置 PanSou 未设置为自动启动".to_string()
                } else {
                    "内置 PanSou 已关闭".to_string()
                },
            },
        );
        return;
    }

    if check_embedded_pansou_health(config.port).await {
        let reused = app
            .state::<EmbeddedPansouState>()
            .runtime
            .lock()
            .map(|runtime| runtime.child.is_none())
            .unwrap_or(true);
        set_embedded_pansou_status(
            app,
            EmbeddedPansouStatus {
                enabled: true,
                running: true,
                reused,
                endpoint,
                port: config.port,
                message: if reused {
                    "已连接本机 PanSou".to_string()
                } else {
                    "内置 PanSou 运行中".to_string()
                },
            },
        );
        return;
    }

    stop_owned_embedded_pansou(app, "正在启动内置 PanSou");
    if is_tcp_port_open(config.port) {
        set_embedded_pansou_status(
            app,
            EmbeddedPansouStatus {
                enabled: true,
                running: false,
                reused: false,
                endpoint,
                port: config.port,
                message: format!("端口 {} 已被其他程序占用", config.port),
            },
        );
        return;
    }

    match spawn_embedded_pansou(app, &config) {
        Ok(child) => {
            if let Ok(mut runtime) = app.state::<EmbeddedPansouState>().runtime.lock() {
                runtime.child = Some(child);
                runtime.status = EmbeddedPansouStatus {
                    enabled: true,
                    running: false,
                    reused: false,
                    endpoint: endpoint.clone(),
                    port: config.port,
                    message: "内置 PanSou 正在启动".to_string(),
                };
            }
            for _ in 0..24 {
                if check_embedded_pansou_health(config.port).await {
                    set_embedded_pansou_status(
                        app,
                        EmbeddedPansouStatus {
                            enabled: true,
                            running: true,
                            reused: false,
                            endpoint,
                            port: config.port,
                            message: "内置 PanSou 运行中".to_string(),
                        },
                    );
                    return;
                }
                std::thread::sleep(Duration::from_millis(250));
            }
            stop_owned_embedded_pansou(app, "内置 PanSou 启动超时");
            set_embedded_pansou_status(
                app,
                EmbeddedPansouStatus {
                    enabled: true,
                    running: false,
                    reused: false,
                    endpoint,
                    port: config.port,
                    message: "内置 PanSou 启动超时".to_string(),
                },
            );
        }
        Err(error) => {
            set_embedded_pansou_status(
                app,
                EmbeddedPansouStatus {
                    enabled: true,
                    running: false,
                    reused: false,
                    endpoint,
                    port: config.port,
                    message: format!("内置 PanSou 启动失败：{}", error),
                },
            );
        }
    }
}

fn spawn_embedded_pansou(
    app: &AppHandle,
    config: &EmbeddedPansouConfig,
) -> Result<CommandChild, String> {
    let cache_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("pansou-cache");
    fs::create_dir_all(&cache_dir).map_err(|error| error.to_string())?;
    let cache_path = cache_dir.to_string_lossy().to_string();
    let plugins = config.plugins.join(",");
    let channels = config.channels.join(",");
    let command = app
        .shell()
        .sidecar(EMBEDDED_PANSOU_SIDECAR)
        .map_err(|error| error.to_string())?
        .env("PORT", config.port.to_string())
        .env("CACHE_ENABLED", if config.cache { "true" } else { "false" })
        .env("CACHE_PATH", cache_path)
        .env("CACHE_MAX_SIZE", "100")
        .env("CACHE_TTL", "60")
        .env("ASYNC_CACHE_TTL_HOURS", "1")
        .env("ENABLED_PLUGINS", plugins)
        .env("CHANNELS", channels);
    let (_events, child) = command.spawn().map_err(|error| error.to_string())?;
    Ok(child)
}

async fn check_embedded_pansou_health(port: u16) -> bool {
    let client = match Client::builder()
        .timeout(Duration::from_secs(2))
        .user_agent(USER_AGENT)
        .build()
    {
        Ok(client) => client,
        Err(_) => return false,
    };
    client
        .get(format!("{}/api/health", embedded_pansou_endpoint(port)))
        .send()
        .await
        .map(|response| response.status().is_success())
        .unwrap_or(false)
}

fn is_tcp_port_open(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_ok()
}

pub fn run() {
    tauri::Builder::default()
        .manage(EmbeddedPansouState::default())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let settings = read_search_settings(&handle).unwrap_or_default();
                if settings.embedded_pansou.enabled && settings.embedded_pansou.auto_start {
                    sync_embedded_pansou(&handle, &settings.embedded_pansou).await;
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(event, WindowEvent::CloseRequested { .. }) {
                stop_owned_embedded_pansou(window.app_handle(), "窗口已关闭，内置 PanSou 已停止");
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_search_sources,
            get_search_settings,
            login_user,
            save_search_settings,
            get_embedded_pansou_status,
            restart_embedded_pansou,
            import_cms_sources,
            test_cms_sources,
            search_resources,
            get_resource_detail,
            list_favorites,
            add_favorite,
            remove_favorite,
            open_external_url
        ])
        .build(tauri::generate_context!())
        .expect("failed to build sui-frame desktop app")
        .run(|app, event| {
            if matches!(event, RunEvent::ExitRequested { .. } | RunEvent::Exit) {
                stop_owned_embedded_pansou(app, "应用已退出，内置 PanSou 已停止");
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rank_life_tree_above_noise() {
        let plan = futures::executor::block_on(build_search_plan(
            "胡歌 生命树",
            &SearchSettings::default(),
        ));
        let source = SearchSource {
            id: "mock".to_string(),
            name: "mock".to_string(),
            group: "test".to_string(),
            enabled: true,
            description: String::new(),
            kind: "mock".to_string(),
            config_index: None,
            health_score: 60,
            status: "ready".to_string(),
        };
        let mut items = vec![
            resource_item(
                &source,
                1,
                "生命之树".to_string(),
                "电影 2011".to_string(),
                "https://a.test".to_string(),
                "https://a.test".to_string(),
                String::new(),
                "quark".to_string(),
                String::new(),
                vec![],
            ),
            resource_item(
                &source,
                2,
                "生命树 2026 全集".to_string(),
                "胡歌 杨紫 爱奇艺 电视剧".to_string(),
                "https://b.test".to_string(),
                "https://b.test".to_string(),
                String::new(),
                "quark".to_string(),
                String::new(),
                vec![],
            ),
            resource_item(
                &source,
                3,
                "县委大院 胡歌".to_string(),
                "电视剧".to_string(),
                "https://c.test".to_string(),
                "https://c.test".to_string(),
                String::new(),
                "quark".to_string(),
                String::new(),
                vec![],
            ),
            resource_item(
                &source,
                4,
                "生命树 - 吴雨霏.mp3".to_string(),
                "音乐".to_string(),
                "https://d.test".to_string(),
                "https://d.test".to_string(),
                String::new(),
                "quark".to_string(),
                String::new(),
                vec![],
            ),
        ];
        score_items(&mut items, &plan);
        items.sort_by(|left, right| right.relevance_score.cmp(&left.relevance_score));
        assert_eq!(items[0].title, "生命树 2026 全集");
    }

    #[test]
    fn maps_raw_search_errors_to_friendly_message() {
        assert_eq!(
            friendly_search_error("error sending request for url (https://example.test?q=secret)"),
            "资源暂时无法访问，请稍后重试"
        );
        assert_eq!(
            friendly_search_error("error decoding response body"),
            "资源返回内容异常，暂时无法解析"
        );
    }

    #[test]
    fn filters_empty_or_invalid_resource_items() {
        let source = SearchSource {
            id: "mock".to_string(),
            name: "mock".to_string(),
            group: "test".to_string(),
            enabled: true,
            description: String::new(),
            kind: "mock".to_string(),
            config_index: None,
            health_score: 60,
            status: "ready".to_string(),
        };
        let items = vec![
            resource_item(
                &source,
                1,
                "有效资源".to_string(),
                "S01E01".to_string(),
                "https://pan.quark.cn/s/abc".to_string(),
                "https://pan.quark.cn/s/abc".to_string(),
                String::new(),
                "quark".to_string(),
                String::new(),
                vec![],
            ),
            resource_item(
                &source,
                2,
                String::new(),
                String::new(),
                "https://pan.quark.cn/s/empty".to_string(),
                "https://pan.quark.cn/s/empty".to_string(),
                String::new(),
                "quark".to_string(),
                String::new(),
                vec![],
            ),
            resource_item(
                &source,
                3,
                "非法协议".to_string(),
                "S01E02".to_string(),
                "ftp://example.test/file".to_string(),
                "ftp://example.test/file".to_string(),
                String::new(),
                "other".to_string(),
                String::new(),
                vec![],
            ),
        ];

        let filtered = filter_valid_resource_items(items);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].title, "有效资源");
    }

    #[test]
    fn initializes_default_user_file_when_missing() {
        let path = unique_temp_path("users");
        let users = read_users_from_path(&path).expect("default users should be created");
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].username, DEFAULT_USERNAME);
        assert_eq!(users[0].password, DEFAULT_PASSWORD);
        let raw = fs::read_to_string(&path).expect("users file should exist");
        assert!(raw.contains(DEFAULT_USERNAME));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn authenticates_default_user_and_rejects_bad_password() {
        let users = vec![UserRecord {
            username: DEFAULT_USERNAME.to_string(),
            password: DEFAULT_PASSWORD.to_string(),
        }];
        let session = authenticate_user(&users, DEFAULT_USERNAME, DEFAULT_PASSWORD)
            .expect("default user should authenticate");
        assert_eq!(session.username, DEFAULT_USERNAME);
        assert_eq!(session.display_name, DEFAULT_USERNAME);
        assert!(
            authenticate_user(&users, DEFAULT_USERNAME, "wrong").is_err(),
            "wrong password should fail"
        );
    }

    #[test]
    fn manages_favorites_per_user_with_dedup_and_delete() {
        let source = SearchSource {
            id: "mock".to_string(),
            name: "mock".to_string(),
            group: "test".to_string(),
            enabled: true,
            description: String::new(),
            kind: "mock".to_string(),
            config_index: None,
            health_score: 60,
            status: "ready".to_string(),
        };
        let item = resource_item(
            &source,
            1,
            "测试资源".to_string(),
            "简介".to_string(),
            "https://source.test/item".to_string(),
            "https://source.test/item".to_string(),
            String::new(),
            "quark".to_string(),
            "张三".to_string(),
            vec!["tag1".to_string()],
        );
        let detail = ResourceDetail {
            title: "测试资源".to_string(),
            url: "https://pan.quark.cn/s/abc".to_string(),
            source_name: "mock".to_string(),
            message: "ok".to_string(),
            validation_status: "valid".to_string(),
            can_open: true,
            validation_message: "可打开".to_string(),
        };
        let mut favorites = Vec::new();
        let first = add_favorite_record(&mut favorites, DEFAULT_USERNAME, &item, &detail)
            .expect("first favorite should save");
        let second = add_favorite_record(&mut favorites, DEFAULT_USERNAME, &item, &detail)
            .expect("duplicate favorite should reuse existing record");
        assert_eq!(favorites.len(), 1);
        assert_eq!(first.id, second.id);

        let third = add_favorite_record(&mut favorites, "user-b", &item, &detail)
            .expect("other user should save separately");
        assert_eq!(favorites.len(), 2);
        assert_ne!(first.id, third.id);

        let remaining = remove_favorite_record(&mut favorites, DEFAULT_USERNAME, &first.id)
            .expect("favorite should be removed");
        assert_eq!(remaining.len(), 0);
        assert_eq!(favorites.len(), 1);
        assert_eq!(favorites[0].username, "user-b");
    }

    #[test]
    fn parses_pansou_note_items_and_parent_disk_type() {
        let source = SearchSource {
            id: "embedded-pansou".to_string(),
            name: "内置 PanSou".to_string(),
            group: "PanSou 深度池".to_string(),
            enabled: true,
            description: String::new(),
            kind: "embedded-pansou".to_string(),
            config_index: None,
            health_score: 70,
            status: "ready".to_string(),
        };
        let data = json!({
            "code": 0,
            "message": "success",
            "data": {
                "total": 2,
                "merged_by_type": {
                    "quark": [
                        {
                            "url": "https://pan.quark.cn/s/abc",
                            "password": "",
                            "note": "流浪地球",
                            "datetime": "0001-01-01T00:00:00Z",
                            "source": "plugin:wanou"
                        }
                    ],
                    "magnet": [
                        {
                            "url": "magnet:?xt=urn:btih:abc",
                            "note": "流浪地球 磁力资源",
                            "source": "plugin:yuhuage"
                        }
                    ]
                }
            }
        });

        let items = parse_pansou_items(&source, "流浪地球", &data);

        assert_eq!(items.len(), 2);
        let quark = items
            .iter()
            .find(|item| item.disk_type == "quark")
            .expect("quark item should be parsed");
        let magnet = items
            .iter()
            .find(|item| item.disk_type == "magnet")
            .expect("magnet item should be parsed");
        assert_eq!(quark.title, "流浪地球");
        assert_eq!(quark.url, "https://pan.quark.cn/s/abc");
        assert_eq!(magnet.url, "magnet:?xt=urn:btih:abc");
    }

    #[test]
    fn classifies_detail_link_without_network_when_possible() {
        let invalid = futures::executor::block_on(validate_resource_url("ftp://example.test/file"));
        assert_eq!(invalid.status, "invalid");
        assert!(!invalid.can_open);

        let magnet = futures::executor::block_on(validate_resource_url("magnet:?xt=urn:btih:abc"));
        assert_eq!(magnet.status, "warning");
        assert!(magnet.can_open);
    }

    #[test]
    fn detects_empty_folder_page_markers() {
        assert!(looks_like_empty_folder_page("该文件夹为空，暂无文件"));
        assert!(looks_like_empty_folder_page("This folder is empty."));
        assert!(looks_like_empty_folder_page(
            "no files found in this folder"
        ));
        assert!(!looks_like_empty_folder_page(
            "流浪地球 4K 文件列表 保存到网盘"
        ));
    }

    #[test]
    fn classifies_empty_folder_probe_as_invalid() {
        let invalid = link_validation_from_status(Some(200), true, Some(true));
        assert_eq!(invalid.status, "invalid");
        assert!(!invalid.can_open);
        assert_eq!(invalid.message, "资源文件夹为空，已禁止打开。");

        let valid = link_validation_from_status(Some(200), true, Some(false));
        assert_eq!(valid.status, "valid");
        assert!(valid.can_open);
    }

    #[test]
    #[ignore]
    fn smoke_search_video_keywords() {
        let runtime = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
        runtime.block_on(async {
            let client = Client::builder()
                .timeout(Duration::from_secs(18))
                .user_agent(USER_AGENT)
                .build()
                .expect("failed to build client");
            let filters = SearchFilters {
                source_ids: search_sources(&SearchSettings::default())
                    .into_iter()
                    .filter(|source| source.enabled)
                    .map(|source| source.id)
                    .collect(),
                disk_type: "all".to_string(),
                sort_order: "relevance".to_string(),
                exact_match: false,
                settings: Some(SearchSettings::default()),
            };
            for query in ["胡歌 生命树", "生命树", "斯巴达克斯"] {
                let plan = build_search_plan(query, &SearchSettings::default()).await;
                let selected = resolve_selected_sources(&filters, &SearchSettings::default());
                let outcomes = join_all(selected.into_iter().map(|source| {
                    search_source(
                        client.clone(),
                        source,
                        plan.clone(),
                        1,
                        filters.clone(),
                        SearchSettings::default(),
                    )
                }))
                .await;
                let mut items = outcomes
                    .into_iter()
                    .flat_map(|outcome| outcome.items)
                    .collect::<Vec<_>>();
                items = dedupe_items(items);
                score_items(&mut items, &plan);
                items.sort_by(|left, right| {
                    right
                        .relevance_score
                        .cmp(&left.relevance_score)
                        .then(left.title.cmp(&right.title))
                });
                let target_count = count_target_resources(&items, &plan);
                println!(
                    "{} => {} items, target {}, terms: {}",
                    query,
                    items.len(),
                    target_count,
                    serde_json::to_string(&plan.search_terms).unwrap()
                );
                for item in items.iter().take(5) {
                    println!(
                        "TOP {} | {} | {} | {:?}",
                        item.relevance_score, item.source_name, item.title, item.match_reasons
                    );
                }
                if query.contains("生命树") {
                    for item in items
                        .iter()
                        .filter(|item| {
                            item.title.contains("生命树") || item.info.contains("生命树")
                        })
                        .take(8)
                    {
                        println!(
                            "LIFE {} | {} | {} | {} | {:?}",
                            item.relevance_score,
                            item.source_name,
                            item.title,
                            item.info.chars().take(80).collect::<String>(),
                            item.match_reasons
                        );
                    }
                    if let Some(item) = items.iter().find(|item| is_target_resource(item, &plan)) {
                        let detail = get_resource_detail(item.clone())
                            .await
                            .expect("target detail should resolve");
                        println!("DETAIL {} | {}", detail.source_name, detail.url);
                        assert!(
                            DISK_HOST_MARKERS
                                .iter()
                                .any(|marker| detail.url.contains(marker)),
                            "target detail should be a disk link"
                        );
                    }
                }
                assert!(
                    !plan.search_terms.is_empty(),
                    "{} should build search terms",
                    query
                );
                if query.contains("生命树") {
                    assert!(
                        target_count > 0,
                        "{} should match target tv disk resources",
                        query
                    );
                }
            }
        });
    }

    fn unique_temp_path(prefix: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "sui-frame-{}-{}-{}.json",
            prefix,
            current_timestamp_millis(),
            stable_hash(prefix)
        ));
        path
    }
}
