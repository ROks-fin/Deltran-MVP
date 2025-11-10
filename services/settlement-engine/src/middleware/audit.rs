use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};
use chrono::Utc;
use futures_util::future::LocalBoxFuture;
use serde_json::json;
use std::future::{ready, Ready};
use std::rc::Rc;
use tracing::info;

use super::auth::Claims;

pub struct AuditLog;

impl<S, B> Transform<S, ServiceRequest> for AuditLog
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AuditLogMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuditLogMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct AuditLogMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for AuditLogMiddleware<S>
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
        let start_time = Utc::now();
        let method = req.method().to_string();
        let path = req.path().to_string();

        // Extract user from JWT claims if available
        let user_id = req
            .extensions()
            .get::<Claims>()
            .map(|c| c.sub.clone())
            .unwrap_or_else(|| "anonymous".to_string());

        let service = self.service.clone();

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
                            "method": method,
                            "path": path,
                            "status": response.status().as_u16(),
                            "duration_ms": duration.num_milliseconds(),
                            "service": "token-engine"
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
                            "method": method,
                            "path": path,
                            "error": e.to_string(),
                            "duration_ms": duration.num_milliseconds(),
                            "service": "token-engine"
                        })
                    );
                }
            }

            res
        })
    }
}
