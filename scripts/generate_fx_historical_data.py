#!/usr/bin/env python3
"""
Generate realistic FX historical data for DelTran Risk Engine
Based on real market characteristics from 2015-2025

Uses geometric Brownian motion with realistic parameters:
- Historical volatilities
- Correlation between pairs
- Market sessions (Asian, European, American)
- Crisis periods (Brexit, COVID, etc.)
"""

import numpy as np
import pandas as pd
from datetime import datetime, timedelta
import psycopg2
from psycopg2.extras import execute_batch
import sys

# Real approximate rates and characteristics (2015-2025 averages)
CURRENCY_CONFIGS = {
    'USD/AED': {
        'initial_rate': 3.6725,  # Pegged to USD
        'annual_volatility': 0.001,  # Very low volatility (pegged)
        'drift': 0.0,
        'spread_bps': 0.5,
    },
    'USD/INR': {
        'initial_rate': 75.50,  # Average over period
        'annual_volatility': 0.06,  # 6% annualized
        'drift': 0.02,  # Gradual depreciation
        'spread_bps': 2.0,
    },
    'EUR/USD': {
        'initial_rate': 1.1200,
        'annual_volatility': 0.08,  # 8% annualized
        'drift': -0.01,
        'spread_bps': 0.1,
    },
    'GBP/USD': {
        'initial_rate': 1.3000,
        'annual_volatility': 0.10,  # 10% annualized (Brexit volatility)
        'drift': -0.015,
        'spread_bps': 0.2,
    },
    'EUR/AED': {
        'initial_rate': 4.1100,
        'annual_volatility': 0.08,
        'drift': -0.01,
        'spread_bps': 3.0,
    },
    'GBP/AED': {
        'initial_rate': 4.7700,
        'annual_volatility': 0.10,
        'drift': -0.015,
        'spread_bps': 3.5,
    },
    'EUR/INR': {
        'initial_rate': 84.60,
        'annual_volatility': 0.09,
        'drift': 0.01,
        'spread_bps': 4.0,
    },
    'GBP/INR': {
        'initial_rate': 98.15,
        'annual_volatility': 0.11,
        'drift': 0.01,
        'spread_bps': 4.5,
    },
    'AED/INR': {
        'initial_rate': 20.56,
        'annual_volatility': 0.06,
        'drift': 0.02,
        'spread_bps': 8.0,
    },
    'SAR/INR': {
        'initial_rate': 20.13,
        'annual_volatility': 0.06,
        'drift': 0.02,
        'spread_bps': 10.0,
    },
}

# Crisis periods with increased volatility
CRISIS_PERIODS = [
    ('2016-06-23', '2016-12-31', 2.5),  # Brexit
    ('2020-03-01', '2020-06-30', 3.0),  # COVID-19
    ('2022-02-24', '2022-05-31', 2.0),  # Ukraine conflict
]

def generate_gbm_path(S0, mu, sigma, T, dt, crisis_multiplier=1.0):
    """
    Generate Geometric Brownian Motion path
    dS = μS dt + σS dW

    Args:
        S0: Initial price
        mu: Drift (annualized return)
        sigma: Volatility (annualized std dev)
        T: Time period in years
        dt: Time step in years
        crisis_multiplier: Volatility multiplier during crisis
    """
    N = int(T / dt)
    t = np.linspace(0, T, N)
    W = np.random.standard_normal(size=N)
    W = np.cumsum(W) * np.sqrt(dt)

    # Adjust volatility for crisis periods
    sigma_adjusted = sigma * crisis_multiplier

    X = (mu - 0.5 * sigma_adjusted**2) * t + sigma_adjusted * W
    S = S0 * np.exp(X)

    return S

def is_crisis_period(date):
    """Check if date falls in a crisis period and return volatility multiplier"""
    for start, end, multiplier in CRISIS_PERIODS:
        if pd.to_datetime(start) <= date <= pd.to_datetime(end):
            return multiplier
    return 1.0

def generate_intraday_ticks(date, daily_open, daily_close, config, num_ticks=100):
    """Generate intraday ticks for a given day"""
    ticks = []

    # Use smaller time increments for intraday
    dt = 1 / (252 * num_ticks)  # 252 trading days per year
    crisis_mult = is_crisis_period(date)

    # Generate path from open to close
    prices = generate_gbm_path(
        daily_open,
        config['drift'],
        config['annual_volatility'],
        1/252,  # One trading day
        dt,
        crisis_mult
    )

    # Scale to end at daily_close
    prices = prices * (daily_close / prices[-1])

    # Generate timestamps throughout the day (24-hour FX market)
    base_time = pd.Timestamp(date)
    time_increments = pd.timedelta_range(
        start='0 hours',
        end='23.75 hours',
        periods=num_ticks
    )

    spread_bps = config['spread_bps']

    for i, (price, time_inc) in enumerate(zip(prices, time_increments)):
        timestamp = base_time + time_inc

        # Determine market session
        hour = timestamp.hour
        if 0 <= hour < 8:
            session = 'ASIAN'
            liquidity = 70
        elif 8 <= hour < 16:
            if 8 <= hour < 12:
                session = 'OVERLAP'  # Asian-European overlap
                liquidity = 90
            else:
                session = 'EUROPEAN'
                liquidity = 95
        else:
            if 16 <= hour < 17:
                session = 'OVERLAP'  # European-American overlap
                liquidity = 100
            else:
                session = 'AMERICAN'
                liquidity = 85

        # Calculate bid-ask spread (tighter during overlaps)
        spread_multiplier = 1.5 if 'OVERLAP' not in session else 1.0
        spread = price * (spread_bps / 10000) * spread_multiplier

        bid = price - spread / 2
        ask = price + spread / 2

        # Synthetic volume (higher during overlaps)
        base_volume = np.random.lognormal(15, 2)  # Log-normal distribution
        volume = base_volume * (1.5 if 'OVERLAP' in session else 1.0)

        ticks.append({
            'tick_timestamp': timestamp,
            'bid_price': round(bid, 8),
            'ask_price': round(ask, 8),
            'volume': round(volume, 2),
            'liquidity_score': liquidity,
            'market_session': session,
            'source': 'SIMULATED'
        })

    return ticks

def generate_daily_data(pair, config, start_date='2015-01-01', end_date='2025-01-01'):
    """Generate daily OHLC data for a currency pair"""
    dates = pd.bdate_range(start=start_date, end=end_date, freq='B')  # Business days
    num_days = len(dates)

    print(f"Generating {num_days} days of data for {pair}...")

    # Generate daily path
    T = num_days / 252  # Years
    dt = 1 / 252  # Daily

    # Split into segments to apply crisis volatility
    daily_data = []
    current_price = config['initial_rate']

    for i, date in enumerate(dates):
        crisis_mult = is_crisis_period(date)

        # Generate one day's price movement
        next_price = generate_gbm_path(
            current_price,
            config['drift'],
            config['annual_volatility'],
            dt,
            dt,
            crisis_mult
        )[0]

        # Generate OHLC with realistic intraday variation
        daily_vol = config['annual_volatility'] * crisis_mult / np.sqrt(252)
        intraday_noise = np.random.normal(0, daily_vol * current_price, 4)

        open_price = current_price
        close_price = next_price
        high_price = max(open_price, close_price) + abs(intraday_noise[0])
        low_price = min(open_price, close_price) - abs(intraday_noise[1])

        # Daily return
        daily_return = (close_price - open_price) / open_price if open_price > 0 else 0

        # Daily volume (log-normal)
        daily_volume = np.random.lognormal(18, 1.5)

        daily_data.append({
            'trade_date': date.date(),
            'open_price': round(open_price, 8),
            'high_price': round(high_price, 8),
            'low_price': round(low_price, 8),
            'close_price': round(close_price, 8),
            'daily_volume': round(daily_volume, 2),
            'daily_return': round(daily_return * 100, 6),  # In percentage
        })

        current_price = next_price

    df = pd.DataFrame(daily_data)

    # Calculate moving averages
    df['sma_7'] = df['close_price'].rolling(window=7).mean()
    df['sma_30'] = df['close_price'].rolling(window=30).mean()
    df['sma_90'] = df['close_price'].rolling(window=90).mean()

    # Calculate realized volatility (rolling 30-day)
    df['daily_volatility'] = df['daily_return'].rolling(window=30).std() * np.sqrt(252)

    return df

def calculate_volatility_metrics(daily_df, pair):
    """Calculate volatility metrics from daily data"""
    vol_data = []

    for i in range(90, len(daily_df)):
        date = daily_df.iloc[i]['trade_date']
        returns = daily_df.iloc[:i+1]['daily_return'].values

        # Different lookback periods
        vol_1d = np.std(returns[-1:]) * np.sqrt(252) if len(returns) >= 1 else None
        vol_7d = np.std(returns[-7:]) * np.sqrt(252) if len(returns) >= 7 else None
        vol_30d = np.std(returns[-30:]) * np.sqrt(252) if len(returns) >= 30 else None
        vol_90d = np.std(returns[-90:]) * np.sqrt(252) if len(returns) >= 90 else None
        vol_365d = np.std(returns[-252:]) * np.sqrt(252) if len(returns) >= 252 else None

        # VaR calculations (95% and 99% confidence)
        recent_returns = returns[-30:]
        var_95 = np.percentile(recent_returns, 5) if len(recent_returns) > 0 else None
        var_99 = np.percentile(recent_returns, 1) if len(recent_returns) > 0 else None

        # Max drawdown/surge in last 30 days
        prices_30d = daily_df.iloc[max(0, i-30):i+1]['close_price'].values
        if len(prices_30d) > 1:
            peak = np.maximum.accumulate(prices_30d)
            drawdown = ((prices_30d - peak) / peak * 100).min()
            surge = ((prices_30d / np.minimum.accumulate(prices_30d) - 1) * 100).max()
        else:
            drawdown = 0
            surge = 0

        vol_data.append({
            'calculation_date': date,
            'volatility_1d': round(vol_1d, 6) if vol_1d else None,
            'volatility_7d': round(vol_7d, 6) if vol_7d else None,
            'volatility_30d': round(vol_30d, 6) if vol_30d else None,
            'volatility_90d': round(vol_90d, 6) if vol_90d else None,
            'volatility_365d': round(vol_365d, 6) if vol_365d else None,
            'var_95_1d': round(var_95, 6) if var_95 else None,
            'var_99_1d': round(var_99, 6) if var_99 else None,
            'max_drawdown_30d': round(drawdown, 6),
            'max_surge_30d': round(surge, 6),
        })

    return pd.DataFrame(vol_data)

def insert_daily_data(conn, pair, df):
    """Insert daily data into database"""
    base, quote = pair.split('/')

    records = []
    for _, row in df.iterrows():
        records.append((
            pair, base, quote,
            row['trade_date'],
            row['open_price'], row['high_price'], row['low_price'], row['close_price'],
            row['daily_volume'],
            row.get('daily_volatility'),
            row['daily_return'],
            row.get('sma_7'), row.get('sma_30'), row.get('sma_90')
        ))

    cur = conn.cursor()
    execute_batch(cur, """
        INSERT INTO fx_rate_daily (
            currency_pair, base_currency, quote_currency,
            trade_date,
            open_price, high_price, low_price, close_price,
            daily_volume, daily_volatility, daily_return,
            sma_7, sma_30, sma_90
        ) VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s)
        ON CONFLICT (currency_pair, trade_date) DO UPDATE SET
            open_price = EXCLUDED.open_price,
            high_price = EXCLUDED.high_price,
            low_price = EXCLUDED.low_price,
            close_price = EXCLUDED.close_price
    """, records, page_size=1000)

    conn.commit()
    print(f"Inserted {len(records)} daily records for {pair}")

def insert_volatility_data(conn, pair, df):
    """Insert volatility metrics into database"""
    records = []
    for _, row in df.iterrows():
        records.append((
            pair,
            row['calculation_date'],
            row.get('volatility_1d'), row.get('volatility_7d'),
            row.get('volatility_30d'), row.get('volatility_90d'), row.get('volatility_365d'),
            row.get('var_95_1d'), row.get('var_99_1d'),
            row['max_drawdown_30d'], row['max_surge_30d']
        ))

    cur = conn.cursor()
    execute_batch(cur, """
        INSERT INTO fx_rate_volatility (
            currency_pair, calculation_date,
            volatility_1d, volatility_7d, volatility_30d, volatility_90d, volatility_365d,
            var_95_1d, var_99_1d,
            max_drawdown_30d, max_surge_30d
        ) VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s)
        ON CONFLICT (currency_pair, calculation_date) DO UPDATE SET
            volatility_30d = EXCLUDED.volatility_30d
    """, records, page_size=1000)

    conn.commit()
    print(f"Inserted {len(records)} volatility records for {pair}")

def main():
    # Database connection
    conn = psycopg2.connect(
        host="localhost",
        port=5432,
        database="deltran",
        user="postgres",
        password="deltran2025"
    )

    print("=" * 70)
    print("DelTran FX Historical Data Generator")
    print("=" * 70)
    print(f"Generating data for {len(CURRENCY_CONFIGS)} currency pairs")
    print(f"Date range: 2015-01-01 to 2025-01-01 (10 years)")
    print(f"Crisis periods included: {len(CRISIS_PERIODS)}")
    print("=" * 70)

    for pair, config in CURRENCY_CONFIGS.items():
        print(f"\n{'='*70}")
        print(f"Processing {pair}")
        print(f"  Initial rate: {config['initial_rate']}")
        print(f"  Volatility: {config['annual_volatility']*100:.1f}%")
        print(f"  Spread: {config['spread_bps']} bps")
        print(f"{'='*70}")

        # Generate daily data
        daily_df = generate_daily_data(pair, config)
        insert_daily_data(conn, pair, daily_df)

        # Calculate and insert volatility metrics
        vol_df = calculate_volatility_metrics(daily_df, pair)
        insert_volatility_data(conn, pair, vol_df)

        print(f"✓ Completed {pair}")

    conn.close()

    print("\n" + "="*70)
    print("✓ All historical data generated successfully!")
    print("="*70)
    print("\nData summary:")
    print(f"  - Daily OHLC data: ~2,500 records per pair")
    print(f"  - Volatility metrics: ~2,400 records per pair")
    print(f"  - Total pairs: {len(CURRENCY_CONFIGS)}")
    print(f"  - Total records: ~{len(CURRENCY_CONFIGS) * 4900:,}")
    print("\nQuery examples:")
    print("  SELECT * FROM fx_rate_daily WHERE currency_pair = 'USD/INR' ORDER BY trade_date DESC LIMIT 30;")
    print("  SELECT * FROM fx_rate_volatility WHERE volatility_30d > 10 ORDER BY calculation_date DESC;")

if __name__ == '__main__':
    np.random.seed(42)  # For reproducibility
    main()
