package clients

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"

	"github.com/afex/hystrix-go/hystrix"
)

// BaseClient provides common HTTP client functionality
type BaseClient struct {
	httpClient  *http.Client
	baseURL     string
	serviceName string
	timeout     time.Duration
}

// NewBaseClient creates a new base HTTP client
func NewBaseClient(baseURL, serviceName string, timeout time.Duration) *BaseClient {
	// Configure circuit breaker for this service
	hystrix.ConfigureCommand(serviceName, hystrix.CommandConfig{
		Timeout:                int(timeout.Milliseconds()),
		MaxConcurrentRequests:  100,
		RequestVolumeThreshold: 10,
		SleepWindow:            5000,
		ErrorPercentThreshold:  50,
	})

	return &BaseClient{
		httpClient: &http.Client{
			Timeout: timeout,
			Transport: &http.Transport{
				MaxIdleConns:        100,
				MaxIdleConnsPerHost: 10,
				IdleConnTimeout:     90 * time.Second,
			},
		},
		baseURL:     baseURL,
		serviceName: serviceName,
		timeout:     timeout,
	}
}

// Post sends a POST request with circuit breaker protection
func (c *BaseClient) Post(ctx context.Context, endpoint string, body interface{}, result interface{}) error {
	var responseData []byte
	var statusCode int

	err := hystrix.Do(c.serviceName, func() error {
		data, code, err := c.doPost(ctx, endpoint, body)
		responseData = data
		statusCode = code
		return err
	}, func(err error) error {
		// Fallback logic - return circuit breaker error
		return fmt.Errorf("circuit breaker open for %s: %w", c.serviceName, err)
	})

	if err != nil {
		return err
	}

	if statusCode >= 400 {
		return fmt.Errorf("%s returned error status %d: %s", c.serviceName, statusCode, string(responseData))
	}

	if result != nil {
		if err := json.Unmarshal(responseData, result); err != nil {
			return fmt.Errorf("failed to unmarshal response: %w", err)
		}
	}

	return nil
}

// Get sends a GET request with circuit breaker protection
func (c *BaseClient) Get(ctx context.Context, endpoint string, result interface{}) error {
	var responseData []byte
	var statusCode int

	err := hystrix.Do(c.serviceName, func() error {
		data, code, err := c.doGet(ctx, endpoint)
		responseData = data
		statusCode = code
		return err
	}, func(err error) error {
		return fmt.Errorf("circuit breaker open for %s: %w", c.serviceName, err)
	})

	if err != nil {
		return err
	}

	if statusCode >= 400 {
		return fmt.Errorf("%s returned error status %d: %s", c.serviceName, statusCode, string(responseData))
	}

	if result != nil {
		if err := json.Unmarshal(responseData, result); err != nil {
			return fmt.Errorf("failed to unmarshal response: %w", err)
		}
	}

	return nil
}

// doPost performs the actual POST request
func (c *BaseClient) doPost(ctx context.Context, endpoint string, body interface{}) ([]byte, int, error) {
	url := c.baseURL + endpoint

	var reqBody io.Reader
	if body != nil {
		jsonData, err := json.Marshal(body)
		if err != nil {
			return nil, 0, fmt.Errorf("failed to marshal request body: %w", err)
		}
		reqBody = bytes.NewBuffer(jsonData)
	}

	req, err := http.NewRequestWithContext(ctx, "POST", url, reqBody)
	if err != nil {
		return nil, 0, fmt.Errorf("failed to create request: %w", err)
	}

	req.Header.Set("Content-Type", "application/json")

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return nil, 0, fmt.Errorf("request failed: %w", err)
	}
	defer resp.Body.Close()

	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, resp.StatusCode, fmt.Errorf("failed to read response body: %w", err)
	}

	return data, resp.StatusCode, nil
}

// doGet performs the actual GET request
func (c *BaseClient) doGet(ctx context.Context, endpoint string) ([]byte, int, error) {
	url := c.baseURL + endpoint

	req, err := http.NewRequestWithContext(ctx, "GET", url, nil)
	if err != nil {
		return nil, 0, fmt.Errorf("failed to create request: %w", err)
	}

	req.Header.Set("Accept", "application/json")

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return nil, 0, fmt.Errorf("request failed: %w", err)
	}
	defer resp.Body.Close()

	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, resp.StatusCode, fmt.Errorf("failed to read response body: %w", err)
	}

	return data, resp.StatusCode, nil
}
