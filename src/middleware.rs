use std::time::Instant;

use axum::{
    extract::Request,
    http::{header, HeaderMap},
    middleware::Next,
    response::IntoResponse,
};
use tracing::{info, instrument};
use uuid::Uuid;

const SENSITIVE_HEADERS: &[header::HeaderName] = &[
    header::AUTHORIZATION,
    header::PROXY_AUTHORIZATION,
    header::COOKIE,
    header::SET_COOKIE,
];

#[derive(Clone)]
#[repr(transparent)]
struct RequestId(Uuid);

pub async fn request_id(mut req: Request, next: Next) -> impl IntoResponse {
    let id = Uuid::now_v7();
    req.extensions_mut().insert(RequestId(id));
    let mut res = next.run(req).await;
    res.headers_mut()
        .insert("x-request-id", id.to_string().try_into().unwrap());
    res
}

fn redact_sensitive(map: &HeaderMap) -> HeaderMap {
    let mut map = map.clone();
    for name in SENSITIVE_HEADERS {
        if let header::Entry::Occupied(mut entry) = map.entry(name) {
            entry.insert("[REDACTED]".parse().unwrap());
        }
    }
    map
}

#[instrument(skip_all, fields(
    request_id = %req.extensions().get::<RequestId>().unwrap().0,
    method = %req.method(),
    path = %req.uri().path(),
    http_version = ?req.version(),
))]
pub async fn trace(req: Request, next: Next) -> impl IntoResponse {
    info!(headers = ?redact_sensitive(req.headers()), "request");
    let start = Instant::now();
    let res = next.run(req).await;
    info!(
        status = res.status().as_u16(),
        duration = start.elapsed().as_millis() as usize,
        headers = ?redact_sensitive(res.headers()),
        "response"
    );
    res
}
