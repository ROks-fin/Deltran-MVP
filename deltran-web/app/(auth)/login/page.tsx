'use client'

import { useState } from 'react'
import { useRouter } from 'next/navigation'
import { motion, AnimatePresence } from 'framer-motion'
import { Wallet, Mail, Lock, Loader2, AlertCircle, Shield } from 'lucide-react'
import { useAuth } from '../../hooks/useAuth'

export default function LoginPage() {
  const router = useRouter()
  const { login, loading, error } = useAuth()

  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [otp, setOtp] = useState('')
  const [showOtp, setShowOtp] = useState(false)
  const [rememberMe, setRememberMe] = useState(false)
  const [validationError, setValidationError] = useState('')

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setValidationError('')

    // Validation
    if (!email || !password) {
      setValidationError('Email and password are required')
      return
    }

    if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) {
      setValidationError('Invalid email format')
      return
    }

    try {
      await login({ email, password, otp: otp || undefined })
      // Redirect to dashboard on success
      router.push('/')
    } catch (err: any) {
      // Error already handled by useAuth hook
      console.error('Login error:', err)
    }
  }

  const errorMessage = validationError || error

  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.95 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={{ duration: 0.5 }}
      className="w-full max-w-md"
    >
      {/* Logo and Title */}
      <div className="text-center mb-8">
        <motion.div
          initial={{ rotate: 0 }}
          animate={{ rotate: 360 }}
          transition={{ duration: 1, ease: 'easeInOut' }}
          className="inline-block p-4 rounded-2xl bg-gradient-to-br from-deltran-gold to-deltran-gold-light mb-4"
        >
          <Wallet className="w-12 h-12 text-black" />
        </motion.div>
        <h1 className="text-3xl font-bold text-white mb-2">DelTran Premium</h1>
        <p className="text-zinc-400">Secure Gateway Access</p>
      </div>

      {/* Login Form */}
      <motion.div
        initial={{ y: 20, opacity: 0 }}
        animate={{ y: 0, opacity: 1 }}
        transition={{ delay: 0.2, duration: 0.5 }}
        className="bg-gradient-to-br from-zinc-900 to-black border border-zinc-800 rounded-xl p-8 shadow-2xl"
      >
        <form onSubmit={handleSubmit} className="space-y-6">
          {/* Error Message */}
          <AnimatePresence>
            {errorMessage && (
              <motion.div
                initial={{ opacity: 0, x: -10 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: 10 }}
                transition={{ type: 'spring', stiffness: 500, damping: 30 }}
                className="bg-red-500/10 border border-red-500/30 rounded-lg p-4 flex items-center gap-3"
              >
                <AlertCircle className="w-5 h-5 text-red-400 flex-shrink-0" />
                <p className="text-sm text-red-400">{errorMessage}</p>
              </motion.div>
            )}
          </AnimatePresence>

          {/* Email Input */}
          <div>
            <label htmlFor="email" className="block text-sm font-medium text-zinc-400 mb-2">
              Email Address
            </label>
            <div className="relative">
              <Mail className="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-zinc-500" />
              <input
                id="email"
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                disabled={loading}
                className="w-full pl-11 pr-4 py-3 bg-zinc-800/50 border border-zinc-700 rounded-lg text-white placeholder-zinc-500 focus:outline-none focus:border-deltran-gold transition-colors disabled:opacity-50"
                placeholder="admin@deltran.com"
              />
            </div>
          </div>

          {/* Password Input */}
          <div>
            <label htmlFor="password" className="block text-sm font-medium text-zinc-400 mb-2">
              Password
            </label>
            <div className="relative">
              <Lock className="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-zinc-500" />
              <input
                id="password"
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                disabled={loading}
                className="w-full pl-11 pr-4 py-3 bg-zinc-800/50 border border-zinc-700 rounded-lg text-white placeholder-zinc-500 focus:outline-none focus:border-deltran-gold transition-colors disabled:opacity-50"
                placeholder="••••••••"
              />
            </div>
          </div>

          {/* OTP Input (Optional) */}
          <AnimatePresence>
            {showOtp && (
              <motion.div
                initial={{ opacity: 0, height: 0 }}
                animate={{ opacity: 1, height: 'auto' }}
                exit={{ opacity: 0, height: 0 }}
              >
                <label htmlFor="otp" className="block text-sm font-medium text-zinc-400 mb-2">
                  Two-Factor Code
                </label>
                <div className="relative">
                  <Shield className="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-zinc-500" />
                  <input
                    id="otp"
                    type="text"
                    value={otp}
                    onChange={(e) => setOtp(e.target.value.replace(/\D/g, '').slice(0, 6))}
                    disabled={loading}
                    maxLength={6}
                    className="w-full pl-11 pr-4 py-3 bg-zinc-800/50 border border-zinc-700 rounded-lg text-white placeholder-zinc-500 focus:outline-none focus:border-deltran-gold transition-colors disabled:opacity-50 tracking-widest"
                    placeholder="000000"
                  />
                </div>
              </motion.div>
            )}
          </AnimatePresence>

          {/* Remember Me & 2FA Toggle */}
          <div className="flex items-center justify-between text-sm">
            <label className="flex items-center gap-2 cursor-pointer group">
              <input
                type="checkbox"
                checked={rememberMe}
                onChange={(e) => setRememberMe(e.target.checked)}
                disabled={loading}
                className="w-4 h-4 rounded border-zinc-700 bg-zinc-800 text-deltran-gold focus:ring-deltran-gold focus:ring-offset-0 disabled:opacity-50"
              />
              <span className="text-zinc-400 group-hover:text-zinc-300 transition-colors">
                Remember me
              </span>
            </label>

            <button
              type="button"
              onClick={() => setShowOtp(!showOtp)}
              disabled={loading}
              className="text-deltran-gold hover:text-deltran-gold-light transition-colors disabled:opacity-50"
            >
              {showOtp ? 'Hide' : 'Enable'} 2FA
            </button>
          </div>

          {/* Submit Button */}
          <motion.button
            type="submit"
            disabled={loading}
            whileHover={{ scale: loading ? 1 : 1.02 }}
            whileTap={{ scale: loading ? 1 : 0.98 }}
            className="w-full py-3 bg-gradient-to-r from-deltran-gold to-deltran-gold-light text-black font-semibold rounded-lg hover:shadow-lg hover:shadow-deltran-gold/20 transition-all disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
          >
            {loading ? (
              <>
                <Loader2 className="w-5 h-5 animate-spin" />
                <span>Signing in...</span>
              </>
            ) : (
              'Sign In'
            )}
          </motion.button>

          {/* Forgot Password Link */}
          <div className="text-center">
            <button
              type="button"
              disabled={loading}
              className="text-sm text-zinc-400 hover:text-deltran-gold transition-colors disabled:opacity-50"
            >
              Forgot your password?
            </button>
          </div>
        </form>
      </motion.div>

      {/* Security Notice */}
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 0.4, duration: 0.5 }}
        className="mt-6 text-center text-xs text-zinc-500"
      >
        <p className="flex items-center justify-center gap-2">
          <Shield className="w-4 h-4" />
          <span>Protected by enterprise-grade encryption</span>
        </p>
      </motion.div>
    </motion.div>
  )
}
