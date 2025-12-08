<script setup>
import { ref, onMounted } from 'vue'

// 菜单项
const menuItems = ref([
  {
    label: '首页',
    icon: 'pi pi-home',
  },
  {
    label: '策略',
    icon: 'pi pi-code',
    items: [
      { label: '策略列表', icon: 'pi pi-list' },
      { label: '新建策略', icon: 'pi pi-plus' },
    ],
  },
  {
    label: '回测',
    icon: 'pi pi-chart-line',
    items: [
      { label: '回测记录', icon: 'pi pi-history' },
      { label: '开始回测', icon: 'pi pi-play' },
    ],
  },
  {
    label: '数据',
    icon: 'pi pi-database',
    items: [
      { label: '行情数据', icon: 'pi pi-chart-bar' },
      { label: '因子库', icon: 'pi pi-box' },
    ],
  },
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

// 子菜单
const subMenu = ref()
const currentSubMenuItems = ref([])
function toggleMenu(event, item) {
  currentSubMenuItems.value = item.items || []
  subMenu.value.toggle(event)
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
        <svg
          width="28"
          height="32"
          viewBox="0 0 35 40"
          fill="none"
          xmlns="http://www.w3.org/2000/svg">
          <path
            d="M17.5 0L35 10V30L17.5 40L0 30V10L17.5 0Z"
            fill="var(--p-primary-color)" />
          <text
            x="17.5"
            y="26"
            text-anchor="middle"
            fill="white"
            font-size="18"
            font-weight="bold">
            F
          </text>
        </svg>
        <span class="font-bold text-lg">Fuxi Quant</span>
      </div>

      <!-- 中间菜单 -->
      <nav class="flex items-center gap-1">
        <template
          v-for="item in menuItems"
          :key="item.label">
          <Button
            v-if="!item.items"
            :label="item.label"
            :icon="item.icon"
            text
            plain
            class="!font-normal" />
          <Button
            v-else
            :label="item.label"
            :icon="item.icon"
            text
            plain
            class="!font-normal"
            @click="(e) => toggleMenu(e, item)" />
        </template>
        <Menu
          ref="subMenu"
          :model="currentSubMenuItems"
          :popup="true" />
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
