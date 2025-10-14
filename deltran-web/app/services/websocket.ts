type MessageHandler = (data: any) => void

export interface WebSocketMessage {
  type: string
  payload: any
  timestamp?: string
}

export class WebSocketService {
  private ws: WebSocket | null = null
  private reconnectAttempts = 0
  private maxReconnectAttempts = 5
  private reconnectDelay = 1000 // Start with 1 second
  private messageHandlers: Map<string, MessageHandler[]> = new Map()
  private url: string = ''
  private token?: string
  private reconnectTimeout?: NodeJS.Timeout
  private heartbeatInterval?: NodeJS.Timeout
  private isIntentionallyClosed = false

  connect(url: string, token?: string) {
    this.url = url
    this.token = token
    this.isIntentionallyClosed = false

    try {
      // Add token to URL if provided
      const wsUrl = token ? `${url}?token=${token}` : url

      this.ws = new WebSocket(wsUrl)

      this.ws.onopen = () => {
        console.log('WebSocket connected')
        this.reconnectAttempts = 0
        this.reconnectDelay = 1000
        this.startHeartbeat()

        // Emit connection event
        this.emit('connection', { status: 'connected' })
      }

      this.ws.onmessage = (event: MessageEvent) => {
        this.handleMessage(event)
      }

      this.ws.onerror = (error) => {
        console.error('WebSocket error:', error)
        this.emit('error', { error })
      }

      this.ws.onclose = () => {
        console.log('WebSocket disconnected')
        this.stopHeartbeat()
        this.emit('connection', { status: 'disconnected' })

        // Try to reconnect if not intentionally closed
        if (!this.isIntentionallyClosed) {
          this.reconnect()
        }
      }
    } catch (error) {
      console.error('Failed to connect WebSocket:', error)
      this.reconnect()
    }
  }

  disconnect() {
    this.isIntentionallyClosed = true
    this.stopHeartbeat()

    if (this.reconnectTimeout) {
      clearTimeout(this.reconnectTimeout)
    }

    if (this.ws) {
      this.ws.close()
      this.ws = null
    }

    this.emit('connection', { status: 'disconnected' })
  }

  subscribe(messageType: string, handler: MessageHandler) {
    if (!this.messageHandlers.has(messageType)) {
      this.messageHandlers.set(messageType, [])
    }

    const handlers = this.messageHandlers.get(messageType)!
    handlers.push(handler)
  }

  unsubscribe(messageType: string, handler: MessageHandler) {
    const handlers = this.messageHandlers.get(messageType)
    if (handlers) {
      const index = handlers.indexOf(handler)
      if (index > -1) {
        handlers.splice(index, 1)
      }
    }
  }

  send(message: WebSocketMessage) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message))
    } else {
      console.warn('WebSocket is not connected')
    }
  }

  private handleMessage(event: MessageEvent) {
    try {
      const message: WebSocketMessage = JSON.parse(event.data)

      // Handle pong responses
      if (message.type === 'pong') {
        return
      }

      // Emit to registered handlers
      this.emit(message.type, message.payload)
    } catch (error) {
      console.error('Failed to parse WebSocket message:', error)
    }
  }

  private emit(messageType: string, data: any) {
    const handlers = this.messageHandlers.get(messageType)
    if (handlers) {
      handlers.forEach((handler) => handler(data))
    }
  }

  private reconnect() {
    if (this.isIntentionallyClosed) return

    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.error('Max reconnection attempts reached')
      this.emit('connection', { status: 'failed' })
      return
    }

    this.reconnectAttempts++
    const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1) // Exponential backoff

    console.log(`Reconnecting in ${delay}ms... (attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts})`)

    this.emit('connection', { status: 'reconnecting', attempts: this.reconnectAttempts })

    this.reconnectTimeout = setTimeout(() => {
      this.connect(this.url, this.token)
    }, delay)
  }

  private startHeartbeat() {
    this.heartbeatInterval = setInterval(() => {
      this.send({ type: 'ping', payload: {} })
    }, 30000) // Send ping every 30 seconds
  }

  private stopHeartbeat() {
    if (this.heartbeatInterval) {
      clearInterval(this.heartbeatInterval)
      this.heartbeatInterval = undefined
    }
  }

  getConnectionState(): 'connecting' | 'connected' | 'disconnected' | 'error' {
    if (!this.ws) return 'disconnected'

    switch (this.ws.readyState) {
      case WebSocket.CONNECTING:
        return 'connecting'
      case WebSocket.OPEN:
        return 'connected'
      case WebSocket.CLOSING:
      case WebSocket.CLOSED:
        return 'disconnected'
      default:
        return 'error'
    }
  }
}

// Singleton instance
export const websocketService = new WebSocketService()
