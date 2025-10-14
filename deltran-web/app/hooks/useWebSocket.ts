'use client'

import { useEffect, useState, useCallback } from 'react'
import { websocketService } from '../services/websocket'

export type ConnectionStatus = 'connecting' | 'connected' | 'disconnected' | 'reconnecting' | 'failed'

export function useWebSocket(url?: string, token?: string) {
  const [status, setStatus] = useState<ConnectionStatus>('disconnected')
  const [reconnectAttempts, setReconnectAttempts] = useState(0)

  useEffect(() => {
    if (!url) return

    // Connection status handler
    const handleConnection = (data: any) => {
      setStatus(data.status)
      if (data.attempts !== undefined) {
        setReconnectAttempts(data.attempts)
      }
    }

    // Subscribe to connection events
    websocketService.subscribe('connection', handleConnection)

    // Connect
    websocketService.connect(url, token)

    // Cleanup on unmount
    return () => {
      websocketService.unsubscribe('connection', handleConnection)
      websocketService.disconnect()
    }
  }, [url, token])

  const subscribe = useCallback((messageType: string, handler: (data: any) => void) => {
    websocketService.subscribe(messageType, handler)

    return () => {
      websocketService.unsubscribe(messageType, handler)
    }
  }, [])

  const send = useCallback((type: string, payload: any) => {
    websocketService.send({ type, payload })
  }, [])

  return {
    status,
    reconnectAttempts,
    subscribe,
    send,
    isConnected: status === 'connected',
  }
}
