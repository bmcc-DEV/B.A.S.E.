//! HTTP routes — canonical v1: identify · prove · usage.

use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Serialize;
use std::sync::Arc;

use crate::billing::{BillingError, BillingState, UsageBreakdown};
use crate::identify::{run_identify, IdentifyRequest, IdentifyResponse};
use crate::prove::{run_prove, ProveRequest, ProveResponse};
use crate::OPENAPI_YAML;

#[derive(Clone)]
pub struct AppState {
    pub billing: BillingState,
}

pub fn extract_api_key(headers: &HeaderMap) -> Option<String> {
    if let Some(auth) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
        let auth = auth.trim();
        if let Some(rest) = auth.strip_prefix("Bearer ") {
            return Some(rest.trim().to_string());
        }
        if let Some(rest) = auth.strip_prefix("bearer ") {
            return Some(rest.trim().to_string());
        }
    }
    headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
}

#[derive(Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    message: String,
    #[serde(rename = "type")]
    kind: String,
    code: String,
}

fn err(status: StatusCode, message: impl Into<String>, code: &str) -> impl IntoResponse {
    (
        status,
        Json(ErrorBody {
            error: ErrorDetail {
                message: message.into(),
                kind: "invalid_request_error".into(),
                code: code.into(),
            },
        }),
    )
}

pub async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "ok": true,
        "service": "base-api",
        "version": "1.0.0",
        "canonical": ["identify", "prove", "usage"],
        "saas_production": false,
        "object": "health"
    }))
}

pub async fn openapi_yaml() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/yaml; charset=utf-8")],
        OPENAPI_YAML,
    )
}

pub async fn usage(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let Some(key) = extract_api_key(&headers) else {
        return err(
            StatusCode::UNAUTHORIZED,
            "Missing Authorization: Bearer <api_key>",
            "missing_api_key",
        )
        .into_response();
    };
    match state.billing.public(&key).await {
        Some(acct) => Json(acct).into_response(),
        None => err(StatusCode::UNAUTHORIZED, "Invalid API key", "invalid_api_key").into_response(),
    }
}

pub async fn prices(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(serde_json::json!({
        "object": "price_book",
        "price_book": state.billing.price_book,
        "unit": "credits",
        "canonical": ["identify", "prove", "usage"],
        "note": "Pay-as-you-go units. Stripe not wired (lab).",
        "saas_production": false
    }))
}

fn auth_key(headers: &HeaderMap) -> Result<String, axum::response::Response> {
    let Some(key) = extract_api_key(headers) else {
        return Err(err(
            StatusCode::UNAUTHORIZED,
            "Missing Authorization: Bearer <api_key>",
            "missing_api_key",
        )
        .into_response());
    };
    Ok(key)
}

pub async fn identify(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<IdentifyRequest>,
) -> impl IntoResponse {
    let key = match auth_key(&headers) {
        Ok(k) => k,
        Err(r) => return r,
    };
    if state.billing.get(&key).await.is_none() {
        return err(StatusCode::UNAUTHORIZED, "Invalid API key", "invalid_api_key").into_response();
    }

    let outcome = match run_identify(req) {
        Ok(o) => o,
        Err(e) => {
            return err(StatusCode::BAD_REQUEST, e.to_string(), "identify_failed").into_response();
        }
    };

    let units = state.billing.quote(
        outcome.firmware_bytes,
        outcome.mmio_events,
        outcome.contracts,
    );

    let (charged, remaining) = match state.billing.charge(&key, units).await {
        Ok(v) => v,
        Err(BillingError::InsufficientCredits { need, have }) => {
            return err(
                StatusCode::PAYMENT_REQUIRED,
                format!("insufficient credits: need {need}, have {have}"),
                "insufficient_credits",
            )
            .into_response();
        }
        Err(BillingError::InvalidKey) => {
            return err(StatusCode::UNAUTHORIZED, "Invalid API key", "invalid_api_key")
                .into_response();
        }
    };

    let draft = outcome.response_without_usage;
    Json(IdentifyResponse {
        id: draft.id,
        object: draft.object,
        label: draft.label,
        hardware: draft.hardware,
        by_contract: draft.by_contract,
        proof: draft.proof,
        usage: UsageBreakdown {
            firmware_bytes: outcome.firmware_bytes,
            mmio_events: outcome.mmio_events,
            contracts: outcome.contracts,
            units,
            credits_charged: charged,
            credits_remaining: remaining,
        },
        honesty: draft.honesty,
    })
    .into_response()
}

pub async fn prove(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<ProveRequest>,
) -> impl IntoResponse {
    let key = match auth_key(&headers) {
        Ok(k) => k,
        Err(r) => return r,
    };
    if state.billing.get(&key).await.is_none() {
        return err(StatusCode::UNAUTHORIZED, "Invalid API key", "invalid_api_key").into_response();
    }

    let outcome = match run_prove(req) {
        Ok(o) => o,
        Err(e) => {
            return err(StatusCode::BAD_REQUEST, e.to_string(), "prove_failed").into_response();
        }
    };

    let units = state.billing.quote(0, 0, outcome.contracts);

    let (charged, remaining) = match state.billing.charge(&key, units).await {
        Ok(v) => v,
        Err(BillingError::InsufficientCredits { need, have }) => {
            return err(
                StatusCode::PAYMENT_REQUIRED,
                format!("insufficient credits: need {need}, have {have}"),
                "insufficient_credits",
            )
            .into_response();
        }
        Err(BillingError::InvalidKey) => {
            return err(StatusCode::UNAUTHORIZED, "Invalid API key", "invalid_api_key")
                .into_response();
        }
    };

    let d = outcome.response_without_usage;
    Json(ProveResponse {
        id: d.id,
        object: d.object,
        label: d.label,
        contracts_proved: d.contracts_proved,
        contracts_total: d.contracts_total,
        all_satisfied: d.all_satisfied,
        backend: d.backend,
        results: d.results,
        deadlock: d.deadlock,
        usage: UsageBreakdown {
            firmware_bytes: 0,
            mmio_events: 0,
            contracts: outcome.contracts,
            units,
            credits_charged: charged,
            credits_remaining: remaining,
        },
        honesty: d.honesty,
    })
    .into_response()
}
