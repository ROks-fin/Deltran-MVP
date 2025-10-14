'use client'

import { useEffect, useRef } from 'react'
import { useMotionValue, useSpring } from 'framer-motion'

export function useAnimatedValue(value: number, duration: number = 1000) {
  const motionValue = useMotionValue(0)
  const springValue = useSpring(motionValue, {
    stiffness: 100,
    damping: 30,
    restDelta: 0.001
  })

  useEffect(() => {
    motionValue.set(value)
  }, [motionValue, value])

  return springValue
}
