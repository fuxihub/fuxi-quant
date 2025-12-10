<script setup>
import { ref, computed, nextTick, watch, onMounted } from 'vue'
import { useVirtualizer } from '@tanstack/vue-virtual'
import { invoke, Channel } from '@tauri-apps/api/core'

// 进入页面时重置会话，确保前后端同步
onMounted(async () => {
  try {
    await invoke('reset_chat')
  } catch (e) {
    console.error('重置会话失败:', e)
  } finally {
    isReady.value = true
    // 就绪后输入框获得焦点
    nextTick(() => {
      const el = inputRef.value?.$el
      if (el) {
        // PrimeVue Textarea 可能直接是 textarea 或包含 textarea
        const textarea = el.tagName === 'TEXTAREA' ? el : el.querySelector('textarea')
        textarea?.focus()
      }
    })
  }
})

// ============ 常量配置 ============
const MAX_MESSAGES = 200
const TYPING_SPEED = { slow: 2, normal: 4, fast: 8 }

// ============ 解析 Thinking 内容 ============
const parseThinking = (content) => {
  if (!content) return { thinking: null, response: '', isThinkingComplete: false }

  const thinkStart = content.indexOf('<think>')
  if (thinkStart === -1) {
    return { thinking: null, response: content, isThinkingComplete: false }
  }

  const thinkEnd = content.indexOf('</think>')
  if (thinkEnd === -1) {
    // thinking 还未结束
    const thinkContent = content.slice(thinkStart + 7)
    return { thinking: thinkContent, response: '', isThinkingComplete: false }
  }

  // thinking 已结束
  const thinkContent = content.slice(thinkStart + 7, thinkEnd)
  const response = content.slice(thinkEnd + 8).trim()
  return { thinking: thinkContent, response, isThinkingComplete: true }
}

// ============ 状态 ============
const messages = ref([])
const inputContent = ref('')
const parentRef = ref(null)
const isTyping = ref(false)
const pendingQueue = ref('')
const isReceiving = ref(false)
const isAtBottom = ref(true)
const shouldFollowBottom = ref(true)
const isReady = ref(false)
const inputRef = ref(null)

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
    thinkingCollapsed: false, // thinking 折叠状态
    thinkingAutoCollapsed: false, // 是否已自动折叠过
    thinkingFollowBottom: true, // thinking 内容跟随底部
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
    await invoke('chat', { message: userQuery, channel })
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

    // 检测 thinking 结束，只自动折叠一次
    if (!currentMsg.thinkingAutoCollapsed && currentMsg.content.includes('</think>')) {
      currentMsg.thinkingCollapsed = true
      currentMsg.thinkingAutoCollapsed = true // 标记已自动折叠，不再重复
    }

    // thinking 内容跟随滚动
    if (currentMsg.thinkingFollowBottom && !currentMsg.thinkingCollapsed) {
      nextTick(() => {
        const thinkingEl = document.querySelector('.thinking-content-active')
        if (thinkingEl) {
          thinkingEl.scrollTop = thinkingEl.scrollHeight
        }
      })
    }

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
    if (currentMsg) {
      currentMsg.isTyping = false
    }
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
  // IME 输入时 keyCode 是 229，真正按 Enter 是 13
  // 同时检查 e.key 和 e.keyCode 来区分 IME 确认和真正的发送
  if (e.key === 'Enter' && e.keyCode === 13 && !e.shiftKey) {
    e.preventDefault()
    sendMessage()
  }
}

const clearMessages = async () => {
  try {
    await invoke('reset_chat')
  } catch (e) {
    console.error('重置会话失败:', e)
  }
  messages.value = []
}

// 切换 thinking 折叠状态
const toggleThinking = (index) => {
  const msg = messages.value[index]
  if (msg) {
    msg.thinkingCollapsed = !msg.thinkingCollapsed
  }
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
                    <!-- Thinking 内容 -->
                    <template v-if="parseThinking(messages[virtualRow.index]?.content).thinking !== null">
                      <!-- 思考过程标题（始终显示，用 v-show 切换图标和内容） -->
                      <div
                        class="flex items-center gap-1 text-surface-400 text-xs mb-1 cursor-pointer hover:text-surface-600 dark:hover:text-surface-300 select-none"
                        @click.stop.prevent="toggleThinking(virtualRow.index)">
                        <i
                          class="pi text-xs"
                          :class="
                            messages[virtualRow.index]?.thinkingCollapsed ? 'pi-chevron-right' : 'pi-chevron-down'
                          "></i>
                        <span>思考过程</span>
                        <span
                          v-if="!parseThinking(messages[virtualRow.index]?.content).isThinkingComplete"
                          class="typing-dots">
                          ...
                        </span>
                      </div>
                      <!-- 展开的内容（用 v-show 保持 DOM 不销毁） -->
                      <div
                        v-show="!messages[virtualRow.index]?.thinkingCollapsed"
                        class="thinking-content text-surface-400 dark:text-surface-500 text-xs pl-4 mb-3 border-l-2 border-surface-200 dark:border-surface-700"
                        :class="{ 'thinking-content-active': messages[virtualRow.index]?.isTyping }"
                        @wheel.stop="messages[virtualRow.index].thinkingFollowBottom = false">
                        {{ parseThinking(messages[virtualRow.index]?.content).thinking }}
                      </div>
                      <!-- 正式回复 -->
                      <div v-if="parseThinking(messages[virtualRow.index]?.content).response">
                        {{ parseThinking(messages[virtualRow.index]?.content).response }}
                      </div>
                    </template>
                    <!-- 无 thinking 的普通消息 -->
                    <template v-else>
                      {{ messages[virtualRow.index]?.content }}
                    </template>
                    <span
                      v-if="
                        messages[virtualRow.index]?.isTyping &&
                        !parseThinking(messages[virtualRow.index]?.content).thinking
                      "
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
          <p>我是阿强，您的量化助手。</p>
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
            ref="inputRef"
            v-model="inputContent"
            rows="1"
            autoResize
            :placeholder="isReady ? '输入消息...' : '初始化中...'"
            :disabled="!isReady"
            class="w-full pr-12 max-h-32 !bg-surface-0 dark:!bg-surface-800"
            @keydown="handleKeydown" />
          <Button
            icon="pi pi-send"
            rounded
            text
            class="!absolute !right-2 !bottom-2 !w-8 !h-8"
            @click="sendMessage"
            :disabled="!isReady || !inputContent.trim() || isTyping" />
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

/* Thinking 内容样式 */
.thinking-content {
  max-height: 15em; /* 约 10 行 */
  line-height: 1.5;
  overflow-y: auto;
  padding-right: 0.75rem; /* 右边距，避免贴着滚动条 */
}
</style>
