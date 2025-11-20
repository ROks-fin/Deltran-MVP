// K6 Load Test for pain.001 (Payment Initiation)
// Tests Gateway's ability to handle real ISO 20022 messages at scale

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const paymentDuration = new Trend('payment_duration');
const paymentsCreated = new Counter('payments_created');

// Load test configuration
export const options = {
    stages: [
        { duration: '1m', target: 50 },   // Ramp up to 50 TPS
        { duration: '3m', target: 100 },  // Stay at 100 TPS
        { duration: '2m', target: 200 },  // Spike to 200 TPS
        { duration: '2m', target: 100 },  // Back to 100 TPS
        { duration: '1m', target: 0 },    // Ramp down
    ],
    thresholds: {
        'http_req_duration': ['p(95)<500', 'p(99)<1000'],  // 95% under 500ms
        'errors': ['rate<0.01'],  // Error rate under 1%
        'http_req_failed': ['rate<0.01'],
    },
};

const BASE_URL = __ENV.GATEWAY_URL || 'http://localhost:8080';

// Generate realistic pain.001 XML
function generatePain001(txId) {
    const endToEndId = `E2E-${txId}-${Date.now()}`;
    const instructionId = `INSTR-${txId}`;
    const amount = (Math.random() * 50000 + 1000).toFixed(2);

    const senders = [
        'Ahmed Al-Mazrouei', 'Mohammed Al-Fahim', 'Fatima Al-Qassimi',
        'Khalid Al-Nahyan', 'Mariam Al-Hashemi'
    ];
    const receivers = [
        'Priya Sharma', 'Rajesh Kumar', 'Sunita Patel',
        'Amit Desai', 'Deepak Singh'
    ];
    const debtorBanks = [
        { bic: 'EBILAEAD', name: 'Emirates NBD' },
        { bic: 'ADCBAEAA', name: 'ADCB' },
        { bic: 'NBADAEAAXXX', name: 'First Abu Dhabi Bank' },
        { bic: 'MASHAEADXXX', name: 'Mashreq Bank' },
    ];
    const creditorBanks = [
        { bic: 'ICICINBB', name: 'ICICI Bank' },
        { bic: 'HDFCINBB', name: 'HDFC Bank' },
        { bic: 'SBININBB', name: 'State Bank of India' },
        { bic: 'AXISINBB', name: 'Axis Bank' },
    ];

    const sender = senders[Math.floor(Math.random() * senders.length)];
    const receiver = receivers[Math.floor(Math.random() * receivers.length)];
    const debtorBank = debtorBanks[Math.floor(Math.random() * debtorBanks.length)];
    const creditorBank = creditorBanks[Math.floor(Math.random() * creditorBanks.length)];

    return `<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pain.001.001.11">
    <CstmrCdtTrfInitn>
        <GrpHdr>
            <MsgId>MSG-${txId}-${Date.now()}</MsgId>
            <CreDtTm>${new Date().toISOString()}</CreDtTm>
            <NbOfTxs>1</NbOfTxs>
            <CtrlSum>${amount}</CtrlSum>
            <InitgPty>
                <Nm>${sender}</Nm>
            </InitgPty>
        </GrpHdr>
        <PmtInf>
            <PmtInfId>PMT-${txId}</PmtInfId>
            <PmtMtd>TRF</PmtMtd>
            <ReqdExctnDt>
                <Dt>${new Date().toISOString().split('T')[0]}</Dt>
            </ReqdExctnDt>
            <Dbtr>
                <Nm>${sender}</Nm>
                <PstlAdr>
                    <Ctry>AE</Ctry>
                </PstlAdr>
            </Dbtr>
            <DbtrAcct>
                <Id>
                    <IBAN>AE${String(Math.floor(Math.random() * 1000000000000000000000)).padStart(21, '0')}</IBAN>
                </Id>
            </DbtrAcct>
            <DbtrAgt>
                <FinInstnId>
                    <BICFI>${debtorBank.bic}</BICFI>
                    <Nm>${debtorBank.name}</Nm>
                </FinInstnId>
            </DbtrAgt>
            <CdtTrfTxInf>
                <PmtId>
                    <InstrId>${instructionId}</InstrId>
                    <EndToEndId>${endToEndId}</EndToEndId>
                </PmtId>
                <Amt>
                    <InstdAmt Ccy="AED">${amount}</InstdAmt>
                </Amt>
                <CdtrAgt>
                    <FinInstnId>
                        <BICFI>${creditorBank.bic}</BICFI>
                        <Nm>${creditorBank.name}</Nm>
                    </FinInstnId>
                </CdtrAgt>
                <Cdtr>
                    <Nm>${receiver}</Nm>
                    <PstlAdr>
                        <Ctry>IN</Ctry>
                    </PstlAdr>
                </Cdtr>
                <CdtrAcct>
                    <Id>
                        <IBAN>IN${String(Math.floor(Math.random() * 1000000000000000000000)).padStart(20, '0')}</IBAN>
                    </Id>
                </CdtrAcct>
                <RmtInf>
                    <Ustrd>Remittance for invoice payment</Ustrd>
                </RmtInf>
            </CdtTrfTxInf>
        </PmtInf>
    </CstmrCdtTrfInitn>
</Document>`;
}

export default function () {
    const txId = __VU * 100000 + __ITER;
    const pain001Xml = generatePain001(txId);

    const params = {
        headers: {
            'Content-Type': 'application/xml',
        },
        timeout: '5s',
    };

    const startTime = Date.now();
    const response = http.post(`${BASE_URL}/iso20022/pain.001`, pain001Xml, params);
    const duration = Date.now() - startTime;

    // Record metrics
    paymentDuration.add(duration);
    errorRate.add(response.status !== 200);

    // Validate response
    const success = check(response, {
        'status is 200': (r) => r.status === 200,
        'response time < 1s': (r) => r.timings.duration < 1000,
        'has deltran_tx_id': (r) => {
            try {
                const body = JSON.parse(r.body);
                return Array.isArray(body) && body.length > 0 && body[0].deltran_tx_id;
            } catch {
                return false;
            }
        },
    });

    if (success) {
        paymentsCreated.add(1);
    }

    // Think time (simulate realistic payment submission rate)
    sleep(Math.random() * 0.5);
}

export function handleSummary(data) {
    return {
        'stdout': JSON.stringify(data, null, 2),
        'stress-tests/results/pain001_load_test_result.json': JSON.stringify(data),
    };
}
