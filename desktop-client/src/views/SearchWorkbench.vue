<template>
  <main class="workbench">
    <aside class="rail">
      <div class="brand-mark">
        <img src="../assets/brand/sui-frame-icon.svg" alt="影岁" />
      </div>
      <a-tooltip title="资源搜索">
        <button class="rail-button active" type="button">
          <Search :size="22" />
        </button>
      </a-tooltip>
      <a-tooltip title="规则状态">
        <button class="rail-button" type="button">
          <Activity :size="22" />
        </button>
      </a-tooltip>
      <div class="rail-spacer"></div>
      <a-tooltip title="桌面端 v0.1.0">
        <button class="rail-button" type="button" @click="openSettingsPanel">
          <Settings :size="22" />
        </button>
      </a-tooltip>
    </aside>

    <section class="stage">
      <header class="hero">
        <div class="hero-title">
          <img src="../assets/brand/sui-frame-icon.svg" alt="影岁" />
          <div>
            <p>影岁</p>
            <h1>影视资源检索台</h1>
          </div>
        </div>
        <div class="hero-meta">
          <span>{{ enabledCount }} 个来源可用</span>
          <span>桌面端</span>
        </div>
      </header>

      <section class="search-panel">
        <div class="search-line">
          <a-input
            v-model:value="query"
            size="large"
            placeholder="输入片名、剧名、演员或关键词"
            allow-clear
            @press-enter="handleSearch"
          />
          <a-button type="primary" size="large" :loading="loading" @click="handleSearch">
            <template #icon><Search :size="18" /></template>
            搜索
          </a-button>
        </div>

        <div class="source-groups">
          <div v-for="group in sourceGroups" :key="group.name" class="source-group">
            <div class="source-group-title">
              <span>{{ group.name }}</span>
              <small>{{ group.sources.filter((source) => source.enabled).length }} / {{ group.sources.length }}</small>
            </div>
            <div class="source-strip">
              <button
                v-for="source in group.sources"
                :key="source.id"
                class="source-chip"
                :class="{ selected: selectedSourceIds.includes(source.id), disabled: !source.enabled, configurable: source.status === 'requiresConfig' }"
                type="button"
                :disabled="!source.enabled || loading"
                @click="toggleSource(source.id)"
              >
                <span>{{ source.name }}</span>
                <small>{{ sourceLabel(source) }}</small>
              </button>
            </div>
          </div>
        </div>

        <div class="filter-row">
          <a-select v-model:value="sortOrder" class="filter-select" :disabled="loading">
            <a-select-option value="relevance">综合排序</a-select-option>
            <a-select-option value="source">按来源归组</a-select-option>
          </a-select>
          <a-select v-model:value="diskType" class="filter-select" :disabled="loading">
            <a-select-option value="all">全部网盘</a-select-option>
            <a-select-option value="quark">夸克</a-select-option>
            <a-select-option value="aliyun">阿里</a-select-option>
            <a-select-option value="baidu">百度</a-select-option>
          </a-select>
          <a-checkbox v-model:checked="exactMatch" :disabled="loading">精确匹配</a-checkbox>
          <a-button :disabled="loading" @click="resetFilters">
            <template #icon><RefreshCw :size="16" /></template>
            重置
          </a-button>
        </div>
      </section>

      <section class="status-board" v-if="states.length">
        <div
          v-for="state in states"
          :key="state.sourceId"
          class="status-card"
          :class="state.status"
        >
          <CheckCircle2 v-if="state.status === 'success'" :size="18" />
          <CircleAlert v-else-if="state.status === 'failed'" :size="18" />
          <LoaderCircle v-else-if="loading" class="spin" :size="18" />
          <Activity v-else :size="18" />
          <span>{{ state.sourceName }}</span>
          <strong>{{ state.count }}</strong>
          <small>{{ state.message }}</small>
        </div>
      </section>

      <section v-if="coverage.length" class="coverage-panel">
        <div class="coverage-head">
          <div>
            <h2>来源覆盖</h2>
            <p>按资源池类型统计本次召回广度和失败情况</p>
          </div>
          <span>{{ coverage.reduce((sum, item) => sum + item.count, 0) }} 条召回</span>
        </div>
        <div class="coverage-grid">
          <article v-for="item in coverage" :key="item.group" class="coverage-card">
            <div>
              <strong>{{ item.group }}</strong>
              <small>{{ item.message }}</small>
            </div>
            <div class="coverage-meter">
              <span :style="{ width: `${coverageRate(item)}%` }"></span>
            </div>
          </article>
        </div>
      </section>

      <section v-if="searchPlan" class="plan-panel">
        <div class="candidate-area">
          <div class="panel-title">
            <span>识别到的影视条目</span>
            <small>{{ searchPlan.originalQuery }}</small>
          </div>
          <div class="candidate-list">
            <button
              v-for="candidate in searchPlan.candidates"
              :key="candidate.id"
              class="candidate-card"
              :class="{ active: candidate.id === searchPlan.activeCandidateId }"
              type="button"
            >
              <strong>{{ candidate.title }}</strong>
              <span>{{ candidate.mediaType }} {{ candidate.year }}</span>
              <small>{{ candidate.actors.join(' / ') || candidate.source }}</small>
            </button>
          </div>
        </div>
        <div class="term-area">
          <div class="panel-title">
            <span>实际资源搜索词</span>
            <small>自动降噪</small>
          </div>
          <div class="term-list">
            <a-tag v-for="term in searchPlan.searchTerms" :key="term" color="blue">{{ term }}</a-tag>
          </div>
        </div>
      </section>

      <section class="result-shell">
        <div class="result-head">
          <div>
            <h2>{{ resultTitle }}</h2>
            <p>{{ resultHint }}</p>
            <div v-if="lastQuery" class="target-summary" :class="{ missed: targetResourceCount === 0 }">
              {{ targetResourceMessage }}
            </div>
          </div>
          <a-button v-if="lastQuery" :disabled="loading" @click="handleSearch">
            <template #icon><RefreshCw :size="16" /></template>
            重新搜索
          </a-button>
        </div>

        <a-empty v-if="!loading && !items.length" description="输入关键词后开始检索" />

        <div v-else class="group-list">
          <section v-for="group in groups" :key="group.key" class="result-group">
            <div class="group-head">
              <h3>{{ group.title }}</h3>
              <span>{{ group.items.length }} 条</span>
            </div>
            <div class="result-list">
              <article
                v-for="item in group.items"
                :key="item.id"
                class="result-item"
                @click="openDetail(item)"
              >
                <div class="result-main">
                  <h3 v-html="highlightTitle(item.title)"></h3>
                  <pre>{{ item.info || '暂无文件摘要' }}</pre>
                  <div class="reason-row">
                    <a-tag color="green">评分 {{ item.relevanceScore }}</a-tag>
                    <a-tag v-for="reason in item.matchReasons.slice(0, 4)" :key="`${item.id}-${reason}`">{{ reason }}</a-tag>
                  </div>
                  <div class="tag-row">
                    <a-tag color="cyan">{{ item.sourceName }}</a-tag>
                    <a-tag v-if="item.diskType">{{ item.diskType }}</a-tag>
                    <a-tag v-if="item.shareUser">{{ item.shareUser }}</a-tag>
                    <a-tag v-for="tag in item.tags.slice(0, 3)" :key="`${item.id}-${tag}`">{{ tag }}</a-tag>
                  </div>
                </div>
                <button class="detail-button" type="button">
                  <Film :size="18" />
                  详情
                </button>
              </article>
            </div>
          </section>
        </div>
      </section>
    </section>

    <a-modal
      v-model:open="settingsOpen"
      title="资源池配置中心"
      width="920px"
      :confirm-loading="settingsSaving"
      ok-text="保存"
      cancel-text="取消"
      @ok="saveSettingsPanel"
    >
      <div class="settings-form">
        <section class="settings-section">
          <div class="settings-section-head">
            <div>
              <h3>PanSou 深度池</h3>
              <p>支持多个 endpoint，并可配置 TG 频道、插件、来源范围和网盘类型。</p>
            </div>
            <a-button @click="addPansouEndpoint">
              <template #icon><Plus :size="16" /></template>
              增加
            </a-button>
          </div>
          <div class="pool-list">
            <article v-for="(endpoint, index) in settingsForm.pansouEndpoints" :key="endpoint.id || index" class="pool-row">
              <a-checkbox v-model:checked="endpoint.enabled" />
              <a-input v-model:value="endpoint.name" placeholder="名称" />
              <a-input v-model:value="endpoint.endpoint" placeholder="http://127.0.0.1:8888" />
              <a-input-password v-model:value="endpoint.token" placeholder="token 可选" />
              <a-select v-model:value="endpoint.src" class="compact-select">
                <a-select-option value="all">全部</a-select-option>
                <a-select-option value="tg">TG</a-select-option>
                <a-select-option value="plugin">插件</a-select-option>
              </a-select>
              <a-input v-model:value="endpoint.channelsText" placeholder="频道，逗号分隔" />
              <a-input v-model:value="endpoint.pluginsText" placeholder="插件，逗号分隔" />
              <a-input v-model:value="endpoint.cloudTypesText" placeholder="网盘类型，逗号分隔" />
              <a-checkbox v-model:checked="endpoint.refresh">刷新</a-checkbox>
              <a-input-number v-model:value="endpoint.concurrency" :min="1" :max="8" class="number-input" />
              <button class="icon-action danger" type="button" @click="removePansouEndpoint(index)">
                <Trash2 :size="16" />
              </button>
            </article>
          </div>
        </section>

        <section class="settings-section">
          <div class="settings-section-head">
            <div>
              <h3>CMS V10 源池</h3>
              <p>支持 JSON 或文本批量导入，健康检测会请求 `生命树` 验证结构和返回量。</p>
            </div>
            <div class="settings-actions">
              <a-button :loading="cmsTesting" @click="handleTestCmsSources">
                <template #icon><Activity :size="16" /></template>
                测试
              </a-button>
              <a-button @click="addCmsSource">
                <template #icon><Plus :size="16" /></template>
                增加
              </a-button>
            </div>
          </div>
          <a-textarea
            v-model:value="cmsImportText"
            :rows="3"
            placeholder="批量导入：每行一个地址，或 名称,地址，也支持 JSON 数组"
          />
          <div class="settings-actions">
            <a-button @click="handleImportCmsSources">导入到源池</a-button>
          </div>
          <div class="pool-list">
            <article v-for="(source, index) in settingsForm.cmsSources" :key="source.id || index" class="pool-row cms-row">
              <a-checkbox v-model:checked="source.enabled" />
              <a-input v-model:value="source.name" placeholder="名称" />
              <a-input v-model:value="source.endpoint" placeholder="https://example.com/api.php/provide/vod/" />
              <span class="health-pill">健康 {{ source.healthScore || 60 }}</span>
              <button class="icon-action danger" type="button" @click="removeCmsSource(index)">
                <Trash2 :size="16" />
              </button>
            </article>
          </div>
          <div v-if="cmsHealthResults.length" class="health-list">
            <div v-for="result in cmsHealthResults" :key="result.endpoint" :class="{ ok: result.ok }">
              <strong>{{ result.name }}</strong>
              <span>{{ result.count }} 条 / {{ result.elapsedMs }}ms</span>
              <small>{{ result.message }}</small>
            </div>
          </div>
        </section>

        <section class="settings-section">
          <div class="settings-section-head">
            <div>
              <h3>Torznab / Newznab 索引器</h3>
              <p>只接入用户自有或有权限的索引器，执行搜索和跳转，不自动下载。</p>
            </div>
            <a-button @click="addIndexer">
              <template #icon><Plus :size="16" /></template>
              增加
            </a-button>
          </div>
          <div class="pool-list">
            <article v-for="(indexer, index) in settingsForm.indexers" :key="indexer.id || index" class="pool-row indexer-row">
              <a-checkbox v-model:checked="indexer.enabled" />
              <a-input v-model:value="indexer.name" placeholder="名称" />
              <a-input v-model:value="indexer.baseUrl" placeholder="Torznab/Newznab API 地址" />
              <a-input-password v-model:value="indexer.apiKey" placeholder="API Key" />
              <a-select v-model:value="indexer.indexerType" class="compact-select">
                <a-select-option value="torznab">Torznab</a-select-option>
                <a-select-option value="newznab">Newznab</a-select-option>
              </a-select>
              <a-input v-model:value="indexer.categoriesText" placeholder="分类 ID，逗号分隔" />
              <button class="icon-action danger" type="button" @click="removeIndexer(index)">
                <Trash2 :size="16" />
              </button>
            </article>
          </div>
        </section>

        <section class="settings-section">
          <label>
            <span>TMDB API Key</span>
            <a-input-password v-model:value="settingsForm.tmdbApiKey" placeholder="可选，用于影视实体识别增强" />
          </label>
          <div class="settings-checks">
            <a-checkbox v-model:checked="settingsForm.pansouCache">PanSou 使用缓存</a-checkbox>
            <a-checkbox v-model:checked="settingsForm.pansouRefresh">全局 PanSou 强制刷新</a-checkbox>
          </div>
          <p class="settings-tip">默认内置公开页面搜索源；不内置 tracker、公开 PanSou 服务或 CMS 源。需要 code/cookie 的来源会显示为需配置，默认不参与搜索。</p>
        </section>
      </div>
    </a-modal>

    <a-modal
      v-model:open="detailOpen"
      title="结果详情"
      width="620px"
      :footer="null"
      centered
      @cancel="closeDetail"
    >
      <div v-if="detailLoading" class="detail-loading">
        <LoaderCircle class="spin" :size="28" />
        <span>正在解析跳转地址</span>
      </div>
      <div v-else-if="detail" class="detail-box">
        <button class="modal-close" type="button" @click="closeDetail">
          <X :size="20" />
        </button>
        <h3>{{ detail.title }}</h3>
        <a :href="detail.url" target="_blank" rel="noreferrer">{{ detail.url }}</a>
        <p v-if="detail.message">{{ detail.message }}</p>
        <div class="detail-actions">
          <a-button @click="copyUrl(detail.url)">
            <template #icon><Clipboard :size="16" /></template>
            复制
          </a-button>
          <a-button type="primary" @click="openUrl(detail.url)">
            <template #icon><ExternalLink :size="16" /></template>
            打开
          </a-button>
        </div>
      </div>
    </a-modal>
  </main>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { message } from 'ant-design-vue'
import {
  Activity,
  CheckCircle2,
  CircleAlert,
  Clipboard,
  ExternalLink,
  Film,
  LoaderCircle,
  Plus,
  RefreshCw,
  Search,
  Settings,
  Trash2,
  X
} from 'lucide-vue-next'
import {
  getSearchSettings,
  getResourceDetail,
  importCmsSources,
  listSearchSources,
  openExternalUrl,
  saveSearchSettings,
  searchResources,
  testCmsSources,
  type CmsHealthResult,
  type CmsSourceConfig,
  type IndexerConfig,
  type PansouEndpointConfig,
  type ResourceDetail,
  type ResourceItem,
  type ResultGroup,
  type SearchPlan,
  type SearchSettings,
  type SearchSource,
  type SourceCoverage,
  type SourceSearchState
} from '../api/native'

type EditablePansouEndpoint = PansouEndpointConfig & {
  channelsText: string
  pluginsText: string
  cloudTypesText: string
}

type EditableIndexer = IndexerConfig & {
  categoriesText: string
}

type EditableSearchSettings = Omit<SearchSettings, 'pansouEndpoints' | 'indexers'> & {
  pansouEndpoints: EditablePansouEndpoint[]
  indexers: EditableIndexer[]
}

const DEFAULT_SETTINGS: EditableSearchSettings = {
  pansouEndpoint: '',
  pansouToken: '',
  pansouRefresh: false,
  pansouEndpoints: [],
  pansouChannels: [],
  pansouPlugins: [],
  pansouSrc: 'all',
  pansouCloudTypes: [],
  pansouCache: true,
  pansouConcurrency: 4,
  cmsEndpoints: [],
  cmsSources: [],
  indexers: [],
  tmdbApiKey: ''
}

const query = ref('')
const lastQuery = ref('')
const loading = ref(false)
const detailLoading = ref(false)
const detailOpen = ref(false)
const settingsOpen = ref(false)
const settingsSaving = ref(false)
const exactMatch = ref(false)
const sortOrder = ref('relevance')
const diskType = ref('all')
const sources = ref<SearchSource[]>([])
const selectedSourceIds = ref<string[]>([])
const items = ref<ResourceItem[]>([])
const groups = ref<ResultGroup[]>([])
const states = ref<SourceSearchState[]>([])
const coverage = ref<SourceCoverage[]>([])
const targetResourceCount = ref(0)
const targetResourceMessage = ref('')
const detail = ref<ResourceDetail>()
const searchPlan = ref<SearchPlan>()
const settingsForm = ref<EditableSearchSettings>(cloneSettings(DEFAULT_SETTINGS))
const cmsImportText = ref('')
const cmsTesting = ref(false)
const cmsHealthResults = ref<CmsHealthResult[]>([])

const enabledCount = computed(() => sources.value.filter((source) => source.enabled).length)
const sourceGroups = computed(() => {
  const order = ['公开页面源', '需配置源', 'PanSou 深度池', 'CMS 源池', '外部索引器']
  const groups = new Map<string, SearchSource[]>()
  for (const source of sources.value) {
    if (!groups.has(source.group)) {
      groups.set(source.group, [])
    }
    groups.get(source.group)?.push(source)
  }
  return Array.from(groups.entries())
    .sort((left, right) => {
      const leftIndex = order.indexOf(left[0])
      const rightIndex = order.indexOf(right[0])
      return (leftIndex === -1 ? 99 : leftIndex) - (rightIndex === -1 ? 99 : rightIndex)
    })
    .map(([name, groupSources]) => ({ name, sources: groupSources }))
})
const resultTitle = computed(() => lastQuery.value ? `“${lastQuery.value}” 的匹配资源` : '资源结果')
const resultHint = computed(() => {
  if (loading.value) return '正在并发检索已启用来源'
  if (!lastQuery.value) return '支持多来源聚合，单个来源失败不影响其他结果'
  return `共 ${items.value.length} 条结果，目标剧命中 ${targetResourceCount.value} 条，${states.value.filter((state) => state.status === 'failed').length} 个来源失败`
})

onMounted(async () => {
  try {
    await loadSettings()
    await refreshSources()
  } catch (error) {
    message.error(String(error))
  }
})

function toggleSource(sourceId: string) {
  if (selectedSourceIds.value.includes(sourceId)) {
    selectedSourceIds.value = selectedSourceIds.value.filter((id) => id !== sourceId)
    return
  }
  selectedSourceIds.value = [...selectedSourceIds.value, sourceId]
}

function resetFilters() {
  diskType.value = 'all'
  sortOrder.value = 'relevance'
  exactMatch.value = false
  selectedSourceIds.value = sources.value.filter((source) => source.enabled).map((source) => source.id)
}

async function handleSearch() {
  const text = query.value.trim()
  if (!text) {
    message.warning('请输入搜索关键词')
    return
  }
  if (!selectedSourceIds.value.length) {
    message.warning('请选择至少一个可用来源')
    return
  }

  loading.value = true
  lastQuery.value = text
  targetResourceCount.value = 0
  targetResourceMessage.value = ''
  states.value = selectedSourceIds.value.map((sourceId) => {
    const source = sources.value.find((item) => item.id === sourceId)
    return {
      sourceId,
      sourceName: source?.name || sourceId,
      group: source?.group || '未知来源',
      kind: source?.kind || 'unknown',
      status: 'empty',
      message: '等待返回',
      count: 0,
      elapsedMs: 0,
      healthScore: source?.healthScore || 0
    }
  })
  try {
    const response = await searchResources(text, 1, {
      sourceIds: selectedSourceIds.value,
      diskType: diskType.value,
      sortOrder: sortOrder.value,
      exactMatch: exactMatch.value,
      settings: serializeSettings(settingsForm.value)
    })
    items.value = response.items
    groups.value = response.groups
    states.value = response.states
    coverage.value = response.coverage
    targetResourceCount.value = response.targetResourceCount
    targetResourceMessage.value = response.targetResourceMessage
    searchPlan.value = response.searchPlan
  } catch (error) {
    message.error(String(error))
  } finally {
    loading.value = false
  }
}

function sourceLabel(source: SearchSource) {
  if (source.status === 'requiresConfig') return '需配置'
  if (!source.enabled) return '暂不可用'
  return source.group
}

async function openDetail(item: ResourceItem) {
  detailOpen.value = true
  detailLoading.value = true
  detail.value = undefined
  try {
    detail.value = await getResourceDetail(item)
  } catch (error) {
    message.error(String(error))
    detailOpen.value = false
  } finally {
    detailLoading.value = false
  }
}

function closeDetail() {
  detailOpen.value = false
  detail.value = undefined
}

async function copyUrl(url: string) {
  await navigator.clipboard.writeText(url)
  message.success('已复制跳转地址')
}

async function openUrl(url: string) {
  await openExternalUrl(url)
}

async function loadSettings() {
  try {
    settingsForm.value = toEditableSettings(await getSearchSettings())
  } catch {
    settingsForm.value = cloneSettings(DEFAULT_SETTINGS)
  }
}

async function refreshSources() {
  sources.value = await listSearchSources(settingsForm.value)
  const enabledIds = sources.value.filter((source) => source.enabled).map((source) => source.id)
  selectedSourceIds.value = selectedSourceIds.value.filter((id) => enabledIds.includes(id))
  if (!selectedSourceIds.value.length) {
    selectedSourceIds.value = enabledIds
  }
}

function openSettingsPanel() {
  cmsImportText.value = ''
  cmsHealthResults.value = []
  settingsOpen.value = true
}

async function saveSettingsPanel() {
  settingsSaving.value = true
  try {
    settingsForm.value = toEditableSettings(await saveSearchSettings(serializeSettings(settingsForm.value)))
    await refreshSources()
    settingsOpen.value = false
    message.success('搜索来源设置已保存')
  } catch (error) {
    message.error(String(error))
  } finally {
    settingsSaving.value = false
  }
}

function addPansouEndpoint() {
  settingsForm.value.pansouEndpoints.push({
    id: '',
    name: `PanSou ${settingsForm.value.pansouEndpoints.length + 1}`,
    endpoint: '',
    token: '',
    enabled: true,
    refresh: false,
    channels: [],
    plugins: [],
    src: 'all',
    cloudTypes: [],
    concurrency: 4,
    channelsText: '',
    pluginsText: '',
    cloudTypesText: ''
  })
}

function removePansouEndpoint(index: number) {
  settingsForm.value.pansouEndpoints.splice(index, 1)
}

function addCmsSource() {
  settingsForm.value.cmsSources.push({
    id: '',
    name: `CMS ${settingsForm.value.cmsSources.length + 1}`,
    endpoint: '',
    enabled: true,
    lastSuccessAt: '',
    failureCount: 0,
    averageCount: 0,
    healthScore: 60
  })
}

function removeCmsSource(index: number) {
  settingsForm.value.cmsSources.splice(index, 1)
}

async function handleImportCmsSources() {
  const text = cmsImportText.value.trim()
  if (!text) {
    message.warning('请输入要导入的 CMS 源')
    return
  }
  try {
    settingsForm.value = toEditableSettings(await importCmsSources(text, serializeSettings(settingsForm.value)))
    cmsImportText.value = ''
    message.success('CMS 源已导入')
  } catch (error) {
    message.error(String(error))
  }
}

async function handleTestCmsSources() {
  cmsTesting.value = true
  try {
    cmsHealthResults.value = await testCmsSources(serializeSettings(settingsForm.value))
    message.success('CMS 源健康检测完成')
  } catch (error) {
    message.error(String(error))
  } finally {
    cmsTesting.value = false
  }
}

function addIndexer() {
  settingsForm.value.indexers.push({
    id: '',
    name: `索引器 ${settingsForm.value.indexers.length + 1}`,
    baseUrl: '',
    apiKey: '',
    indexerType: 'torznab',
    categories: [],
    enabled: true,
    categoriesText: ''
  })
}

function removeIndexer(index: number) {
  settingsForm.value.indexers.splice(index, 1)
}

function coverageRate(item: SourceCoverage) {
  if (!item.total) return 0
  return Math.max(8, Math.round((item.success / item.total) * 100))
}

function toEditableSettings(settings: SearchSettings): EditableSearchSettings {
  return {
    ...settings,
    pansouEndpoints: (settings.pansouEndpoints || []).map((endpoint) => ({
      ...endpoint,
      src: endpoint.src || 'all',
      channelsText: (endpoint.channels || []).join(','),
      pluginsText: (endpoint.plugins || []).join(','),
      cloudTypesText: (endpoint.cloudTypes || []).join(',')
    })),
    cmsSources: settings.cmsSources || [],
    indexers: (settings.indexers || []).map((indexer) => ({
      ...indexer,
      indexerType: indexer.indexerType || 'torznab',
      categoriesText: (indexer.categories || []).join(',')
    })),
    pansouCache: settings.pansouCache !== false,
    pansouConcurrency: settings.pansouConcurrency || 4
  }
}

function serializeSettings(settings: EditableSearchSettings): SearchSettings {
  const pansouEndpoints: PansouEndpointConfig[] = settings.pansouEndpoints.map((endpoint) => ({
    id: endpoint.id,
    name: endpoint.name,
    endpoint: endpoint.endpoint,
    token: endpoint.token,
    enabled: endpoint.enabled,
    refresh: endpoint.refresh,
    channels: splitList(endpoint.channelsText),
    plugins: splitList(endpoint.pluginsText),
    src: endpoint.src || 'all',
    cloudTypes: splitList(endpoint.cloudTypesText),
    concurrency: endpoint.concurrency || 4
  }))
  const indexers: IndexerConfig[] = settings.indexers.map((indexer) => ({
    id: indexer.id,
    name: indexer.name,
    baseUrl: indexer.baseUrl,
    apiKey: indexer.apiKey,
    indexerType: indexer.indexerType || 'torznab',
    categories: splitList(indexer.categoriesText),
    enabled: indexer.enabled
  }))
  const cmsSources: CmsSourceConfig[] = settings.cmsSources.map((source) => ({ ...source }))
  return {
    ...settings,
    pansouEndpoint: pansouEndpoints[0]?.endpoint || '',
    pansouToken: pansouEndpoints[0]?.token || '',
    pansouRefresh: settings.pansouRefresh,
    pansouEndpoints,
    pansouChannels: settings.pansouChannels || [],
    pansouPlugins: settings.pansouPlugins || [],
    pansouSrc: settings.pansouSrc || 'all',
    pansouCloudTypes: settings.pansouCloudTypes || [],
    pansouCache: settings.pansouCache !== false,
    pansouConcurrency: settings.pansouConcurrency || 4,
    cmsEndpoints: cmsSources.map((source) => source.endpoint).filter(Boolean),
    cmsSources,
    indexers,
    tmdbApiKey: settings.tmdbApiKey
  }
}

function splitList(text: string) {
  return text
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean)
}

function cloneSettings(settings: EditableSearchSettings): EditableSearchSettings {
  return JSON.parse(JSON.stringify(settings)) as EditableSearchSettings
}

function highlightTitle(title: string) {
  const safeTitle = escapeHtml(title)
  const keyword = lastQuery.value.trim()
  if (!keyword) return safeTitle
  const safeKeyword = escapeHtml(keyword)
  return safeTitle.split(safeKeyword).join(`<span class="highlight">${safeKeyword}</span>`)
}

function escapeHtml(text: string) {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;')
}
</script>

<style scoped>
.workbench {
  display: grid;
  grid-template-columns: 76px minmax(0, 1fr);
  min-height: 100vh;
  background:
    radial-gradient(circle at 18% 12%, rgba(139, 205, 195, 0.18), transparent 30%),
    linear-gradient(135deg, #0b1220 0%, #101b2a 45%, #f4f7f6 45.2%, #f4f7f6 100%);
}

.rail {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 18px;
  padding: 22px 12px;
  background: #0d1624;
  border-right: 1px solid rgba(255, 255, 255, 0.08);
}

.brand-mark img {
  width: 44px;
  height: 44px;
  border-radius: 12px;
  display: block;
}

.rail-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 46px;
  height: 46px;
  color: #b5c6d8;
  background: transparent;
  border: 0;
  border-radius: 12px;
  cursor: pointer;
}

.rail-button.active,
.rail-button:hover {
  color: #10202b;
  background: #d8f4ef;
}

.rail-spacer {
  flex: 1;
}

.stage {
  min-width: 0;
  padding: 30px 34px 42px;
  overflow: auto;
}

.hero {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 28px;
  color: #f7fbff;
}

.hero-title {
  display: flex;
  align-items: center;
  gap: 18px;
}

.hero-title img {
  width: 62px;
  height: 62px;
  border-radius: 16px;
  box-shadow: 0 18px 40px rgba(0, 0, 0, 0.28);
}

.hero-title p {
  margin: 0 0 4px;
  color: #9bd9d0;
  font-size: 16px;
  font-weight: 700;
}

.hero-title h1 {
  margin: 0;
  font-size: 38px;
  line-height: 1.1;
  letter-spacing: 0;
}

.hero-meta {
  display: flex;
  gap: 10px;
}

.hero-meta span {
  padding: 8px 12px;
  color: #dce8f0;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 999px;
}

.search-panel,
.plan-panel,
.result-shell {
  background: rgba(255, 255, 255, 0.96);
  border: 1px solid #e2e9ee;
  border-radius: 8px;
  box-shadow: 0 24px 60px rgba(25, 39, 56, 0.12);
}

.search-panel {
  padding: 24px;
}

.search-line {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 118px;
  gap: 12px;
}

.search-line :deep(.ant-input-affix-wrapper),
.search-line :deep(.ant-input) {
  font-size: 18px;
}

.source-groups {
  display: grid;
  gap: 14px;
  padding: 20px 0 12px;
}

.source-group {
  display: grid;
  gap: 8px;
}

.source-group-title {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  color: #718196;
  font-size: 13px;
}

.source-group-title span {
  color: #334155;
  font-weight: 700;
}

.source-strip {
  display: flex;
  gap: 10px;
  overflow-x: auto;
}

.source-chip {
  display: inline-flex;
  flex: 0 0 auto;
  flex-direction: column;
  gap: 3px;
  min-width: 112px;
  padding: 10px 14px;
  text-align: left;
  color: #596878;
  background: #f0f4f6;
  border: 1px solid #e2e8ee;
  border-radius: 8px;
  cursor: pointer;
}

.source-chip span {
  color: #1f2933;
  font-weight: 700;
}

.source-chip small {
  font-size: 12px;
}

.source-chip.selected {
  background: #e6f8f5;
  border-color: #82d3c9;
}

.source-chip.disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.source-chip.configurable {
  background: #f7f2e6;
  border-color: #ead59b;
}

.filter-row {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  align-items: center;
  padding-top: 10px;
}

.filter-select {
  width: 150px;
}

.status-board {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 12px;
  margin: 16px 0;
}

.status-card {
  display: grid;
  grid-template-columns: 20px 1fr auto;
  gap: 8px;
  align-items: center;
  padding: 12px;
  background: rgba(255, 255, 255, 0.9);
  border: 1px solid #e2e9ee;
  border-radius: 8px;
}

.status-card small {
  grid-column: 2 / 4;
  color: #718196;
}

.status-card.success {
  color: #0a8f8a;
}

.status-card.failed {
  color: #d64848;
}

.coverage-panel {
  margin: 16px 0;
  padding: 18px;
  background: rgba(255, 255, 255, 0.94);
  border: 1px solid #e2e9ee;
  border-radius: 8px;
  box-shadow: 0 18px 46px rgba(25, 39, 56, 0.1);
}

.coverage-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 18px;
  margin-bottom: 14px;
}

.coverage-head h2 {
  margin: 0 0 5px;
  font-size: 20px;
}

.coverage-head p,
.coverage-card small {
  margin: 0;
  color: #718196;
}

.coverage-head > span {
  flex: 0 0 auto;
  padding: 7px 10px;
  color: #0a6f6b;
  background: #e6f8f5;
  border: 1px solid #bce7e1;
  border-radius: 8px;
}

.coverage-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(210px, 1fr));
  gap: 12px;
}

.coverage-card {
  display: grid;
  gap: 10px;
  padding: 12px;
  background: #f7fafb;
  border: 1px solid #e2e9ee;
  border-radius: 8px;
}

.coverage-card strong {
  display: block;
  margin-bottom: 4px;
}

.coverage-meter {
  height: 7px;
  overflow: hidden;
  background: #e5edf1;
  border-radius: 999px;
}

.coverage-meter span {
  display: block;
  height: 100%;
  background: linear-gradient(90deg, #2bb8a8, #e8c770);
}

.plan-panel {
  display: grid;
  grid-template-columns: minmax(0, 1.15fr) minmax(0, 0.85fr);
  gap: 18px;
  margin: 16px 0;
  padding: 18px;
}

.panel-title {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  align-items: center;
  margin-bottom: 12px;
}

.panel-title span {
  font-size: 16px;
  font-weight: 700;
}

.panel-title small {
  color: #718196;
}

.candidate-list {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: 10px;
}

.candidate-card {
  display: flex;
  flex-direction: column;
  gap: 5px;
  min-height: 92px;
  padding: 12px;
  text-align: left;
  color: #536274;
  background: #f5f8fa;
  border: 1px solid #e2e9ee;
  border-radius: 8px;
}

.candidate-card.active {
  background: #e6f8f5;
  border-color: #82d3c9;
}

.candidate-card strong {
  color: #1f2933;
  font-size: 17px;
}

.candidate-card small {
  color: #718196;
}

.term-list,
.reason-row {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.result-shell {
  margin-top: 16px;
  padding: 22px 24px;
}

.result-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 18px;
  margin-bottom: 18px;
}

.result-head h2 {
  margin: 0 0 6px;
  font-size: 22px;
}

.result-head p {
  margin: 0;
  color: #718196;
}

.target-summary {
  display: inline-flex;
  margin-top: 10px;
  padding: 7px 10px;
  color: #0a6f6b;
  background: #e6f8f5;
  border: 1px solid #bce7e1;
  border-radius: 8px;
  font-weight: 700;
}

.target-summary.missed {
  color: #9a5a13;
  background: #fff7e8;
  border-color: #f0d199;
}

.group-list,
.result-list {
  display: grid;
  gap: 12px;
}

.result-group {
  display: grid;
  gap: 12px;
}

.group-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 0;
  border-bottom: 1px solid #e8eef3;
}

.group-head h3 {
  margin: 0;
  font-size: 18px;
}

.group-head span {
  color: #718196;
}

.result-item {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 18px;
  padding: 18px;
  background: #ffffff;
  border: 1px solid #e5ebef;
  border-radius: 8px;
  cursor: pointer;
  transition: transform 0.18s ease, box-shadow 0.18s ease, border-color 0.18s ease;
}

.result-item:hover {
  border-color: #9fdad4;
  box-shadow: 0 16px 34px rgba(24, 61, 74, 0.1);
  transform: translateY(-1px);
}

.result-main h3 {
  margin: 0 0 10px;
  font-size: 21px;
  line-height: 1.35;
}

.result-main pre {
  display: -webkit-box;
  max-height: 112px;
  margin: 0 0 12px;
  overflow: hidden;
  color: #536274;
  font-family: inherit;
  line-height: 1.55;
  white-space: pre-wrap;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 5;
}

.tag-row,
.reason-row {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.reason-row {
  margin-bottom: 8px;
}

.settings-form {
  display: grid;
  gap: 18px;
  max-height: 68vh;
  overflow: auto;
  padding-right: 4px;
}

.settings-form label {
  display: grid;
  gap: 7px;
}

.settings-form label > span {
  color: #334155;
  font-weight: 700;
}

.settings-tip {
  margin: 0;
  color: #718196;
  line-height: 1.6;
}

.settings-section {
  display: grid;
  gap: 12px;
  padding: 16px;
  background: #f8fbfc;
  border: 1px solid #e2e9ee;
  border-radius: 8px;
}

.settings-section-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.settings-section-head h3 {
  margin: 0 0 5px;
  font-size: 18px;
}

.settings-section-head p {
  margin: 0;
  color: #718196;
  line-height: 1.5;
}

.settings-actions,
.settings-checks {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  align-items: center;
}

.pool-list {
  display: grid;
  gap: 10px;
}

.pool-row {
  display: grid;
  grid-template-columns: 28px minmax(100px, 0.72fr) minmax(180px, 1.25fr) minmax(130px, 0.8fr) 96px minmax(120px, 0.8fr) minmax(120px, 0.8fr) minmax(120px, 0.8fr) 58px 76px 34px;
  gap: 8px;
  align-items: center;
  padding: 10px;
  background: #ffffff;
  border: 1px solid #e4eaef;
  border-radius: 8px;
}

.cms-row {
  grid-template-columns: 28px minmax(120px, 0.8fr) minmax(260px, 1.6fr) 92px 34px;
}

.indexer-row {
  grid-template-columns: 28px minmax(110px, 0.7fr) minmax(240px, 1.5fr) minmax(130px, 0.75fr) 104px minmax(130px, 0.8fr) 34px;
}

.compact-select {
  width: 96px;
}

.number-input {
  width: 76px;
}

.icon-action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  color: #64748b;
  background: #f3f6f8;
  border: 1px solid #dfe7ec;
  border-radius: 8px;
  cursor: pointer;
}

.icon-action.danger:hover {
  color: #c93636;
  background: #fff1f1;
  border-color: #ffc6c6;
}

.health-pill {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-height: 32px;
  color: #0a6f6b;
  background: #e6f8f5;
  border: 1px solid #bce7e1;
  border-radius: 8px;
}

.health-list {
  display: grid;
  gap: 8px;
}

.health-list > div {
  display: grid;
  grid-template-columns: minmax(120px, 0.8fr) 110px minmax(0, 1fr);
  gap: 10px;
  padding: 9px 10px;
  color: #b95050;
  background: #fff8f8;
  border: 1px solid #ffd7d7;
  border-radius: 8px;
}

.health-list > div.ok {
  color: #0a6f6b;
  background: #f2fbf9;
  border-color: #c9ede8;
}

.detail-button {
  align-self: center;
  display: inline-flex;
  gap: 8px;
  align-items: center;
  height: 38px;
  padding: 0 14px;
  color: #0a6f6b;
  background: #e6f8f5;
  border: 1px solid #bce7e1;
  border-radius: 8px;
  cursor: pointer;
}

.detail-loading {
  display: flex;
  gap: 12px;
  align-items: center;
  justify-content: center;
  min-height: 180px;
  color: #607086;
}

.detail-box {
  position: relative;
  padding-top: 8px;
}

.modal-close {
  position: absolute;
  top: -54px;
  right: -4px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  background: transparent;
  border: 0;
  cursor: pointer;
}

.detail-box h3 {
  margin: 0 0 12px;
  font-size: 20px;
}

.detail-box a {
  display: block;
  overflow-wrap: anywhere;
  color: #0a74d9;
  font-size: 18px;
  font-weight: 700;
}

.detail-box p {
  margin: 14px 0 0;
  color: #718196;
}

.detail-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 28px;
}

.spin {
  animation: spin 0.9s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

@media (max-width: 980px) {
  .workbench {
    grid-template-columns: 64px minmax(0, 1fr);
  }

  .stage {
    padding: 22px 18px 34px;
  }

  .hero {
    align-items: flex-start;
    flex-direction: column;
    gap: 14px;
  }

  .hero-title h1 {
    font-size: 30px;
  }

  .search-line {
    grid-template-columns: 1fr;
  }

  .plan-panel {
    grid-template-columns: 1fr;
  }

  .result-item {
    grid-template-columns: 1fr;
  }

  .coverage-head,
  .settings-section-head {
    flex-direction: column;
  }

  .pool-row,
  .cms-row,
  .indexer-row,
  .health-list > div {
    grid-template-columns: 1fr;
  }

  .pool-row > :deep(.ant-checkbox-wrapper),
  .pool-row > :deep(.ant-checkbox) {
    justify-self: start;
  }

  .compact-select,
  .number-input {
    width: 100%;
  }

  .detail-button {
    justify-content: center;
    width: 100%;
  }
}
</style>
