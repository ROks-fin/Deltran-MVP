from datetime import datetime
from decimal import Decimal
from enum import Enum
from typing import List, Optional
from uuid import UUID

from pydantic import BaseModel, Field, validator


class AccountType(str, Enum):
    CHECKING = "CHECKING"
    SAVINGS = "SAVINGS"
    CORPORATE = "CORPORATE"
    NOSTRO = "NOSTRO"
    VOSTRO = "VOSTRO"


class SettlementMethod(str, Enum):
    INSTANT = "INSTANT"
    PVP = "PVP"
    NETTING = "NETTING"
    CORRESPONDENT = "CORRESPONDENT"


class PaymentCategory(str, Enum):
    TRADE = "TRADE"
    SERVICES = "SERVICES"
    INVESTMENT = "INVESTMENT"
    PERSONAL = "PERSONAL"
    GOVERNMENT = "GOVERNMENT"
    CHARITY = "CHARITY"
    PENSION = "PENSION"
    TAX = "TAX"


class ScreeningStatus(str, Enum):
    PASS = "PASS"
    FAIL = "FAIL"
    REVIEW = "REVIEW"


class SanctionsList(str, Enum):
    OFAC = "OFAC"
    UN = "UN"
    EU = "EU"
    HMT = "HMT"
    CONSOLIDATED = "CONSOLIDATED"


class RiskCategory(str, Enum):
    LOW = "LOW"
    MEDIUM = "MEDIUM"
    HIGH = "HIGH"


class TransactionStatus(str, Enum):
    INITIATED = "INITIATED"
    VALIDATED = "VALIDATED"
    SCREENED = "SCREENED"
    APPROVED = "APPROVED"
    REJECTED = "REJECTED"
    SETTLED = "SETTLED"
    COMPLETED = "COMPLETED"
    FAILED = "FAILED"
    CANCELLED = "CANCELLED"


class RiskFactor(str, Enum):
    HIGH_VALUE = "HIGH_VALUE"
    HIGH_RISK_COUNTRY = "HIGH_RISK_COUNTRY"
    NEW_COUNTERPARTY = "NEW_COUNTERPARTY"
    UNUSUAL_PATTERN = "UNUSUAL_PATTERN"
    SANCTIONS_PROXIMITY = "SANCTIONS_PROXIMITY"
    PEP_INVOLVEMENT = "PEP_INVOLVEMENT"


class RiskMitigation(str, Enum):
    ENHANCED_SCREENING = "ENHANCED_SCREENING"
    MANUAL_REVIEW = "MANUAL_REVIEW"
    ADDITIONAL_DOCS = "ADDITIONAL_DOCS"


class ChargeType(str, Enum):
    CORRESPONDENT = "CORRESPONDENT"
    INTERMEDIARY = "INTERMEDIARY"
    BENEFICIARY = "BENEFICIARY"
    REGULATORY = "REGULATORY"


class Address(BaseModel):
    street_name: Optional[str] = Field(None, max_length=70)
    building_number: Optional[str] = Field(None, max_length=16)
    post_code: Optional[str] = Field(None, max_length=16)
    town_name: Optional[str] = Field(None, max_length=35)
    country_subdivision: Optional[str] = Field(None, max_length=35)
    country: Optional[str] = Field(None, regex="^[A-Z]{2}$")


class Amount(BaseModel):
    value: str = Field(..., regex="^[0-9]+\\.[0-9]{2}$")
    currency: str = Field(..., regex="^[A-Z]{3}$")

    @validator('value')
    def validate_amount(cls, v):
        try:
            decimal_value = Decimal(v)
            if decimal_value <= 0:
                raise ValueError("Amount must be positive")
            return v
        except:
            raise ValueError("Invalid decimal format")


class Account(BaseModel):
    iban: str = Field(..., regex="^[A-Z]{2}[0-9]{2}[A-Z0-9]{1,30}$")
    account_type: Optional[AccountType] = None


class Agent(BaseModel):
    bic: str = Field(..., regex="^[A-Z]{4}[A-Z]{2}[A-Z0-9]{2}([A-Z0-9]{3})?$")
    name: Optional[str] = Field(None, max_length=140)
    clearing_system_id: Optional[str] = Field(None, max_length=35)


class Identification(BaseModel):
    organization_id: Optional[str] = Field(None, max_length=35)
    private_id: Optional[str] = Field(None, max_length=35)


class Party(BaseModel):
    name: str = Field(..., max_length=140)
    account: Account
    agent: Agent
    address: Optional[Address] = None
    identification: Optional[Identification] = None


class PaymentPurpose(BaseModel):
    category: PaymentCategory
    subcategory: Optional[str] = Field(None, max_length=35)
    description: Optional[str] = Field(None, max_length=140)


class TravelRuleInfo(BaseModel):
    name: Optional[str] = Field(None, max_length=140)
    account: Optional[str] = Field(None, max_length=34)
    address: Optional[Address] = None
    id_number: Optional[str] = Field(None, max_length=35)


class TravelRule(BaseModel):
    threshold_met: bool
    threshold_amount: Optional[str] = Field(None, regex="^[0-9]+\\.[0-9]{2}$")
    fields_complete: bool
    originator_info: Optional[TravelRuleInfo] = None
    beneficiary_info: Optional[TravelRuleInfo] = None


class SanctionsMatch(BaseModel):
    list: str
    match_score: float = Field(..., ge=0, le=1)
    entity_id: str


class SanctionsCheck(BaseModel):
    status: ScreeningStatus
    timestamp: datetime
    lists_checked: Optional[List[SanctionsList]] = None
    matches: Optional[List[SanctionsMatch]] = None


class PEPCheck(BaseModel):
    status: ScreeningStatus
    timestamp: datetime
    risk_category: Optional[RiskCategory] = None


class Screening(BaseModel):
    sanctions_check: SanctionsCheck
    pep_check: PEPCheck


class AuditEntry(BaseModel):
    action: str = Field(..., max_length=50)
    timestamp: datetime
    actor: str = Field(..., max_length=100)
    details: Optional[dict] = None


class RegulatoryReporting(BaseModel):
    travel_rule: TravelRule
    screening: Screening
    audit_trail: Optional[List[AuditEntry]] = None


class RiskAssessment(BaseModel):
    risk_score: Optional[float] = Field(None, ge=0, le=100)
    risk_factors: Optional[List[RiskFactor]] = None
    risk_mitigation: Optional[List[RiskMitigation]] = None


class Charge(BaseModel):
    type: ChargeType
    amount: str = Field(..., regex="^[0-9]+\\.[0-9]{2}$")
    currency: str = Field(..., regex="^[A-Z]{3}$")


class SettlementDetails(BaseModel):
    settlement_date: Optional[str] = Field(None, regex="^[0-9]{4}-[0-9]{2}-[0-9]{2}$")
    settlement_currency: Optional[str] = Field(None, regex="^[A-Z]{3}$")
    exchange_rate: Optional[float] = Field(None, gt=0)
    charges: Optional[List[Charge]] = None
    netting_batch_id: Optional[str] = None


class StatusHistory(BaseModel):
    status: TransactionStatus
    timestamp: datetime
    reason: Optional[str] = Field(None, max_length=255)


class ValidatorSignature(BaseModel):
    validator: str
    signature: str = Field(..., regex="^[0-9a-f]{128}$")


class LedgerProof(BaseModel):
    block_hash: Optional[str] = Field(None, regex="^[0-9a-f]{64}$")
    block_number: Optional[int] = Field(None, ge=0)
    transaction_index: Optional[int] = Field(None, ge=0)
    merkle_proof: Optional[List[str]] = Field(None, regex="^[0-9a-f]{64}$")
    validator_signatures: Optional[List[ValidatorSignature]] = None


class CTR(BaseModel):
    """Cross-border Transaction Record - Canonical model"""

    transaction_id: UUID = Field(..., description="UUIDv7 transaction identifier")
    uetr: UUID = Field(..., description="Unique End-to-end Transaction Reference")
    timestamp: datetime
    amount: Amount
    debtor_party: Party
    creditor_party: Party
    settlement_method: SettlementMethod
    payment_purpose: PaymentPurpose
    regulatory_reporting: RegulatoryReporting

    # Optional fields
    risk_assessment: Optional[RiskAssessment] = None
    settlement_details: Optional[SettlementDetails] = None
    status: Optional[TransactionStatus] = TransactionStatus.INITIATED
    status_history: Optional[List[StatusHistory]] = None
    ledger_proof: Optional[LedgerProof] = None

    @validator('transaction_id')
    def validate_transaction_id_version(cls, v):
        if v.version != 7:
            raise ValueError("Transaction ID must be UUIDv7")
        return v

    @validator('uetr')
    def validate_uetr_version(cls, v):
        if v.version != 4:
            raise ValueError("UETR must be UUIDv4")
        return v

    class Config:
        use_enum_values = True
        json_encoders = {
            datetime: lambda v: v.isoformat(),
            UUID: lambda v: str(v)
        }


class CTRCreateRequest(BaseModel):
    """Request model for creating new CTR"""
    amount: Amount
    debtor_party: Party
    creditor_party: Party
    settlement_method: SettlementMethod
    payment_purpose: PaymentPurpose
    risk_assessment: Optional[RiskAssessment] = None


class CTRResponse(BaseModel):
    """Response model for CTR operations"""
    transaction_id: UUID
    uetr: UUID
    status: TransactionStatus
    timestamp: datetime
    message: Optional[str] = None


class CTRStatusResponse(BaseModel):
    """Response model for CTR status queries"""
    transaction_id: UUID
    uetr: UUID
    status: TransactionStatus
    current_step: Optional[str] = None
    settlement_details: Optional[SettlementDetails] = None
    ledger_proof: Optional[LedgerProof] = None
    estimated_completion: Optional[datetime] = None