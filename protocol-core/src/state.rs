//! Protocol state machine
//!
//! Enforces valid state transitions and 2-phase commit semantics.

use crate::{Error, Result};
use serde::{Deserialize, Serialize};

/// Protocol state (maps to protobuf ProtocolState enum)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ProtocolState {
    /// Payment initiated
    PaymentInitiated = 1,
    /// Validation passed
    PaymentValidated = 2,
    /// Eligibility confirmed (both tokens valid)
    PaymentEligibilityConfirmed = 3,
    /// Proposed for netting
    PaymentNettingProposed = 4,

    /// Netting approved by all banks
    NettingApproved = 10,
    /// 2PC PREPARE phase
    SettlementPending = 11,
    /// 2PC COMMIT phase
    SettlementFinalized = 12,
    /// Proof generated and signed
    ProofGenerated = 13,

    /// Validation/sanctions failed
    PaymentRejected = 20,
    /// Netting ACK timeout
    NettingTimeout = 21,
    /// Partial settlement (some banks failed)
    SettlementPartial = 22,
    /// Full rollback
    SettlementFailed = 23,
}

impl ProtocolState {
    /// Check if transition is valid
    pub fn can_transition_to(&self, next: ProtocolState) -> bool {
        use ProtocolState::*;

        matches!(
            (self, next),
            // Payment flow
            | (PaymentInitiated, PaymentValidated)
            | (PaymentInitiated, PaymentRejected)
            | (PaymentValidated, PaymentEligibilityConfirmed)
            | (PaymentValidated, PaymentRejected)
            | (PaymentEligibilityConfirmed, PaymentNettingProposed)
            | (PaymentEligibilityConfirmed, PaymentRejected)

            // Netting flow
            | (PaymentNettingProposed, NettingApproved)
            | (PaymentNettingProposed, NettingTimeout)
            | (NettingApproved, SettlementPending)

            // 2PC flow
            | (SettlementPending, SettlementFinalized)
            | (SettlementPending, SettlementPartial)
            | (SettlementPending, SettlementFailed)
            | (SettlementFinalized, ProofGenerated)

            // Retry paths
            | (NettingTimeout, PaymentNettingProposed)  // Requeue
            | (SettlementPartial, ProofGenerated)       // Partial proof

            // Terminal states can't transition
        )
    }

    /// Check if state is terminal
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ProtocolState::PaymentRejected
                | ProtocolState::SettlementFailed
                | ProtocolState::ProofGenerated
        )
    }

    /// Check if state requires bank confirmations
    pub fn requires_bank_confirmations(&self) -> bool {
        matches!(
            self,
            ProtocolState::PaymentNettingProposed | ProtocolState::SettlementPending
        )
    }
}

/// State machine for protocol enforcement
#[derive(Debug)]
pub struct StateMachine {
    current: ProtocolState,
}

impl StateMachine {
    /// Create new state machine at initial state
    pub fn new() -> Self {
        Self {
            current: ProtocolState::PaymentInitiated,
        }
    }

    /// Create state machine at specific state
    pub fn at_state(state: ProtocolState) -> Self {
        Self { current: state }
    }

    /// Get current state
    pub fn current(&self) -> ProtocolState {
        self.current
    }

    /// Transition to next state
    pub fn transition(&mut self, next: ProtocolState) -> Result<()> {
        if self.current.is_terminal() {
            return Err(Error::InvalidStateTransition {
                from: format!("{:?}", self.current),
                to: format!("{:?}", next),
                reason: "current state is terminal".to_string(),
            });
        }

        if !self.current.can_transition_to(next) {
            return Err(Error::InvalidStateTransition {
                from: format!("{:?}", self.current),
                to: format!("{:?}", next),
                reason: "transition not allowed by state machine".to_string(),
            });
        }

        self.current = next;
        Ok(())
    }

    /// Force transition (for testing/recovery)
    pub fn force_transition(&mut self, next: ProtocolState) {
        self.current = next;
    }
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_payment_flow() {
        let mut sm = StateMachine::new();

        // Happy path
        assert!(sm.transition(ProtocolState::PaymentValidated).is_ok());
        assert!(sm
            .transition(ProtocolState::PaymentEligibilityConfirmed)
            .is_ok());
        assert!(sm
            .transition(ProtocolState::PaymentNettingProposed)
            .is_ok());
        assert!(sm.transition(ProtocolState::NettingApproved).is_ok());
        assert!(sm.transition(ProtocolState::SettlementPending).is_ok());
        assert!(sm.transition(ProtocolState::SettlementFinalized).is_ok());
        assert!(sm.transition(ProtocolState::ProofGenerated).is_ok());

        // Terminal state
        assert!(sm.current().is_terminal());
        assert!(sm.transition(ProtocolState::PaymentInitiated).is_err());
    }

    #[test]
    fn test_rejection_flow() {
        let mut sm = StateMachine::new();

        assert!(sm.transition(ProtocolState::PaymentValidated).is_ok());
        assert!(sm.transition(ProtocolState::PaymentRejected).is_ok());

        // Terminal state
        assert!(sm.current().is_terminal());
    }

    #[test]
    fn test_partial_settlement_flow() {
        let mut sm = StateMachine::at_state(ProtocolState::SettlementPending);

        assert!(sm.transition(ProtocolState::SettlementPartial).is_ok());
        assert!(sm.transition(ProtocolState::ProofGenerated).is_ok());
    }

    #[test]
    fn test_invalid_transition() {
        let mut sm = StateMachine::new();

        // Can't jump directly to settlement
        assert!(sm.transition(ProtocolState::SettlementFinalized).is_err());
    }

    #[test]
    fn test_netting_timeout_retry() {
        let mut sm = StateMachine::at_state(ProtocolState::PaymentNettingProposed);

        assert!(sm.transition(ProtocolState::NettingTimeout).is_ok());
        assert!(sm
            .transition(ProtocolState::PaymentNettingProposed)
            .is_ok()); // Requeue
    }
}