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
}
