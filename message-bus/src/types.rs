//! Type definitions for message bus

use serde::{Deserialize, Serialize};

/// Message type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageType {
    /// Payment instruction
    PaymentInstruction,
    /// Netting proposal
    NettingProposal,
    /// Bank confirmation (ACK/NACK)
    BankConfirmation,
    /// Settlement finalization
    SettlementFinalize,
    /// Settlement proof
    SettlementProof,
    /// Checkpoint
    Checkpoint,
    /// Adapter transfer request
    AdapterTransfer,
    /// Adapter transfer response
    AdapterResponse,
    /// Risk assessment
    RiskAssessment,
    /// Compliance check
    ComplianceCheck,
    /// System event
    SystemEvent,
}

impl MessageType {
    /// Get NATS subject prefix for this message type
    pub fn subject_prefix(&self) -> &'static str {
        match self {
            MessageType::PaymentInstruction => "deltran.payment.instruction",
            MessageType::NettingProposal => "deltran.netting.proposal",
            MessageType::BankConfirmation => "deltran.bank.confirmation",
            MessageType::SettlementFinalize => "deltran.settlement.finalize",
            MessageType::SettlementProof => "deltran.settlement.proof",
            MessageType::Checkpoint => "deltran.checkpoint",
            MessageType::AdapterTransfer => "deltran.adapter.transfer",
            MessageType::AdapterResponse => "deltran.adapter.response",
            MessageType::RiskAssessment => "deltran.risk.assessment",
            MessageType::ComplianceCheck => "deltran.compliance.check",
            MessageType::SystemEvent => "deltran.system.event",
        }
    }

    /// Get JetStream stream name for this message type
    pub fn stream_name(&self) -> &'static str {
        match self {
            MessageType::PaymentInstruction => "PAYMENT_INSTRUCTIONS",
            MessageType::NettingProposal => "NETTING_PROPOSALS",
            MessageType::BankConfirmation => "BANK_CONFIRMATIONS",
            MessageType::SettlementFinalize => "SETTLEMENT_FINALIZATIONS",
            MessageType::SettlementProof => "SETTLEMENT_PROOFS",
            MessageType::Checkpoint => "CHECKPOINTS",
            MessageType::AdapterTransfer => "ADAPTER_TRANSFERS",
            MessageType::AdapterResponse => "ADAPTER_RESPONSES",
            MessageType::RiskAssessment => "RISK_ASSESSMENTS",
            MessageType::ComplianceCheck => "COMPLIANCE_CHECKS",
            MessageType::SystemEvent => "SYSTEM_EVENTS",
        }
    }
}

/// Partition key for routing messages
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PartitionKey {
    /// Partition by corridor ID
    Corridor(String),
    /// Partition by bank ID
    Bank(String),
    /// Partition by both corridor and bank
    CorridorBank {
        /// Corridor ID
        corridor_id: String,
        /// Bank ID
        bank_id: String,
    },
    /// Broadcast to all partitions
    Broadcast,
}

impl PartitionKey {
    /// Get partitioning string for NATS subject
    pub fn to_subject_segment(&self) -> String {
        match self {
            PartitionKey::Corridor(id) => format!("corridor.{}", sanitize_subject(id)),
            PartitionKey::Bank(id) => format!("bank.{}", sanitize_subject(id)),
            PartitionKey::CorridorBank { corridor_id, bank_id } => {
                format!(
                    "corridor.{}.bank.{}",
                    sanitize_subject(corridor_id),
                    sanitize_subject(bank_id)
                )
            }
            PartitionKey::Broadcast => "broadcast".to_string(),
        }
    }

    /// Compute partition number (0-31) for load balancing
    pub fn partition_number(&self, num_partitions: u32) -> u32 {
        let hash = match self {
            PartitionKey::Corridor(id) => blake3::hash(id.as_bytes()),
            PartitionKey::Bank(id) => blake3::hash(id.as_bytes()),
            PartitionKey::CorridorBank { corridor_id, bank_id } => {
                let combined = format!("{}:{}", corridor_id, bank_id);
                blake3::hash(combined.as_bytes())
            }
            PartitionKey::Broadcast => return 0, // Broadcast goes to partition 0
        };

        let hash_bytes = hash.as_bytes();
        let hash_u32 = u32::from_le_bytes([hash_bytes[0], hash_bytes[1], hash_bytes[2], hash_bytes[3]]);
        hash_u32 % num_partitions
    }
}

/// Sanitize string for use in NATS subject
fn sanitize_subject(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partition_key_subject() {
        let key = PartitionKey::Corridor("USD-EUR".to_string());
        assert_eq!(key.to_subject_segment(), "corridor.USD-EUR");

        let key = PartitionKey::Bank("BANKGB2L".to_string());
        assert_eq!(key.to_subject_segment(), "bank.BANKGB2L");

        let key = PartitionKey::CorridorBank {
            corridor_id: "USD-EUR".to_string(),
            bank_id: "BANKGB2L".to_string(),
        };
        assert_eq!(key.to_subject_segment(), "corridor.USD-EUR.bank.BANKGB2L");
    }

    #[test]
    fn test_partition_number() {
        let key = PartitionKey::Corridor("USD-EUR".to_string());
        let partition = key.partition_number(32);
        assert!(partition < 32);

        // Same key should always hash to same partition
        let partition2 = key.partition_number(32);
        assert_eq!(partition, partition2);

        // Different keys should likely hash to different partitions
        let key2 = PartitionKey::Corridor("GBP-USD".to_string());
        let partition3 = key2.partition_number(32);
        assert_ne!(partition, partition3);
    }

    #[test]
    fn test_sanitize_subject() {
        assert_eq!(sanitize_subject("BANK/GB2L"), "BANK_GB2L");
        assert_eq!(sanitize_subject("USD-EUR"), "USD-EUR");
        assert_eq!(sanitize_subject("test@123"), "test_123");
    }
}
