<script setup>
import { ref } from 'vue'

const activeTab = ref('agents')
const menu = ref()
const selectedAgent = ref(null)

const menuItems = [
  { id: 'agents', label: '智能体管理', icon: 'pi pi-box' },
  { id: 'exchanges', label: '交易所配置', icon: 'pi pi-wallet' },
  { id: 'profile', label: '个人偏好', icon: 'pi pi-user' },
]

const agentMenuItems = [
  {
    label: '设为默认',
    icon: 'pi pi-star',
    command: () => setAsDefault(selectedAgent.value),
  },
  {
    label: '删除',
    icon: 'pi pi-trash',
    class: 'text-red-500',
    command: () => deleteAgent(selectedAgent.value),
  },
]

// 模拟数据
const agents = ref([
  {
    id: 1,
    name: 'DeepSeek-R1-Distill-Llama-8B',
    version: 'v1.0.2',
    size: '5.6GB',
    status: 'ready',
    date: '2025-12-01',
    isDefault: true,
  },
  {
    id: 2,
    name: 'Fuxi-Trend-Master',
    version: 'v2.1.0',
    size: '25MB',
    status: 'active',
    date: '2025-12-08',
    isDefault: false,
  },
  {
    id: 3,
    name: 'Qwen2.5-14B-Instruct',
    version: 'v1.0',
    size: '9.2GB',
    status: 'ready',
    date: '2025-12-05',
    isDefault: false,
  },
])

const toggleMenu = (event, agent) => {
  selectedAgent.value = agent
  menu.value.toggle(event)
}

const setAsDefault = (agent) => {
  if (!agent) return
  agents.value.forEach((a) => (a.isDefault = false))
  agent.isDefault = true
}

const deleteAgent = (agent) => {
  if (!agent) return
  agents.value = agents.value.filter((a) => a.id !== agent.id)
}

const exchanges = ref([
  { id: 'binance', name: 'Binance', apiKey: 'vm************************29d', secretKey: '' },
  { id: 'okx', name: 'OKX', apiKey: '', secretKey: '' },
])

const profile = ref({
  username: 'Husky',
  email: 'husky@fuxihub.com',
})
</script>

<template>
  <div class="flex h-full gap-6 p-6 max-w-7xl mx-auto">
    <!-- 左侧导航 -->
    <div class="w-64 flex-none space-y-2">
      <div
        v-for="item in menuItems"
        :key="item.id">
        <button
          @click="activeTab = item.id"
          class="w-full flex items-center gap-3 px-4 py-3 rounded-xl transition-all text-left font-medium duration-200"
          :class="
            activeTab === item.id
              ? 'bg-primary text-primary-contrast shadow-lg shadow-primary/30 translate-x-1'
              : 'text-surface-600 dark:text-surface-400 hover:bg-surface-100 dark:hover:bg-surface-800 hover:text-surface-900 dark:hover:text-surface-50'
          ">
          <i
            :class="item.icon"
            class="text-lg"></i>
          {{ item.label }}
        </button>
      </div>
    </div>

    <!-- 右侧内容区 -->
    <div
      class="flex-1 min-w-0 bg-surface-0 dark:bg-surface-900 rounded-2xl border border-surface-200 dark:border-surface-700 shadow-sm overflow-hidden flex flex-col">
      <!-- 标题栏 -->
      <div
        class="px-8 py-6 border-b border-surface-200 dark:border-surface-700 bg-surface-50/50 dark:bg-surface-900/50">
        <h2 class="text-2xl font-bold text-surface-900 dark:text-surface-50">
          {{ menuItems.find((i) => i.id === activeTab)?.label }}
        </h2>
        <p class="text-surface-500 mt-1 text-sm">管理您的{{ menuItems.find((i) => i.id === activeTab)?.label }}</p>
      </div>

      <!-- 内容主体 -->
      <div class="p-8 overflow-y-auto flex-1">
        <!-- 1. 智能体管理 -->
        <transition
          name="fade"
          mode="out-in">
          <div
            v-if="activeTab === 'agents'"
            class="space-y-4">
            <div
              v-for="agent in agents"
              :key="agent.id"
              class="group flex items-center justify-between p-5 border rounded-xl hover:shadow-md transition-all"
              :class="
                agent.isDefault
                  ? 'border-primary bg-primary/5 dark:bg-primary/10'
                  : 'border-surface-200 dark:border-surface-700 bg-surface-0 dark:bg-surface-800 hover:border-primary/50'
              ">
              <div class="flex items-center gap-5">
                <div
                  class="w-12 h-12 rounded-xl flex items-center justify-center transition-transform group-hover:scale-110"
                  :class="agent.isDefault ? 'bg-primary text-primary-contrast' : 'bg-primary/10 text-primary'">
                  <i class="pi pi-box text-xl"></i>
                </div>
                <div>
                  <div class="flex items-center gap-2 mb-1">
                    <div class="font-bold text-lg text-surface-900 dark:text-surface-50">{{ agent.name }}</div>
                    <span
                      v-if="agent.isDefault"
                      class="px-2 py-0.5 rounded text-xs bg-primary text-primary-contrast font-medium">
                      默认
                    </span>
                  </div>
                  <div class="flex items-center gap-3 text-sm text-surface-500">
                    <span class="bg-surface-100 dark:bg-surface-700 px-2 py-0.5 rounded text-xs">
                      {{ agent.version }}
                    </span>
                    <span>{{ agent.size }}</span>
                    <span>{{ agent.date }}</span>
                  </div>
                </div>
              </div>
              <div class="flex items-center gap-3">
                <span
                  class="px-3 py-1 rounded-full text-xs font-medium flex items-center gap-1.5"
                  :class="
                    agent.status === 'active'
                      ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400'
                      : 'bg-surface-100 text-surface-600 dark:bg-surface-700 dark:text-surface-400'
                  ">
                  <span
                    class="w-1.5 h-1.5 rounded-full"
                    :class="agent.status === 'active' ? 'bg-green-500' : 'bg-surface-500'"></span>
                  {{ agent.status === 'active' ? '运行中' : '已就绪' }}
                </span>
                <Button
                  icon="pi pi-ellipsis-v"
                  text
                  rounded
                  severity="secondary"
                  @click="toggleMenu($event, agent)" />
              </div>
            </div>

            <!-- 添加新智能体按钮 -->
            <button
              class="w-full py-4 border-2 border-dashed border-surface-200 dark:border-surface-700 rounded-xl text-surface-500 hover:text-primary hover:border-primary/50 hover:bg-surface-50 dark:hover:bg-surface-800/50 transition-all flex items-center justify-center gap-2 group">
              <div
                class="w-8 h-8 rounded-full bg-surface-100 dark:bg-surface-800 flex items-center justify-center group-hover:bg-primary/10 group-hover:text-primary transition-colors">
                <i class="pi pi-plus"></i>
              </div>
              <span class="font-medium">添加新智能体</span>
            </button>

            <Menu
              ref="menu"
              :model="agentMenuItems"
              :popup="true" />
          </div>

          <!-- 2. 交易所配置 -->
          <div
            v-else-if="activeTab === 'exchanges'"
            class="grid gap-6">
            <div
              v-for="ex in exchanges"
              :key="ex.id"
              class="p-6 border border-surface-200 dark:border-surface-700 rounded-xl bg-surface-0 dark:bg-surface-800 hover:shadow-md transition-shadow">
              <div class="flex items-center justify-between mb-6">
                <div class="flex items-center gap-3">
                  <div
                    class="w-10 h-10 rounded-full bg-surface-100 dark:bg-surface-700 flex items-center justify-center font-bold text-lg text-surface-600 dark:text-surface-300">
                    {{ ex.name[0] }}
                  </div>
                  <span class="text-lg font-bold text-surface-900 dark:text-surface-50">{{ ex.name }}</span>
                </div>
                <div class="text-xs px-2 py-1 bg-surface-100 dark:bg-surface-700 rounded text-surface-500">
                  {{ ex.apiKey ? '已配置' : '未配置' }}
                </div>
              </div>
              <div class="grid gap-5">
                <div class="space-y-2">
                  <label class="text-sm font-medium text-surface-700 dark:text-surface-300">API Key</label>
                  <input
                    type="text"
                    v-model="ex.apiKey"
                    class="w-full p-3 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-900 text-surface-900 dark:text-surface-50 focus:ring-2 focus:ring-primary focus:border-transparent outline-none transition-all"
                    placeholder="输入 API Key" />
                </div>
                <div class="space-y-2">
                  <label class="text-sm font-medium text-surface-700 dark:text-surface-300">Secret Key</label>
                  <input
                    type="password"
                    v-model="ex.secretKey"
                    class="w-full p-3 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-900 text-surface-900 dark:text-surface-50 focus:ring-2 focus:ring-primary focus:border-transparent outline-none transition-all"
                    placeholder="输入 Secret Key" />
                </div>
              </div>
              <div class="flex justify-end mt-6 pt-4 border-t border-surface-100 dark:border-surface-700/50">
                <Button
                  label="验证并保存"
                  icon="pi pi-check"
                  size="small" />
              </div>
            </div>
          </div>

          <!-- 3. 个人偏好 -->
          <div
            v-else-if="activeTab === 'profile'"
            class="max-w-2xl">
            <div class="space-y-8">
              <!-- 头像区域 -->
              <div
                class="flex items-center gap-6 p-6 bg-surface-50 dark:bg-surface-800/50 rounded-xl border border-dashed border-surface-300 dark:border-surface-700">
                <div
                  class="w-24 h-24 rounded-full bg-white dark:bg-surface-700 shadow-sm flex items-center justify-center overflow-hidden border-4 border-white dark:border-surface-600">
                  <img
                    :src="`https://api.dicebear.com/7.x/avataaars/svg?seed=${profile.username}`"
                    alt="Avatar"
                    class="w-full h-full" />
                </div>
                <div>
                  <h3 class="font-bold text-lg mb-1">头像设置</h3>
                  <p class="text-surface-500 text-sm mb-4">支持 JPG, PNG 格式，最大 2MB</p>
                  <div class="flex gap-3">
                    <Button
                      label="上传新头像"
                      size="small"
                      outlined />
                    <Button
                      label="移除"
                      size="small"
                      severity="danger"
                      text />
                  </div>
                </div>
              </div>

              <div class="grid gap-6">
                <div class="space-y-2">
                  <label class="block font-medium text-surface-700 dark:text-surface-300">用户名</label>
                  <input
                    type="text"
                    v-model="profile.username"
                    class="w-full p-3 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-900 text-surface-900 dark:text-surface-50 focus:ring-2 focus:ring-primary focus:border-transparent outline-none transition-all" />
                </div>

                <div class="space-y-2">
                  <label class="block font-medium text-surface-700 dark:text-surface-300">邮箱地址</label>
                  <input
                    type="email"
                    v-model="profile.email"
                    class="w-full p-3 rounded-lg border border-surface-200 dark:border-surface-600 bg-surface-50 dark:bg-surface-900 text-surface-900 dark:text-surface-50 focus:ring-2 focus:ring-primary focus:border-transparent outline-none transition-all" />
                </div>
              </div>

              <div class="pt-6 border-t border-surface-200 dark:border-surface-700 flex justify-end">
                <Button
                  label="保存更改"
                  icon="pi pi-save" />
              </div>
            </div>
          </div>
        </transition>
      </div>
    </div>
  </div>
</template>

<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
