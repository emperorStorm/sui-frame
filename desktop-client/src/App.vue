<template>
  <a-config-provider :theme="themeConfig">
    <SearchWorkbench />
  </a-config-provider>
</template>

<script setup lang="ts">
import SearchWorkbench from './views/SearchWorkbench.vue'

const themeConfig = {
  token: {
    colorPrimary: '#0a8f8a',
    colorSuccess: '#2f9f67',
    colorWarning: '#c89c3b',
    colorError: '#d64848',
    borderRadius: 8,
    fontSize: 14,
    controlHeight: 36,
    colorText: '#1f2933',
    colorTextSecondary: '#657487',
    colorBorder: '#dce4eb',
    fontFamily: '"PingFang SC", "Microsoft YaHei", "Noto Sans CJK SC", sans-serif'
  },
  components: {
    Button: {
      primaryShadow: '0 12px 24px rgba(10, 143, 138, 0.22)'
    },
    Modal: {
      titleFontSize: 22,
      borderRadiusLG: 10
    },
    Tag: {
      borderRadiusSM: 6
    }
  }
}
</script>
