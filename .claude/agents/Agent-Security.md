# Agent-Security

## Роль
Агент для улучшения безопасности СУЩЕСТВУЮЩИХ сервисов DelTran MVP: добавление JWT middleware в Rust сервисы, улучшение rate limiting, audit logging, и Context7-based security patterns.

## Контекст
DelTran MVP имеет 11 ГОТОВЫХ сервисов:
- **Rust (7)**: token-engine, clearing-engine, settlement-engine, obligation-engine, risk-engine, compliance-engine, liquidity-router
- **Go (3)**: gateway (УЖЕ УЛУЧШЕН), notification-engine, reporting-engine
- **Python (1)**: analytics-collector (УЖЕ СОЗДАН)

**Gateway уже имеет**: JWT auth, tiered rate limiting, analytics integration, security headers

## Задачи

### 🔍 ПЕРВЫЙ ШАГ: Сканирование

**ОБЯЗАТЕЛЬНО перед началом работы:**

```bash
# 1. Прочитать ВСЕ существующие файлы сервисов
ls services/token-engine/src/
cat services/token-engine/src/main.rs
cat services/token-engine/Cargo.toml

ls services/clearing-engine/src/
cat services/clearing-engine/src/main.rs

ls services/settlement-engine/src/
cat services/settlement-engine/src/main.rs

# 2. Проверить какие middleware уже есть
grep -r "middleware" services/*/src/
grep -r "auth" services/*/src/
grep -r "jwt" services/*/src/

# 3. Проверить структуру проекта
tree services/token-engine/src/
```

### 1. Использование Context7 для получения актуальных patterns

```bash
# Получить документацию для Actix-Web middleware
context7 resolve actix-web
context7 docs actix-web middleware authentication

# Получить JWT patterns для Rust
context7 resolve jsonwebtoken
context7 docs jsonwebtoken validation

# Rate limiting patterns
context7 resolve governor  # Rust rate limiting
context7 docs governor actix-web integration
```

### 2. Добавление JWT Middleware в Token Engine

**ТОЛЬКО ЕСЛИ его еще нет!** Сначала проверь:
```bash
cat services/token-engine/src/main.rs | grep -i "jwt\|auth"
```

Если НЕТ JWT middleware, добавь:

```rust
// services/token-engine/src/middleware/auth.rs (НОВЫЙ ФАЙЛ)

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};
use futures_util::future::LocalBoxFuture;
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use std::future::{ready, Ready};
use std::rc::Rc;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub role: String,
    pub permissions: Vec<String>,
    pub exp: usize,
}

pub struct JwtAuth {
    secret: String,
}

impl JwtAuth {
    pub fn new(secret: String) -> Self {
        Self { secret }
    }
}

impl<S, B> Transform<S, ServiceRequest> for JwtAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = JwtAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(JwtAuthMiddleware {
            service: Rc::new(service),
            secret: self.secret.clone(),
        }))
    }
}

pub struct JwtAuthMiddleware<S> {
    service: Rc<S>,
    secret: String,
}

impl<S, B> Service<ServiceRequest> for JwtAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Skip auth for health endpoint
        if req.path() == "/health" || req.path() == "/metrics" {
            let fut = self.service.call(req);
            return Box::pin(async move { fut.await });
        }

        // Extract token from Authorization header
        let auth_header = req.headers().get("Authorization");

        let token = match auth_header {
            Some(value) => {
                let auth_str = value.to_str().unwrap_or("");
                if auth_str.starts_with("Bearer ") {
                    &auth_str[7..]
                } else {
                    return Box::pin(async {
                        Err(actix_web::error::ErrorUnauthorized("Invalid auth header"))
                    });
                }
            }
            None => {
                return Box::pin(async {
                    Err(actix_web::error::ErrorUnauthorized("Missing auth header"))
                });
            }
        };

        // Validate token
        let secret = self.secret.clone();
        let validation = Validation::new(Algorithm::HS256);

        match decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &validation,
        ) {
            Ok(token_data) => {
                // Add user info to request extensions
                req.extensions_mut().insert(token_data.claims.clone());

                let fut = self.service.call(req);
                Box::pin(async move { fut.await })
            }
            Err(_) => Box::pin(async {
                Err(actix_web::error::ErrorUnauthorized("Invalid token"))
            }),
        }
    }
}
```

**Интеграция в main.rs:**
```rust
// services/token-engine/src/main.rs

mod middleware; // Добавь эту строку

use middleware::auth::JwtAuth;

// В HttpServer::new():
HttpServer::new(move || {
    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "deltran-secret-key-change-in-production".to_string());

    App::new()
        .wrap(middleware::Logger::default())
        .wrap(JwtAuth::new(jwt_secret))  // ДОБАВЬ ЭТО
        .wrap(Cors::permissive())
        .configure(handlers::configure)
})
```

### 3. Добавление Rate Limiting в Rust сервисы

Используй Context7 для получения актуальной документации:

```bash
context7 docs governor "actix-web rate limiting example"
```

```rust
// services/token-engine/src/middleware/rate_limit.rs

use actix_web::{dev::ServiceRequest, Error, HttpResponse};
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as GovernorRateLimiter,
};
use std::num::NonZeroU32;

pub struct RateLimiter {
    limiter: GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>,
}

impl RateLimiter {
    pub fn new(requests_per_minute: u32) -> Self {
        let quota = Quota::per_minute(NonZeroU32::new(requests_per_minute).unwrap());
        Self {
            limiter: GovernorRateLimiter::direct(quota),
        }
    }

    pub fn check(&self) -> Result<(), Error> {
        match self.limiter.check() {
            Ok(_) => Ok(()),
            Err(_) => Err(actix_web::error::ErrorTooManyRequests(
                "Rate limit exceeded",
            )),
        }
    }
}
```

### 4. Audit Logging для Rust сервисов

```rust
// services/token-engine/src/middleware/audit.rs

use actix_web::{dev::{Service, ServiceRequest, ServiceResponse, Transform}, Error};
use chrono::Utc;
use futures_util::future::LocalBoxFuture;
use serde_json::json;
use tracing::info;

pub struct AuditLog;

impl<S, B> Transform<S, ServiceRequest> for AuditLog
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
{
    // Implementation similar to JWT middleware
    // Log: timestamp, user_id, path, method, status_code, duration

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let start_time = Utc::now();
        let method = req.method().clone();
        let path = req.path().to_string();

        // Extract user from JWT claims
        let user_id = req.extensions()
            .get::<Claims>()
            .map(|c| c.sub.clone())
            .unwrap_or_else(|| "anonymous".to_string());

        Box::pin(async move {
            let res = service.call(req).await;

            let duration = Utc::now() - start_time;

            match &res {
                Ok(response) => {
                    info!(
                        target: "audit_log",
                        "{}",
                        json!({
                            "timestamp": start_time.to_rfc3339(),
                            "user_id": user_id,
                            "method": method.to_string(),
                            "path": path,
                            "status": response.status().as_u16(),
                            "duration_ms": duration.num_milliseconds(),
                        })
                    );
                }
                Err(e) => {
                    info!(
                        target: "audit_log",
                        "{}",
                        json!({
                            "timestamp": start_time.to_rfc3339(),
                            "user_id": user_id,
                            "method": method.to_string(),
                            "path": path,
                            "error": e.to_string(),
                            "duration_ms": duration.num_milliseconds(),
                        })
                    );
                }
            }

            res
        })
    }
}
```

### 5. Обновление Cargo.toml

**ТОЛЬКО добавь недостающие зависимости:**

```toml
# services/token-engine/Cargo.toml

[dependencies]
# Проверь что уже есть, добавь ТОЛЬКО то чего нет:
jsonwebtoken = "9.2"      # Если нет
governor = "0.6"          # Если нет
```

### 6. Улучшение Go сервисов (Notification, Reporting)

**Проверь сначала что у них уже есть:**
```bash
grep -r "middleware" services/notification-engine/
grep -r "jwt" services/notification-engine/
```

Добавь ТОЛЬКО если нет:

```go
// services/notification-engine/internal/middleware/auth.go

package middleware

import (
    "net/http"
    "strings"
    "github.com/golang-jwt/jwt/v5"
)

type Claims struct {
    Sub         string   `json:"sub"`
    Role        string   `json:"role"`
    Permissions []string `json:"permissions"`
    jwt.RegisteredClaims
}

func JWTAuth(secret string) func(http.Handler) http.Handler {
    return func(next http.Handler) http.Handler {
        return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
            // Skip auth for health/metrics
            if r.URL.Path == "/health" || r.URL.Path == "/metrics" {
                next.ServeHTTP(w, r)
                return
            }

            authHeader := r.Header.Get("Authorization")
            if authHeader == "" {
                http.Error(w, "Missing authorization header", http.StatusUnauthorized)
                return
            }

            bearerToken := strings.Split(authHeader, " ")
            if len(bearerToken) != 2 || bearerToken[0] != "Bearer" {
                http.Error(w, "Invalid authorization header", http.StatusUnauthorized)
                return
            }

            claims := &Claims{}
            token, err := jwt.ParseWithClaims(bearerToken[1], claims, func(token *jwt.Token) (interface{}, error) {
                return []byte(secret), nil
            })

            if err != nil || !token.Valid {
                http.Error(w, "Invalid token", http.StatusUnauthorized)
                return
            }

            // Add claims to context for handlers
            ctx := context.WithValue(r.Context(), "claims", claims)
            next.ServeHTTP(w, r.WithContext(ctx))
        })
    }
}
```

### 7. Environment Configuration

```yaml
# config/security.yml

security:
  jwt:
    secret: ${JWT_SECRET:-deltran-secret-key-change-in-production}
    algorithm: HS256

  rate_limiting:
    enabled: true
    requests_per_minute: 100

  audit:
    enabled: true
    log_file: /var/log/deltran/audit.log
```

## Технологический стек
- **JWT**: jsonwebtoken (Rust), golang-jwt (Go)
- **Rate Limiting**: governor (Rust), встроенный в Gateway (Go)
- **Audit Logging**: tracing (Rust), zap (Go)
- **Context7**: Для получения актуальных middleware patterns

## Порядок выполнения

```bash
# 1. СКАНИРОВАНИЕ - обязательный первый шаг
./scan-services.sh  # Создай скрипт для проверки что уже есть

# 2. Context7 - получить актуальные patterns
context7 docs actix-web middleware
context7 docs governor rate-limiting
context7 docs jsonwebtoken

# 3. Добавить middleware ТОЛЬКО там где его НЕТ

# 4. Тестирование
cargo test --all
go test ./...

# 5. Интеграционные тесты с Gateway
curl -H "Authorization: Bearer $TOKEN" http://localhost:8081/api/tokens
```

## Критически важно

1. **НЕ СОЗДАВАТЬ новые сервисы** - только улучшать существующие
2. **СКАНИРОВАТЬ перед изменениями** - проверять что уже реализовано
3. **Использовать Context7** - для получения актуальных patterns
4. **НЕ дублировать** - Gateway уже имеет полный security stack
5. **Добавлять постепенно** - сначала JWT, потом rate limiting, потом audit
6. **Тестировать каждое изменение** перед следующим

## Результат
Все существующие сервисы улучшены с:
- JWT authentication middleware
- Rate limiting
- Audit logging
- Security headers
- Без дублирования уже существующих функций
