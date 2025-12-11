<script setup>
import { ref, computed, nextTick, watch, onMounted } from 'vue'
import { useVirtualizer } from '@tanstack/vue-virtual'
import { invoke, Channel } from '@tauri-apps/api/core'
import { marked } from 'marked'
import { markedHighlight } from 'marked-highlight'
import hljs from 'highlight.js'

// ============ Markdown 配置 ============
marked.use(
  markedHighlight({
    highlight: (code, lang) => {
      if (lang && hljs.getLanguage(lang)) {
        return hljs.highlight(code, { language: lang }).value
      }
      return code
    },
  }),
  { breaks: true, gfm: true }
)

const renderMarkdown = (content) => {
  if (!content) return ''
  return marked.parse(content)
}

// 进入页面时清空上下文
onMounted(async () => {
  try {
    await invoke('clear_chat')
  } catch (e) {
    console.error('重置智能体上下文失败:', e)
  } finally {
    isReady.value = true
    // 就绪后输入框获得焦点
    nextTick(() => {
      const el = inputRef.value?.$el
      if (el) {
        const textarea = el.tagName === 'TEXTAREA' ? el : el.querySelector('textarea')
        textarea?.focus()
      }
    })
  }
})

// ============ 常量配置 ============
const MAX_MESSAGES = 200
const TYPING_SPEED = { slow: 2, normal: 4, fast: 8 }

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

// 滚动位置锁定（非跟随模式时保持位置稳定）
const lockedScrollTop = ref(null)
const isUserScrolling = ref(false)
let scrollLockTimer = null

// 滚动节流（避免频繁滚动导致抖动）
let lastScrollTime = 0
const SCROLL_THROTTLE = 50 // ms

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

// 监听当前消息内容变化（用于锁定滚动位置）
const currentMsgContent = computed(() => {
  const last = messages.value[messages.value.length - 1]
  return last ? (last.content?.length || 0) + (last.thinkingContent?.length || 0) : 0
})

watch(currentMsgContent, () => {
  // 非跟随模式且用户未在主动滚动时，保持滚动位置稳定
  if (!shouldFollowBottom.value && !isUserScrolling.value && parentRef.value && lockedScrollTop.value !== null) {
    // 使用 requestAnimationFrame 确保在 DOM 更新后恢复位置
    requestAnimationFrame(() => {
      if (parentRef.value && lockedScrollTop.value !== null && !isUserScrolling.value) {
        parentRef.value.scrollTop = lockedScrollTop.value
      }
    })
  }
})

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
    content: '', // 正式回复内容
    thinkingContent: '', // 思考内容
    isTyping: true,
    isThinking: false, // 是否正在思考
    isThinkingComplete: false, // 思考是否已结束
    thinkingCollapsed: false,
    thinkingAutoCollapsed: false,
  })

  // 滚动到底部（watch 会在测量后自动滚动）
  shouldFollowBottom.value = true
  isAtBottom.value = true
  // 立即平滑滚到底，避免发送后不跟随
  scrollToBottom(true)

  // 开始流式对话
  isTyping.value = true
  isReceiving.value = true

  // 启动渲染循环
  requestAnimationFrame(renderLoop)

  // 当前消息引用
  const currentMsg = messages.value[messages.value.length - 1]

  // 创建 Channel 接收流式响应
  const channel = new Channel()
  channel.onmessage = (event) => {
    if (event.type === 'ThinkBegin') {
      // 开始思考
      currentMsg.isThinking = true
    } else if (event.type === 'ThinkEnd') {
      // 思考结束
      currentMsg.isThinking = false
      currentMsg.isThinkingComplete = true
    } else if (event.type === 'Token') {
      // 收到 token
      if (currentMsg.isThinking) {
        // 思考中的内容
        currentMsg.thinkingContent += event.data
        // 若保持底部，思考过程也跟随到底（节流避免抖动）
        if (shouldFollowBottom.value) {
          const now = Date.now()
          if (now - lastScrollTime > SCROLL_THROTTLE) {
            lastScrollTime = now
            scrollToBottom(false)
          }
        }
      } else {
        // 正式回复内容，添加到队列
        pendingQueue.value += event.data
      }
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

    // 跟随滚动（使用 instant 避免动画堆积）
    if (shouldFollowBottom.value) {
      scrollToBottom(false)
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
    // 打字结束后平滑滚动到底部
    if (shouldFollowBottom.value) {
      scrollToBottom(true)
    }
  }
}

// ============ 滚动控制 ============
const checkScroll = () => {
  if (!parentRef.value) return
  const { scrollTop, scrollHeight, clientHeight } = parentRef.value
  const distanceFromBottom = scrollHeight - scrollTop - clientHeight
  isAtBottom.value = distanceFromBottom < 30

  // 更新锁定位置（仅在非跟随模式且用户未主动滚动时）
  if (!shouldFollowBottom.value && !isUserScrolling.value) {
    lockedScrollTop.value = scrollTop
  }
}

const handleWheel = (e) => {
  // 检测滚动方向
  const isScrollingUp = e.deltaY < 0

  // 向上滚动时脱离跟随模式
  if (isScrollingUp) {
    // 立即锁定当前滚动位置
    if (parentRef.value) {
      lockedScrollTop.value = parentRef.value.scrollTop
    }
    shouldFollowBottom.value = false
    isAtBottom.value = false
  }

  // 标记用户正在滚动
  isUserScrolling.value = true

  // 延迟更新锁定位置，等滚动稳定后
  clearTimeout(scrollLockTimer)
  scrollLockTimer = setTimeout(() => {
    if (parentRef.value && !shouldFollowBottom.value) {
      lockedScrollTop.value = parentRef.value.scrollTop
    }
    isUserScrolling.value = false
  }, 150)
}

const scrollToBottom = (smooth = false) => {
  // 清除锁定位置
  lockedScrollTop.value = null

  // 使用原生 scrollTop 滚动，避免虚拟滚动 scrollToIndex 在高度变化时抖动
  if (parentRef.value) {
    const target = parentRef.value.scrollHeight - parentRef.value.clientHeight
    if (smooth) {
      parentRef.value.scrollTo({ top: target, behavior: 'smooth' })
    } else {
      parentRef.value.scrollTop = target
    }
  }
}

const scrollToBottomSmooth = () => {
  if (parentRef.value) {
    const target = parentRef.value.scrollHeight - parentRef.value.clientHeight
    parentRef.value.scrollTo({ top: target, behavior: 'smooth' })
  }
}

const handleScrollToBottom = () => {
  shouldFollowBottom.value = true
  isAtBottom.value = true
  lockedScrollTop.value = null
  isUserScrolling.value = false
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
  messages.value = []
  try {
    await invoke('clear_chat')
  } catch (e) {
    console.error('清空上下文失败:', e)
  }
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
        class="absolute inset-0 overflow-y-auto scroll-container"
        @scroll="checkScroll"
        @wheel.passive="handleWheel">
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
                    <template v-if="messages[virtualRow.index]?.thinkingContent">
                      <!-- 思考过程标题 -->
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
                          v-if="messages[virtualRow.index]?.isThinking"
                          class="text-primary/80">
                          （思考中）
                        </span>
                      </div>
                      <!-- 展开的内容 -->
                      <div
                        v-show="!messages[virtualRow.index]?.thinkingCollapsed"
                        class="thinking-content text-surface-400 dark:text-surface-500 text-xs pl-4 mb-3 border-l-2 border-surface-200 dark:border-surface-700"
                        :class="{ 'thinking-content-active': messages[virtualRow.index]?.isThinking }">
                        {{ messages[virtualRow.index]?.thinkingContent }}
                      </div>
                    </template>
                    <!-- 正式回复 (Markdown) -->
                    <div
                      v-if="messages[virtualRow.index]?.content"
                      class="markdown-content"
                      v-html="renderMarkdown(messages[virtualRow.index]?.content)"></div>
                    <!-- 打字中省略号（无内容且非思考中时显示） -->
                    <span
                      v-if="
                        messages[virtualRow.index]?.isTyping &&
                        !messages[virtualRow.index]?.content &&
                        !messages[virtualRow.index]?.isThinking
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
/* 滚动容器样式 - 禁用浏览器默认锚定，手动控制 */
.scroll-container {
  overflow-anchor: none;
}

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
  line-height: 1.5;
}

/* Markdown 内容样式 */
.markdown-content {
  white-space: normal; /* 覆盖外层 whitespace-pre-wrap，避免多余空白 */
  line-height: 1.6;
}

.markdown-content p {
  margin: 0.3em 0;
}

.markdown-content strong {
  font-weight: 600;
}

.markdown-content h1,
.markdown-content h2,
.markdown-content h3,
.markdown-content h4 {
  font-weight: 600;
  margin: 0.8em 0 0.3em;
}

.markdown-content h1 {
  font-size: 1.25em;
}
.markdown-content h2 {
  font-size: 1.15em;
}
.markdown-content h3 {
  font-size: 1.05em;
}
.markdown-content h4 {
  font-size: 1em;
}

.markdown-content ul,
.markdown-content ol {
  display: block;
  margin: 0.2em 0;
  padding: 0 0 0 1.5em;
}

.markdown-content ul {
  list-style-type: disc;
}
.markdown-content ol {
  list-style-type: decimal;
}

.markdown-content li {
  display: list-item;
  margin: 0;
  padding: 0;
  line-height: 1.5;
}

.markdown-content li + li {
  margin-top: 0.1em;
}

.markdown-content li > p,
.markdown-content li > p:first-child,
.markdown-content li > p:last-child {
  margin: 0;
  padding: 0;
}

.markdown-content li > ul,
.markdown-content li > ol {
  margin: 0.15em 0 0 0;
}

.markdown-content code {
  background: var(--p-surface-100);
  color: #c7254e;
  padding: 0.15em 0.4em;
  border-radius: 4px;
  font-size: 0.875em;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

.app-dark .markdown-content code {
  background: var(--p-surface-700);
  color: #f8b4b4;
}

.markdown-content pre {
  background: var(--p-surface-100);
  padding: 0.75em;
  border-radius: 6px;
  overflow-x: auto;
  margin: 0.5em 0;
  line-height: 1.4;
  -ms-overflow-style: none;
  scrollbar-width: none;
}

.markdown-content pre::-webkit-scrollbar {
  display: none;
}

.app-dark .markdown-content pre {
  background: var(--p-surface-800);
}

.markdown-content pre code {
  background: transparent;
  padding: 0;
  font-size: 0.8125em;
}

.markdown-content blockquote {
  border-left: 2px solid var(--p-primary-color);
  padding-left: 0.75em;
  margin: 0.5em 0;
  color: var(--p-text-muted-color);
}

.markdown-content table {
  border-collapse: collapse;
  margin: 0.5em 0;
}

.markdown-content th,
.markdown-content td {
  border: 1px solid var(--p-surface-200);
  padding: 0.35em 0.5em;
}

.app-dark .markdown-content th,
.app-dark .markdown-content td {
  border-color: var(--p-surface-700);
}

.markdown-content th {
  background: var(--p-surface-50);
  font-weight: 600;
}

.app-dark .markdown-content th {
  background: var(--p-surface-800);
}

.markdown-content a {
  color: var(--p-primary-color);
}

.markdown-content hr {
  border: none;
  border-top: 1px solid var(--p-surface-200);
  margin: 0.75em 0;
}

.app-dark .markdown-content hr {
  border-color: var(--p-surface-700);
}

/* Highlight.js 代码高亮 - 浅色主题 */
.hljs-comment,
.hljs-quote {
  color: #6a737d;
}
.hljs-keyword,
.hljs-selector-tag {
  color: #d73a49;
}
.hljs-string,
.hljs-attr {
  color: #032f62;
}
.hljs-number,
.hljs-literal {
  color: #005cc5;
}
.hljs-variable,
.hljs-template-variable {
  color: #e36209;
}
.hljs-tag {
  color: #22863a;
}
.hljs-name,
.hljs-selector-id,
.hljs-selector-class {
  color: #6f42c1;
}
.hljs-function {
  color: #6f42c1;
}
.hljs-built_in {
  color: #005cc5;
}
.hljs-type,
.hljs-class {
  color: #6f42c1;
}
.hljs-title {
  color: #6f42c1;
}
.hljs-params {
  color: #24292e;
}
.hljs-regexp {
  color: #032f62;
}
.hljs-symbol {
  color: #005cc5;
}
.hljs-meta {
  color: #6a737d;
}
.hljs-deletion {
  color: #cb2431;
  background: #ffeef0;
}
.hljs-addition {
  color: #22863a;
  background: #e6ffed;
}

/* Highlight.js 代码高亮 - 暗色主题 */
.app-dark .hljs-comment,
.app-dark .hljs-quote {
  color: #8b949e;
}
.app-dark .hljs-keyword,
.app-dark .hljs-selector-tag {
  color: #ff7b72;
}
.app-dark .hljs-string,
.app-dark .hljs-attr {
  color: #a5d6ff;
}
.app-dark .hljs-number,
.app-dark .hljs-literal {
  color: #79c0ff;
}
.app-dark .hljs-variable,
.app-dark .hljs-template-variable {
  color: #ffa657;
}
.app-dark .hljs-tag {
  color: #7ee787;
}
.app-dark .hljs-name,
.app-dark .hljs-selector-id,
.app-dark .hljs-selector-class {
  color: #d2a8ff;
}
.app-dark .hljs-function {
  color: #d2a8ff;
}
.app-dark .hljs-built_in {
  color: #79c0ff;
}
.app-dark .hljs-type,
.app-dark .hljs-class {
  color: #d2a8ff;
}
.app-dark .hljs-title {
  color: #d2a8ff;
}
.app-dark .hljs-params {
  color: #c9d1d9;
}
.app-dark .hljs-regexp {
  color: #a5d6ff;
}
.app-dark .hljs-symbol {
  color: #79c0ff;
}
.app-dark .hljs-meta {
  color: #8b949e;
}
.app-dark .hljs-deletion {
  color: #ffa198;
  background: rgba(248, 81, 73, 0.1);
}
.app-dark .hljs-addition {
  color: #7ee787;
  background: rgba(46, 160, 67, 0.15);
}
</style>
