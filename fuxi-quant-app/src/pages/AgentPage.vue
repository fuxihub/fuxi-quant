<script setup>
import { ref, computed, nextTick, watch, onMounted } from 'vue'
import { useVirtualizer } from '@tanstack/vue-virtual'
import { invoke, Channel } from '@tauri-apps/api/core'
import { resolveResource } from '@tauri-apps/api/path'

// ============ 常量配置 ============
const MAX_MESSAGES = 200 // 最大消息数量限制
const TYPING_SPEED = { slow: 2, normal: 4, fast: 8 } // 打字速度（字符/帧）
const MAX_TOKENS = 32768 // 最大生成 token 数
const MODEL_NAME = 'resources/Qwen3-0.6B-Q8_0.gguf' // 模型资源路径

// ============ 状态 ============
const messages = ref([{ role: 'assistant', content: '你好！我是阿强，您的量化交易助手。有什么我可以帮你的吗？' }])
const inputContent = ref('')
const parentRef = ref(null)
const isTyping = ref(false)
const pendingQueue = ref('')
const isReceiving = ref(false)
const isAtBottom = ref(true)
const shouldFollowBottom = ref(true)
const isModelLoaded = ref(false)
const isLoadingModel = ref(false)
const loadError = ref('')

// ============ 模型加载 ============
const loadModel = async () => {
  if (isModelLoaded.value || isLoadingModel.value) return

  isLoadingModel.value = true
  loadError.value = ''

  try {
    // 检查模型是否已加载
    const loaded = await invoke('is_model_loaded')
    if (loaded) {
      isModelLoaded.value = true
      isLoadingModel.value = false
      return
    }

    // 解析模型资源路径
    const modelPath = await resolveResource(MODEL_NAME)
    await invoke('load_model', { modelPath })
    isModelLoaded.value = true
  } catch (e) {
    loadError.value = `模型加载失败: ${e}`
    console.error('模型加载失败:', e)
  } finally {
    isLoadingModel.value = false
  }
}

// 组件挂载时加载模型
onMounted(() => {
  loadModel()
})

// ============ 虚拟滚动配置 ============
const virtualizerOptions = computed(() => ({
  count: messages.value.length,
  getScrollElement: () => parentRef.value,
  estimateSize: () => 60, // 预估每条消息高度
  overscan: 5, // 预渲染额外行数
}))

const virtualizer = useVirtualizer(virtualizerOptions)
const virtualRows = computed(() => virtualizer.value.getVirtualItems())
const totalSize = computed(() => virtualizer.value.getTotalSize())

// 监听消息变化，测量实际高度并滚动
watch(
  () => messages.value.length,
  () => {
    nextTick(() => {
      virtualizer.value.measure()
      // 如果应该跟随底部，则滚动
      if (shouldFollowBottom.value) {
        scrollToBottom()
      }
    })
  }
)

// ============ 发送消息 ============
const sendMessage = async () => {
  if (!inputContent.value.trim() || isTyping.value) return

  // 模型未加载时提示
  if (!isModelLoaded.value) {
    if (isLoadingModel.value) {
      return // 正在加载中
    }
    // 尝试重新加载
    await loadModel()
    if (!isModelLoaded.value) {
      return
    }
  }

  // 清理历史消息的打字状态，释放内存
  messages.value.forEach((msg) => {
    if (msg.isTyping) msg.isTyping = false
  })

  // 限制消息数量
  if (messages.value.length >= MAX_MESSAGES) {
    messages.value = messages.value.slice(-MAX_MESSAGES + 2)
  }

  // 添加用户消息
  messages.value.push({
    role: 'user',
    content: inputContent.value,
  })

  const userQuery = inputContent.value
  inputContent.value = ''

  // 添加空的 AI 消息（打字中状态）
  messages.value.push({
    role: 'assistant',
    content: '',
    isTyping: true,
  })

  // 滚动到底部（watch 会在测量后自动滚动）
  shouldFollowBottom.value = true
  isAtBottom.value = true

  // 开始流式对话
  isTyping.value = true
  isReceiving.value = true

  // 启动渲染循环
  requestAnimationFrame(renderLoop)

  // 创建 Channel 接收流式响应
  const channel = new Channel()
  channel.onmessage = (event) => {
    if (event.type === 'Token') {
      // 收到 token，添加到队列
      pendingQueue.value += event.data
    } else if (event.type === 'Done') {
      // 完成
      isReceiving.value = false
    } else if (event.type === 'Error') {
      // 错误处理
      console.error('对话错误:', event.data)
      pendingQueue.value += `\n[错误: ${event.data}]`
      isReceiving.value = false
    }
  }

  try {
    await invoke('chat', {
      message: userQuery,
      maxTokens: MAX_TOKENS,
      channel,
    })
  } catch (e) {
    console.error('调用失败:', e)
    pendingQueue.value += `\n[错误: ${e}]`
    isReceiving.value = false
  }
}

// ============ 打字机渲染循环（优化版：按批次渲染） ============
const renderLoop = () => {
  if (pendingQueue.value.length > 0) {
    // 根据积压量动态调整速度
    const backlog = pendingQueue.value.length
    let speed = TYPING_SPEED.slow
    if (backlog > 100) speed = TYPING_SPEED.fast
    else if (backlog > 50) speed = TYPING_SPEED.normal

    // 批量消费字符（不再逐字创建 DOM 节点）
    const chunk = pendingQueue.value.slice(0, speed)
    pendingQueue.value = pendingQueue.value.slice(speed)

    const currentMsg = messages.value[messages.value.length - 1]
    currentMsg.content += chunk

    // 跟随滚动
    if (shouldFollowBottom.value) {
      scrollToBottom()
    }
  }

  // 继续循环或结束
  if (pendingQueue.value.length > 0 || isReceiving.value) {
    requestAnimationFrame(renderLoop)
  } else {
    // 打字结束，立即清理状态
    isTyping.value = false
    const currentMsg = messages.value[messages.value.length - 1]
    if (currentMsg) currentMsg.isTyping = false
  }
}

// ============ 滚动控制 ============
const checkScroll = () => {
  if (!parentRef.value) return
  // 如果正在跟随滚动，不更新 isAtBottom 避免按钮闪烁
  if (shouldFollowBottom.value) {
    isAtBottom.value = true
    return
  }
  const { scrollTop, scrollHeight, clientHeight } = parentRef.value
  const distanceFromBottom = scrollHeight - scrollTop - clientHeight
  isAtBottom.value = distanceFromBottom < 30
}

const handleWheel = () => {
  shouldFollowBottom.value = false
}

const scrollToBottom = () => {
  // 使用虚拟滚动的 scrollToIndex 确保正确定位
  const lastIndex = messages.value.length - 1
  if (lastIndex >= 0) {
    virtualizer.value.scrollToIndex(lastIndex, { align: 'end' })
  }
}

const scrollToBottomSmooth = () => {
  const lastIndex = messages.value.length - 1
  if (lastIndex >= 0) {
    virtualizer.value.scrollToIndex(lastIndex, { align: 'end', behavior: 'smooth' })
  }
}

const handleScrollToBottom = () => {
  shouldFollowBottom.value = true
  isAtBottom.value = true
  scrollToBottomSmooth()
}

const handleKeydown = (e) => {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    sendMessage()
  }
}

const clearMessages = async () => {
  // 清空后端会话
  if (isModelLoaded.value) {
    try {
      await invoke('clear_chat')
    } catch (e) {
      console.error('清空会话失败:', e)
    }
  }
  messages.value = [{ role: 'assistant', content: '你好！我是阿强，您的量化交易助手。有什么我可以帮你的吗？' }]
}
</script>

<template>
  <div
    class="flex flex-col h-full bg-surface-0 dark:bg-surface-900 rounded-xl border border-surface-200 dark:border-surface-700 shadow-sm overflow-hidden">
    <!-- 聊天头部 -->
    <div
      class="flex-none flex items-center justify-between px-4 py-3 border-b border-surface-200 dark:border-surface-700 bg-surface-50 dark:bg-surface-900/50">
      <div class="flex items-center gap-2">
        <i class="pi pi-microchip-ai text-primary text-xl"></i>
        <span class="font-medium text-lg">阿强</span>
        <!-- 模型状态指示 -->
        <span
          v-if="isLoadingModel"
          class="text-xs text-surface-500 flex items-center gap-1">
          <i class="pi pi-spin pi-spinner"></i>
          加载模型中...
        </span>
        <span
          v-else-if="loadError"
          class="text-xs text-red-500 cursor-pointer"
          v-tooltip="loadError"
          @click="loadModel">
          <i class="pi pi-exclamation-triangle"></i>
          加载失败，点击重试
        </span>
        <span
          v-else-if="isModelLoaded"
          class="text-xs text-green-500">
          <i class="pi pi-check-circle"></i>
          已就绪
        </span>
      </div>
      <Button
        icon="pi pi-refresh"
        text
        rounded
        severity="secondary"
        v-tooltip="'清空对话'"
        @click="clearMessages" />
    </div>

    <!-- 消息列表（虚拟滚动） -->
    <div class="relative flex-1 overflow-hidden">
      <div
        ref="parentRef"
        class="absolute inset-0 overflow-y-auto"
        @scroll="checkScroll"
        @wheel="handleWheel">
        <!-- 虚拟滚动容器 -->
        <div
          class="relative w-full"
          :style="{ height: `${totalSize}px` }">
          <!-- 内容居中容器 -->
          <div class="max-w-[960px] mx-auto px-4">
            <!-- 虚拟化的消息项 -->
            <div
              v-for="virtualRow in virtualRows"
              :key="virtualRow.key"
              :ref="(el) => virtualizer.measureElement(el)"
              :data-index="virtualRow.index"
              class="absolute left-0 right-0 py-2"
              :style="{ transform: `translateY(${virtualRow.start}px)` }">
              <div class="max-w-[960px] mx-auto px-4">
                <div
                  class="flex w-full"
                  :class="{
                    'justify-end': messages[virtualRow.index]?.role === 'user',
                    'justify-start': messages[virtualRow.index]?.role === 'assistant',
                  }">
                  <!-- 用户消息 -->
                  <div
                    v-if="messages[virtualRow.index]?.role === 'user'"
                    class="max-w-[80%] whitespace-pre-wrap leading-relaxed break-words text-sm p-3 rounded-lg shadow-sm bg-surface-100 dark:bg-surface-700 text-surface-900 dark:text-surface-50">
                    {{ messages[virtualRow.index].content }}
                  </div>
                  <!-- AI 消息 -->
                  <div
                    v-else
                    class="max-w-[90%] whitespace-pre-wrap leading-relaxed break-words text-sm px-1 py-3 text-surface-900 dark:text-surface-50">
                    {{ messages[virtualRow.index]?.content }}
                    <span
                      v-if="messages[virtualRow.index]?.isTyping"
                      class="typing-dots">
                      ...
                    </span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
        <!-- 空状态 -->
        <div
          v-if="messages.length === 0"
          class="absolute inset-0 flex flex-col items-center justify-center text-surface-400">
          <i class="pi pi-comments text-4xl mb-2"></i>
          <p>开始对话吧</p>
        </div>
      </div>

      <!-- 滚动到底部按钮 -->
      <Transition name="scroll-btn">
        <div
          v-if="!isAtBottom"
          class="absolute bottom-4 left-1/2 -translate-x-1/2">
          <div
            class="scroll-btn-wrapper"
            :class="{ 'is-typing': isTyping }">
            <Button
              icon="pi pi-chevron-down"
              rounded
              severity="secondary"
              class="!shadow-lg"
              v-tooltip.top="'滚动到底部'"
              @click="handleScrollToBottom" />
          </div>
        </div>
      </Transition>
    </div>

    <!-- 输入区域 -->
    <div class="flex-none p-4 border-t border-surface-200 dark:border-surface-700 bg-surface-50 dark:bg-surface-900/50">
      <div class="max-w-[960px] mx-auto w-full">
        <div class="relative">
          <Textarea
            v-model="inputContent"
            rows="1"
            autoResize
            :placeholder="isModelLoaded ? '输入消息...' : isLoadingModel ? '模型加载中...' : '模型未加载'"
            :disabled="!isModelLoaded"
            class="w-full pr-12 max-h-32 !bg-surface-0 dark:!bg-surface-800"
            @keydown="handleKeydown" />
          <Button
            icon="pi pi-send"
            rounded
            text
            class="!absolute !right-2 !bottom-2 !w-8 !h-8"
            @click="sendMessage"
            :disabled="!inputContent.trim() || isTyping || !isModelLoaded" />
        </div>
      </div>
    </div>
  </div>
</template>

<style>
/* 打字中的省略号动画 */
.typing-dots {
  display: inline-block;
  animation: dots 1.2s ease-in-out infinite;
  color: var(--p-text-muted-color);
}

@keyframes dots {
  0%,
  20% {
    opacity: 0.3;
  }
  50% {
    opacity: 1;
  }
  80%,
  100% {
    opacity: 0.3;
  }
}

/* 滚动到底部按钮过渡动画 */
.scroll-btn-enter-active,
.scroll-btn-leave-active {
  transition: all 0.2s ease-out;
}

.scroll-btn-enter-from,
.scroll-btn-leave-to {
  opacity: 0;
  transform: translateY(10px);
}

/* 滚动按钮旋转边框动画 */
.scroll-btn-wrapper {
  position: relative;
  border-radius: 50%;
}

.scroll-btn-wrapper::before {
  content: '';
  position: absolute;
  inset: -3px;
  border-radius: 50%;
  padding: 3px;
  background: conic-gradient(from 0deg, transparent 0deg, var(--p-primary-color) 90deg, transparent 90deg);
  -webkit-mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
  mask: linear-gradient(#fff 0 0) content-box, linear-gradient(#fff 0 0);
  -webkit-mask-composite: xor;
  mask-composite: exclude;
  opacity: 0;
  transition: opacity 0.2s ease;
}

.scroll-btn-wrapper.is-typing::before {
  opacity: 1;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}
</style>
