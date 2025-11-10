use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse, http::StatusCode,
};
use futures_util::future::LocalBoxFuture;
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as GovernorRateLimiter,
};
use std::future::{ready, Ready};
use std::num::NonZeroU32;
use std::rc::Rc;

pub struct RateLimiter {
    limiter: Rc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
}

impl RateLimiter {
    pub fn new(requests_per_minute: u32) -> Self {
        let quota = Quota::per_minute(NonZeroU32::new(requests_per_minute).unwrap());
        Self {
            limiter: Rc::new(GovernorRateLimiter::direct(quota)),
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for RateLimiter
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RateLimiterMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RateLimiterMiddleware {
            service: Rc::new(service),
            limiter: self.limiter.clone(),
        }))
    }
}

pub struct RateLimiterMiddleware<S> {
    service: Rc<S>,
    limiter: Rc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
}

impl<S, B> Service<ServiceRequest> for RateLimiterMiddleware<S>
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
        // Skip rate limiting for health endpoint
        if req.path() == "/health" {
            let fut = self.service.call(req);
            return Box::pin(async move { fut.await });
        }

        // Check rate limit
        match self.limiter.check() {
            Ok(_) => {
                let fut = self.service.call(req);
                Box::pin(async move { fut.await })
            }
            Err(_) => {
                tracing::warn!("Rate limit exceeded for path: {}", req.path());
                Box::pin(async {
                    Err(actix_web::error::ErrorTooManyRequests(
                        "Rate limit exceeded. Please try again later.",
                    ))
                })
            }
        }
    }
}
