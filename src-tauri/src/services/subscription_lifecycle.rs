use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubscriptionLifecycleStatus {
    Active,
    GraceActive,
    PendingInstallation,
    InstallationDoneAwaitingPayment,
    Suspended,
    Cancelled,
}

impl SubscriptionLifecycleStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::GraceActive => "grace_active",
            Self::PendingInstallation => "pending_installation",
            Self::InstallationDoneAwaitingPayment => "installation_done_awaiting_payment",
            Self::Suspended => "suspended",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(input: &str) -> Result<Self, LifecycleTransitionError> {
        match input.trim().to_ascii_lowercase().as_str() {
            "active" => Ok(Self::Active),
            "grace_active" => Ok(Self::GraceActive),
            "pending_installation" => Ok(Self::PendingInstallation),
            "installation_done_awaiting_payment" => Ok(Self::InstallationDoneAwaitingPayment),
            "suspended" => Ok(Self::Suspended),
            "cancelled" => Ok(Self::Cancelled),
            other => Err(LifecycleTransitionError::UnknownStatus(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubscriptionLifecycleEvent {
    OrderRequested,
    PaymentPaid,
    InstallationCompleted,
    Cancel,
    Reopen,
}

impl SubscriptionLifecycleEvent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OrderRequested => "order_requested",
            Self::PaymentPaid => "payment_paid",
            Self::InstallationCompleted => "installation_completed",
            Self::Cancel => "cancel",
            Self::Reopen => "reopen",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleTransitionError {
    UnknownStatus(String),
    IllegalTransition {
        from: SubscriptionLifecycleStatus,
        event: SubscriptionLifecycleEvent,
    },
}

impl fmt::Display for LifecycleTransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownStatus(status) => write!(f, "unknown subscription status: {status}"),
            Self::IllegalTransition { from, event } => write!(
                f,
                "illegal lifecycle transition: {} -> {}",
                from.as_str(),
                event.as_str()
            ),
        }
    }
}

pub fn transition_status(
    current: SubscriptionLifecycleStatus,
    event: SubscriptionLifecycleEvent,
) -> Result<SubscriptionLifecycleStatus, LifecycleTransitionError> {
    match event {
        SubscriptionLifecycleEvent::OrderRequested => match current {
            SubscriptionLifecycleStatus::Cancelled => {
                Err(LifecycleTransitionError::IllegalTransition {
                    from: current,
                    event,
                })
            }
            _ => Ok(SubscriptionLifecycleStatus::PendingInstallation),
        },
        SubscriptionLifecycleEvent::Cancel => match current {
            SubscriptionLifecycleStatus::Cancelled => {
                Err(LifecycleTransitionError::IllegalTransition {
                    from: current,
                    event,
                })
            }
            _ => Ok(SubscriptionLifecycleStatus::Cancelled),
        },
        SubscriptionLifecycleEvent::Reopen => match current {
            SubscriptionLifecycleStatus::Cancelled => {
                Ok(SubscriptionLifecycleStatus::PendingInstallation)
            }
            _ => Err(LifecycleTransitionError::IllegalTransition {
                from: current,
                event,
            }),
        },
        SubscriptionLifecycleEvent::PaymentPaid
        | SubscriptionLifecycleEvent::InstallationCompleted => match current {
            SubscriptionLifecycleStatus::Cancelled => {
                Err(LifecycleTransitionError::IllegalTransition {
                    from: current,
                    event,
                })
            }
            _ => Ok(current),
        },
    }
}

pub fn resolve_activation_status(
    current: SubscriptionLifecycleStatus,
    installation_completed: bool,
    payment_paid: bool,
) -> Result<SubscriptionLifecycleStatus, LifecycleTransitionError> {
    let event = if payment_paid {
        SubscriptionLifecycleEvent::PaymentPaid
    } else {
        SubscriptionLifecycleEvent::InstallationCompleted
    };
    let _ = transition_status(current, event)?;

    if matches!(
        current,
        SubscriptionLifecycleStatus::Active | SubscriptionLifecycleStatus::GraceActive
    ) && payment_paid
    {
        return Ok(SubscriptionLifecycleStatus::Active);
    }

    if installation_completed && payment_paid {
        Ok(SubscriptionLifecycleStatus::Active)
    } else if installation_completed {
        Ok(SubscriptionLifecycleStatus::GraceActive)
    } else {
        Ok(SubscriptionLifecycleStatus::PendingInstallation)
    }
}

pub fn should_disable_pppoe_for_subscription_status(status: &str) -> bool {
    !matches!(
        SubscriptionLifecycleStatus::parse(status),
        Ok(SubscriptionLifecycleStatus::Active | SubscriptionLifecycleStatus::GraceActive)
    )
}

#[cfg(test)]
mod tests {
    use super::{
        resolve_activation_status, should_disable_pppoe_for_subscription_status, transition_status,
        LifecycleTransitionError, SubscriptionLifecycleEvent, SubscriptionLifecycleStatus,
    };

    #[test]
    fn cancel_and_reopen_guards_work() {
        let cancelled = transition_status(
            SubscriptionLifecycleStatus::PendingInstallation,
            SubscriptionLifecycleEvent::Cancel,
        )
        .expect("cancel should be allowed from pending_installation");
        assert_eq!(cancelled, SubscriptionLifecycleStatus::Cancelled);

        let reopened = transition_status(cancelled, SubscriptionLifecycleEvent::Reopen)
            .expect("reopen should be allowed from cancelled");
        assert_eq!(reopened, SubscriptionLifecycleStatus::PendingInstallation);

        let illegal = transition_status(
            SubscriptionLifecycleStatus::Active,
            SubscriptionLifecycleEvent::Reopen,
        )
        .expect_err("reopen from active must be rejected");
        assert!(matches!(
            illegal,
            LifecycleTransitionError::IllegalTransition {
                from: SubscriptionLifecycleStatus::Active,
                event: SubscriptionLifecycleEvent::Reopen
            }
        ));
    }

    #[test]
    fn activation_resolution_paid_before_install_keeps_pending_installation() {
        let target = resolve_activation_status(
            SubscriptionLifecycleStatus::PendingInstallation,
            false,
            true,
        )
        .expect("paid-before-install should resolve to pending_installation");
        assert_eq!(target, SubscriptionLifecycleStatus::PendingInstallation);
    }

    #[test]
    fn activation_resolution_install_done_unpaid_waits_payment() {
        let target = resolve_activation_status(
            SubscriptionLifecycleStatus::PendingInstallation,
            true,
            false,
        )
        .expect("install-complete unpaid should resolve to grace_active");
        assert_eq!(target, SubscriptionLifecycleStatus::GraceActive);
    }

    #[test]
    fn activation_resolution_install_done_and_paid_is_active() {
        let target =
            resolve_activation_status(SubscriptionLifecycleStatus::PendingInstallation, true, true)
                .expect("install-complete paid should resolve to active");
        assert_eq!(target, SubscriptionLifecycleStatus::Active);
    }

    #[test]
    fn activation_resolution_paid_grace_subscription_stays_active() {
        let target =
            resolve_activation_status(SubscriptionLifecycleStatus::GraceActive, true, true)
                .expect("paid grace subscription should resolve to active");
        assert_eq!(target, SubscriptionLifecycleStatus::Active);
    }

    #[test]
    fn pppoe_disable_policy_matches_subscription_lifecycle_states() {
        assert!(!should_disable_pppoe_for_subscription_status("active"));
        assert!(!should_disable_pppoe_for_subscription_status(
            "grace_active"
        ));
        assert!(should_disable_pppoe_for_subscription_status(
            "pending_installation"
        ));
        assert!(should_disable_pppoe_for_subscription_status(
            "installation_done_awaiting_payment"
        ));
        assert!(should_disable_pppoe_for_subscription_status("suspended"));
        assert!(should_disable_pppoe_for_subscription_status("cancelled"));
    }
}
