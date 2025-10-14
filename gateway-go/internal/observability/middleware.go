package observability

import (
	"net/http"
	"time"
)

// responseWriter wraps http.ResponseWriter to capture status code and response size
type responseWriter struct {
	http.ResponseWriter
	statusCode   int
	bytesWritten int64
}

func newResponseWriter(w http.ResponseWriter) *responseWriter {
	return &responseWriter{
		ResponseWriter: w,
		statusCode:     http.StatusOK,
	}
}

func (rw *responseWriter) WriteHeader(code int) {
	rw.statusCode = code
	rw.ResponseWriter.WriteHeader(code)
}

func (rw *responseWriter) Write(b []byte) (int, error) {
	n, err := rw.ResponseWriter.Write(b)
	rw.bytesWritten += int64(n)
	return n, err
}

// MetricsMiddleware creates HTTP middleware for Prometheus metrics
func MetricsMiddleware(metrics *Metrics) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			start := time.Now()

			// Wrap response writer to capture status and size
			rw := newResponseWriter(w)

			// Call next handler
			next.ServeHTTP(rw, r)

			// Record metrics
			duration := time.Since(start)
			metrics.RecordHTTPRequest(
				r.Method,
				r.URL.Path,
				rw.statusCode,
				duration,
				rw.bytesWritten,
			)
		})
	}
}
