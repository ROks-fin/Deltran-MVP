-- Initialize Test Banks for Multi-Region Stress Test
-- UAE, Israel, Pakistan, India
-- PostgreSQL 15+

SET search_path TO deltran;

-- ============================================
-- CLEAN EXISTING TEST DATA
-- ============================================

-- Remove existing test banks if they exist
DELETE FROM clearing_windows WHERE bank_id IN (
    SELECT id FROM banks WHERE bic_code IN (
        'ENBXAEADXXX', 'LUMIILITXXX', 'HABBPKKKXXX', 'SBININBBXXX'
    )
);

DELETE FROM bank_accounts WHERE bank_id IN (
    SELECT id FROM banks WHERE bic_code IN (
        'ENBXAEADXXX', 'LUMIILITXXX', 'HABBPKKKXXX', 'SBININBBXXX'
    )
);

DELETE FROM banks WHERE bic_code IN (
    'ENBXAEADXXX', 'LUMIILITXXX', 'HABBPKKKXXX', 'SBININBBXXX'
);

-- ============================================
-- INSERT TEST BANKS
-- ============================================

-- UAE Bank
INSERT INTO banks (
    id,
    bic_code,
    name,
    country_code,
    is_active,
    risk_rating,
    kyc_status,
    kyc_verified_at,
    contact_email,
    metadata
) VALUES (
    'bank-uae-001'::UUID,
    'ENBXAEADXXX',
    'Emirates National Bank',
    'AE',
    true,
    'LOW',
    'verified',
    NOW(),
    'operations@enb.ae',
    jsonb_build_object(
        'primary_currency', 'AED',
        'secondary_currencies', ARRAY['USD', 'EUR', 'SAR'],
        'timezone', 'Asia/Dubai',
        'regulatory_authority', 'Central Bank of UAE',
        'swift_enabled', true
    )
);

-- Israel Bank
INSERT INTO banks (
    id,
    bic_code,
    name,
    country_code,
    is_active,
    risk_rating,
    kyc_status,
    kyc_verified_at,
    contact_email,
    metadata
) VALUES (
    'bank-il-001'::UUID,
    'LUMIILITXXX',
    'Bank Leumi Israel',
    'IL',
    true,
    'LOW',
    'verified',
    NOW(),
    'operations@leumi.co.il',
    jsonb_build_object(
        'primary_currency', 'ILS',
        'secondary_currencies', ARRAY['USD', 'EUR', 'GBP'],
        'timezone', 'Asia/Jerusalem',
        'regulatory_authority', 'Bank of Israel',
        'swift_enabled', true
    )
);

-- Pakistan Bank
INSERT INTO banks (
    id,
    bic_code,
    name,
    country_code,
    is_active,
    risk_rating,
    kyc_status,
    kyc_verified_at,
    contact_email,
    metadata
) VALUES (
    'bank-pk-001'::UUID,
    'HABBPKKKXXX',
    'Habib Bank Limited',
    'PK',
    true,
    'MEDIUM',
    'verified',
    NOW(),
    'operations@hbl.com.pk',
    jsonb_build_object(
        'primary_currency', 'PKR',
        'secondary_currencies', ARRAY['USD', 'AED', 'SAR'],
        'timezone', 'Asia/Karachi',
        'regulatory_authority', 'State Bank of Pakistan',
        'swift_enabled', true
    )
);

-- India Bank
INSERT INTO banks (
    id,
    bic_code,
    name,
    country_code,
    is_active,
    risk_rating,
    kyc_status,
    kyc_verified_at,
    contact_email,
    metadata
) VALUES (
    'bank-in-001'::UUID,
    'SBININBBXXX',
    'State Bank of India',
    'IN',
    true,
    'LOW',
    'verified',
    NOW(),
    'operations@sbi.co.in',
    jsonb_build_object(
        'primary_currency', 'INR',
        'secondary_currencies', ARRAY['USD', 'EUR', 'AED'],
        'timezone', 'Asia/Kolkata',
        'regulatory_authority', 'Reserve Bank of India',
        'swift_enabled', true
    )
);

-- ============================================
-- BANK ACCOUNTS (Settlement Accounts)
-- ============================================

-- UAE Bank Accounts
INSERT INTO bank_accounts (bank_id, account_number, currency, balance, available_balance, reserved_balance, is_active)
VALUES
    ('bank-uae-001'::UUID, 'UAE-AED-001', 'AED', 500000000.00, 500000000.00, 0, true),
    ('bank-uae-001'::UUID, 'UAE-USD-001', 'USD', 100000000.00, 100000000.00, 0, true),
    ('bank-uae-001'::UUID, 'UAE-EUR-001', 'EUR', 50000000.00, 50000000.00, 0, true);

-- Israel Bank Accounts
INSERT INTO bank_accounts (bank_id, account_number, currency, balance, available_balance, reserved_balance, is_active)
VALUES
    ('bank-il-001'::UUID, 'IL-ILS-001', 'ILS', 300000000.00, 300000000.00, 0, true),
    ('bank-il-001'::UUID, 'IL-USD-001', 'USD', 80000000.00, 80000000.00, 0, true),
    ('bank-il-001'::UUID, 'IL-EUR-001', 'EUR', 40000000.00, 40000000.00, 0, true);

-- Pakistan Bank Accounts
INSERT INTO bank_accounts (bank_id, account_number, currency, balance, available_balance, reserved_balance, is_active)
VALUES
    ('bank-pk-001'::UUID, 'PK-PKR-001', 'PKR', 30000000000.00, 30000000000.00, 0, true),
    ('bank-pk-001'::UUID, 'PK-USD-001', 'USD', 100000000.00, 100000000.00, 0, true);

-- India Bank Accounts
INSERT INTO bank_accounts (bank_id, account_number, currency, balance, available_balance, reserved_balance, is_active)
VALUES
    ('bank-in-001'::UUID, 'IN-INR-001', 'INR', 40000000000.00, 40000000000.00, 0, true),
    ('bank-in-001'::UUID, 'IN-USD-001', 'USD', 150000000.00, 150000000.00, 0, true);

-- ============================================
-- CLEARING WINDOWS
-- ============================================

-- UAE Clearing Window (Dubai time, Sunday-Thursday)
INSERT INTO clearing_windows (
    bank_id,
    window_name,
    timezone,
    start_time,
    end_time,
    active_days,
    currencies,
    is_active,
    priority
) VALUES (
    'bank-uae-001'::UUID,
    'UAE Standard Clearing',
    'Asia/Dubai',
    '08:00:00'::TIME,
    '16:00:00'::TIME,
    ARRAY[1,2,3,4,7], -- Monday-Thursday + Sunday (UAE week)
    ARRAY['AED', 'USD', 'EUR', 'SAR'],
    true,
    100
);

-- Israel Clearing Window (Jerusalem time, Sunday-Thursday)
INSERT INTO clearing_windows (
    bank_id,
    window_name,
    timezone,
    start_time,
    end_time,
    active_days,
    currencies,
    is_active,
    priority
) VALUES (
    'bank-il-001'::UUID,
    'Israel Standard Clearing',
    'Asia/Jerusalem',
    '09:00:00'::TIME,
    '17:00:00'::TIME,
    ARRAY[1,2,3,4,7], -- Sunday-Thursday
    ARRAY['ILS', 'USD', 'EUR', 'GBP'],
    true,
    100
);

-- Pakistan Clearing Window (Karachi time, Monday-Friday)
INSERT INTO clearing_windows (
    bank_id,
    window_name,
    timezone,
    start_time,
    end_time,
    active_days,
    currencies,
    is_active,
    priority
) VALUES (
    'bank-pk-001'::UUID,
    'Pakistan Standard Clearing',
    'Asia/Karachi',
    '09:00:00'::TIME,
    '17:00:00'::TIME,
    ARRAY[1,2,3,4,5], -- Monday-Friday
    ARRAY['PKR', 'USD', 'AED'],
    true,
    100
);

-- India Clearing Window (Kolkata time, Monday-Saturday)
INSERT INTO clearing_windows (
    bank_id,
    window_name,
    timezone,
    start_time,
    end_time,
    active_days,
    currencies,
    is_active,
    priority
) VALUES (
    'bank-in-001'::UUID,
    'India Standard Clearing',
    'Asia/Kolkata',
    '10:00:00'::TIME,
    '18:00:00'::TIME,
    ARRAY[1,2,3,4,5,6], -- Monday-Saturday
    ARRAY['INR', 'USD', 'EUR', 'AED'],
    true,
    100
);

-- ============================================
-- VERIFICATION QUERIES
-- ============================================

-- Show all test banks
SELECT
    b.bic_code,
    b.name,
    b.country_code,
    b.is_active,
    b.risk_rating,
    b.metadata->>'primary_currency' as primary_currency,
    b.metadata->>'timezone' as timezone
FROM banks b
WHERE b.bic_code IN ('ENBXAEADXXX', 'LUMIILITXXX', 'HABBPKKKXXX', 'SBININBBXXX')
ORDER BY b.bic_code;

-- Show all bank accounts
SELECT
    b.bic_code,
    b.name,
    ba.currency,
    ba.account_number,
    ba.balance,
    ba.is_active
FROM banks b
JOIN bank_accounts ba ON b.id = ba.bank_id
WHERE b.bic_code IN ('ENBXAEADXXX', 'LUMIILITXXX', 'HABBPKKKXXX', 'SBININBBXXX')
ORDER BY b.bic_code, ba.currency;

-- Show clearing windows
SELECT
    b.bic_code,
    b.name,
    cw.window_name,
    cw.timezone,
    cw.start_time,
    cw.end_time,
    cw.active_days,
    cw.currencies,
    cw.is_active
FROM banks b
JOIN clearing_windows cw ON b.id = cw.bank_id
WHERE b.bic_code IN ('ENBXAEADXXX', 'LUMIILITXXX', 'HABBPKKKXXX', 'SBININBBXXX')
ORDER BY b.bic_code;

-- Test clearing window function
SELECT
    b.bic_code,
    b.name,
    is_bank_in_clearing_window(b.id, NULL, NOW()) as in_window_now
FROM banks b
WHERE b.bic_code IN ('ENBXAEADXXX', 'LUMIILITXXX', 'HABBPKKKXXX', 'SBININBBXXX')
ORDER BY b.bic_code;

-- Show liquidity pool status
SELECT * FROM v_liquidity_pool_status
ORDER BY pool_name, currency;

-- Show FX rates
SELECT
    from_currency,
    to_currency,
    rate,
    rate_source,
    valid_from
FROM v_latest_fx_rates
WHERE from_currency = 'USD'
ORDER BY to_currency;

COMMENT ON SCHEMA deltran IS 'DelTran Payment Rail - Test Banks Initialized: UAE, Israel, Pakistan, India - Ready for 3000 TPS stress test';
