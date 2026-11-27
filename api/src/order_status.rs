use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    Pending,
    Confirmed,
    Preparing,
    Delivering,
    Completed,
    Cancelled,
}

impl OrderStatus {
    pub fn can_transition_to(self, next: OrderStatus) -> bool {
        use OrderStatus::*;
        matches!(
            (self, next),
            (Pending, Confirmed)
                | (Pending, Cancelled)
                | (Confirmed, Preparing)
                | (Confirmed, Cancelled)
                | (Preparing, Delivering)
                | (Delivering, Completed)
        )
    }
    pub fn ensure_can_transition_to(self, next: OrderStatus) -> Result<(), AppError> {
        if self.can_transition_to(next) {
            Ok(())
        } else {
            Err(AppError::InvalidTransition(format!(
                "không thể chuyển từ '{self:?}' sang '{next:?}'"
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    const ALL: [OrderStatus; 6] = [
        OrderStatus::Pending,
        OrderStatus::Confirmed,
        OrderStatus::Preparing,
        OrderStatus::Delivering,
        OrderStatus::Completed,
        OrderStatus::Cancelled,
    ];

    #[test]
    fn only_intended_transitions_are_allowed() {
        use OrderStatus::*;
        let allowed: HashSet<(OrderStatus, OrderStatus)> = [
            (Pending, Confirmed),
            (Pending, Cancelled),
            (Confirmed, Preparing),
            (Confirmed, Cancelled),
            (Preparing, Delivering),
            (Delivering, Completed),
        ]
        .into_iter()
        .collect();

        for &from in &ALL {
            for &to in &ALL {
                let expected = allowed.contains(&(from, to));
                assert_eq!(
                    from.can_transition_to(to),
                    expected,
                    "sai ở cặp {from:?} -> {to:?}"
                );
            }
        }
    }

    #[test]
    fn terminal_states_have_no_outgoing_transition() {
        for &to in &ALL {
            assert!(!OrderStatus::Completed.can_transition_to(to));
            assert!(!OrderStatus::Cancelled.can_transition_to(to));
        }
    }

    #[test]
    fn ensure_can_transition_to_returns_ok_for_valid_transition() {
        assert!(
            OrderStatus::Pending
                .ensure_can_transition_to(OrderStatus::Confirmed)
                .is_ok()
        );
    }

    #[test]
    fn ensure_can_transition_to_returns_err_for_invalid_transition() {
        let err = OrderStatus::Pending
            .ensure_can_transition_to(OrderStatus::Delivering)
            .unwrap_err();
        assert!(matches!(err, AppError::InvalidTransition(_)));
    }
}
