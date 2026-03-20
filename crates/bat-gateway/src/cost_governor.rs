use anyhow::Result;
use chrono::{DateTime, Utc, TimeZone, Datelike};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

use bat_types::config::{ModelInfo, RoutingStrategy};
use crate::db::Database;

/// Daily usage tracking for cost governance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyUsage {
    pub date: String, // YYYY-MM-DD format
    pub total_cost_usd: f32,
    pub model_usage: HashMap<String, ModelUsage>,
}

/// Usage statistics for a specific model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_cost_usd: f32,
    pub request_count: u32,
}

/// Session usage tracking for per-session budgets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionUsage {
    pub session_id: uuid::Uuid,
    pub total_cost_usd: f32,
    pub model_usage: HashMap<String, ModelUsage>,
    pub started_at: DateTime<Utc>,
}

/// Cost governor for managing token usage and budget enforcement.
pub struct CostGovernor {
    db: std::sync::Arc<Database>,
}

impl CostGovernor {
    pub fn new(db: std::sync::Arc<Database>) -> Self {
        Self { db }
    }

    /// Record token usage for a model and calculate cost.
    pub async fn record_usage(
        &self,
        session_id: uuid::Uuid,
        model: &str,
        input_tokens: u32,
        output_tokens: u32,
    ) -> Result<f32> {
        let model_info = ModelInfo::for_model(model);
        let cost = if let Some(info) = model_info {
            let input_cost = (input_tokens as f32 / 1000.0) * info.cost_per_1k_input;
            let output_cost = (output_tokens as f32 / 1000.0) * info.cost_per_1k_output;
            input_cost + output_cost
        } else {
            0.0 // Unknown models assumed free
        };

        // Record in database
        self.db.record_token_usage(session_id, model, input_tokens, output_tokens, cost)?;
        
        debug!(
            "Recorded usage: session={} model={} input_tokens={} output_tokens={} cost=${:.4}",
            session_id, model, input_tokens, output_tokens, cost
        );

        Ok(cost)
    }

    /// Get current daily usage.
    pub fn get_daily_usage(&self) -> Result<DailyUsage> {
        let today = Utc::now().format("%Y-%m-%d").to_string();
        
        match self.db.get_daily_usage(&today)? {
            Some(usage) => Ok(usage),
            None => Ok(DailyUsage {
                date: today,
                total_cost_usd: 0.0,
                model_usage: HashMap::new(),
            })
        }
    }

    /// Get current session usage.
    pub fn get_session_usage(&self, session_id: uuid::Uuid) -> Result<SessionUsage> {
        match self.db.get_session_usage(session_id)? {
            Some(usage) => Ok(usage),
            None => Ok(SessionUsage {
                session_id,
                total_cost_usd: 0.0,
                model_usage: HashMap::new(),
                started_at: Utc::now(),
            })
        }
    }

    /// Check if a request should be allowed given current budgets.
    /// Returns true if the request is within budget, false if it should be blocked.
    pub fn check_budget_limits(
        &self,
        daily_budget: Option<f32>,
        session_budget: Option<f32>,
        session_id: uuid::Uuid,
        estimated_cost: f32,
    ) -> Result<BudgetStatus> {
        let daily_usage = self.get_daily_usage()?;
        let session_usage = self.get_session_usage(session_id)?;

        // Check daily budget
        if let Some(daily_limit) = daily_budget {
            let projected_daily = daily_usage.total_cost_usd + estimated_cost;
            if projected_daily > daily_limit {
                return Ok(BudgetStatus::DailyLimitExceeded {
                    current: daily_usage.total_cost_usd,
                    limit: daily_limit,
                    projected: projected_daily,
                });
            }
            
            // Warn when approaching limit (80% of budget)
            if projected_daily > daily_limit * 0.8 && daily_usage.total_cost_usd <= daily_limit * 0.8 {
                warn!(
                    "Daily budget approaching limit: ${:.2} / ${:.2} (80% threshold crossed)",
                    projected_daily, daily_limit
                );
            }
        }

        // Check session budget
        if let Some(session_limit) = session_budget {
            let projected_session = session_usage.total_cost_usd + estimated_cost;
            if projected_session > session_limit {
                return Ok(BudgetStatus::SessionLimitExceeded {
                    current: session_usage.total_cost_usd,
                    limit: session_limit,
                    projected: projected_session,
                });
            }
            
            // Warn when approaching limit (80% of budget)
            if projected_session > session_limit * 0.8 && session_usage.total_cost_usd <= session_limit * 0.8 {
                warn!(
                    "Session budget approaching limit: ${:.2} / ${:.2} (80% threshold crossed)",
                    projected_session, session_limit
                );
            }
        }

        Ok(BudgetStatus::WithinLimits)
    }

    /// Determine if model selection should be downgraded due to budget constraints.
    pub fn should_downgrade_for_budget(
        &self,
        daily_budget: Option<f32>,
        session_budget: Option<f32>,
        session_id: uuid::Uuid,
    ) -> Result<bool> {
        let daily_usage = self.get_daily_usage()?;
        let session_usage = self.get_session_usage(session_id)?;

        // Downgrade if we're at 80% of either budget
        if let Some(daily_limit) = daily_budget {
            if daily_usage.total_cost_usd >= daily_limit * 0.8 {
                info!("Downgrading model selection due to daily budget constraint: ${:.2} / ${:.2}",
                      daily_usage.total_cost_usd, daily_limit);
                return Ok(true);
            }
        }

        if let Some(session_limit) = session_budget {
            if session_usage.total_cost_usd >= session_limit * 0.8 {
                info!("Downgrading model selection due to session budget constraint: ${:.2} / ${:.2}",
                      session_usage.total_cost_usd, session_limit);
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Get usage statistics for the metrics dashboard.
    pub fn get_usage_stats(&self) -> Result<UsageStats> {
        let daily_usage = self.get_daily_usage()?;
        let weekly_usage = self.get_weekly_usage()?;
        let monthly_usage = self.get_monthly_usage()?;

        Ok(UsageStats {
            daily_usage,
            weekly_usage,
            monthly_usage,
        })
    }

    /// Get usage for the past 7 days.
    fn get_weekly_usage(&self) -> Result<Vec<DailyUsage>> {
        let mut usage_data = Vec::new();
        let today = Utc::now();

        for days_back in 0..7 {
            let date = today - chrono::Duration::days(days_back);
            let date_str = date.format("%Y-%m-%d").to_string();
            
            let daily_usage = match self.db.get_daily_usage(&date_str)? {
                Some(usage) => usage,
                None => DailyUsage {
                    date: date_str,
                    total_cost_usd: 0.0,
                    model_usage: HashMap::new(),
                }
            };
            
            usage_data.push(daily_usage);
        }

        usage_data.reverse(); // Oldest to newest
        Ok(usage_data)
    }

    /// Get usage for the current month.
    fn get_monthly_usage(&self) -> Result<f32> {
        let today = Utc::now();
        let month_start = Utc.with_ymd_and_hms(today.year(), today.month(), 1, 0, 0, 0)
            .single()
            .unwrap_or_else(|| Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc());
        self.db.get_usage_since(month_start)
    }
}

/// Status of budget check for a request.
#[derive(Debug, Clone, PartialEq)]
pub enum BudgetStatus {
    WithinLimits,
    DailyLimitExceeded {
        current: f32,
        limit: f32,
        projected: f32,
    },
    SessionLimitExceeded {
        current: f32,
        limit: f32,
        projected: f32,
    },
}

/// Usage statistics for the dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub daily_usage: DailyUsage,
    pub weekly_usage: Vec<DailyUsage>,
    pub monthly_usage: f32,
}

impl ModelUsage {
    pub fn new() -> Self {
        Self {
            input_tokens: 0,
            output_tokens: 0,
            total_cost_usd: 0.0,
            request_count: 0,
        }
    }

    pub fn add_usage(&mut self, input_tokens: u32, output_tokens: u32, cost: f32) {
        self.input_tokens += input_tokens;
        self.output_tokens += output_tokens;
        self.total_cost_usd += cost;
        self.request_count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_usage_accumulation() {
        let mut usage = ModelUsage::new();
        
        usage.add_usage(1000, 500, 0.05);
        usage.add_usage(2000, 800, 0.08);
        
        assert_eq!(usage.input_tokens, 3000);
        assert_eq!(usage.output_tokens, 1300);
        assert_eq!(usage.total_cost_usd, 0.13);
        assert_eq!(usage.request_count, 2);
    }

    #[test]
    fn test_budget_status_within_limits() {
        // Test with no budget constraints
        let status = BudgetStatus::WithinLimits;
        assert_eq!(status, BudgetStatus::WithinLimits);
    }
}