<template>
  <main v-if="!currentUser" class="login-shell">
    <section class="login-panel">
      <div class="login-brand">
        <img src="../assets/brand/sui-frame-icon.svg" alt="影岁" />
        <div>
          <p>影岁</p>
          <h1>登录资源工作台</h1>
        </div>
      </div>
      <div class="login-form">
        <label>
          <span>用户名</span>
          <a-input v-model:value="loginForm.username" size="large" autocomplete="username" @press-enter="handleLogin" />
        </label>
        <label>
          <span>密码</span>
          <a-input-password
            v-model:value="loginForm.password"
            size="large"
            autocomplete="current-password"
            @press-enter="handleLogin"
          />
        </label>
        <a-button type="primary" size="large" :loading="loginLoading" @click="handleLogin">
          登录
        </a-button>
      </div>
      <div class="login-meta">
        <span>默认账号 admin / 123456</span>
        <span v-if="appVersion">v{{ appVersion }}</span>
      </div>
    </section>
  </main>

  <main v-else class="workbench">
    <aside class="rail">
      <div class="brand-mark">
        <img src="../assets/brand/sui-frame-icon.svg" alt="影岁" />
      </div>
      <a-tooltip title="资源搜索">
        <button class="rail-button" :class="{ active: activeView === 'search' }" type="button" @click="activeView = 'search'">
          <Search :size="22" />
        </button>
      </a-tooltip>
      <a-tooltip title="我的收藏">
        <button class="rail-button" :class="{ active: activeView === 'favorites' }" type="button" @click="openFavoritesView">
          <Bookmark :size="22" />
        </button>
      </a-tooltip>
      <a-tooltip title="数据源配置">
        <button class="rail-button" :class="{ active: activeView === 'sources' }" type="button" @click="activeView = 'sources'">
          <Database :size="22" />
        </button>
      </a-tooltip>
      <div class="rail-spacer"></div>
      <AppNotificationCenter />
      <a-tooltip title="设置">
        <button class="rail-button" type="button" @click="openSettingsPanel">
          <Settings :size="22" />
        </button>
      </a-tooltip>
      <a-tooltip :title="`${currentUser?.displayName || '用户'}，退出登录`">
        <button class="rail-button" type="button" @click="handleLogout">
          <LogOut :size="22" />
        </button>
      </a-tooltip>
    </aside>

    <section class="stage">
      <header class="hero">
        <div class="hero-title">
          <img src="../assets/brand/sui-frame-icon.svg" alt="影岁" />
          <div>
            <p>影岁</p>
            <h1>{{ activeViewTitle }}</h1>
          </div>
        </div>
        <div class="user-badge">
          <UserRound :size="18" />
          <span>{{ currentUser?.displayName || '用户' }}</span>
        </div>
      </header>

      <section v-if="activeView === 'search'" class="search-view">
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
        </section>

        <section class="source-summary-panel" :class="{ collapsed: !sourceSummaryOpen }">
          <button class="panel-title summary-panel-toggle" type="button" @click="sourceSummaryOpen = !sourceSummaryOpen">
            <span>来源执行摘要</span>
            <span class="panel-title-meta">
              <small>{{ sourceExecutionSummary }}</small>
              <ChevronDown class="summary-chevron" :class="{ open: sourceSummaryOpen }" :size="18" />
            </span>
          </button>
          <div v-if="sourceSummaryOpen" class="source-summary-grid">
            <article
              v-for="group in sourceGroups"
              :key="group.name"
              class="source-summary-card"
              :class="{ open: expandedSourceGroup === group.name }"
            >
              <button class="source-summary-trigger" type="button" @click="toggleSourceSummary(group.name)">
                <span class="source-summary-copy">
                  <strong>{{ group.name }}</strong>
                  <span>{{ groupRuntimeSummary(group.name) }}</span>
                  <small>{{ groupSelectionSummary(group) }}</small>
                </span>
                <ChevronDown class="summary-chevron" :class="{ open: expandedSourceGroup === group.name }" :size="18" />
              </button>
              <div v-if="expandedSourceGroup === group.name" class="source-summary-detail">
                <div v-if="successfulGroupStates(group.name).length" class="source-summary-detail-list">
                  <div v-for="state in successfulGroupStates(group.name)" :key="state.sourceId">
                    <span>{{ state.sourceName }}</span>
                    <strong>{{ state.count }} 条</strong>
                  </div>
                </div>
                <p v-else>暂无成功返回来源</p>
              </div>
            </article>
          </div>
        </section>

        <div class="output-scroll">
          <section class="result-shell">
            <div class="result-head">
              <div>
                <h2>{{ resultTitle }}</h2>
                <p>{{ resultHint }}</p>
                <div v-if="!loading && targetResourceMessage" class="target-summary" :class="{ missed: targetResourceCount === 0 }">
                  {{ targetResourceMessage }}
                </div>
              </div>
            </div>

            <section v-if="loading" class="search-loading-panel">
              <div class="loading-orbit">
                <LoaderCircle class="spin" :size="30" />
                <span></span>
              </div>
              <div class="loading-copy">
                <strong>正在聚合资源</strong>
                <p>关键词 “{{ lastQuery }}”，并发检索 {{ selectedEnabledCount }} 个来源</p>
              </div>
              <div class="loading-meter">
                <i></i>
              </div>
              <div class="loading-stats">
                <span>已返回 {{ totalReturnedCount }} 条</span>
                <span>成功 {{ successSourceCount }} 个</span>
                <span v-if="failedSourceCount">失败 {{ failedSourceCount }} 个</span>
              </div>
            </section>

            <a-empty v-else-if="!items.length" :description="lastQuery ? '暂未检索到匹配资源' : '输入关键词后开始检索'" />

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
                        <a-tag v-if="isMagnetResource(item)" color="orange">磁力链接</a-tag>
                        <a-tag v-else-if="isCloudResource(item)" color="blue">云盘链接</a-tag>
                        <a-tag v-if="item.diskType && !isMagnetResource(item)">{{ formatDiskType(item.diskType) }}</a-tag>
                        <a-tag v-if="item.shareUser">{{ item.shareUser }}</a-tag>
                        <a-tag v-for="tag in item.tags.slice(0, 3)" :key="`${item.id}-${tag}`">{{ tag }}</a-tag>
                      </div>
                    </div>
                    <div class="result-actions">
                      <button class="detail-button" type="button" @click.stop="openDetail(item)">
                        <Film :size="18" />
                        详情
                      </button>
                      <button
                        class="favorite-button"
                        type="button"
                        :disabled="favoriteLoadingIds.includes(item.id)"
                        @click.stop="handleFavorite(item)"
                      >
                        <LoaderCircle v-if="favoriteLoadingIds.includes(item.id)" class="spin" :size="18" />
                        <Bookmark v-else :size="18" />
                        收藏
                      </button>
                    </div>
                  </article>
                </div>
              </section>
            </div>
          </section>
        </div>
      </section>

      <section v-else-if="activeView === 'favorites'" class="favorites-view">
        <div class="favorites-head">
          <div>
            <h2>我的收藏</h2>
            <p>当前用户 {{ currentUser?.displayName || '用户' }} 已收藏 {{ favorites.length }} 条资源。</p>
          </div>
          <a-button :loading="favoritesLoading" @click="loadFavorites">
            <template #icon><RefreshCw :size="16" /></template>
            刷新
          </a-button>
        </div>

        <div class="favorites-scroll">
          <section v-if="favoritesLoading" class="favorites-loading">
            <LoaderCircle class="spin" :size="26" />
            <span>正在读取收藏资源</span>
          </section>
          <section v-else-if="!favorites.length" class="favorites-empty">
            <Bookmark :size="34" />
            <h3>还没有收藏资源</h3>
            <p>在资源搜索结果中点击“收藏”，解析后的可打开链接会保存在这里。</p>
          </section>
          <section v-else class="favorites-list">
            <article v-for="favorite in favorites" :key="favorite.id" class="favorite-card">
              <div class="favorite-main">
                <h3>{{ favorite.title }}</h3>
                <p>{{ favorite.info || '暂无文件摘要' }}</p>
                <a :href="favorite.url" target="_blank" rel="noreferrer">{{ favorite.url }}</a>
                <div class="tag-row">
                  <a-tag color="cyan">{{ favorite.sourceName || '未知来源' }}</a-tag>
                  <a-tag v-if="isMagnetFavorite(favorite)" color="orange">磁力链接</a-tag>
                  <a-tag v-else-if="isCloudDiskType(favorite.diskType)" color="blue">云盘链接</a-tag>
                  <a-tag v-if="favorite.diskType && !isMagnetFavorite(favorite)">{{ formatDiskType(favorite.diskType) }}</a-tag>
                  <a-tag v-if="favorite.shareUser">{{ favorite.shareUser }}</a-tag>
                  <a-tag>{{ formatFavoriteTime(favorite.createdAt) }}</a-tag>
                </div>
              </div>
              <div class="favorite-actions">
                <a-button @click="openUrl(favorite.url)">
                  <template #icon><ExternalLink :size="16" /></template>
                  打开
                </a-button>
                <a-button @click="copyUrl(favorite.url)">
                  <template #icon><Clipboard :size="16" /></template>
                  复制
                </a-button>
                <a-button danger :loading="removingFavoriteId === favorite.id" @click="handleRemoveFavorite(favorite.id)">
                  <template #icon><Trash2 :size="16" /></template>
                  删除
                </a-button>
              </div>
            </article>
          </section>
        </div>
      </section>

      <section v-else class="source-config-view">
        <div class="source-config-head">
          <div>
            <h2>数据源配置</h2>
            <p>默认启用所有可用来源，搜索页会直接使用这里的来源集合。</p>
          </div>
          <div class="source-config-actions">
            <span>{{ selectedEnabledCount }} / {{ enabledCount }} 已选</span>
            <a-button :disabled="loading" @click="selectAllSources">全选可用</a-button>
            <a-button :disabled="loading" @click="clearSelectedSources">清空</a-button>
          </div>
        </div>
        <div class="source-config-groups">
          <section v-for="group in sourceGroups" :key="group.name" class="source-config-group">
            <div class="source-config-group-head">
              <div>
                <h3>{{ group.name }}</h3>
                <p>{{ groupSelectionSummary(group) }}</p>
              </div>
              <a-button
                v-if="group.name === '内置聚合源'"
                :type="embeddedPansouStatus?.running ? 'default' : 'primary'"
                :danger="embeddedPansouStatus?.running"
                :loading="embeddedPansouRestarting"
                :disabled="embeddedPansouServiceDisabled"
                @click="handleToggleEmbeddedPansou"
              >
                <template #icon><Power :size="16" /></template>
                {{ embeddedPansouServiceActionText }}
              </a-button>
            </div>
            <div class="source-config-grid">
              <button
                v-for="source in group.sources"
                :key="source.id"
                class="source-config-card"
                :class="{ selected: selectedSourceIds.includes(source.id), disabled: !source.enabled, configurable: source.status === 'requiresConfig' }"
                type="button"
                :disabled="!source.enabled || loading"
                @click="toggleSource(source.id)"
              >
                <span class="source-check" :class="{ selected: selectedSourceIds.includes(source.id), disabled: !source.enabled }">
                  <CheckCircle2 v-if="selectedSourceIds.includes(source.id)" :size="16" />
                  <CircleAlert v-else-if="!source.enabled" :size="16" />
                </span>
                <div>
                  <strong>{{ source.name }}</strong>
                  <small>{{ sourceLabel(source) }}</small>
                  <p>{{ source.description }}</p>
                  <div class="source-card-meta">
                    <span>健康 {{ source.healthScore || 0 }}</span>
                    <span>{{ source.kind }}</span>
                  </div>
                </div>
              </button>
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
        <section class="settings-section update-section">
          <div class="settings-section-head">
            <div>
              <h3>软件更新</h3>
              <p>{{ updateStatus }}</p>
            </div>
            <a-button type="primary" :loading="checking || installing" :disabled="installing" @click="checkUpdate">
              <template #icon><RefreshCw :size="16" /></template>
              检查更新
            </a-button>
          </div>
          <div class="update-grid">
            <div>
              <span>当前版本</span>
              <strong>{{ appVersion || '读取中' }}</strong>
            </div>
            <div>
              <span>更新源</span>
              <strong>OSS 稳定通道</strong>
            </div>
          </div>
          <a-progress
            v-if="installing"
            class="update-progress"
            :percent="updateProgress"
            :show-info="false"
            size="small"
          />
        </section>

        <section class="settings-section">
          <div class="settings-section-head">
            <div>
              <h3>内置 PanSou</h3>
              <p>{{ embeddedPansouStatus?.message || '随影岁启动的本地聚合搜索服务。' }}</p>
            </div>
          </div>
          <div class="embedded-pansou-panel">
            <div class="embedded-status" :class="{ running: embeddedPansouStatus?.running }">
              <Activity :size="18" />
              <div>
                <strong>{{ embeddedPansouStatus?.running ? '运行中' : '未运行' }}</strong>
                <small>{{ settingsForm.embeddedPansou.enabled ? embeddedPansouEndpoint : '已关闭' }}</small>
              </div>
              <span>{{ embeddedPansouStatus?.reused ? '复用本机服务' : '影岁托管' }}</span>
            </div>
            <div class="embedded-controls">
              <a-checkbox v-model:checked="settingsForm.embeddedPansou.enabled">启用</a-checkbox>
              <a-checkbox v-model:checked="settingsForm.embeddedPansou.autoStart">启动时自动开启</a-checkbox>
              <a-checkbox v-model:checked="settingsForm.embeddedPansou.cache">使用缓存</a-checkbox>
              <a-checkbox v-model:checked="settingsForm.embeddedPansou.refresh">强制刷新</a-checkbox>
            </div>
            <div class="embedded-grid">
              <label>
                <span>端口</span>
                <a-input-number v-model:value="settingsForm.embeddedPansou.port" :min="1024" :max="65535" class="number-input" />
              </label>
              <label>
                <span>来源范围</span>
                <a-select v-model:value="settingsForm.embeddedPansou.src">
                  <a-select-option value="all">全部</a-select-option>
                  <a-select-option value="tg">TG</a-select-option>
                  <a-select-option value="plugin">插件</a-select-option>
                </a-select>
              </label>
              <label>
                <span>并发</span>
                <a-input-number v-model:value="settingsForm.embeddedPansou.concurrency" :min="1" :max="8" class="number-input" />
              </label>
            </div>
            <a-input v-model:value="settingsForm.embeddedPansou.channelsText" placeholder="频道，逗号分隔；留空使用 PanSou 默认频道" />
            <a-input v-model:value="settingsForm.embeddedPansou.pluginsText" placeholder="插件，逗号分隔；默认已内置常用影视/网盘插件" />
            <a-input v-model:value="settingsForm.embeddedPansou.cloudTypesText" placeholder="网盘类型，逗号分隔，例如 quark,aliyun,baidu；留空跟随搜索筛选" />
          </div>
        </section>

        <section class="settings-section">
          <div class="settings-section-head">
            <div>
              <h3>自定义 PanSou</h3>
              <p>连接你自己部署的 PanSou endpoint，可与内置 PanSou 同时搜索。</p>
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
        <h3>{{ detail.title }}</h3>
        <div class="detail-meta">
          <span>链接类型：{{ detail.linkTypeLabel || '网页链接' }}</span>
          <span v-if="detail.diskType">资源类型：{{ formatDiskType(detail.diskType) }}</span>
        </div>
        <a class="detail-url" :href="detail.url" target="_blank" rel="noreferrer">{{ detail.url }}</a>
        <p v-if="detail.message">{{ detail.message }}</p>
        <div class="detail-validation" :class="detail.validationStatus">
          {{ detail.validationMessage }}
        </div>
        <div class="detail-actions">
          <a-button @click="copyUrl(detail.url)">
            <template #icon><Clipboard :size="16" /></template>
            复制
          </a-button>
          <a-button type="primary" :disabled="!detail.canOpen" @click="openUrl(detail.url)">
            <template #icon><ExternalLink :size="16" /></template>
            打开
          </a-button>
        </div>
      </div>
    </a-modal>
  </main>
</template>

<script setup lang="ts">
import { computed, h, onMounted, ref } from 'vue'
import { message, Modal } from 'ant-design-vue'
import {
  Activity,
  Bookmark,
  CheckCircle2,
  ChevronDown,
  CircleAlert,
  Clipboard,
  Database,
  ExternalLink,
  Film,
  LoaderCircle,
  LogOut,
  Plus,
  Power,
  RefreshCw,
  Search,
  Settings,
  Trash2,
  UserRound
} from 'lucide-vue-next'
import AppNotificationCenter from '../components/AppNotificationCenter.vue'
import {
  addFavorite,
  checkAppUpdate,
  formatUpdateError,
  getEmbeddedPansouStatus,
  getCurrentVersion,
  getSearchSettings,
  getResourceDetail,
  importCmsSources,
  installAppUpdate,
  listFavorites,
  listSearchSources,
  loginUser,
  openExternalUrl,
  removeFavorite,
  saveSearchSettings,
  searchResources,
  startEmbeddedPansou,
  stopEmbeddedPansou,
  testCmsSources,
  type CmsHealthResult,
  type CmsSourceConfig,
  type EmbeddedPansouConfig,
  type EmbeddedPansouStatus,
  type FavoriteResource,
  type IndexerConfig,
  type PansouEndpointConfig,
  type ResourceDetail,
  type ResourceItem,
  type ResultGroup,
  type SearchSettings,
  type SearchSource,
  type SourceSearchState,
  type UpdateCheckResult,
  type UserSession
} from '../api/native'

type EditablePansouEndpoint = PansouEndpointConfig & {
  channelsText: string
  pluginsText: string
  cloudTypesText: string
}

type EditableEmbeddedPansou = EmbeddedPansouConfig & {
  channelsText: string
  pluginsText: string
  cloudTypesText: string
}

type EditableIndexer = IndexerConfig & {
  categoriesText: string
}

type EditableSearchSettings = Omit<SearchSettings, 'embeddedPansou' | 'pansouEndpoints' | 'indexers'> & {
  embeddedPansou: EditableEmbeddedPansou
  pansouEndpoints: EditablePansouEndpoint[]
  indexers: EditableIndexer[]
}

const DEFAULT_EMBEDDED_PANSOU_PLUGINS = [
  'labi',
  'zhizhen',
  'shandian',
  'duoduo',
  'muou',
  'wanou',
  'hunhepan',
  'jikepan',
  'panwiki',
  'pansearch',
  'qupansou',
  'hdr4k',
  'pan666',
  'susu',
  'fox4k',
  'pianku',
  'clmao',
  'hdmoli',
  'yuhuage',
  'xinjuc',
  'aikanzy'
]

const DEFAULT_SETTINGS: EditableSearchSettings = {
  embeddedPansou: {
    enabled: true,
    autoStart: false,
    port: 10323,
    src: 'all',
    channels: [],
    plugins: DEFAULT_EMBEDDED_PANSOU_PLUGINS,
    cloudTypes: [],
    refresh: false,
    cache: true,
    concurrency: 4,
    channelsText: '',
    pluginsText: DEFAULT_EMBEDDED_PANSOU_PLUGINS.join(','),
    cloudTypesText: ''
  },
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

const EMBEDDED_PANSOU_SOURCE_ID = 'embedded-pansou'

const query = ref('')
const lastQuery = ref('')
const loading = ref(false)
const detailLoading = ref(false)
const loginLoading = ref(false)
const favoritesLoading = ref(false)
const removingFavoriteId = ref('')
const detailOpen = ref(false)
const settingsOpen = ref(false)
const settingsSaving = ref(false)
const activeView = ref<'search' | 'favorites' | 'sources'>('search')
const sourceSummaryOpen = ref(false)
const expandedSourceGroup = ref('')
const sources = ref<SearchSource[]>([])
const selectedSourceIds = ref<string[]>([])
const items = ref<ResourceItem[]>([])
const groups = ref<ResultGroup[]>([])
const states = ref<SourceSearchState[]>([])
const targetResourceCount = ref(0)
const targetResourceMessage = ref('')
const searchElapsedMs = ref(0)
const detail = ref<ResourceDetail>()
const currentUser = ref<UserSession>()
const loginForm = ref({ username: 'admin', password: '123456' })
const favorites = ref<FavoriteResource[]>([])
const favoriteLoadingIds = ref<string[]>([])
const settingsForm = ref<EditableSearchSettings>(cloneSettings(DEFAULT_SETTINGS))
const cmsImportText = ref('')
const cmsTesting = ref(false)
const cmsHealthResults = ref<CmsHealthResult[]>([])
const embeddedPansouStatus = ref<EmbeddedPansouStatus>()
const embeddedPansouRestarting = ref(false)
const appVersion = ref('')
const checking = ref(false)
const installing = ref(false)
const updateProgress = ref(0)
const updateStatus = ref('可手动检查 OSS 稳定通道中的新版本')

const enabledCount = computed(() => sources.value.filter((source) => source.enabled).length)
const selectedEnabledCount = computed(() => {
  const selected = new Set(selectedSourceIds.value)
  return sources.value.filter((source) => source.enabled && selected.has(source.id)).length
})
const totalReturnedCount = computed(() => states.value.reduce((sum, state) => sum + state.count, 0))
const successSourceCount = computed(() => states.value.filter((state) => state.status === 'success').length)
const failedSourceCount = computed(() => states.value.filter((state) => state.status === 'failed').length)
const sourceExecutionSummary = computed(() => {
  if (loading.value) return '正在并发检索已选来源'
  if (!lastQuery.value) return '搜索后会在这里汇总各来源返回数量'
  const baseSummary = `成功 ${successSourceCount.value} 个，失败 ${failedSourceCount.value} 个，返回 ${totalReturnedCount.value} 条`
  const elapsedText = formatElapsedSeconds(searchElapsedMs.value)
  return elapsedText ? `${baseSummary}，耗时 ${elapsedText}` : baseSummary
})
const embeddedPansouEndpoint = computed(() => `http://127.0.0.1:${settingsForm.value.embeddedPansou.port || 10323}`)
const embeddedPansouServiceActionText = computed(() => embeddedPansouStatus.value?.running ? '关闭服务' : '开启服务')
const embeddedPansouServiceDisabled = computed(() => {
  return loading.value || (!embeddedPansouStatus.value?.running && !settingsForm.value.embeddedPansou.enabled)
})
const sourceGroups = computed(() => {
  const order = ['内置聚合源', '公开页面源', '需配置源', 'PanSou 深度池', 'CMS 源池', '外部索引器']
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
const activeViewTitle = computed(() => {
  if (activeView.value === 'search') return '影视资源检索台'
  if (activeView.value === 'favorites') return '我的收藏'
  return '数据源配置'
})
const resultHint = computed(() => {
  if (loading.value) return '正在并发检索已启用来源'
  if (!lastQuery.value) return '输入关键词后按默认策略聚合可用来源'
  return `共 ${items.value.length} 条结果，${sourceExecutionSummary.value}`
})

onMounted(async () => {
  try {
    await loadCurrentVersion()
  } catch (error) {
    message.error(String(error))
  }
})

async function loadCurrentVersion() {
  try {
    appVersion.value = await getCurrentVersion()
  } catch {
    appVersion.value = ''
  }
}

async function handleLogin() {
  if (loginLoading.value) return
  const username = loginForm.value.username.trim()
  const password = loginForm.value.password.trim()
  if (!username || !password) {
    message.warning('请输入用户名和密码')
    return
  }
  loginLoading.value = true
  try {
    currentUser.value = await loginUser(username, password)
    await initializeWorkbench()
    message.success(`欢迎回来，${currentUser.value.displayName}`)
  } catch (error) {
    currentUser.value = undefined
    message.error(String(error))
  } finally {
    loginLoading.value = false
  }
}

async function initializeWorkbench() {
  await loadSettings()
  await refreshSources()
  await loadFavorites()
  activeView.value = 'search'
}

function handleLogout() {
  currentUser.value = undefined
  favorites.value = []
  items.value = []
  groups.value = []
  states.value = []
  lastQuery.value = ''
  detail.value = undefined
  detailOpen.value = false
  activeView.value = 'search'
}

async function loadFavorites() {
  if (!currentUser.value) return
  favoritesLoading.value = true
  try {
    favorites.value = await listFavorites(currentUser.value.username)
  } catch (error) {
    message.error(String(error))
  } finally {
    favoritesLoading.value = false
  }
}

async function openFavoritesView() {
  activeView.value = 'favorites'
  await loadFavorites()
}

async function checkUpdate() {
  if (checking.value || installing.value) return
  checking.value = true
  updateStatus.value = '正在检查新版本'
  try {
    const result = await checkAppUpdate()
    appVersion.value = result.currentVersion
    if (!result.update) {
      updateStatus.value = '已是最新版本'
      message.success('已是最新版本')
      return
    }
    updateStatus.value = `发现新版本 ${result.update.version}`
    Modal.confirm({
      title: '发现新版本',
      content: h('div', { class: 'update-confirm' }, [
        h('p', `当前版本：${result.currentVersion}`),
        h('p', `最新版本：${result.update.version}`),
        result.update.body
          ? h('div', [
              h('p', '更新内容：'),
              h('pre', { class: 'update-body' }, result.update.body)
            ])
          : null
      ]),
      okText: '立即更新',
      cancelText: '暂不更新',
      onOk: () => installUpdate(result.update!)
    })
  } catch (error) {
    updateStatus.value = '检查更新失败'
    message.error(formatUpdateError(error))
  } finally {
    checking.value = false
  }
}

async function installUpdate(update: NonNullable<UpdateCheckResult['update']>) {
  installing.value = true
  updateProgress.value = 0
  updateStatus.value = '正在下载更新'
  let downloadedBytes = 0
  let totalBytes = 0
  try {
    await installAppUpdate(update, event => {
      if (event.event === 'Started') {
        downloadedBytes = 0
        totalBytes = event.data.contentLength || 0
        updateProgress.value = 0
      }
      if (event.event === 'Progress') {
        downloadedBytes += event.data.chunkLength
        updateProgress.value = totalBytes
          ? Math.min(Math.round((downloadedBytes / totalBytes) * 100), 99)
          : Math.max(updateProgress.value, 1)
      }
      if (event.event === 'Finished') {
        updateProgress.value = 100
        updateStatus.value = '更新安装完成，正在重启'
      }
    })
  } catch (error) {
    installing.value = false
    updateStatus.value = downloadedBytes > 0 ? '更新安装失败' : '更新下载失败'
    message.error(formatUpdateError(error))
  }
}

function toggleSource(sourceId: string) {
  if (selectedSourceIds.value.includes(sourceId)) {
    selectedSourceIds.value = selectedSourceIds.value.filter((id) => id !== sourceId)
    return
  }
  selectedSourceIds.value = [...selectedSourceIds.value, sourceId]
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

  const startedAt = performance.now()
  loading.value = true
  lastQuery.value = text
  targetResourceCount.value = 0
  targetResourceMessage.value = ''
  searchElapsedMs.value = 0
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
      diskType: 'all',
      sortOrder: 'relevance',
      exactMatch: false,
      settings: serializeSettings(settingsForm.value)
    })
    items.value = response.items
    groups.value = response.groups
    states.value = response.states
    targetResourceCount.value = response.targetResourceCount
    targetResourceMessage.value = response.targetResourceMessage
    searchElapsedMs.value = performance.now() - startedAt
  } catch (error) {
    message.error(String(error))
  } finally {
    loading.value = false
  }
}

function sourceLabel(source: SearchSource) {
  if (source.status === 'requiresConfig') return '需配置'
  if (!source.enabled) return '暂不可用'
  return selectedSourceIds.value.includes(source.id) ? '已启用' : '未启用'
}

function isMagnetResource(item: ResourceItem) {
  return item.diskType?.toLowerCase() === 'magnet' || item.url?.trim().toLowerCase().startsWith('magnet:')
}

function isMagnetFavorite(item: FavoriteResource) {
  return item.diskType?.toLowerCase() === 'magnet' || item.url?.trim().toLowerCase().startsWith('magnet:')
}

function isCloudResource(item: ResourceItem) {
  return isCloudDiskType(item.diskType)
}

function isCloudDiskType(diskType: string) {
  const value = diskType.trim().toLowerCase()
  return Boolean(value && !['cms', 'torznab', 'newznab', 'magnet', 'ed2k', 'download', 'web'].includes(value))
}

function formatDiskType(diskType: string) {
  return diskType.trim()
}

function selectAllSources() {
  selectedSourceIds.value = sources.value.filter((source) => source.enabled).map((source) => source.id)
}

function clearSelectedSources() {
  selectedSourceIds.value = []
}

function groupSelectionSummary(group: { name: string; sources: SearchSource[] }) {
  const selected = new Set(selectedSourceIds.value)
  const enabledSources = group.sources.filter((source) => source.enabled)
  const selectedCount = enabledSources.filter((source) => selected.has(source.id)).length
  return `已选 ${selectedCount} / 可用 ${enabledSources.length}`
}

function toggleSourceSummary(groupName: string) {
  expandedSourceGroup.value = expandedSourceGroup.value === groupName ? '' : groupName
}

function successfulGroupStates(groupName: string) {
  return states.value.filter((state) => state.group === groupName && state.status === 'success' && state.count > 0)
}

function groupRuntimeSummary(groupName: string) {
  const groupStates = states.value.filter((state) => state.group === groupName)
  if (!lastQuery.value || !groupStates.length) return '等待搜索'
  const count = groupStates.reduce((sum, state) => sum + state.count, 0)
  const success = groupStates.filter((state) => state.status === 'success').length
  const failed = groupStates.filter((state) => state.status === 'failed').length
  if (loading.value) return `已返回 ${count} 条`
  return failed ? `成功 ${success}，返回 ${count} 条，失败 ${failed}` : `成功 ${success}，返回 ${count} 条`
}

function formatElapsedSeconds(ms: number) {
  if (!Number.isFinite(ms) || ms <= 0) return ''
  const seconds = Math.max(ms / 1000, 0.01)
  return `${seconds.toFixed(2).replace(/\.?0+$/, '')} 秒`
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

async function handleFavorite(item: ResourceItem) {
  if (!currentUser.value || favoriteLoadingIds.value.includes(item.id)) return
  favoriteLoadingIds.value = [...favoriteLoadingIds.value, item.id]
  try {
    const resourceDetail = await getResourceDetail(item)
    const url = resourceDetail.url.trim()
    if (!url) {
      message.warning('未解析到可收藏的跳转地址')
      return
    }
    if (resourceDetail.validationStatus === 'invalid' || !resourceDetail.canOpen) {
      message.warning(resourceDetail.validationMessage || '该资源链接不可用，未加入收藏')
      return
    }
    const existing = favorites.value.find((favorite) => favorite.url === url)
    if (existing) {
      message.info('已收藏过')
      return
    }
    const favorite = await addFavorite(currentUser.value.username, item, resourceDetail)
    favorites.value = mergeFavorite(favorites.value, favorite)
    message.success('已加入我的收藏')
  } catch (error) {
    message.error(String(error))
  } finally {
    favoriteLoadingIds.value = favoriteLoadingIds.value.filter((id) => id !== item.id)
  }
}

async function handleRemoveFavorite(favoriteId: string) {
  if (!currentUser.value || removingFavoriteId.value) return
  removingFavoriteId.value = favoriteId
  try {
    favorites.value = await removeFavorite(currentUser.value.username, favoriteId)
    message.success('已删除收藏')
  } catch (error) {
    message.error(String(error))
  } finally {
    removingFavoriteId.value = ''
  }
}

function mergeFavorite(list: FavoriteResource[], favorite: FavoriteResource) {
  if (list.some((item) => item.id === favorite.id || item.url === favorite.url)) {
    return list.map((item) => (item.id === favorite.id || item.url === favorite.url ? favorite : item))
  }
  return [favorite, ...list]
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

function formatFavoriteTime(value: string) {
  const timestamp = Number(value)
  if (!Number.isFinite(timestamp) || timestamp <= 0) return '收藏时间未知'
  return new Date(timestamp).toLocaleString('zh-CN', { hour12: false })
}

async function loadSettings() {
  try {
    settingsForm.value = toEditableSettings(await getSearchSettings())
    await refreshEmbeddedPansouStatus()
  } catch {
    settingsForm.value = cloneSettings(DEFAULT_SETTINGS)
  }
}

async function refreshSources(preventEmbeddedAutoStart = false) {
  const sourceSettings = preventEmbeddedAutoStart ? cloneSettings(settingsForm.value) : settingsForm.value
  if (preventEmbeddedAutoStart) {
    sourceSettings.embeddedPansou.autoStart = false
  }
  sources.value = await listSearchSources(sourceSettings)
  await refreshEmbeddedPansouStatus()
  const enabledIds = sources.value.filter((source) => source.enabled).map((source) => source.id)
  selectedSourceIds.value = selectedSourceIds.value.filter((id) => enabledIds.includes(id))
  if (!selectedSourceIds.value.length) {
    selectedSourceIds.value = enabledIds
  }
}

async function refreshEmbeddedPansouStatus() {
  try {
    embeddedPansouStatus.value = await getEmbeddedPansouStatus()
  } catch {
    embeddedPansouStatus.value = undefined
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
    await refreshEmbeddedPansouStatus()
    await refreshSources()
    settingsOpen.value = false
    message.success('搜索来源设置已保存')
  } catch (error) {
    message.error(String(error))
  } finally {
    settingsSaving.value = false
  }
}

async function handleToggleEmbeddedPansou() {
  embeddedPansouRestarting.value = true
  try {
    const settings = serializeSettings(settingsForm.value)
    const shouldStop = Boolean(embeddedPansouStatus.value?.running)
    embeddedPansouStatus.value = shouldStop
      ? await stopEmbeddedPansou(settings)
      : await startEmbeddedPansou(settings)
    await refreshSources(shouldStop)
    if (embeddedPansouStatus.value.running) {
      if (!selectedSourceIds.value.includes(EMBEDDED_PANSOU_SOURCE_ID)) {
        selectedSourceIds.value = [...selectedSourceIds.value, EMBEDDED_PANSOU_SOURCE_ID]
      }
      message.success('内置 PanSou 已开启')
    } else {
      message.success('内置 PanSou 已关闭')
    }
  } catch (error) {
    message.error(String(error))
  } finally {
    embeddedPansouRestarting.value = false
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

function toEditableSettings(settings: SearchSettings): EditableSearchSettings {
  const embeddedPansou = settings.embeddedPansou || DEFAULT_SETTINGS.embeddedPansou
  return {
    ...settings,
    embeddedPansou: {
      ...embeddedPansou,
      port: embeddedPansou.port || 10323,
      src: embeddedPansou.src || 'all',
      plugins: embeddedPansou.plugins?.length ? embeddedPansou.plugins : DEFAULT_EMBEDDED_PANSOU_PLUGINS,
      channelsText: (embeddedPansou.channels || []).join(','),
      pluginsText: (embeddedPansou.plugins?.length ? embeddedPansou.plugins : DEFAULT_EMBEDDED_PANSOU_PLUGINS).join(','),
      cloudTypesText: (embeddedPansou.cloudTypes || []).join(',')
    },
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
  const embeddedPansou: EmbeddedPansouConfig = {
    enabled: settings.embeddedPansou.enabled,
    autoStart: settings.embeddedPansou.autoStart,
    port: settings.embeddedPansou.port || 10323,
    src: settings.embeddedPansou.src || 'all',
    channels: splitList(settings.embeddedPansou.channelsText),
    plugins: splitList(settings.embeddedPansou.pluginsText),
    cloudTypes: splitList(settings.embeddedPansou.cloudTypesText),
    refresh: settings.embeddedPansou.refresh,
    cache: settings.embeddedPansou.cache !== false,
    concurrency: settings.embeddedPansou.concurrency || 4
  }
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
    embeddedPansou,
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
.login-shell {
  display: grid;
  place-items: center;
  min-height: 100vh;
  padding: 28px;
  background:
    radial-gradient(circle at 18% 14%, rgba(139, 205, 195, 0.2), transparent 30%),
    linear-gradient(135deg, #0b1220 0%, #101b2a 54%, #eef5f4 54.2%, #eef5f4 100%);
}

.login-panel {
  width: min(460px, 100%);
  padding: 30px;
  background: rgba(255, 255, 255, 0.96);
  border: 1px solid #dfe8ed;
  border-radius: 8px;
  box-shadow: 0 28px 72px rgba(15, 28, 44, 0.2);
}

.login-brand {
  display: flex;
  align-items: center;
  gap: 16px;
  margin-bottom: 28px;
}

.login-brand img {
  width: 58px;
  height: 58px;
  border-radius: 16px;
  box-shadow: 0 16px 34px rgba(13, 22, 36, 0.2);
}

.login-brand p {
  margin: 0 0 5px;
  color: #0f9489;
  font-size: 15px;
  font-weight: 700;
}

.login-brand h1 {
  margin: 0;
  color: #10202b;
  font-size: 28px;
  line-height: 1.2;
  letter-spacing: 0;
}

.login-form {
  display: grid;
  gap: 16px;
}

.login-form label {
  display: grid;
  gap: 8px;
}

.login-form label > span {
  color: #334155;
  font-weight: 700;
}

:deep(.ant-btn) {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  line-height: 1;
}

:deep(.ant-btn > .ant-btn-icon),
:deep(.ant-btn .ant-btn-loading-icon) {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  line-height: 0;
}

:deep(.ant-btn svg) {
  display: block;
  flex: 0 0 auto;
}

.login-meta {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  margin-top: 18px;
  color: #718196;
  font-size: 13px;
}

.workbench {
  display: grid;
  grid-template-columns: 76px minmax(0, 1fr);
  height: 100vh;
  min-height: 0;
  overflow: hidden;
  background:
    radial-gradient(circle at 18% 12%, rgba(139, 205, 195, 0.18), transparent 30%),
    linear-gradient(135deg, #0b1220 0%, #101b2a 45%, #f4f7f6 45.2%, #f4f7f6 100%);
}

.rail {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 18px;
  min-height: 0;
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
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  gap: 16px;
  min-width: 0;
  min-height: 0;
  padding: 30px 34px;
  overflow: hidden;
}

.hero {
  display: flex;
  align-items: center;
  justify-content: space-between;
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

.user-badge {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  color: #d8f4ef;
  background: rgba(11, 18, 32, 0.28);
  border: 1px solid rgba(216, 244, 239, 0.24);
  border-radius: 8px;
  font-weight: 700;
}

.search-panel,
.result-shell {
  background: rgba(255, 255, 255, 0.96);
  border: 1px solid #e2e9ee;
  border-radius: 8px;
  box-shadow: 0 24px 60px rgba(25, 39, 56, 0.12);
}

.search-panel {
  padding: 24px;
}

.search-view,
.favorites-view,
.source-config-view {
  display: grid;
  gap: 16px;
  min-height: 0;
}

.search-view {
  grid-template-rows: auto auto minmax(0, 1fr);
}

.source-config-view {
  grid-template-rows: auto minmax(0, 1fr);
  overflow: hidden;
}

.favorites-view {
  grid-template-rows: auto minmax(0, 1fr);
  overflow: hidden;
}

.output-scroll {
  min-height: 0;
  overflow-y: auto;
  padding-right: 6px;
  scrollbar-gutter: stable;
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

.panel-title {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  align-items: center;
  width: 100%;
  padding: 0;
  color: inherit;
  text-align: left;
  background: transparent;
  border: 0;
  cursor: pointer;
}

.panel-title > span:first-child {
  font-size: 16px;
  font-weight: 700;
}

.panel-title-meta {
  display: inline-flex;
  align-items: center;
  gap: 8px;
}

.panel-title small {
  color: #718196;
}

.reason-row {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.result-shell {
  padding: 22px 24px;
}

.source-summary-panel,
.source-config-group {
  padding: 18px 20px;
  background: rgba(255, 255, 255, 0.96);
  border: 1px solid #e2e9ee;
  border-radius: 8px;
  box-shadow: 0 18px 46px rgba(25, 39, 56, 0.08);
}

.source-summary-panel.collapsed {
  padding-bottom: 18px;
}

.source-summary-grid,
.source-config-groups {
  display: grid;
  gap: 12px;
}

.source-summary-grid {
  grid-template-columns: repeat(auto-fit, minmax(210px, 1fr));
  margin-top: 12px;
}

.source-summary-card {
  overflow: hidden;
  background: #f8fbfc;
  border: 1px solid #e3ebf0;
  border-radius: 8px;
}

.source-summary-trigger {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  width: 100%;
  padding: 12px 14px;
  color: inherit;
  text-align: left;
  background: transparent;
  border: 0;
  cursor: pointer;
}

.source-summary-copy {
  display: grid;
  gap: 6px;
}

.source-summary-copy strong {
  color: #10202b;
  font-size: 16px;
}

.source-summary-copy span {
  color: #0f9489;
  font-size: 14px;
  font-weight: 700;
}

.source-summary-copy small {
  color: #718196;
}

.summary-chevron {
  flex: 0 0 auto;
  margin-top: 2px;
  color: #7a8796;
  transition: transform 0.18s ease;
}

.summary-chevron.open {
  transform: rotate(180deg);
}

.source-summary-detail {
  padding: 0 14px 14px;
}

.source-summary-detail-list {
  display: grid;
  gap: 8px;
  padding: 12px;
  background: #ffffff;
  border: 1px solid #e3ebf0;
  border-radius: 8px;
}

.source-summary-detail-list > div {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  color: #334155;
}

.source-summary-detail p {
  margin: 0;
  padding: 12px;
  color: #718196;
}

.source-config-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 18px;
}

.source-config-head h2 {
  margin: 0 0 6px;
  font-size: 24px;
}

.source-config-head p {
  margin: 0;
  color: #718196;
}

.source-config-actions {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.source-config-actions > span {
  padding: 8px 12px;
  color: #0a6f6b;
  background: #e6f8f5;
  border: 1px solid #bce7e1;
  border-radius: 8px;
  font-weight: 700;
}

.source-config-groups {
  overflow-y: auto;
  padding-right: 6px;
  scrollbar-gutter: stable;
}

.source-config-group {
  display: grid;
  gap: 14px;
}

.source-config-group-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.source-config-group-head > div {
  min-width: 0;
}

.source-config-group-head h3 {
  margin: 0 0 6px;
  font-size: 18px;
}

.source-config-group-head p {
  margin: 0;
  color: #718196;
}

.source-config-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(210px, 1fr));
  gap: 12px;
}

.source-config-card {
  display: grid;
  grid-template-columns: 22px minmax(0, 1fr);
  gap: 12px;
  align-items: start;
  padding: 14px;
  text-align: left;
  color: #334155;
  background: #f7fafb;
  border: 1px solid #e3ebf0;
  border-radius: 8px;
  cursor: pointer;
  transition: transform 0.18s ease, box-shadow 0.18s ease, border-color 0.18s ease;
}

.source-config-card:hover:not(:disabled) {
  transform: translateY(-1px);
  border-color: #9fdad4;
  box-shadow: 0 16px 34px rgba(24, 61, 74, 0.08);
}

.source-config-card.selected {
  background: #eefaf8;
  border-color: #9de1d9;
}

.source-config-card.configurable {
  background: #f9f5ea;
  border-color: #ead59b;
}

.source-config-card.disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.source-check {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  margin-top: 2px;
  border-radius: 999px;
  border: 1px solid #cfd9e1;
  color: transparent;
}

.source-check.selected {
  color: #0f9489;
  border-color: #0f9489;
  background: #e6f8f5;
}

.source-check.disabled {
  color: #c89c3b;
  border-color: #ead59b;
  background: #fff7e8;
}

.source-config-card strong,
.source-config-card small,
.source-config-card p {
  display: block;
}

.source-config-card strong {
  color: #10202b;
  font-size: 17px;
}

.source-config-card small {
  margin-top: 4px;
  color: #0a6f6b;
  font-weight: 700;
}

.source-config-card p {
  margin: 10px 0 0;
  color: #718196;
  line-height: 1.5;
}

.source-card-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 10px;
}

.source-card-meta span {
  padding: 4px 8px;
  color: #5c6b7d;
  background: #edf3f7;
  border-radius: 999px;
  font-size: 12px;
}

.favorites-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 18px;
  padding: 18px 20px;
  background: rgba(255, 255, 255, 0.96);
  border: 1px solid #e2e9ee;
  border-radius: 8px;
  box-shadow: 0 18px 46px rgba(25, 39, 56, 0.08);
}

.favorites-head h2 {
  margin: 0 0 6px;
  font-size: 24px;
}

.favorites-head p {
  margin: 0;
  color: #718196;
}

.favorites-scroll {
  min-height: 0;
  overflow-y: auto;
  padding-right: 6px;
  scrollbar-gutter: stable;
}

.favorites-loading,
.favorites-empty,
.favorite-card {
  background: rgba(255, 255, 255, 0.96);
  border: 1px solid #e2e9ee;
  border-radius: 8px;
  box-shadow: 0 18px 46px rgba(25, 39, 56, 0.08);
}

.favorites-loading,
.favorites-empty {
  display: grid;
  place-items: center;
  gap: 10px;
  min-height: 260px;
  color: #607086;
  text-align: center;
}

.favorites-empty {
  padding: 28px;
}

.favorites-empty svg {
  color: #0f9489;
}

.favorites-empty h3 {
  margin: 0;
  color: #10202b;
  font-size: 20px;
}

.favorites-empty p {
  max-width: 420px;
  margin: 0;
  color: #718196;
  line-height: 1.6;
}

.favorites-list {
  display: grid;
  gap: 12px;
}

.favorite-card {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 18px;
  padding: 18px;
}

.favorite-main {
  min-width: 0;
}

.favorite-main h3 {
  margin: 0 0 10px;
  color: #10202b;
  font-size: 20px;
  line-height: 1.35;
}

.favorite-main p {
  display: -webkit-box;
  max-height: 70px;
  margin: 0 0 10px;
  overflow: hidden;
  color: #536274;
  line-height: 1.55;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 3;
}

.favorite-main a {
  display: block;
  margin-bottom: 10px;
  overflow-wrap: anywhere;
  color: #0a74d9;
  font-weight: 700;
}

.favorite-actions {
  display: flex;
  align-items: flex-end;
  justify-content: flex-end;
  gap: 8px;
  flex-wrap: wrap;
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

.search-loading-panel {
  position: relative;
  display: grid;
  grid-template-columns: 54px minmax(0, 1fr);
  gap: 14px 16px;
  overflow: hidden;
  padding: 20px;
  background:
    linear-gradient(120deg, rgba(230, 248, 245, 0.92), rgba(255, 255, 255, 0.96)),
    #ffffff;
  border: 1px solid #bce7e1;
  border-radius: 8px;
}

.search-loading-panel::after {
  position: absolute;
  inset: 0;
  content: "";
  background: linear-gradient(90deg, transparent, rgba(15, 148, 137, 0.1), transparent);
  transform: translateX(-100%);
  animation: scan 1.8s ease-in-out infinite;
}

.loading-orbit {
  position: relative;
  z-index: 1;
  display: grid;
  place-items: center;
  width: 54px;
  height: 54px;
  color: #0f9489;
  background: #ffffff;
  border: 1px solid #bce7e1;
  border-radius: 999px;
}

.loading-orbit span {
  position: absolute;
  inset: 7px;
  border: 1px dashed #9fdad4;
  border-radius: 999px;
  animation: spin 2.4s linear infinite;
}

.loading-copy {
  position: relative;
  z-index: 1;
  display: grid;
  gap: 5px;
  align-self: center;
}

.loading-copy strong {
  color: #10202b;
  font-size: 18px;
}

.loading-copy p {
  margin: 0;
  color: #5f6f83;
}

.loading-meter {
  position: relative;
  z-index: 1;
  grid-column: 1 / -1;
  height: 8px;
  overflow: hidden;
  background: #dce9e7;
  border-radius: 999px;
}

.loading-meter i {
  display: block;
  width: 38%;
  height: 100%;
  background: linear-gradient(90deg, #0f9489, #62c7bd);
  border-radius: inherit;
  animation: loading-meter 1.35s ease-in-out infinite;
}

.loading-stats {
  position: relative;
  z-index: 1;
  grid-column: 1 / -1;
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.loading-stats span {
  padding: 5px 9px;
  color: #0a6f6b;
  background: #ffffff;
  border: 1px solid #c8ebe6;
  border-radius: 999px;
  font-size: 12px;
  font-weight: 700;
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
  grid-template-columns: minmax(0, 1fr);
  gap: 14px;
  padding: 18px;
  background: #ffffff;
  border: 1px solid #e5ebef;
  border-radius: 8px;
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

.update-section {
  background: #eefaf8;
  border-color: #c7e9e4;
}

.update-grid {
  display: grid;
  gap: 12px;
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.update-grid > div {
  background: #ffffff;
  border: 1px solid #dce9e7;
  border-radius: 8px;
  display: grid;
  gap: 5px;
  padding: 12px 14px;
}

.update-grid span {
  color: #718196;
  font-size: 12px;
}

.update-grid strong {
  color: #1f2933;
  font-size: 15px;
}

.update-progress {
  max-width: 360px;
}

:global(.update-body) {
  margin: 0;
  max-height: 300px;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-word;
}

.settings-actions,
.settings-checks {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  align-items: center;
}

.embedded-pansou-panel {
  display: grid;
  gap: 10px;
}

.embedded-status {
  display: grid;
  grid-template-columns: 28px minmax(0, 1fr) auto;
  gap: 10px;
  align-items: center;
  padding: 12px;
  color: #64748b;
  background: #ffffff;
  border: 1px solid #e4eaef;
  border-radius: 8px;
}

.embedded-status.running {
  color: #0f9489;
  border-color: #9de1d9;
}

.embedded-status strong,
.embedded-status small {
  display: block;
}

.embedded-status small {
  margin-top: 3px;
  color: #718196;
}

.embedded-status > span {
  color: #718196;
  font-size: 12px;
}

.embedded-controls {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
}

.embedded-controls :deep(.ant-checkbox-wrapper) {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  margin-inline-start: 0;
  min-height: 32px;
  line-height: 1;
}

.embedded-controls :deep(.ant-checkbox) {
  top: 0;
  display: inline-flex;
  flex: 0 0 16px;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
}

.embedded-controls :deep(.ant-checkbox-inner) {
  width: 16px;
  height: 16px;
  min-width: 16px;
  border-radius: 5px;
}

.embedded-controls :deep(.ant-checkbox-checked::after) {
  display: none;
}

.embedded-controls :deep(.ant-checkbox + span) {
  padding-inline: 0;
  white-space: nowrap;
}

.embedded-grid {
  display: grid;
  gap: 10px;
  grid-template-columns: 110px minmax(140px, 1fr) 100px;
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

.result-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  align-self: end;
}

.detail-button {
  display: inline-flex;
  gap: 8px;
  align-items: center;
  justify-content: center;
  height: 38px;
  padding: 0 14px;
  color: #0a6f6b;
  line-height: 1;
  background: #e6f8f5;
  border: 1px solid #bce7e1;
  border-radius: 8px;
  cursor: pointer;
}

.favorite-button {
  display: inline-flex;
  gap: 8px;
  align-items: center;
  justify-content: center;
  height: 38px;
  padding: 0 14px;
  color: #344255;
  line-height: 1;
  background: #f6f8fb;
  border: 1px solid #dfe7ec;
  border-radius: 8px;
  cursor: pointer;
}

.detail-button svg,
.favorite-button svg {
  display: block;
  flex: 0 0 auto;
}

.favorite-button:disabled {
  cursor: wait;
  opacity: 0.75;
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

.detail-url {
  word-break: break-word;
}

.detail-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin: 0 0 12px;
}

.detail-meta span {
  padding: 4px 9px;
  color: #335064;
  font-size: 13px;
  font-weight: 700;
  background: #eef6f7;
  border: 1px solid #d8e8ec;
  border-radius: 999px;
}

.detail-box p {
  margin: 14px 0 0;
  color: #718196;
}

.detail-validation {
  margin-top: 14px;
  padding: 10px 12px;
  line-height: 1.5;
  border-radius: 8px;
}

.detail-validation.valid {
  color: #0a6f6b;
  background: #e6f8f5;
  border: 1px solid #bce7e1;
}

.detail-validation.warning {
  color: #8a5a08;
  background: #fff7e8;
  border: 1px solid #f0d199;
}

.detail-validation.invalid {
  color: #b73c3c;
  background: #fff2f2;
  border: 1px solid #ffc9c9;
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

@keyframes scan {
  50%,
  100% {
    transform: translateX(100%);
  }
}

@keyframes loading-meter {
  0% {
    transform: translateX(-100%);
  }

  55% {
    transform: translateX(60%);
  }

  100% {
    transform: translateX(260%);
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

  .source-summary-grid,
  .source-config-grid {
    grid-template-columns: 1fr;
  }

  .result-item {
    grid-template-columns: 1fr;
  }

  .source-config-head,
  .favorites-head,
  .settings-section-head {
    flex-direction: column;
  }

  .update-grid {
    grid-template-columns: 1fr;
  }

  .pool-row,
  .cms-row,
  .indexer-row,
  .favorite-card,
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

  .result-actions,
  .favorite-actions {
    justify-content: stretch;
  }

  .detail-button,
  .favorite-button,
  .favorite-actions :deep(.ant-btn) {
    justify-content: center;
    width: 100%;
  }
}
</style>
