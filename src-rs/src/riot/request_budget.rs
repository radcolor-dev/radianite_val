use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scenario {
    RiotClientOnly,
    ValorantMenus,
    UnchangedLiveMatch,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RequestBudget {
    pub local_riot: u64,
    pub authenticated_riot: u64,
    pub public_https: u64,
    pub tauri_ipc_reads: u64,
}

impl RequestBudget {
    pub fn baseline(scenario: Scenario, window: Duration) -> Self {
        let frontend_refreshes = window.as_secs() / 5;
        let mut budget = Self {
            tauri_ipc_reads: frontend_refreshes * 4,
            ..Self::default()
        };

        match scenario {
            Scenario::RiotClientOnly => {
                budget.local_riot = window.as_secs();
            }
            Scenario::ValorantMenus | Scenario::UnchangedLiveMatch => {
                let core_ticks = window.as_secs() / 2;
                budget.local_riot = core_ticks * 4;
                budget.authenticated_riot = (window.as_secs() / 30) * 2 + 1;
                budget.public_https = 4;

                if scenario == Scenario::UnchangedLiveMatch {
                    budget.authenticated_riot += core_ticks * 2;
                }
            }
        }

        budget
    }

    pub fn optimized_warm(scenario: Scenario, window: Duration) -> Self {
        let seconds = window.as_secs();
        match scenario {
            Scenario::RiotClientOnly => Self {
                local_riot: seconds / 3,
                ..Self::default()
            },
            Scenario::ValorantMenus => {
                let ticks = seconds / 3;
                Self {
                    local_riot: ticks
                        + seconds / 12
                        + div_ceil(seconds, 300)
                        + div_ceil(seconds, 600),
                    authenticated_riot: 1 + div_ceil(seconds, 300) * 2,
                    public_https: 0,
                    tauri_ipc_reads: 0,
                }
            }
            Scenario::UnchangedLiveMatch => {
                let ticks = seconds / 2;
                Self {
                    local_riot: ticks
                        + seconds / 10
                        + div_ceil(seconds, 300)
                        + div_ceil(seconds, 600),
                    authenticated_riot: 4,
                    public_https: 0,
                    tauri_ipc_reads: 0,
                }
            }
        }
    }
}

fn div_ceil(value: u64, divisor: u64) -> u64 {
    value.div_ceil(divisor)
}

#[cfg(test)]
mod tests {
    use super::{RequestBudget, Scenario};
    use std::time::Duration;

    const TEN_MINUTES: Duration = Duration::from_secs(10 * 60);

    #[test]
    fn captures_riot_client_only_baseline() {
        assert_eq!(
            RequestBudget::baseline(Scenario::RiotClientOnly, TEN_MINUTES),
            RequestBudget {
                local_riot: 600,
                authenticated_riot: 0,
                public_https: 0,
                tauri_ipc_reads: 480,
            }
        );
    }

    #[test]
    fn captures_live_match_baseline() {
        assert_eq!(
            RequestBudget::baseline(Scenario::UnchangedLiveMatch, TEN_MINUTES),
            RequestBudget {
                local_riot: 1_200,
                authenticated_riot: 641,
                public_https: 4,
                tauri_ipc_reads: 480,
            }
        );
    }

    #[test]
    fn captures_valorant_menus_baseline() {
        assert_eq!(
            RequestBudget::baseline(Scenario::ValorantMenus, TEN_MINUTES),
            RequestBudget {
                local_riot: 1_200,
                authenticated_riot: 41,
                public_https: 4,
                tauri_ipc_reads: 480,
            }
        );
    }

    #[test]
    fn captures_optimized_warm_budgets() {
        assert_eq!(
            RequestBudget::optimized_warm(Scenario::RiotClientOnly, TEN_MINUTES),
            RequestBudget {
                local_riot: 200,
                authenticated_riot: 0,
                public_https: 0,
                tauri_ipc_reads: 0,
            }
        );
        assert_eq!(
            RequestBudget::optimized_warm(Scenario::ValorantMenus, TEN_MINUTES),
            RequestBudget {
                local_riot: 253,
                authenticated_riot: 5,
                public_https: 0,
                tauri_ipc_reads: 0,
            }
        );
        assert_eq!(
            RequestBudget::optimized_warm(Scenario::UnchangedLiveMatch, TEN_MINUTES),
            RequestBudget {
                local_riot: 363,
                authenticated_riot: 4,
                public_https: 0,
                tauri_ipc_reads: 0,
            }
        );
    }
}
