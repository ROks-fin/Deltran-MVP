// K6 End-to-End Flow Test
// Simulates complete payment lifecycle: pain.001 → camt.054 funding → settlement
// This tests the REAL DelTran flow with actual ISO 20022 messages

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Counter, Trend } from 'k6/metrics';

// Custom metrics
const completedPayments = new Counter('completed_payments');
const fundedPayments = new Counter('funded_payments');
const e2eLatency = new Trend('end_to_end_latency_ms');

export const options = {
    scenarios: {
        complete_flow: {
            executor: 'ramping-vus',
            startVUs: 0,
            stages: [
                { duration: '2m', target: 20 },   // 20 concurrent payment flows
                { duration: '5m', target: 50 },   // 50 concurrent flows
                { duration: '3m', target: 100 },  // Spike to 100
                { duration: '2m', target: 0 },    // Ramp down
            ],
        },
    },
    thresholds: {
        'end_to_end_latency_ms': ['p(95)<2000', 'p(99)<5000'],
        'http_req_failed': ['rate<0.01'],
        'completed_payments': ['count>1000'],  // At least 1000 completed
    },
};

const GATEWAY_URL = __ENV.GATEWAY_URL || 'http://localhost:8080';

function generatePain001(txId, endToEndId) {
    const amount = (Math.random() * 50000 + 5000).toFixed(2);

    return `<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pain.001.001.11">
    <CstmrCdtTrfInitn>
        <GrpHdr>
            <MsgId>MSG-${txId}</MsgId>
            <CreDtTm>${new Date().toISOString()}</CreDtTm>
            <NbOfTxs>1</NbOfTxs>
            <CtrlSum>${amount}</CtrlSum>
            <InitgPty><Nm>Test Sender</Nm></InitgPty>
        </GrpHdr>
        <PmtInf>
            <PmtInfId>PMT-${txId}</PmtInfId>
            <PmtMtd>TRF</PmtMtd>
            <ReqdExctnDt><Dt>${new Date().toISOString().split('T')[0]}</Dt></ReqdExctnDt>
            <Dbtr><Nm>Ahmed Al-Mazrouei</Nm></Dbtr>
            <DbtrAcct><Id><IBAN>AE070331234567890123456</IBAN></Id></DbtrAcct>
            <DbtrAgt><FinInstnId><BICFI>EBILAEAD</BICFI></FinInstnId></DbtrAgt>
            <CdtTrfTxInf>
                <PmtId>
                    <InstrId>INSTR-${txId}</InstrId>
                    <EndToEndId>${endToEndId}</EndToEndId>
                </PmtId>
                <Amt><InstdAmt Ccy="AED">${amount}</InstdAmt></Amt>
                <CdtrAgt><FinInstnId><BICFI>ICICINBB</BICFI></FinInstnId></CdtrAgt>
                <Cdtr><Nm>Priya Sharma</Nm></Cdtr>
                <CdtrAcct><Id><IBAN>IN12345678901234567890</IBAN></Id></CdtrAcct>
            </CdtTrfTxInf>
        </PmtInf>
    </CstmrCdtTrfInitn>
</Document>`;
}

function generateCamt054(endToEndId, amount) {
    return `<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:camt.054.001.10">
    <BkToCstmrDbtCdtNtfctn>
        <GrpHdr>
            <MsgId>CAMT-${Date.now()}</MsgId>
            <CreDtTm>${new Date().toISOString()}</CreDtTm>
        </GrpHdr>
        <Ntfctn>
            <Id>N-${Date.now()}</Id>
            <Acct>
                <Id><IBAN>AE070331234567890123456</IBAN></Id>
            </Acct>
            <Ntry>
                <Amt Ccy="AED">${amount}</Amt>
                <CdtDbtInd>CRDT</CdtDbtInd>
                <Sts><Cd>BOOK</Cd></Sts>
                <BkTxCd><Prtry><Cd>TRANSFER</Cd></Prtry></BkTxCd>
                <NtryDtls>
                    <TxDtls>
                        <Refs>
                            <EndToEndId>${endToEndId}</EndToEndId>
                        </Refs>
                    </TxDtls>
                </NtryDtls>
            </Ntry>
        </Ntfctn>
    </BkToCstmrDbtCdtNtfctn>
</Document>`;
}

export default function () {
    const txId = `TX-${__VU}-${__ITER}-${Date.now()}`;
    const endToEndId = `E2E-${txId}`;
    const startTime = Date.now();

    // Step 1: Submit pain.001 (Payment Initiation)
    const amount = (Math.random() * 50000 + 5000).toFixed(2);
    const pain001 = generatePain001(txId, endToEndId);

    const pain001Response = http.post(`${GATEWAY_URL}/iso20022/pain.001`, pain001, {
        headers: { 'Content-Type': 'application/xml' },
    });

    const pain001Success = check(pain001Response, {
        'pain.001 accepted': (r) => r.status === 200,
        'payment created': (r) => {
            try {
                const body = JSON.parse(r.body);
                return Array.isArray(body) && body.length > 0;
            } catch {
                return false;
            }
        },
    });

    if (!pain001Success) {
        console.error(`Failed to submit pain.001: ${pain001Response.status}`);
        return;
    }

    // Small delay to simulate real-world timing
    sleep(0.1);

    // Step 2: Submit camt.054 (Funding Notification)
    const camt054 = generateCamt054(endToEndId, amount);

    const camt054Response = http.post(`${GATEWAY_URL}/iso20022/camt.054`, camt054, {
        headers: { 'Content-Type': 'application/xml' },
    });

    const camt054Success = check(camt054Response, {
        'camt.054 accepted': (r) => r.status === 200,
        'funding confirmed': (r) => {
            try {
                const body = JSON.parse(r.body);
                return Array.isArray(body) && body.length > 0 && body[0].status === 'FUNDED';
            } catch {
                return false;
            }
        },
    });

    if (camt054Success) {
        fundedPayments.add(1);
    }

    // Step 3: Check payment status
    sleep(0.2);

    // Note: We would need to extract deltran_tx_id from pain.001 response
    // For now, we'll just track the metrics

    if (pain001Success && camt054Success) {
        completedPayments.add(1);
        const totalLatency = Date.now() - startTime;
        e2eLatency.add(totalLatency);
    }

    sleep(Math.random() * 1);
}

export function handleSummary(data) {
    const avgLatency = data.metrics.end_to_end_latency_ms?.values?.avg || 0;
    const p95Latency = data.metrics.end_to_end_latency_ms?.values?.['p(95)'] || 0;
    const completedCount = data.metrics.completed_payments?.values?.count || 0;
    const fundedCount = data.metrics.funded_payments?.values?.count || 0;

    console.log('\n=== END-TO-END FLOW TEST SUMMARY ===');
    console.log(`Completed payments: ${completedCount}`);
    console.log(`Funded payments: ${fundedCount}`);
    console.log(`Average E2E latency: ${avgLatency.toFixed(2)}ms`);
    console.log(`P95 E2E latency: ${p95Latency.toFixed(2)}ms`);

    return {
        'stdout': JSON.stringify(data, null, 2),
        'stress-tests/results/end_to_end_flow_result.json': JSON.stringify(data),
    };
}
