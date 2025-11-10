#!/usr/bin/env python3
"""
Convert SQLx compile-time macros to runtime in nostro.rs and reconciliation.rs
"""

import re

def convert_nostro_account_queries(file_path):
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()

    # Fix imports
    content = re.sub(
        r'use sqlx::PgPool;',
        'use sqlx::{PgPool, Row};',
        content
    )

    # Fix the create_account query
    content = re.sub(
        r'sqlx::query\(\s*r#"\s*INSERT INTO nostro_accounts[^)]+\)VALUES \(\$1, \$2, \$3, \$4, \$5, \$6, \$7, true, \$8\)\s*"#,\s*account_id,\s*bank,\s*account_number,\s*currency,\s*initial_balance,\s*initial_balance,\s*Decimal::ZERO,\s*Utc::now\(\)\s*\)',
        r'''sqlx::query(
            r#"
            INSERT INTO nostro_accounts (
                id, bank, account_number, currency,
                ledger_balance, available_balance, locked_balance,
                is_active, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, true, $8)
            "#
        )
        .bind(account_id)
        .bind(bank)
        .bind(account_number)
        .bind(currency)
        .bind(initial_balance)
        .bind(initial_balance)
        .bind(Decimal::ZERO)
        .bind(Utc::now())''',
        content,
        flags=re.DOTALL
    )

    with open(file_path, 'w', encoding='utf-8') as f:
        f.write(content)

    print(f"Converted {file_path}")

if __name__ == '__main__':
    import sys
    if len(sys.argv) > 1:
        convert_nostro_account_queries(sys.argv[1])
