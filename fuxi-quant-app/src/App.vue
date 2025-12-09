<script setup>
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import AppLayout from './layouts/AppLayout.vue'

const greetMsg = ref('')
const name = ref('')

async function greet() {
  greetMsg.value = await invoke('greet', { name: name.value })
}
</script>

<template>
  <AppLayout>
    <!-- 示例页面内容 -->
    <div class="max-w-6xl mx-auto">
      <Card>
        <template #title>欢迎使用 Fuxi Quant</template>
        <template #subtitle>量化交易回测框架</template>
        <template #content>
          <p class="mb-4 selectable">Fuxi Quant 是一个基于 Rust 开发的量化交易回测框架，支持合约交易（多空双向）。</p>

          <div class="flex gap-2 mb-4">
            <InputText
              v-model="name"
              placeholder="输入名称..."
              class="flex-1" />
            <Button
              label="问候"
              icon="pi pi-send"
              @click="greet" />
          </div>

          <Message
            v-if="greetMsg"
            severity="success"
            :closable="false">
            {{ greetMsg }}
          </Message>
        </template>
      </Card>

      <!-- 功能卡片区 -->
      <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mt-4">
        <Card>
          <template #header>
            <div class="flex items-center justify-center p-4 bg-primary/10 rounded-t">
              <i class="pi pi-code text-4xl text-primary"></i>
            </div>
          </template>
          <template #title>策略开发</template>
          <template #content>
            <p class="text-sm text-surface-500">使用 Rhai 脚本语言编写量化策略，支持 Polars DataFrame 操作。</p>
          </template>
          <template #footer>
            <Button
              label="开始编写"
              icon="pi pi-arrow-right"
              iconPos="right"
              text />
          </template>
        </Card>

        <Card>
          <template #header>
            <div class="flex items-center justify-center p-4 bg-green-500/10 rounded-t">
              <i class="pi pi-chart-line text-4xl text-green-500"></i>
            </div>
          </template>
          <template #title>历史回测</template>
          <template #content>
            <p class="text-sm text-surface-500">基于历史数据对策略进行回测验证，分析收益与风险指标。</p>
          </template>
          <template #footer>
            <Button
              label="开始回测"
              icon="pi pi-arrow-right"
              iconPos="right"
              text
              severity="success" />
          </template>
        </Card>

        <Card>
          <template #header>
            <div class="flex items-center justify-center p-4 bg-orange-500/10 rounded-t">
              <i class="pi pi-database text-4xl text-orange-500"></i>
            </div>
          </template>
          <template #title>数据管理</template>
          <template #content>
            <p class="text-sm text-surface-500">管理行情数据、因子数据，支持多种数据源接入。</p>
          </template>
          <template #footer>
            <Button
              label="管理数据"
              icon="pi pi-arrow-right"
              iconPos="right"
              text
              severity="warn" />
          </template>
        </Card>
      </div>
    </div>
  </AppLayout>
</template>
