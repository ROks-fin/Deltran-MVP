package resilience

import (
	"context"
	"errors"
	"fmt"
	"sync"
	"time"
)

var (
	ErrCircuitOpen     = errors.New("circuit breaker is open")
	ErrTooManyRequests = errors.New("too many requests in half-open state")
)

// State represents circuit breaker state
type State int

const (
	// StateClosed allows all requests
	StateClosed State = iota
	// StateOpen rejects all requests
	StateOpen
	// StateHalfOpen allows limited requests to test recovery
	StateHalfOpen
)

func (s State) String() string {
	switch s {
	case StateClosed:
		return "closed"
	case StateOpen:
		return "open"
	case StateHalfOpen:
		return "half-open"
	default:
		return "unknown"
	}
}

// Counts holds circuit breaker statistics
type Counts struct {
	Requests             uint32
	TotalSuccesses       uint32
	TotalFailures        uint32
	ConsecutiveSuccesses uint32
	ConsecutiveFailures  uint32
}

// Config holds circuit breaker configuration
type Config struct {
	// Name is the circuit breaker name
	Name string

	// MaxRequests is the maximum number of requests allowed in half-open state
	MaxRequests uint32

	// Interval is the cyclic period in closed state to clear counts
	Interval time.Duration

	// Timeout is the period after which the circuit breaker moves from open to half-open
	Timeout time.Duration

	// ReadyToTrip returns true when circuit should trip to open
	ReadyToTrip func(counts Counts) bool

	// OnStateChange is called when state changes
	OnStateChange func(name string, from State, to State)
}

// DefaultConfig returns default circuit breaker configuration
func DefaultConfig(name string) *Config {
	return &Config{
		Name:        name,
		MaxRequests: 3,
		Interval:    60 * time.Second,
		Timeout:     30 * time.Second,
		ReadyToTrip: func(counts Counts) bool {
			// Trip if failure rate > 50% and at least 5 requests
			failureRatio := float64(counts.TotalFailures) / float64(counts.Requests)
			return counts.Requests >= 5 && failureRatio >= 0.5
		},
		OnStateChange: func(name string, from State, to State) {
			fmt.Printf("Circuit breaker '%s': %s -> %s\n", name, from, to)
		},
	}
}

// CircuitBreaker implements circuit breaker pattern
type CircuitBreaker struct {
	name          string
	maxRequests   uint32
	interval      time.Duration
	timeout       time.Duration
	readyToTrip   func(counts Counts) bool
	onStateChange func(name string, from State, to State)

	mutex      sync.Mutex
	state      State
	generation uint64
	counts     Counts
	expiry     time.Time
}

// NewCircuitBreaker creates a new circuit breaker
func NewCircuitBreaker(config *Config) *CircuitBreaker {
	cb := &CircuitBreaker{
		name:          config.Name,
		maxRequests:   config.MaxRequests,
		interval:      config.Interval,
		timeout:       config.Timeout,
		readyToTrip:   config.ReadyToTrip,
		onStateChange: config.OnStateChange,
	}

	cb.toNewGeneration(time.Now())

	return cb
}

// Execute executes the given function if circuit breaker allows
func (cb *CircuitBreaker) Execute(fn func() error) error {
	generation, err := cb.beforeRequest()
	if err != nil {
		return err
	}

	defer func() {
		if r := recover(); r != nil {
			cb.afterRequest(generation, false)
			panic(r)
		}
	}()

	err = fn()
	cb.afterRequest(generation, err == nil)

	return err
}

// ExecuteContext executes the given function with context
func (cb *CircuitBreaker) ExecuteContext(ctx context.Context, fn func(context.Context) error) error {
	generation, err := cb.beforeRequest()
	if err != nil {
		return err
	}

	defer func() {
		if r := recover(); r != nil {
			cb.afterRequest(generation, false)
			panic(r)
		}
	}()

	// Check if context is already cancelled
	select {
	case <-ctx.Done():
		cb.afterRequest(generation, false)
		return ctx.Err()
	default:
	}

	err = fn(ctx)
	cb.afterRequest(generation, err == nil)

	return err
}

// beforeRequest checks if request is allowed
func (cb *CircuitBreaker) beforeRequest() (uint64, error) {
	cb.mutex.Lock()
	defer cb.mutex.Unlock()

	now := time.Now()
	state, generation := cb.currentState(now)

	if state == StateOpen {
		return generation, ErrCircuitOpen
	} else if state == StateHalfOpen && cb.counts.Requests >= cb.maxRequests {
		return generation, ErrTooManyRequests
	}

	cb.counts.Requests++
	return generation, nil
}

// afterRequest updates counts based on request result
func (cb *CircuitBreaker) afterRequest(before uint64, success bool) {
	cb.mutex.Lock()
	defer cb.mutex.Unlock()

	now := time.Now()
	state, generation := cb.currentState(now)

	// Ignore if generation has changed
	if generation != before {
		return
	}

	if success {
		cb.onSuccess(state, now)
	} else {
		cb.onFailure(state, now)
	}
}

// onSuccess handles successful request
func (cb *CircuitBreaker) onSuccess(state State, now time.Time) {
	switch state {
	case StateClosed:
		cb.counts.TotalSuccesses++
		cb.counts.ConsecutiveSuccesses++
		cb.counts.ConsecutiveFailures = 0

	case StateHalfOpen:
		cb.counts.TotalSuccesses++
		cb.counts.ConsecutiveSuccesses++
		cb.counts.ConsecutiveFailures = 0

		// If enough consecutive successes, close circuit
		if cb.counts.ConsecutiveSuccesses >= cb.maxRequests {
			cb.setState(StateClosed, now)
		}
	}
}

// onFailure handles failed request
func (cb *CircuitBreaker) onFailure(state State, now time.Time) {
	switch state {
	case StateClosed:
		cb.counts.TotalFailures++
		cb.counts.ConsecutiveFailures++
		cb.counts.ConsecutiveSuccesses = 0

		// Check if should trip
		if cb.readyToTrip(cb.counts) {
			cb.setState(StateOpen, now)
		}

	case StateHalfOpen:
		// Any failure in half-open immediately opens circuit
		cb.setState(StateOpen, now)
	}
}

// currentState returns current state and generation
func (cb *CircuitBreaker) currentState(now time.Time) (State, uint64) {
	switch cb.state {
	case StateClosed:
		// Check if interval expired
		if !cb.expiry.IsZero() && cb.expiry.Before(now) {
			cb.toNewGeneration(now)
		}

	case StateOpen:
		// Check if timeout expired
		if cb.expiry.Before(now) {
			cb.setState(StateHalfOpen, now)
		}
	}

	return cb.state, cb.generation
}

// setState changes circuit breaker state
func (cb *CircuitBreaker) setState(state State, now time.Time) {
	if cb.state == state {
		return
	}

	prev := cb.state
	cb.state = state

	cb.toNewGeneration(now)

	if cb.onStateChange != nil {
		cb.onStateChange(cb.name, prev, state)
	}
}

// toNewGeneration starts a new generation
func (cb *CircuitBreaker) toNewGeneration(now time.Time) {
	cb.generation++
	cb.counts = Counts{}

	var zero time.Time
	switch cb.state {
	case StateClosed:
		if cb.interval == 0 {
			cb.expiry = zero
		} else {
			cb.expiry = now.Add(cb.interval)
		}

	case StateOpen:
		cb.expiry = now.Add(cb.timeout)

	default: // StateHalfOpen
		cb.expiry = zero
	}
}

// State returns current state
func (cb *CircuitBreaker) State() State {
	cb.mutex.Lock()
	defer cb.mutex.Unlock()

	now := time.Now()
	state, _ := cb.currentState(now)
	return state
}

// Counts returns current counts
func (cb *CircuitBreaker) Counts() Counts {
	cb.mutex.Lock()
	defer cb.mutex.Unlock()

	return cb.counts
}

// Name returns circuit breaker name
func (cb *CircuitBreaker) Name() string {
	return cb.name
}

// Reset resets circuit breaker to closed state
func (cb *CircuitBreaker) Reset() {
	cb.mutex.Lock()
	defer cb.mutex.Unlock()

	cb.state = StateClosed
	cb.toNewGeneration(time.Now())
}

// CircuitBreakerManager manages multiple circuit breakers
type CircuitBreakerManager struct {
	breakers map[string]*CircuitBreaker
	mutex    sync.RWMutex
}

// NewCircuitBreakerManager creates a new circuit breaker manager
func NewCircuitBreakerManager() *CircuitBreakerManager {
	return &CircuitBreakerManager{
		breakers: make(map[string]*CircuitBreaker),
	}
}

// Get returns or creates a circuit breaker
func (m *CircuitBreakerManager) Get(name string, config *Config) *CircuitBreaker {
	m.mutex.RLock()
	cb, exists := m.breakers[name]
	m.mutex.RUnlock()

	if exists {
		return cb
	}

	m.mutex.Lock()
	defer m.mutex.Unlock()

	// Double-check after acquiring write lock
	cb, exists = m.breakers[name]
	if exists {
		return cb
	}

	// Use provided config or default
	if config == nil {
		config = DefaultConfig(name)
	} else {
		config.Name = name
	}

	cb = NewCircuitBreaker(config)
	m.breakers[name] = cb

	return cb
}

// GetAll returns all circuit breakers
func (m *CircuitBreakerManager) GetAll() map[string]*CircuitBreaker {
	m.mutex.RLock()
	defer m.mutex.RUnlock()

	result := make(map[string]*CircuitBreaker, len(m.breakers))
	for k, v := range m.breakers {
		result[k] = v
	}

	return result
}

// Remove removes a circuit breaker
func (m *CircuitBreakerManager) Remove(name string) {
	m.mutex.Lock()
	defer m.mutex.Unlock()

	delete(m.breakers, name)
}

// Reset resets a circuit breaker
func (m *CircuitBreakerManager) Reset(name string) {
	m.mutex.RLock()
	cb, exists := m.breakers[name]
	m.mutex.RUnlock()

	if exists {
		cb.Reset()
	}
}

// ResetAll resets all circuit breakers
func (m *CircuitBreakerManager) ResetAll() {
	m.mutex.RLock()
	defer m.mutex.RUnlock()

	for _, cb := range m.breakers {
		cb.Reset()
	}
}

// Stats holds circuit breaker statistics
type Stats struct {
	Name                 string `json:"name"`
	State                string `json:"state"`
	Requests             uint32 `json:"requests"`
	TotalSuccesses       uint32 `json:"total_successes"`
	TotalFailures        uint32 `json:"total_failures"`
	ConsecutiveSuccesses uint32 `json:"consecutive_successes"`
	ConsecutiveFailures  uint32 `json:"consecutive_failures"`
	Generation           uint64 `json:"generation"`
}

// GetStats returns statistics for all circuit breakers
func (m *CircuitBreakerManager) GetStats() []Stats {
	m.mutex.RLock()
	defer m.mutex.RUnlock()

	stats := make([]Stats, 0, len(m.breakers))

	for _, cb := range m.breakers {
		cb.mutex.Lock()
		stat := Stats{
			Name:                 cb.name,
			State:                cb.state.String(),
			Requests:             cb.counts.Requests,
			TotalSuccesses:       cb.counts.TotalSuccesses,
			TotalFailures:        cb.counts.TotalFailures,
			ConsecutiveSuccesses: cb.counts.ConsecutiveSuccesses,
			ConsecutiveFailures:  cb.counts.ConsecutiveFailures,
			Generation:           cb.generation,
		}
		cb.mutex.Unlock()

		stats = append(stats, stat)
	}

	return stats
}
