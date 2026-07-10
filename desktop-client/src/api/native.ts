import { invoke as tauriInvoke } from '@tauri-apps/api/core'

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

function isTauriRuntime() {
  return typeof window !== 'undefined' && Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__)
}

function invoke<T>(command: string, args?: Parameters<typeof tauriInvoke>[1]) {
  if (!isTauriRuntime()) {
    return Promise.reject(new Error('当前页面需要在影岁桌面客户端中运行，请通过 Tauri 开发模式或已安装的桌面应用打开。'))
  }
  return tauriInvoke<T>(command, args)
}

export function listSearchSources(settings?: SearchSettings) {
  return invoke<SearchSource[]>('list_search_sources', { settings })
}

export function getSearchSettings() {
  return invoke<SearchSettings>('get_search_settings')
}

export function saveSearchSettings(settings: SearchSettings) {
  return invoke<SearchSettings>('save_search_settings', { settings })
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

export function openExternalUrl(url: string) {
  return invoke<void>('open_external_url', { url })
}
