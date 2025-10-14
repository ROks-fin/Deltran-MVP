package iso20022

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func createTestValidator() *Validator {
	return NewValidator([]string{"USD", "EUR", "GBP"}, false)
}

func TestValidatePacs008_Valid(t *testing.T) {
	validator := createTestValidator()

	validXML := `<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pacs.008.001.08">
	<FIToFICstmrCdtTrf>
		<GrpHdr>
			<MsgId>DELTRAN-20251013-001</MsgId>
			<CreDtTm>2025-10-13T14:30:00Z</CreDtTm>
			<NbOfTxs>1</NbOfTxs>
			<TtlIntrBkSttlmAmt Ccy="USD">5000.00</TtlIntrBkSttlmAmt>
			<SttlmInf>
				<SttlmMtd>CLRG</SttlmMtd>
			</SttlmInf>
		</GrpHdr>
		<CdtTrfTxInf>
			<PmtId>
				<InstrId>INSTR001</InstrId>
				<EndToEndId>E2E001</EndToEndId>
				<TxId>TX001</TxId>
			</PmtId>
			<IntrBkSttlmAmt Ccy="USD">5000.00</IntrBkSttlmAmt>
			<InstgAgt>
				<BICFI>CHASUS33XXX</BICFI>
			</InstgAgt>
			<InstdAgt>
				<BICFI>DEUTDEFFXXX</BICFI>
			</InstdAgt>
			<Dbtr>
				<Nm>JPMorgan Chase Bank</Nm>
				<PstlAdr>
					<Ctry>US</Ctry>
				</PstlAdr>
				<FinInstnId>
					<BICFI>CHASUS33</BICFI>
				</FinInstnId>
			</Dbtr>
			<DbtrAcct>
				<Id>
					<IBAN>GB33BUKB20201555555555</IBAN>
				</Id>
			</DbtrAcct>
			<Cdtr>
				<Nm>Deutsche Bank AG</Nm>
				<PstlAdr>
					<Ctry>DE</Ctry>
				</PstlAdr>
				<FinInstnId>
					<BICFI>DEUTDEFF</BICFI>
				</FinInstnId>
			</Cdtr>
			<CdtrAcct>
				<Id>
					<IBAN>DE89370400440532013000</IBAN>
				</Id>
			</CdtrAcct>
			<RmtInf>
				<Ustrd>Invoice payment</Ustrd>
			</RmtInf>
		</CdtTrfTxInf>
	</FIToFICstmrCdtTrf>
</Document>`

	result, err := validator.ValidatePacs008([]byte(validXML))
	require.NoError(t, err)
	assert.True(t, result.Valid)
	assert.Empty(t, result.Errors)
}

func TestValidatePacs008_MissingMandatoryFields(t *testing.T) {
	validator := createTestValidator()

	tests := []struct {
		name          string
		xml           string
		expectedError string
	}{
		{
			name: "missing MsgId",
			xml: `<?xml version="1.0"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pacs.008.001.08">
	<FIToFICstmrCdtTrf>
		<GrpHdr>
			<MsgId></MsgId>
			<CreDtTm>2025-10-13T14:30:00Z</CreDtTm>
			<NbOfTxs>1</NbOfTxs>
			<TtlIntrBkSttlmAmt Ccy="USD">5000.00</TtlIntrBkSttlmAmt>
		</GrpHdr>
		<CdtTrfTxInf>
			<PmtId><EndToEndId>E2E001</EndToEndId></PmtId>
			<IntrBkSttlmAmt Ccy="USD">5000.00</IntrBkSttlmAmt>
			<Dbtr><FinInstnId><BICFI>CHASUS33</BICFI></FinInstnId></Dbtr>
			<Cdtr><FinInstnId><BICFI>DEUTDEFF</BICFI></FinInstnId></Cdtr>
		</CdtTrfTxInf>
	</FIToFICstmrCdtTrf>
</Document>`,
			expectedError: "MISSING_FIELD",
		},
		{
			name: "missing EndToEndId",
			xml: `<?xml version="1.0"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pacs.008.001.08">
	<FIToFICstmrCdtTrf>
		<GrpHdr>
			<MsgId>MSG001</MsgId>
			<CreDtTm>2025-10-13T14:30:00Z</CreDtTm>
			<NbOfTxs>1</NbOfTxs>
			<TtlIntrBkSttlmAmt Ccy="USD">5000.00</TtlIntrBkSttlmAmt>
		</GrpHdr>
		<CdtTrfTxInf>
			<PmtId><EndToEndId></EndToEndId></PmtId>
			<IntrBkSttlmAmt Ccy="USD">5000.00</IntrBkSttlmAmt>
			<Dbtr><FinInstnId><BICFI>CHASUS33</BICFI></FinInstnId></Dbtr>
			<Cdtr><FinInstnId><BICFI>DEUTDEFF</BICFI></FinInstnId></Cdtr>
		</CdtTrfTxInf>
	</FIToFICstmrCdtTrf>
</Document>`,
			expectedError: "MISSING_FIELD",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result, err := validator.ValidatePacs008([]byte(tt.xml))
			require.NoError(t, err)
			assert.False(t, result.Valid)
			assert.NotEmpty(t, result.Errors)
			found := false
			for _, e := range result.Errors {
				if e.Code == tt.expectedError {
					found = true
					break
				}
			}
			assert.True(t, found, "Expected error code %s not found", tt.expectedError)
		})
	}
}

func TestValidatePacs008_InvalidBIC(t *testing.T) {
	validator := createTestValidator()

	invalidXML := `<?xml version="1.0"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pacs.008.001.08">
	<FIToFICstmrCdtTrf>
		<GrpHdr>
			<MsgId>MSG001</MsgId>
			<CreDtTm>2025-10-13T14:30:00Z</CreDtTm>
			<NbOfTxs>1</NbOfTxs>
			<TtlIntrBkSttlmAmt Ccy="USD">5000.00</TtlIntrBkSttlmAmt>
		</GrpHdr>
		<CdtTrfTxInf>
			<PmtId><EndToEndId>E2E001</EndToEndId></PmtId>
			<IntrBkSttlmAmt Ccy="USD">5000.00</IntrBkSttlmAmt>
			<Dbtr><FinInstnId><BICFI>INVALID</BICFI></FinInstnId></Dbtr>
			<Cdtr><FinInstnId><BICFI>DEUTDEFF</BICFI></FinInstnId></Cdtr>
		</CdtTrfTxInf>
	</FIToFICstmrCdtTrf>
</Document>`

	result, err := validator.ValidatePacs008([]byte(invalidXML))
	require.NoError(t, err)
	assert.False(t, result.Valid)
	assert.NotEmpty(t, result.Errors)

	found := false
	for _, e := range result.Errors {
		if e.Code == "INVALID_BIC" {
			found = true
			break
		}
	}
	assert.True(t, found, "Expected INVALID_BIC error")
}

func TestValidatePacs008_InvalidAmount(t *testing.T) {
	validator := createTestValidator()

	tests := []struct {
		name          string
		amount        string
		ccy           string
		expectedError string
	}{
		{
			name:          "zero amount",
			amount:        "0.00",
			ccy:           "USD",
			expectedError: "INVALID_VALUE",
		},
		{
			name:          "negative amount",
			amount:        "-100.00",
			ccy:           "USD",
			expectedError: "INVALID_VALUE",
		},
		{
			name:          "invalid format",
			amount:        "abc",
			ccy:           "USD",
			expectedError: "INVALID_FORMAT",
		},
		{
			name:          "exceeds limit",
			amount:        "20000000.00",
			ccy:           "USD",
			expectedError: "AMOUNT_EXCEEDS_LIMIT",
		},
		{
			name:          "too many decimals",
			amount:        "100.123",
			ccy:           "USD",
			expectedError: "INVALID_PRECISION",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			xml := `<?xml version="1.0"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pacs.008.001.08">
	<FIToFICstmrCdtTrf>
		<GrpHdr>
			<MsgId>MSG001</MsgId>
			<CreDtTm>2025-10-13T14:30:00Z</CreDtTm>
			<NbOfTxs>1</NbOfTxs>
			<TtlIntrBkSttlmAmt Ccy="` + tt.ccy + `">` + tt.amount + `</TtlIntrBkSttlmAmt>
		</GrpHdr>
		<CdtTrfTxInf>
			<PmtId><EndToEndId>E2E001</EndToEndId></PmtId>
			<IntrBkSttlmAmt Ccy="` + tt.ccy + `">` + tt.amount + `</IntrBkSttlmAmt>
			<Dbtr><FinInstnId><BICFI>CHASUS33</BICFI></FinInstnId></Dbtr>
			<Cdtr><FinInstnId><BICFI>DEUTDEFF</BICFI></FinInstnId></Cdtr>
		</CdtTrfTxInf>
	</FIToFICstmrCdtTrf>
</Document>`

			result, err := validator.ValidatePacs008([]byte(xml))
			require.NoError(t, err)
			assert.False(t, result.Valid)

			found := false
			for _, e := range result.Errors {
				if e.Code == tt.expectedError {
					found = true
					break
				}
			}
			assert.True(t, found, "Expected error code %s not found", tt.expectedError)
		})
	}
}

func TestValidatePacs008_EndToEndIdLength(t *testing.T) {
	validator := createTestValidator()

	// EndToEndId > 35 characters
	longId := "THIS_IS_A_VERY_LONG_END_TO_END_ID_THAT_EXCEEDS_35_CHARACTERS"
	xml := `<?xml version="1.0"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pacs.008.001.08">
	<FIToFICstmrCdtTrf>
		<GrpHdr>
			<MsgId>MSG001</MsgId>
			<CreDtTm>2025-10-13T14:30:00Z</CreDtTm>
			<NbOfTxs>1</NbOfTxs>
			<TtlIntrBkSttlmAmt Ccy="USD">5000.00</TtlIntrBkSttlmAmt>
		</GrpHdr>
		<CdtTrfTxInf>
			<PmtId><EndToEndId>` + longId + `</EndToEndId></PmtId>
			<IntrBkSttlmAmt Ccy="USD">5000.00</IntrBkSttlmAmt>
			<Dbtr><FinInstnId><BICFI>CHASUS33</BICFI></FinInstnId></Dbtr>
			<Cdtr><FinInstnId><BICFI>DEUTDEFF</BICFI></FinInstnId></Cdtr>
		</CdtTrfTxInf>
	</FIToFICstmrCdtTrf>
</Document>`

	result, err := validator.ValidatePacs008([]byte(xml))
	require.NoError(t, err)
	assert.False(t, result.Valid)

	found := false
	for _, e := range result.Errors {
		if e.Code == "INVALID_LENGTH" && e.FieldPath == "CdtTrfTxInf/PmtId/EndToEndId" {
			found = true
			break
		}
	}
	assert.True(t, found, "Expected INVALID_LENGTH error for EndToEndId")
}

func TestIsValidBIC(t *testing.T) {
	validator := createTestValidator()

	tests := []struct {
		bic   string
		valid bool
	}{
		{"CHASUS33", true},
		{"CHASUS33XXX", true},
		{"DEUTDEFF", true},
		{"DEUTDEFFXXX", true},
		{"chasus33", true},  // lowercase should be accepted (converted to uppercase)
		{"INVALID", false},
		{"CHAS", false},
		{"CHASUS3", false},
		{"CHASUS333", false},
		{"12345678", false},
	}

	for _, tt := range tests {
		t.Run(tt.bic, func(t *testing.T) {
			result := validator.isValidBIC(tt.bic)
			assert.Equal(t, tt.valid, result, "BIC %s validation failed", tt.bic)
		})
	}
}

func TestIsValidIBAN(t *testing.T) {
	validator := createTestValidator()

	tests := []struct {
		iban  string
		valid bool
	}{
		{"GB33BUKB20201555555555", true},
		{"DE89370400440532013000", true},
		{"FR1420041010050500013M02606", true},
		{"gb33bukb20201555555555", true}, // lowercase should work
		{"GB33 BUKB 2020 1555 5555 55", true}, // spaces should be removed
		{"INVALID", false},
		{"GB1234", false}, // too short
		{"1234567890123456", false}, // no country code
	}

	for _, tt := range tests {
		t.Run(tt.iban, func(t *testing.T) {
			result := validator.isValidIBAN(tt.iban)
			assert.Equal(t, tt.valid, result, "IBAN %s validation failed", tt.iban)
		})
	}
}

func TestStrictMode(t *testing.T) {
	validator := NewValidator([]string{"USD", "EUR", "GBP"}, true)

	// This XML has a warning (unsupported currency)
	xml := `<?xml version="1.0"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pacs.008.001.08">
	<FIToFICstmrCdtTrf>
		<GrpHdr>
			<MsgId>MSG001</MsgId>
			<CreDtTm>2025-10-13T14:30:00Z</CreDtTm>
			<NbOfTxs>1</NbOfTxs>
			<TtlIntrBkSttlmAmt Ccy="JPY">5000.00</TtlIntrBkSttlmAmt>
		</GrpHdr>
		<CdtTrfTxInf>
			<PmtId><EndToEndId>E2E001</EndToEndId></PmtId>
			<IntrBkSttlmAmt Ccy="JPY">5000.00</IntrBkSttlmAmt>
			<Dbtr><FinInstnId><BICFI>CHASUS33</BICFI></FinInstnId></Dbtr>
			<Cdtr><FinInstnId><BICFI>DEUTDEFF</BICFI></FinInstnId></Cdtr>
		</CdtTrfTxInf>
	</FIToFICstmrCdtTrf>
</Document>`

	result, err := validator.ValidatePacs008([]byte(xml))
	require.NoError(t, err)

	// In strict mode, warnings should be converted to errors
	assert.False(t, result.Valid)
	assert.NotEmpty(t, result.Errors)
	assert.Empty(t, result.Warnings)
}

func TestValidatePacs008_MissingCurrency(t *testing.T) {
	validator := createTestValidator()

	xml := `<?xml version="1.0"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pacs.008.001.08">
	<FIToFICstmrCdtTrf>
		<GrpHdr>
			<MsgId>MSG001</MsgId>
			<CreDtTm>2025-10-13T14:30:00Z</CreDtTm>
			<NbOfTxs>1</NbOfTxs>
			<TtlIntrBkSttlmAmt>5000.00</TtlIntrBkSttlmAmt>
		</GrpHdr>
		<CdtTrfTxInf>
			<PmtId><EndToEndId>E2E001</EndToEndId></PmtId>
			<IntrBkSttlmAmt Ccy="USD">5000.00</IntrBkSttlmAmt>
			<Dbtr><FinInstnId><BICFI>CHASUS33</BICFI></FinInstnId></Dbtr>
			<Cdtr><FinInstnId><BICFI>DEUTDEFF</BICFI></FinInstnId></Cdtr>
		</CdtTrfTxInf>
	</FIToFICstmrCdtTrf>
</Document>`

	result, err := validator.ValidatePacs008([]byte(xml))
	require.NoError(t, err)
	assert.False(t, result.Valid)

	found := false
	for _, e := range result.Errors {
		if e.Code == "MISSING_ATTRIBUTE" {
			found = true
			break
		}
	}
	assert.True(t, found, "Expected MISSING_ATTRIBUTE error for currency")
}
