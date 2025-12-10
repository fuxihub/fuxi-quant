<script setup>
import { ref, nextTick } from 'vue'

const messages = ref([{ role: 'assistant', content: '你好！我是阿强，您的量化交易助手。有什么我可以帮你的吗？' }])
const inputContent = ref('')
const messagesContainer = ref(null)
const isTyping = ref(false)
const pendingQueue = ref('')
const isReceiving = ref(false)
const isAtBottom = ref(true)
const shouldFollowBottom = ref(true) // 是否应该跟随滚动到底部（用户主动触发时为 true）

const sendMessage = async () => {
  if (!inputContent.value.trim() || isTyping.value) return

  // 1. 性能优化：在开始新一轮对话前，清理历史消息的 chunks，释放 DOM 压力
  // 历史消息将回退到 v-html 渲染，不再持有大量 span 节点
  messages.value.forEach((msg) => {
    if (msg.chunks) msg.chunks = null
  })

  // 添加用户消息
  messages.value.push({
    role: 'user',
    content: inputContent.value,
  })

  const userQuery = inputContent.value
  inputContent.value = ''

  // 添加空的 AI 消息
  const aiMsg = {
    role: 'assistant',
    content: '',
    chunks: [],
  }
  messages.value.push(aiMsg)

  // 发送消息时，强制开启跟随模式并滚动到底部
  shouldFollowBottom.value = true
  isAtBottom.value = true
  await nextTick()
  scrollToBottomInstant()

  // 模拟 AI 回复
  isTyping.value = true
  isReceiving.value = true
  const responseText = `收到你的消息: "${userQuery}"。\n目前我还在开发中，暂时无法处理复杂的量化指令。但为了演示快速渲染效果，这里有一段较长的文本：\n\n量化交易是指以先进的数学模型替代人为的主观判断，利用计算机技术从庞大的历史数据中海选能带来超额收益的多种“大概率”事件以制定策略，极大地减少了投资者情绪波动的影响，避免在市场极度狂热或悲观的情况下作出非理性的投资决策。`

  // 启动渲染循环
  requestAnimationFrame(renderLoop)

  // 模拟网络请求返回数据
  // 这里模拟一次性返回大量数据，或者分段返回
  setTimeout(() => {
    // 模拟数据到达，推入缓冲区
    pendingQueue.value += responseText

    // 模拟数据传输结束
    isReceiving.value = false
  }, 500)
}

const renderLoop = async () => {
  if (pendingQueue.value.length > 0) {
    // 阶梯式线性速度策略，保证节奏平滑
    const backlog = pendingQueue.value.length
    let consumeCount = 1

    if (backlog > 100) consumeCount = 5
    else if (backlog > 50) consumeCount = 3
    else if (backlog > 20) consumeCount = 2

    const chunk = pendingQueue.value.slice(0, consumeCount)
    pendingQueue.value = pendingQueue.value.slice(consumeCount)

    const currentMsg = messages.value[messages.value.length - 1]
    const baseIndex = currentMsg.chunks.length
    const newChunks = []

    // 使用 index 遍历，以便生成稳定的 key
    for (let i = 0; i < chunk.length; i++) {
      const char = chunk[i]
      newChunks.push({
        type: char === '\n' ? 'br' : 'text',
        value: char,
        key: `${baseIndex + i}`,
      })
      currentMsg.content += char
    }

    // 批量更新 chunks，减少响应式触发频率
    if (newChunks.length > 0) {
      currentMsg.chunks.push(...newChunks)
    }

    // 只有在跟随模式下才自动滚动（使用 instant 避免卡顿）
    if (shouldFollowBottom.value) {
      scrollToBottomInstant()
    }
  }

  // 如果缓冲区还有数据，或者还在接收数据中，继续循环
  if (pendingQueue.value.length > 0 || isReceiving.value) {
    requestAnimationFrame(renderLoop)
  } else {
    isTyping.value = false
  }
}

const checkScroll = () => {
  if (!messagesContainer.value) return
  const { scrollTop, scrollHeight, clientHeight } = messagesContainer.value
  const distanceFromBottom = scrollHeight - scrollTop - clientHeight
  // 阈值设为 30px
  isAtBottom.value = distanceFromBottom < 30
}

// 用户主动滚动（wheel 事件只有用户操作才触发，程序设置 scrollTop 不会触发）
const handleWheel = () => {
  // 用户滚动时立即关闭跟随模式
  shouldFollowBottom.value = false
}

// 立即滚动（用于 renderLoop，无动画避免卡顿）
const scrollToBottomInstant = () => {
  if (messagesContainer.value) {
    messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight
  }
}

// 平滑滚动（用于用户点击按钮）
const scrollToBottomSmooth = async () => {
  await nextTick()
  if (messagesContainer.value) {
    messagesContainer.value.scrollTo({
      top: messagesContainer.value.scrollHeight,
      behavior: 'smooth',
    })
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
        @click="messages = []" />
    </div>

    <!-- 消息列表 -->
    <div class="relative flex-1 overflow-hidden">
      <div
        ref="messagesContainer"
        class="absolute inset-0 overflow-y-auto p-4"
        @scroll="checkScroll"
        @wheel="handleWheel">
        <div class="max-w-[960px] mx-auto w-full flex flex-col gap-4">
          <div
            v-for="(msg, index) in messages"
            :key="index"
            class="flex w-full"
            :class="{ 'justify-end': msg.role === 'user', 'justify-start': msg.role === 'assistant' }">
            <!-- 消息气泡 -->
            <div
              v-if="msg.role === 'user'"
              class="whitespace-pre-wrap leading-relaxed break-words text-sm p-3 rounded-lg shadow-sm bg-surface-100 dark:bg-surface-700 text-surface-900 dark:text-surface-50">
              {{ msg.content }}
            </div>
            <div
              v-else
              class="whitespace-pre-wrap leading-relaxed break-words text-sm px-1 py-3 text-surface-900 dark:text-surface-50">
              <template v-if="msg.chunks">
                <template
                  v-for="chunk in msg.chunks"
                  :key="chunk.key">
                  <br v-if="chunk.type === 'br'" />
                  <span
                    v-else
                    class="typing-char">
                    {{ chunk.value }}
                  </span>
                </template>
              </template>
              <span
                v-else
                v-html="msg.content"></span>
            </div>
          </div>

          <div
            v-if="messages.length === 0"
            class="flex-1 flex flex-col items-center justify-center text-surface-400">
            <i class="pi pi-comments text-4xl mb-2"></i>
            <p>开始对话吧</p>
          </div>
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
            placeholder="输入消息..."
            class="w-full pr-12 max-h-32 !bg-surface-0 dark:!bg-surface-800"
            @keydown="handleKeydown" />
          <Button
            icon="pi pi-send"
            rounded
            text
            class="!absolute !right-2 !bottom-2 !w-8 !h-8"
            @click="sendMessage"
            :disabled="!inputContent.trim() || isTyping" />
        </div>
      </div>
    </div>
  </div>
</template>

<style>
.typing-char {
  animation: fade-in 0.25s ease-out forwards;
  opacity: 0;
  display: inline-block;
}

@keyframes fade-in {
  from {
    opacity: 0;
    transform: translateY(4px);
    filter: blur(2px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
    filter: blur(0);
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
