'use client'

import { useState, useEffect } from 'react'
import { AuthService, User, LoginCredentials } from '../services/auth.service'

export function useAuth() {
  const [user, setUser] = useState<User | null>(AuthService.getUser())
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    // Subscribe to auth state changes
    const unsubscribe = AuthService.onAuthStateChange((newUser) => {
      setUser(newUser)
    })

    return unsubscribe
  }, [])

  const login = async (credentials: LoginCredentials) => {
    setLoading(true)
    setError(null)

    try {
      const user = await AuthService.login(credentials)
      setUser(user)
      return user
    } catch (err: any) {
      const errorMessage = err.message || 'Login failed'
      setError(errorMessage)
      throw err
    } finally {
      setLoading(false)
    }
  }

  const logout = async () => {
    setLoading(true)
    setError(null)

    try {
      await AuthService.logout()
      setUser(null)
    } catch (err: any) {
      const errorMessage = err.message || 'Logout failed'
      setError(errorMessage)
      throw err
    } finally {
      setLoading(false)
    }
  }

  return {
    user,
    loading,
    error,
    login,
    logout,
    isAuthenticated: AuthService.isAuthenticated(),
  }
}
