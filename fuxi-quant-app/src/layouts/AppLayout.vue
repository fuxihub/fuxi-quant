<script setup>
import { ref, onMounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'

const router = useRouter()
const route = useRoute()

// 菜单项
const menuItems = ref([
  { label: '智能体', icon: 'pi pi-microchip-ai', route: '/agent' },
  { label: '策略', icon: 'pi pi-code', route: '/strategy' },
  { label: '回测', icon: 'pi pi-chart-line', route: '/backtest' },
  { label: '数据', icon: 'pi pi-database', route: '/data' },
  { label: '实盘', icon: 'pi pi-bolt', route: '/live' },
  { label: '系统设置', icon: 'pi pi-cog', route: '/settings' },
])

// 用户菜单项
const userMenuItems = ref([
  { label: '个人设置', icon: 'pi pi-cog' },
  { label: '帮助文档', icon: 'pi pi-question-circle' },
  { separator: true },
  { label: '退出登录', icon: 'pi pi-sign-out' },
])

// 暗黑模式
const isDark = ref(false)

onMounted(() => {
  // 检测系统主题偏好
  const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches
  isDark.value = prefersDark
  applyTheme()
})

function toggleDarkMode() {
  isDark.value = !isDark.value
  applyTheme()
}

function applyTheme() {
  if (isDark.value) {
    document.documentElement.classList.add('app-dark')
  } else {
    document.documentElement.classList.remove('app-dark')
  }
}

function navigateTo(path) {
  router.push(path)
}

function isActive(path) {
  return route.path === path
}

// 用户菜单
const userMenu = ref()
function toggleUserMenu(event) {
  userMenu.value.toggle(event)
}
</script>

<template>
  <div class="h-screen flex flex-col overflow-hidden">
    <!-- 顶部菜单栏 -->
    <header
      class="flex-shrink-0 flex items-center justify-between px-4 py-2 border-b border-surface-200 dark:border-surface-700 bg-surface-0 dark:bg-surface-900">
      <!-- 左侧 Logo -->
      <div class="flex items-center gap-2">
        <!-- <span class="font-bold text-lg">伏羲量化</span> -->
      </div>

      <!-- 中间菜单 -->
      <nav class="flex items-center gap-1">
        <Button
          v-for="item in menuItems"
          :key="item.label"
          :label="item.label"
          :icon="item.icon"
          text
          :plain="!isActive(item.route)"
          :class="{ '!font-normal': !isActive(item.route), '!font-semibold': isActive(item.route) }"
          @click="navigateTo(item.route)" />
      </nav>

      <!-- 右侧：主题切换 + 用户头像 -->
      <div class="flex items-center gap-2">
        <!-- 主题切换按钮 -->
        <Button
          :icon="isDark ? 'pi pi-sun' : 'pi pi-moon'"
          severity="secondary"
          text
          rounded
          @click="toggleDarkMode"
          v-tooltip.bottom="isDark ? '切换亮色模式' : '切换暗色模式'" />

        <!-- 用户头像 -->
        <Avatar
          icon="pi pi-user"
          shape="circle"
          class="cursor-pointer"
          style="background-color: var(--p-primary-color); color: white"
          @click="toggleUserMenu"
          v-tooltip.bottom="'我的'" />
        <Menu
          ref="userMenu"
          :model="userMenuItems"
          :popup="true" />
      </div>
    </header>

    <!-- 主内容区域 -->
    <main class="flex-1 overflow-auto p-4">
      <slot />
    </main>
  </div>
</template>
