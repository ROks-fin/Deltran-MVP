package iso20022

import (
	"encoding/xml"
	"fmt"
	"regexp"
	"strings"
	"time"

	"github.com/shopspring/decimal"
)

// Validator validates ISO 20022 messages with strict rules
type Validator struct {
	supportedCurrencies map[string]bool
	maxAmounts          map[string]decimal.Decimal
	strictMode          bool
}

// NewValidator creates a new ISO 20022 validator
func NewValidator(supportedCurrencies []string, strictMode bool) *Validator {
	currencyMap := make(map[string]bool)
	for _, ccy := range supportedCurrencies {
		currencyMap[ccy] = true
	}

	maxAmounts := map[string]decimal.Decimal{
		"USD": decimal.NewFromInt(10000000),
		"EUR": decimal.NewFromInt(10000000),
		"GBP": decimal.NewFromInt(10000000),
		"INR": decimal.NewFromInt(750000000),
		"AED": decimal.NewFromInt(37000000),
		"PKR": decimal.NewFromInt(2800000000),
		"NIS": decimal.NewFromInt(35000000),
	}

	return &Validator{
		supportedCurrencies: currencyMap,
		maxAmounts:          maxAmounts,
		strictMode:          strictMode,
	}
}

// ValidationError represents a validation error
type ValidationError struct {
	Code      string `json:"code"`
	Severity  string `json:"severity"` // ERROR, WARNING, INFO
	FieldPath string `json:"field_path"`
	Message   string `json:"message"`
}

// ValidationResult contains validation results
type ValidationResult struct {
	Valid    bool              `json:"valid"`
	Errors   []ValidationError `json:"errors"`
	Warnings []ValidationError `json:"warnings"`
}

// AddError adds an error to the validation result
func (r *ValidationResult) AddError(code, fieldPath, message string) {
	r.Valid = false
	r.Errors = append(r.Errors, ValidationError{
		Code:      code,
		Severity:  "ERROR",
		FieldPath: fieldPath,
		Message:   message,
	})
}

// AddWarning adds a warning to the validation result
func (r *ValidationResult) AddWarning(code, fieldPath, message string) {
	r.Warnings = append(r.Warnings, ValidationError{
		Code:      code,
		Severity:  "WARNING",
		FieldPath: fieldPath,
		Message:   message,
	})
}

// Pacs008Document represents pacs.008 message structure (simplified)
type Pacs008Document struct {
	XMLName           xml.Name          `xml:"Document"`
	FIToFICstmrCdtTrf FIToFICstmrCdtTrf `xml:"FIToFICstmrCdtTrf"`
}

// FIToFICstmrCdtTrf represents the main body of pacs.008
type FIToFICstmrCdtTrf struct {
	GrpHdr    GroupHeader              `xml:"GrpHdr"`
	CdtTrfTxInf CreditTransferTxInfo   `xml:"CdtTrfTxInf"`
}

// GroupHeader represents group header information
type GroupHeader struct {
	MsgId              string       `xml:"MsgId"`
	CreDtTm            string       `xml:"CreDtTm"`
	NbOfTxs            string       `xml:"NbOfTxs"`
	TtlIntrBkSttlmAmt  ActiveAmount `xml:"TtlIntrBkSttlmAmt"`
	IntrBkSttlmDt      string       `xml:"IntrBkSttlmDt"`
	SttlmInf           *SttlmInf    `xml:"SttlmInf,omitempty"`
}

// SttlmInf represents settlement information
type SttlmInf struct {
	SttlmMtd string `xml:"SttlmMtd"`
}

// CreditTransferTxInfo represents credit transfer transaction information
type CreditTransferTxInfo struct {
	PmtId           PaymentIdentification `xml:"PmtId"`
	IntrBkSttlmAmt  ActiveAmount          `xml:"IntrBkSttlmAmt"`
	IntrBkSttlmDt   string                `xml:"IntrBkSttlmDt,omitempty"`
	InstgAgt        *BranchAndFinInstnId  `xml:"InstgAgt,omitempty"`
	InstdAgt        *BranchAndFinInstnId  `xml:"InstdAgt,omitempty"`
	Dbtr            PartyIdentification   `xml:"Dbtr"`
	DbtrAcct        *CashAccount          `xml:"DbtrAcct,omitempty"`
	Cdtr            PartyIdentification   `xml:"Cdtr"`
	CdtrAcct        *CashAccount          `xml:"CdtrAcct,omitempty"`
	RmtInf          *RemittanceInfo       `xml:"RmtInf,omitempty"`
}

// PaymentIdentification represents payment identification
type PaymentIdentification struct {
	InstrId    string `xml:"InstrId,omitempty"`
	EndToEndId string `xml:"EndToEndId"`
	TxId       string `xml:"TxId,omitempty"`
	UETR       string `xml:"UETR,omitempty"`
}

// ActiveAmount represents an amount with currency
type ActiveAmount struct {
	Ccy   string `xml:"Ccy,attr"`
	Value string `xml:",chardata"`
}

// PartyIdentification represents party information
type PartyIdentification struct {
	Nm         string               `xml:"Nm,omitempty"`
	PstlAdr    *PostalAddress       `xml:"PstlAdr,omitempty"`
	FinInstnId *BranchAndFinInstnId `xml:"FinInstnId,omitempty"`
}

// BranchAndFinInstnId represents financial institution identification
type BranchAndFinInstnId struct {
	BICFI string `xml:"BICFI"`
}

// CashAccount represents account information
type CashAccount struct {
	Id AccountIdentification `xml:"Id"`
}

// AccountIdentification represents account ID (IBAN or other)
type AccountIdentification struct {
	IBAN  string `xml:"IBAN,omitempty"`
	Othr  *OtherAccountId `xml:"Othr,omitempty"`
}

// OtherAccountId represents other account identification
type OtherAccountId struct {
	Id string `xml:"Id"`
}

// PostalAddress represents postal address
type PostalAddress struct {
	Ctry string `xml:"Ctry,omitempty"`
}

// RemittanceInfo represents remittance information
type RemittanceInfo struct {
	Ustrd string `xml:"Ustrd,omitempty"`
}

// ValidatePacs008 validates a pacs.008 message
func (v *Validator) ValidatePacs008(xmlData []byte) (*ValidationResult, error) {
	result := &ValidationResult{Valid: true}

	var doc Pacs008Document
	if err := xml.Unmarshal(xmlData, &doc); err != nil {
		result.AddError("XML_PARSE", "root", fmt.Sprintf("XML parsing failed: %v", err))
		return result, nil
	}

	// Validate Group Header
	v.validateGroupHeader(&doc.FIToFICstmrCdtTrf.GrpHdr, result)

	// Validate Credit Transfer Transaction
	v.validateCreditTransferTx(&doc.FIToFICstmrCdtTrf.CdtTrfTxInf, result)

	// Strict mode: treat warnings as errors
	if v.strictMode && len(result.Warnings) > 0 {
		for _, w := range result.Warnings {
			result.AddError(w.Code, w.FieldPath, w.Message)
		}
		result.Warnings = nil
	}

	return result, nil
}

// validateGroupHeader validates the group header
func (v *Validator) validateGroupHeader(hdr *GroupHeader, result *ValidationResult) {
	// 1. MsgId: mandatory, 1-35 characters
	if hdr.MsgId == "" {
		result.AddError("MISSING_FIELD", "GrpHdr/MsgId", "MsgId is mandatory")
	} else if len(hdr.MsgId) > 35 {
		result.AddError("INVALID_LENGTH", "GrpHdr/MsgId", fmt.Sprintf("MsgId must be ≤35 characters, got %d", len(hdr.MsgId)))
	}

	// 2. CreDtTm: mandatory, ISO 8601 format
	if hdr.CreDtTm == "" {
		result.AddError("MISSING_FIELD", "GrpHdr/CreDtTm", "CreDtTm is mandatory")
	} else {
		if _, err := time.Parse(time.RFC3339, hdr.CreDtTm); err != nil {
			// Try alternative format
			if _, err2 := time.Parse("2006-01-02T15:04:05", hdr.CreDtTm); err2 != nil {
				result.AddError("INVALID_FORMAT", "GrpHdr/CreDtTm", fmt.Sprintf("CreDtTm must be ISO 8601 format: %v", err))
			}
		}
	}

	// 3. NbOfTxs: mandatory, must be > 0
	if hdr.NbOfTxs == "" {
		result.AddError("MISSING_FIELD", "GrpHdr/NbOfTxs", "NbOfTxs is mandatory")
	} else {
		var nbTxs int
		if _, err := fmt.Sscanf(hdr.NbOfTxs, "%d", &nbTxs); err != nil {
			result.AddError("INVALID_FORMAT", "GrpHdr/NbOfTxs", fmt.Sprintf("NbOfTxs must be numeric: %v", err))
		} else if nbTxs <= 0 {
			result.AddError("INVALID_VALUE", "GrpHdr/NbOfTxs", "NbOfTxs must be greater than 0")
		}
	}

	// 4. TtlIntrBkSttlmAmt: mandatory
	v.validateAmount(&hdr.TtlIntrBkSttlmAmt, "GrpHdr/TtlIntrBkSttlmAmt", result)

	// 5. SttlmInf/SttlmMtd: check if CLRG (clearing)
	if hdr.SttlmInf != nil && hdr.SttlmInf.SttlmMtd != "" {
		if hdr.SttlmInf.SttlmMtd != "CLRG" && hdr.SttlmInf.SttlmMtd != "INDA" && hdr.SttlmInf.SttlmMtd != "INGA" {
			result.AddWarning("UNSUPPORTED_STTLM_MTD", "GrpHdr/SttlmInf/SttlmMtd",
				fmt.Sprintf("Settlement method '%s' may not be supported. Expected: CLRG (clearing)", hdr.SttlmInf.SttlmMtd))
		}
	}
}

// validateCreditTransferTx validates credit transfer transaction info
func (v *Validator) validateCreditTransferTx(tx *CreditTransferTxInfo, result *ValidationResult) {
	// 1. PmtId/EndToEndId: mandatory, ≤35 characters
	if tx.PmtId.EndToEndId == "" {
		result.AddError("MISSING_FIELD", "CdtTrfTxInf/PmtId/EndToEndId", "EndToEndId is mandatory")
	} else if len(tx.PmtId.EndToEndId) > 35 {
		result.AddError("INVALID_LENGTH", "CdtTrfTxInf/PmtId/EndToEndId",
			fmt.Sprintf("EndToEndId must be ≤35 characters, got %d", len(tx.PmtId.EndToEndId)))
	}

	// 2. IntrBkSttlmAmt: mandatory
	v.validateAmount(&tx.IntrBkSttlmAmt, "CdtTrfTxInf/IntrBkSttlmAmt", result)

	// 3. Debtor: mandatory
	v.validateParty(&tx.Dbtr, "CdtTrfTxInf/Dbtr", "Debtor", result)

	// 4. Debtor Account (optional but validate if present)
	if tx.DbtrAcct != nil {
		v.validateAccount(tx.DbtrAcct, "CdtTrfTxInf/DbtrAcct", result)
	}

	// 5. Creditor: mandatory
	v.validateParty(&tx.Cdtr, "CdtTrfTxInf/Cdtr", "Creditor", result)

	// 6. Creditor Account (optional but validate if present)
	if tx.CdtrAcct != nil {
		v.validateAccount(tx.CdtrAcct, "CdtTrfTxInf/CdtrAcct", result)
	}

	// 7. Instructing Agent (optional but validate BIC if present)
	if tx.InstgAgt != nil && tx.InstgAgt.BICFI != "" {
		if !v.isValidBIC(tx.InstgAgt.BICFI) {
			result.AddError("INVALID_BIC", "CdtTrfTxInf/InstgAgt/FinInstnId/BICFI",
				fmt.Sprintf("Invalid BIC format: %s", tx.InstgAgt.BICFI))
		}
	}

	// 8. Instructed Agent (optional but validate BIC if present)
	if tx.InstdAgt != nil && tx.InstdAgt.BICFI != "" {
		if !v.isValidBIC(tx.InstdAgt.BICFI) {
			result.AddError("INVALID_BIC", "CdtTrfTxInf/InstdAgt/FinInstnId/BICFI",
				fmt.Sprintf("Invalid BIC format: %s", tx.InstdAgt.BICFI))
		}
	}
}

// validateAmount validates an amount field
func (v *Validator) validateAmount(amt *ActiveAmount, path string, result *ValidationResult) {
	// 1. Currency: mandatory
	if amt.Ccy == "" {
		result.AddError("MISSING_ATTRIBUTE", path+"@Ccy", "Currency code is mandatory")
		return
	}

	// 2. Currency: must be 3 uppercase letters (ISO 4217)
	if len(amt.Ccy) != 3 || !isUpperAlpha(amt.Ccy) {
		result.AddError("INVALID_FORMAT", path+"@Ccy", fmt.Sprintf("Currency must be 3 uppercase letters (ISO 4217), got: %s", amt.Ccy))
		return
	}

	// 3. Currency: must be in supported list
	if !v.supportedCurrencies[amt.Ccy] {
		result.AddWarning("UNSUPPORTED_CURRENCY", path+"@Ccy", fmt.Sprintf("Currency '%s' is not in supported list", amt.Ccy))
	}

	// 4. Amount value: mandatory, must be > 0
	if amt.Value == "" {
		result.AddError("MISSING_VALUE", path, "Amount value is mandatory")
		return
	}

	amount, err := decimal.NewFromString(amt.Value)
	if err != nil {
		result.AddError("INVALID_FORMAT", path, fmt.Sprintf("Invalid decimal format: %s", amt.Value))
		return
	}

	if amount.LessThanOrEqual(decimal.Zero) {
		result.AddError("INVALID_VALUE", path, "Amount must be greater than zero")
		return
	}

	// 5. Check maximum amount limit
	if maxAmt, ok := v.maxAmounts[amt.Ccy]; ok {
		if amount.GreaterThan(maxAmt) {
			result.AddError("AMOUNT_EXCEEDS_LIMIT", path,
				fmt.Sprintf("Amount %s exceeds limit %s for %s", amount.String(), maxAmt.String(), amt.Ccy))
		}
	}

	// 6. Check decimal places (max 5 for most currencies, 2 for major currencies)
	if strings.Contains(amt.Value, ".") {
		parts := strings.Split(amt.Value, ".")
		if len(parts) == 2 {
			decimals := len(parts[1])
			maxDecimals := 5
			if amt.Ccy == "USD" || amt.Ccy == "EUR" || amt.Ccy == "GBP" {
				maxDecimals = 2
			}
			if decimals > maxDecimals {
				result.AddError("INVALID_PRECISION", path,
					fmt.Sprintf("Amount has %d decimal places, max %d for %s", decimals, maxDecimals, amt.Ccy))
			}
		}
	}
}

// validateParty validates party information (Debtor or Creditor)
func (v *Validator) validateParty(party *PartyIdentification, path, partyType string, result *ValidationResult) {
	// 1. Name: recommended (warning if missing)
	if party.Nm == "" {
		result.AddWarning("MISSING_FIELD", path+"/Nm", fmt.Sprintf("%s name is recommended", partyType))
	} else if len(party.Nm) > 140 {
		result.AddError("INVALID_LENGTH", path+"/Nm", fmt.Sprintf("%s name must be ≤140 characters", partyType))
	}

	// 2. FinInstnId/BICFI: mandatory
	if party.FinInstnId == nil {
		result.AddError("MISSING_ELEMENT", path+"/FinInstnId", fmt.Sprintf("%s FinInstnId is mandatory", partyType))
		return
	}

	if party.FinInstnId.BICFI == "" {
		result.AddError("MISSING_FIELD", path+"/FinInstnId/BICFI", fmt.Sprintf("%s BIC is mandatory", partyType))
		return
	}

	if !v.isValidBIC(party.FinInstnId.BICFI) {
		result.AddError("INVALID_BIC", path+"/FinInstnId/BICFI",
			fmt.Sprintf("Invalid BIC format: %s", party.FinInstnId.BICFI))
	}

	// 3. Country code (optional but validate if present)
	if party.PstlAdr != nil && party.PstlAdr.Ctry != "" {
		if len(party.PstlAdr.Ctry) != 2 || !isUpperAlpha(party.PstlAdr.Ctry) {
			result.AddError("INVALID_FORMAT", path+"/PstlAdr/Ctry",
				fmt.Sprintf("Country code must be 2 uppercase letters (ISO 3166), got: %s", party.PstlAdr.Ctry))
		}
	}
}

// validateAccount validates account information
func (v *Validator) validateAccount(acct *CashAccount, path string, result *ValidationResult) {
	// Either IBAN or Other must be present
	if acct.Id.IBAN == "" && (acct.Id.Othr == nil || acct.Id.Othr.Id == "") {
		result.AddError("MISSING_FIELD", path+"/Id", "Account identification (IBAN or Other) is mandatory")
		return
	}

	// Validate IBAN if present
	if acct.Id.IBAN != "" {
		if !v.isValidIBAN(acct.Id.IBAN) {
			result.AddError("INVALID_IBAN", path+"/Id/IBAN", fmt.Sprintf("Invalid IBAN format: %s", acct.Id.IBAN))
		}
	}

	// Validate Other account ID if present
	if acct.Id.Othr != nil && acct.Id.Othr.Id != "" {
		if len(acct.Id.Othr.Id) > 34 {
			result.AddError("INVALID_LENGTH", path+"/Id/Othr/Id", "Account ID must be ≤34 characters")
		}
	}
}

// isValidBIC validates BIC format
func (v *Validator) isValidBIC(bic string) bool {
	// BIC format: 4 letters (institution) + 2 letters (country) + 2 alphanumeric (location) + optional 3 alphanumeric (branch)
	bic = strings.ToUpper(bic)
	if len(bic) != 8 && len(bic) != 11 {
		return false
	}

	bicRegex := regexp.MustCompile(`^[A-Z]{6}[A-Z0-9]{2}([A-Z0-9]{3})?$`)
	return bicRegex.MatchString(bic)
}

// isValidIBAN validates IBAN format (basic check)
func (v *Validator) isValidIBAN(iban string) bool {
	// Remove spaces
	iban = strings.ReplaceAll(iban, " ", "")
	iban = strings.ToUpper(iban)

	// IBAN length: 15-34 characters
	if len(iban) < 15 || len(iban) > 34 {
		return false
	}

	// First 2 characters must be letters (country code)
	if !isUpperAlpha(iban[0:2]) {
		return false
	}

	// Next 2 characters must be digits (check digits)
	if !isDigits(iban[2:4]) {
		return false
	}

	// Rest must be alphanumeric
	if !isAlphanumeric(iban[4:]) {
		return false
	}

	// TODO: Implement full IBAN check digit validation (mod-97 algorithm)
	return true
}

// Helper functions
func isUpperAlpha(s string) bool {
	for _, c := range s {
		if c < 'A' || c > 'Z' {
			return false
		}
	}
	return true
}

func isDigits(s string) bool {
	for _, c := range s {
		if c < '0' || c > '9' {
			return false
		}
	}
	return true
}

func isAlphanumeric(s string) bool {
	for _, c := range s {
		if !((c >= '0' && c <= '9') || (c >= 'A' && c <= 'Z')) {
			return false
		}
	}
	return true
}
