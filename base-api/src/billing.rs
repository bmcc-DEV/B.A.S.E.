//! B.A.S.E. Identify API — pay-as-you-go metering (lab).
//! Honesty: `saas_production: false` · Stripe/outros PSPs = plug futuro.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Preços em “units” (como tokens de IA).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceBook {
    /// Units por KiB de firmware (ceil)
    pub per_kib_firmware: u64,
    /// Units por evento MMIO
    pub per_mmio_event: u64,
    /// Units por contrato temporal / requirement
    pub per_contract: u64,
}

impl Default for PriceBook {
    fn default() -> Self {
        Self {
            per_kib_firmware: 1,
            per_mmio_event: 2,
            per_contract: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageBreakdown {
    pub firmware_bytes: u64,
    pub mmio_events: u64,
    pub contracts: u64,
    pub units: u64,
    pub credits_charged: u64,
    pub credits_remaining: u64,
}

#[derive(Debug, Clone)]
pub struct Account {
    pub api_key: String,
    pub label: String,
    pub credits: u64,
    pub total_units: u64,
    pub request_count: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AccountPublic {
    pub label: String,
    pub credits: u64,
    pub total_units: u64,
    pub request_count: u64,
    pub price_book: PriceBook,
    pub saas_production: bool,
}

#[derive(Clone)]
pub struct BillingState {
    inner: Arc<RwLock<HashMap<String, Account>>>,
    pub price_book: PriceBook,
}

impl BillingState {
    pub fn new_dev(seed_key: String, seed_credits: u64) -> Self {
        let mut map = HashMap::new();
        map.insert(
            seed_key.clone(),
            Account {
                api_key: seed_key,
                label: "dev".into(),
                credits: seed_credits,
                total_units: 0,
                request_count: 0,
            },
        );
        Self {
            inner: Arc::new(RwLock::new(map)),
            price_book: PriceBook::default(),
        }
    }

    pub fn quote(
        &self,
        firmware_bytes: u64,
        mmio_events: u64,
        contracts: u64,
    ) -> u64 {
        let kib = if firmware_bytes == 0 {
            0
        } else {
            firmware_bytes.div_ceil(1024).max(1)
        };
        kib * self.price_book.per_kib_firmware
            + mmio_events * self.price_book.per_mmio_event
            + contracts * self.price_book.per_contract
    }

    pub async fn get(&self, key: &str) -> Option<Account> {
        self.inner.read().await.get(key).cloned()
    }

    /// Reserva e debita units. Err = créditos insuficientes.
    pub async fn charge(
        &self,
        key: &str,
        units: u64,
    ) -> Result<(u64, u64), BillingError> {
        let mut map = self.inner.write().await;
        let acct = map.get_mut(key).ok_or(BillingError::InvalidKey)?;
        if acct.credits < units {
            return Err(BillingError::InsufficientCredits {
                need: units,
                have: acct.credits,
            });
        }
        acct.credits -= units;
        acct.total_units += units;
        acct.request_count += 1;
        Ok((units, acct.credits))
    }

    pub async fn public(&self, key: &str) -> Option<AccountPublic> {
        let a = self.get(key).await?;
        Some(AccountPublic {
            label: a.label,
            credits: a.credits,
            total_units: a.total_units,
            request_count: a.request_count,
            price_book: self.price_book.clone(),
            saas_production: false,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BillingError {
    #[error("invalid api key")]
    InvalidKey,
    #[error("insufficient credits: need {need}, have {have}")]
    InsufficientCredits { need: u64, have: u64 },
}
