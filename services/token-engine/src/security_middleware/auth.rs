use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse,
};
use futures_util::future::LocalBoxFuture;
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use std::future::{ready, Ready};
use std::rc::Rc;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        // Skip auth for health and metrics endpoints
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
                        Err(actix_web::error::ErrorUnauthorized("Invalid auth header format"))
                    });
                }
            }
            None => {
                return Box::pin(async {
                    Err(actix_web::error::ErrorUnauthorized("Missing Authorization header"))
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
                // Add user info to request extensions for handlers to access
                req.extensions_mut().insert(token_data.claims.clone());

                let fut = self.service.call(req);
                Box::pin(async move { fut.await })
            }
            Err(err) => {
                tracing::warn!("JWT validation failed: {:?}", err);
                Box::pin(async {
                    Err(actix_web::error::ErrorUnauthorized("Invalid or expired token"))
                })
            }
        }
    }
}
