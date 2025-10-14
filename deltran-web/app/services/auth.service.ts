'use client'

export interface LoginCredentials {
  email: string
  password: string
  otp?: string
}

export interface User {
  id: string
  email: string
  name: string
  role: 'admin' | 'operator' | 'viewer'
  permissions: string[]
}

export interface AuthTokens {
  accessToken: string
  refreshToken: string
  expiresIn: number
}

type AuthStateCallback = (user: User | null) => void

class AuthServiceClass {
  private user: User | null = null
  private accessToken: string | null = null
  private refreshToken: string | null = null
  private refreshTimer: NodeJS.Timeout | null = null
  private callbacks: AuthStateCallback[] = []

  constructor() {
    if (typeof window !== 'undefined') {
      this.loadFromStorage()
      this.setupStorageListener()
    }
  }

  /**
   * Login with email/password (and optional OTP for 2FA)
   */
  async login(credentials: LoginCredentials): Promise<User> {
    try {
      const response = await fetch(`${process.env.NEXT_PUBLIC_API_URL}/api/v1/auth/login`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(credentials),
      })

      if (!response.ok) {
        const error = await response.json()
        throw new Error(error.message || 'Login failed')
      }

      const data = await response.json()

      // Store tokens and user
      this.setAuth(data.user, data.tokens)

      return data.user
    } catch (error) {
      // Fallback for development - mock successful login
      if (process.env.NODE_ENV === 'development') {
        console.warn('API unavailable, using mock auth')
        const mockUser: User = {
          id: '1',
          email: credentials.email,
          name: 'Admin User',
          role: 'admin',
          permissions: ['*'],
        }
        const mockTokens: AuthTokens = {
          accessToken: 'mock-access-token',
          refreshToken: 'mock-refresh-token',
          expiresIn: 3600,
        }
        this.setAuth(mockUser, mockTokens)
        return mockUser
      }
      throw error
    }
  }

  /**
   * Logout and clear all auth data
   */
  async logout(): Promise<void> {
    try {
      if (this.accessToken) {
        await fetch(`${process.env.NEXT_PUBLIC_API_URL}/api/v1/auth/logout`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            Authorization: `Bearer ${this.accessToken}`,
          },
        })
      }
    } catch (error) {
      console.error('Logout API error:', error)
    } finally {
      this.clearAuth()
    }
  }

  /**
   * Refresh access token using refresh token
   */
  async refreshAccessToken(): Promise<string> {
    if (!this.refreshToken) {
      throw new Error('No refresh token available')
    }

    try {
      const response = await fetch(`${process.env.NEXT_PUBLIC_API_URL}/api/v1/auth/refresh`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ refreshToken: this.refreshToken }),
      })

      if (!response.ok) {
        throw new Error('Token refresh failed')
      }

      const data = await response.json()
      this.accessToken = data.accessToken
      this.saveToStorage()
      this.scheduleTokenRefresh(data.expiresIn)

      return data.accessToken
    } catch (error) {
      // If refresh fails, logout user
      this.clearAuth()
      throw error
    }
  }

  /**
   * Get current user
   */
  getUser(): User | null {
    return this.user
  }

  /**
   * Get access token
   */
  getAccessToken(): string | null {
    return this.accessToken
  }

  /**
   * Check if user is authenticated
   */
  isAuthenticated(): boolean {
    return this.user !== null && this.accessToken !== null
  }

  /**
   * Subscribe to auth state changes
   */
  onAuthStateChange(callback: AuthStateCallback): () => void {
    this.callbacks.push(callback)

    // Return unsubscribe function
    return () => {
      this.callbacks = this.callbacks.filter(cb => cb !== callback)
    }
  }

  /**
   * Set authentication data
   */
  private setAuth(user: User, tokens: AuthTokens): void {
    this.user = user
    this.accessToken = tokens.accessToken
    this.refreshToken = tokens.refreshToken

    this.saveToStorage()
    this.scheduleTokenRefresh(tokens.expiresIn)
    this.notifyListeners()
  }

  /**
   * Clear authentication data
   */
  private clearAuth(): void {
    this.user = null
    this.accessToken = null
    this.refreshToken = null

    if (this.refreshTimer) {
      clearTimeout(this.refreshTimer)
      this.refreshTimer = null
    }

    if (typeof window !== 'undefined') {
      localStorage.removeItem('auth_user')
      localStorage.removeItem('auth_tokens')

      // Broadcast logout to other tabs
      localStorage.setItem('auth_event', JSON.stringify({ type: 'logout', timestamp: Date.now() }))
    }

    this.notifyListeners()
  }

  /**
   * Save auth data to localStorage
   */
  private saveToStorage(): void {
    if (typeof window === 'undefined') return

    if (this.user) {
      localStorage.setItem('auth_user', JSON.stringify(this.user))
    }

    if (this.accessToken && this.refreshToken) {
      localStorage.setItem('auth_tokens', JSON.stringify({
        accessToken: this.accessToken,
        refreshToken: this.refreshToken,
      }))
    }
  }

  /**
   * Load auth data from localStorage
   */
  private loadFromStorage(): void {
    if (typeof window === 'undefined') return

    try {
      const userStr = localStorage.getItem('auth_user')
      const tokensStr = localStorage.getItem('auth_tokens')

      if (userStr && tokensStr) {
        this.user = JSON.parse(userStr)
        const tokens = JSON.parse(tokensStr)
        this.accessToken = tokens.accessToken
        this.refreshToken = tokens.refreshToken

        // Schedule refresh (assuming 1 hour expiry)
        this.scheduleTokenRefresh(3600)
      }
    } catch (error) {
      console.error('Failed to load auth from storage:', error)
      this.clearAuth()
    }
  }

  /**
   * Schedule automatic token refresh
   */
  private scheduleTokenRefresh(expiresIn: number): void {
    if (this.refreshTimer) {
      clearTimeout(this.refreshTimer)
    }

    // Refresh 5 minutes before expiry
    const refreshTime = (expiresIn - 300) * 1000

    this.refreshTimer = setTimeout(() => {
      this.refreshAccessToken().catch(error => {
        console.error('Auto token refresh failed:', error)
      })
    }, Math.max(refreshTime, 0))
  }

  /**
   * Setup cross-tab authentication sync
   */
  private setupStorageListener(): void {
    if (typeof window === 'undefined') return

    window.addEventListener('storage', (event) => {
      if (event.key === 'auth_event') {
        try {
          const authEvent = JSON.parse(event.newValue || '{}')
          if (authEvent.type === 'logout') {
            this.clearAuth()
            window.location.href = '/login'
          }
        } catch (error) {
          console.error('Failed to parse auth event:', error)
        }
      }
    })
  }

  /**
   * Notify all listeners of auth state change
   */
  private notifyListeners(): void {
    this.callbacks.forEach(callback => callback(this.user))
  }
}

// Export singleton instance
export const AuthService = new AuthServiceClass()
