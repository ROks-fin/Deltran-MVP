'use client'

import { motion } from 'framer-motion'
import { Wifi, WifiOff, Loader2, AlertTriangle } from 'lucide-react'
import { ConnectionStatus } from '@/app/hooks/useWebSocket'

interface ConnectionIndicatorProps {
  status: ConnectionStatus
  reconnectAttempts?: number
}

export function ConnectionIndicator({ status, reconnectAttempts = 0 }: ConnectionIndicatorProps) {
  const getStatusConfig = () => {
    switch (status) {
      case 'connected':
        return {
          icon: Wifi,
          color: 'green',
          bgColor: 'bg-green-400',
          textColor: 'text-green-400',
          label: 'Live',
          tooltip: 'Connected to live updates',
          animate: true,
        }
      case 'connecting':
        return {
          icon: Loader2,
          color: 'yellow',
          bgColor: 'bg-yellow-400',
          textColor: 'text-yellow-400',
          label: 'Connecting',
          tooltip: 'Connecting to server...',
          animate: true,
          spin: true,
        }
      case 'reconnecting':
        return {
          icon: Loader2,
          color: 'yellow',
          bgColor: 'bg-yellow-400',
          textColor: 'text-yellow-400',
          label: `Reconnecting (${reconnectAttempts}/5)`,
          tooltip: `Reconnection attempt ${reconnectAttempts} of 5`,
          animate: true,
          spin: true,
        }
      case 'disconnected':
        return {
          icon: WifiOff,
          color: 'red',
          bgColor: 'bg-red-400',
          textColor: 'text-red-400',
          label: 'Disconnected',
          tooltip: 'Connection lost. Trying to reconnect...',
          animate: false,
        }
      case 'failed':
        return {
          icon: AlertTriangle,
          color: 'red',
          bgColor: 'bg-red-400',
          textColor: 'text-red-400',
          label: 'Connection Failed',
          tooltip: 'Failed to connect. Please refresh the page.',
          animate: false,
        }
      default:
        return {
          icon: WifiOff,
          color: 'zinc',
          bgColor: 'bg-zinc-400',
          textColor: 'text-zinc-400',
          label: 'Offline',
          tooltip: 'Not connected',
          animate: false,
        }
    }
  }

  const config = getStatusConfig()
  const Icon = config.icon

  return (
    <div className="flex items-center gap-2 group relative">
      {/* Indicator Dot */}
      {config.animate ? (
        <motion.div
          animate={{
            scale: [1, 1.2, 1],
            opacity: [0.5, 1, 0.5],
          }}
          transition={{
            duration: 2,
            repeat: Infinity,
            ease: 'easeInOut',
          }}
          className={`w-2 h-2 rounded-full ${config.bgColor}`}
        />
      ) : (
        <div className={`w-2 h-2 rounded-full ${config.bgColor}`} />
      )}

      {/* Icon */}
      <Icon
        className={`w-4 h-4 ${config.textColor} ${config.spin ? 'animate-spin' : ''}`}
      />

      {/* Label */}
      <span className={`text-sm ${config.textColor}`}>{config.label}</span>

      {/* Tooltip */}
      <div className="absolute bottom-full right-0 mb-2 hidden group-hover:block z-50">
        <div className="bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 shadow-xl whitespace-nowrap">
          <p className="text-xs text-zinc-300">{config.tooltip}</p>
        </div>
      </div>
    </div>
  )
}
