import { invoke as tauriInvoke } from '@tauri-apps/api/core'
import { getVersion } from '@tauri-apps/api/app'
import { relaunch } from '@tauri-apps/plugin-process'
import { check, type DownloadEvent, type Update } from '@tauri-apps/plugin-updater'

const UPDATE_CHECK_TIMEOUT_MS = 15000
const UPDATE_DOWNLOAD_TIMEOUT_MS = 120000

export interface SearchSource {
  id: string
  name: string
  group: string
  enabled: boolean
  description: string
  kind: string
  configIndex?: number
  healthScore: number
  status: 'ready' | 'configured' | 'requiresConfig' | string
}

export interface SearchFilters {
  sourceIds: string[]
  diskType: string
  sortOrder: string
  exactMatch: boolean
  settings?: SearchSettings
}

export interface SearchSettings {
  embeddedPansou: EmbeddedPansouConfig
  pansouEndpoint: string
  pansouToken: string
  pansouRefresh: boolean
  pansouEndpoints: PansouEndpointConfig[]
  pansouChannels: string[]
  pansouPlugins: string[]
  pansouSrc: 'all' | 'tg' | 'plugin'
  pansouCloudTypes: string[]
  pansouCache: boolean
  pansouConcurrency: number
  cmsEndpoints: string[]
  cmsSources: CmsSourceConfig[]
  indexers: IndexerConfig[]
  tmdbApiKey: string
}

export interface EmbeddedPansouConfig {
  enabled: boolean
  autoStart: boolean
  port: number
  src: 'all' | 'tg' | 'plugin'
  channels: string[]
  plugins: string[]
  cloudTypes: string[]
  refresh: boolean
  cache: boolean
  concurrency: number
}

export interface EmbeddedPansouStatus {
  enabled: boolean
  running: boolean
  reused: boolean
  endpoint: string
  port: number
  message: string
}

export interface PansouEndpointConfig {
  id: string
  name: string
  endpoint: string
  token: string
  enabled: boolean
  refresh: boolean
  channels: string[]
  plugins: string[]
  src: 'all' | 'tg' | 'plugin'
  cloudTypes: string[]
  concurrency: number
}

export interface CmsSourceConfig {
  id: string
  name: string
  endpoint: string
  enabled: boolean
  lastSuccessAt: string
  failureCount: number
  averageCount: number
  healthScore: number
}

export interface IndexerConfig {
  id: string
  name: string
  baseUrl: string
  apiKey: string
  indexerType: 'torznab' | 'newznab'
  categories: string[]
  enabled: boolean
}

export interface CmsHealthResult {
  id: string
  name: string
  endpoint: string
  ok: boolean
  count: number
  elapsedMs: number
  message: string
}

export interface SourceCoverage {
  group: string
  total: number
  success: number
  failed: number
  disabled: number
  count: number
  message: string
}

export interface MediaCandidate {
  id: string
  title: string
  originalTitle: string
  year: string
  mediaType: string
  actors: string[]
  aliases: string[]
  platforms: string[]
  overview: string
  confidence: number
  source: string
}

export interface SearchPlan {
  originalQuery: string
  activeCandidateId: string
  candidates: MediaCandidate[]
  searchTerms: string[]
  includeKeywords: string[]
  excludeKeywords: string[]
}

export interface ResourcePayload {
  title: string
  finalUrl: string
  detailUrl: string
  password: string
}

export interface ResourceItem {
  id: string
  title: string
  info: string
  url: string
  sourceId: string
  sourceName: string
  diskType: string
  shareUser: string
  tags: string[]
  payload: ResourcePayload
  relevanceScore: number
  relevanceLevel: 'high' | 'possible' | 'low'
  matchReasons: string[]
}

export interface ResultGroup {
  key: 'high' | 'possible' | 'low'
  title: string
  items: ResourceItem[]
}

export interface ResourceDetail {
  title: string
  url: string
  sourceName: string
  message: string
  validationStatus: 'valid' | 'warning' | 'invalid'
  canOpen: boolean
  validationMessage: string
}

export interface UserSession {
  username: string
  displayName: string
  loginAt: string
}

export interface FavoriteResource {
  id: string
  username: string
  title: string
  url: string
  sourceName: string
  diskType: string
  shareUser: string
  info: string
  createdAt: string
  resourceId: string
}

export interface SourceSearchState {
  sourceId: string
  sourceName: string
  group: string
  kind: string
  status: 'success' | 'empty' | 'failed' | 'disabled'
  message: string
  count: number
  elapsedMs: number
  healthScore: number
}

export interface SearchResponse {
  items: ResourceItem[]
  groups: ResultGroup[]
  states: SourceSearchState[]
  coverage: SourceCoverage[]
  targetResourceCount: number
  targetResourceMessage: string
  searchPlan: SearchPlan
}

export interface UpdateCheckResult {
  currentVersion: string
  update?: Update
}

export function isTauriRuntime() {
  return typeof window !== 'undefined' && Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__)
}

function invoke<T>(command: string, args?: Parameters<typeof tauriInvoke>[1]) {
  if (!isTauriRuntime()) {
    return Promise.reject(new Error('当前页面需要在影岁桌面客户端中运行，请通过 Tauri 开发模式或已安装的桌面应用打开。'))
  }
  return tauriInvoke<T>(command, args)
}

export function getCurrentVersion() {
  return getVersion()
}

export function formatUpdateError(error: unknown) {
  const text = String(error)
  if (text.includes('error sending request') || text.includes('timed out') || text.includes('timeout')) {
    return '连接更新服务失败，请稍后重试或检查当前网络。'
  }
  return text
}

export async function checkAppUpdate(): Promise<UpdateCheckResult> {
  const update = await check({ timeout: UPDATE_CHECK_TIMEOUT_MS })
  return {
    currentVersion: update?.currentVersion || await getVersion(),
    update: update || undefined
  }
}

export async function installAppUpdate(
  update: Update,
  onEvent: (event: DownloadEvent) => void
) {
  await update.downloadAndInstall(onEvent, { timeout: UPDATE_DOWNLOAD_TIMEOUT_MS })
  await relaunch()
}

export function listSearchSources(settings?: SearchSettings) {
  return invoke<SearchSource[]>('list_search_sources', { settings })
}

export function getSearchSettings() {
  return invoke<SearchSettings>('get_search_settings')
}

export function loginUser(username: string, password: string) {
  return invoke<UserSession>('login_user', { username, password })
}

export function saveSearchSettings(settings: SearchSettings) {
  return invoke<SearchSettings>('save_search_settings', { settings })
}

export function getEmbeddedPansouStatus() {
  return invoke<EmbeddedPansouStatus>('get_embedded_pansou_status')
}

export function restartEmbeddedPansou(settings: SearchSettings) {
  return invoke<EmbeddedPansouStatus>('restart_embedded_pansou', { settings })
}

export function importCmsSources(text: string, settings: SearchSettings) {
  return invoke<SearchSettings>('import_cms_sources', { text, settings })
}

export function testCmsSources(settings: SearchSettings) {
  return invoke<CmsHealthResult[]>('test_cms_sources', { settings })
}

export function searchResources(query: string, page: number, filters: SearchFilters) {
  return invoke<SearchResponse>('search_resources', { query, page, filters })
}

export function getResourceDetail(item: ResourceItem) {
  return invoke<ResourceDetail>('get_resource_detail', { item })
}

export function listFavorites(username: string) {
  return invoke<FavoriteResource[]>('list_favorites', { username })
}

export function addFavorite(username: string, item: ResourceItem, detail: ResourceDetail) {
  return invoke<FavoriteResource>('add_favorite', { username, item, detail })
}

export function removeFavorite(username: string, favoriteId: string) {
  return invoke<FavoriteResource[]>('remove_favorite', { username, favoriteId })
}

export function openExternalUrl(url: string) {
  return invoke<void>('open_external_url', { url })
}
