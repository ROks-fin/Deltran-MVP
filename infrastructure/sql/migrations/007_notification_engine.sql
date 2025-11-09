-- ============================================================================
-- NOTIFICATION ENGINE SCHEMA
-- Version: 1.0
-- Description: Tables for WebSocket, email, SMS, and push notifications
-- ============================================================================

-- Notification subscriptions
CREATE TABLE IF NOT EXISTS notification_subscriptions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bank_id UUID REFERENCES banks(id),
    user_id UUID,
    user_email VARCHAR(255),
    user_phone VARCHAR(20),

    -- Subscription details
    event_type VARCHAR(50) NOT NULL,
    channel VARCHAR(20) NOT NULL, -- Email, SMS, WebSocket, Webhook, Push
    destination TEXT NOT NULL, -- email address, phone, webhook URL, device token

    -- Filters
    filters JSONB DEFAULT '{}'::jsonb, -- Additional filtering criteria
    priority_filter VARCHAR(20), -- Only send notifications above this priority

    -- Status
    is_active BOOLEAN DEFAULT true,
    is_verified BOOLEAN DEFAULT false,
    verified_at TIMESTAMPTZ,

    -- Rate limiting
    max_notifications_per_hour INTEGER DEFAULT 100,
    current_hour_count INTEGER DEFAULT 0,
    hour_reset_at TIMESTAMPTZ,

    -- Preferences
    quiet_hours_start TIME,
    quiet_hours_end TIME,
    timezone VARCHAR(50) DEFAULT 'UTC',
    language VARCHAR(5) DEFAULT 'en',

    -- Metadata
    metadata JSONB DEFAULT '{}'::jsonb,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT valid_channel CHECK (channel IN ('Email', 'SMS', 'WebSocket', 'Webhook', 'Push')),
    INDEX idx_notif_subs_bank (bank_id),
    INDEX idx_notif_subs_user (user_id),
    INDEX idx_notif_subs_event (event_type),
    INDEX idx_notif_subs_channel (channel),
    INDEX idx_notif_subs_active (is_active)
);

-- Notification history
CREATE TABLE IF NOT EXISTS notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subscription_id UUID REFERENCES notification_subscriptions(id),

    -- Notification details
    event_type VARCHAR(50) NOT NULL,
    event_id UUID, -- Reference to original event
    channel VARCHAR(20) NOT NULL,
    destination TEXT NOT NULL,

    -- Content
    subject VARCHAR(255),
    content TEXT NOT NULL,
    content_html TEXT,
    template_id VARCHAR(100),
    template_variables JSONB,

    -- Priority
    priority VARCHAR(20) DEFAULT 'normal', -- low, normal, high, urgent

    -- Status tracking
    status VARCHAR(20) NOT NULL DEFAULT 'Pending',
    sent_at TIMESTAMPTZ,
    delivered_at TIMESTAMPTZ,
    read_at TIMESTAMPTZ,
    clicked_at TIMESTAMPTZ,
    failed_at TIMESTAMPTZ,

    -- Retry logic
    retry_count INTEGER DEFAULT 0,
    max_retries INTEGER DEFAULT 3,
    next_retry_at TIMESTAMPTZ,

    -- Error tracking
    error_message TEXT,
    error_code VARCHAR(50),

    -- External references
    external_id VARCHAR(255), -- Provider's message ID
    provider VARCHAR(50), -- SendGrid, Twilio, etc.

    -- Metadata
    metadata JSONB DEFAULT '{}'::jsonb,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT valid_status CHECK (status IN ('Pending', 'Sent', 'Delivered', 'Read', 'Failed', 'Bounced', 'Spam')),
    CONSTRAINT valid_priority CHECK (priority IN ('low', 'normal', 'high', 'urgent')),
    INDEX idx_notifications_subscription (subscription_id),
    INDEX idx_notifications_event (event_type, event_id),
    INDEX idx_notifications_status (status),
    INDEX idx_notifications_created (created_at DESC),
    INDEX idx_notifications_channel (channel),
    INDEX idx_notifications_destination (destination)
);

-- WebSocket connections tracking
CREATE TABLE IF NOT EXISTS websocket_connections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    connection_id VARCHAR(100) UNIQUE NOT NULL,
    server_instance VARCHAR(100) NOT NULL, -- For horizontal scaling

    -- User details
    user_id UUID,
    bank_id UUID REFERENCES banks(id),
    session_token VARCHAR(255),

    -- Connection details
    client_ip INET,
    user_agent TEXT,
    protocol_version VARCHAR(10),

    -- Status
    status VARCHAR(20) NOT NULL DEFAULT 'Connected',
    last_ping TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_pong TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Subscriptions
    subscribed_events TEXT[], -- Array of event types
    subscribed_channels TEXT[], -- Array of channel names

    -- Metrics
    messages_sent INTEGER DEFAULT 0,
    messages_received INTEGER DEFAULT 0,
    bytes_sent BIGINT DEFAULT 0,
    bytes_received BIGINT DEFAULT 0,

    -- Metadata
    metadata JSONB DEFAULT '{}'::jsonb,

    connected_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    disconnected_at TIMESTAMPTZ,

    CONSTRAINT valid_ws_status CHECK (status IN ('Connected', 'Disconnected', 'Error')),
    INDEX idx_ws_connections_user (user_id),
    INDEX idx_ws_connections_bank (bank_id),
    INDEX idx_ws_connections_status (status),
    INDEX idx_ws_connections_server (server_instance, status),
    INDEX idx_ws_connections_connected (connected_at DESC)
);

-- WebSocket messages log
CREATE TABLE IF NOT EXISTS websocket_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    connection_id UUID NOT NULL REFERENCES websocket_connections(id),

    -- Message details
    direction VARCHAR(10) NOT NULL, -- Inbound, Outbound
    message_type VARCHAR(50) NOT NULL,
    payload JSONB NOT NULL,
    payload_size INTEGER,

    -- Status
    status VARCHAR(20) DEFAULT 'Sent',
    delivered_at TIMESTAMPTZ,
    error_message TEXT,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT valid_direction CHECK (direction IN ('Inbound', 'Outbound')),
    INDEX idx_ws_messages_connection (connection_id, created_at DESC),
    INDEX idx_ws_messages_type (message_type)
);

-- Notification templates
CREATE TABLE IF NOT EXISTS notification_templates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    template_code VARCHAR(100) UNIQUE NOT NULL,
    template_name VARCHAR(255) NOT NULL,

    -- Content
    subject_template VARCHAR(500),
    body_template_text TEXT NOT NULL,
    body_template_html TEXT,

    -- Supported channels
    supported_channels VARCHAR(20)[] DEFAULT ARRAY['Email', 'SMS', 'WebSocket'],

    -- Localization
    language VARCHAR(5) NOT NULL DEFAULT 'en',
    category VARCHAR(50), -- transaction, compliance, settlement, system

    -- Variables
    required_variables TEXT[], -- Array of required template variables
    optional_variables TEXT[],
    sample_variables JSONB,

    -- Status
    is_active BOOLEAN DEFAULT true,
    version INTEGER DEFAULT 1,

    -- Metadata
    description TEXT,
    metadata JSONB DEFAULT '{}'::jsonb,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by VARCHAR(100),

    INDEX idx_templates_code (template_code),
    INDEX idx_templates_category (category),
    INDEX idx_templates_language (language),
    INDEX idx_templates_active (is_active)
);

-- Notification rate limiting
CREATE TABLE IF NOT EXISTS notification_rate_limits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_type VARCHAR(50) NOT NULL, -- User, Bank, Global
    entity_id UUID,

    -- Rate limits
    channel VARCHAR(20) NOT NULL,
    max_per_minute INTEGER,
    max_per_hour INTEGER,
    max_per_day INTEGER,

    -- Current counts
    current_minute_count INTEGER DEFAULT 0,
    current_hour_count INTEGER DEFAULT 0,
    current_day_count INTEGER DEFAULT 0,

    -- Reset timestamps
    minute_reset_at TIMESTAMPTZ,
    hour_reset_at TIMESTAMPTZ,
    day_reset_at TIMESTAMPTZ,

    -- Status
    is_throttled BOOLEAN DEFAULT false,
    throttled_until TIMESTAMPTZ,

    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(entity_type, entity_id, channel),
    INDEX idx_rate_limits_entity (entity_type, entity_id),
    INDEX idx_rate_limits_throttled (is_throttled, throttled_until)
);

-- Notification delivery stats
CREATE TABLE IF NOT EXISTS notification_delivery_stats (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    date DATE NOT NULL,
    hour INTEGER NOT NULL,

    -- Grouping
    channel VARCHAR(20) NOT NULL,
    event_type VARCHAR(50),
    priority VARCHAR(20),

    -- Counts
    sent_count INTEGER DEFAULT 0,
    delivered_count INTEGER DEFAULT 0,
    failed_count INTEGER DEFAULT 0,
    bounced_count INTEGER DEFAULT 0,
    read_count INTEGER DEFAULT 0,
    clicked_count INTEGER DEFAULT 0,

    -- Performance metrics
    avg_delivery_time_seconds DECIMAL(10,2),
    max_delivery_time_seconds DECIMAL(10,2),
    min_delivery_time_seconds DECIMAL(10,2),

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(date, hour, channel, event_type, priority),
    INDEX idx_delivery_stats_date (date DESC, hour DESC),
    INDEX idx_delivery_stats_channel (channel)
);

-- Email provider settings
CREATE TABLE IF NOT EXISTS email_provider_config (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider_name VARCHAR(50) NOT NULL UNIQUE, -- SendGrid, SMTP, Mock
    is_active BOOLEAN DEFAULT true,
    is_default BOOLEAN DEFAULT false,

    -- SMTP settings
    smtp_host VARCHAR(255),
    smtp_port INTEGER,
    smtp_username VARCHAR(255),
    smtp_password_encrypted BYTEA,
    smtp_use_tls BOOLEAN DEFAULT true,

    -- API settings
    api_key_encrypted BYTEA,
    api_endpoint VARCHAR(500),

    -- From address
    from_email VARCHAR(255),
    from_name VARCHAR(255),
    reply_to_email VARCHAR(255),

    -- Limits
    rate_limit_per_second INTEGER DEFAULT 10,
    daily_quota INTEGER,
    daily_sent INTEGER DEFAULT 0,
    quota_reset_at DATE,

    -- Metadata
    metadata JSONB DEFAULT '{}'::jsonb,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    INDEX idx_email_provider_active (is_active),
    INDEX idx_email_provider_default (is_default)
);

-- SMS provider settings
CREATE TABLE IF NOT EXISTS sms_provider_config (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider_name VARCHAR(50) NOT NULL UNIQUE, -- Twilio, Mock
    is_active BOOLEAN DEFAULT true,
    is_default BOOLEAN DEFAULT false,

    -- API settings
    api_key_encrypted BYTEA,
    api_secret_encrypted BYTEA,
    api_endpoint VARCHAR(500),
    account_sid VARCHAR(255),

    -- From number
    from_number VARCHAR(20),

    -- Limits
    rate_limit_per_second INTEGER DEFAULT 5,
    daily_quota INTEGER,
    daily_sent INTEGER DEFAULT 0,
    quota_reset_at DATE,

    -- Metadata
    metadata JSONB DEFAULT '{}'::jsonb,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    INDEX idx_sms_provider_active (is_active),
    INDEX idx_sms_provider_default (is_default)
);

-- Functions and triggers

-- Update notification rate limit counters
CREATE OR REPLACE FUNCTION update_notification_rate_limit()
RETURNS TRIGGER AS $$
DECLARE
    limit_record notification_rate_limits%ROWTYPE;
BEGIN
    -- Get or create rate limit record
    SELECT * INTO limit_record
    FROM notification_rate_limits
    WHERE entity_type = 'User'
        AND entity_id = (SELECT user_id FROM notification_subscriptions WHERE id = NEW.subscription_id)
        AND channel = NEW.channel;

    IF NOT FOUND THEN
        RETURN NEW;
    END IF;

    -- Increment counters
    UPDATE notification_rate_limits
    SET
        current_minute_count = current_minute_count + 1,
        current_hour_count = current_hour_count + 1,
        current_day_count = current_day_count + 1,
        updated_at = NOW()
    WHERE id = limit_record.id;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_rate_limit
    AFTER INSERT ON notifications
    FOR EACH ROW
    WHEN (NEW.status = 'Sent')
    EXECUTE FUNCTION update_notification_rate_limit();

-- Auto-cleanup old WebSocket connections
CREATE OR REPLACE FUNCTION cleanup_stale_websocket_connections()
RETURNS INTEGER AS $$
DECLARE
    disconnected_count INTEGER;
BEGIN
    UPDATE websocket_connections
    SET
        status = 'Disconnected',
        disconnected_at = NOW()
    WHERE status = 'Connected'
        AND last_pong < NOW() - INTERVAL '5 minutes';

    GET DIAGNOSTICS disconnected_count = ROW_COUNT;
    RETURN disconnected_count;
END;
$$ LANGUAGE plpgsql;

-- Aggregate notification stats
CREATE OR REPLACE FUNCTION aggregate_notification_stats()
RETURNS VOID AS $$
BEGIN
    INSERT INTO notification_delivery_stats (
        date, hour, channel, event_type, priority,
        sent_count, delivered_count, failed_count, read_count,
        avg_delivery_time_seconds
    )
    SELECT
        DATE(created_at) as date,
        EXTRACT(HOUR FROM created_at) as hour,
        channel,
        event_type,
        priority,
        COUNT(*) FILTER (WHERE status IN ('Sent', 'Delivered')) as sent_count,
        COUNT(*) FILTER (WHERE status = 'Delivered') as delivered_count,
        COUNT(*) FILTER (WHERE status = 'Failed') as failed_count,
        COUNT(*) FILTER (WHERE status = 'Read') as read_count,
        AVG(EXTRACT(EPOCH FROM (delivered_at - created_at))) as avg_delivery_time_seconds
    FROM notifications
    WHERE created_at >= DATE_TRUNC('hour', NOW() - INTERVAL '1 hour')
        AND created_at < DATE_TRUNC('hour', NOW())
    GROUP BY date, hour, channel, event_type, priority
    ON CONFLICT (date, hour, channel, event_type, priority) DO UPDATE SET
        sent_count = EXCLUDED.sent_count,
        delivered_count = EXCLUDED.delivered_count,
        failed_count = EXCLUDED.failed_count,
        read_count = EXCLUDED.read_count,
        avg_delivery_time_seconds = EXCLUDED.avg_delivery_time_seconds;
END;
$$ LANGUAGE plpgsql;

-- Views

-- Active WebSocket connections summary
CREATE OR REPLACE VIEW v_websocket_connections_active AS
SELECT
    server_instance,
    COUNT(*) as connection_count,
    COUNT(DISTINCT bank_id) as unique_banks,
    COUNT(DISTINCT user_id) as unique_users,
    SUM(messages_sent) as total_messages_sent,
    SUM(messages_received) as total_messages_received,
    AVG(EXTRACT(EPOCH FROM (NOW() - connected_at))) as avg_connection_age_seconds
FROM websocket_connections
WHERE status = 'Connected'
    AND last_pong > NOW() - INTERVAL '2 minutes'
GROUP BY server_instance;

-- Notification delivery metrics
CREATE OR REPLACE VIEW v_notification_metrics_24h AS
SELECT
    channel,
    event_type,
    COUNT(*) as total_sent,
    COUNT(*) FILTER (WHERE status = 'Delivered') as delivered,
    COUNT(*) FILTER (WHERE status = 'Failed') as failed,
    COUNT(*) FILTER (WHERE retry_count > 0) as retried,
    ROUND(100.0 * COUNT(*) FILTER (WHERE status = 'Delivered') / NULLIF(COUNT(*), 0), 2) as delivery_rate_percent,
    AVG(EXTRACT(EPOCH FROM (delivered_at - created_at))) as avg_delivery_time_seconds
FROM notifications
WHERE created_at > NOW() - INTERVAL '24 hours'
GROUP BY channel, event_type
ORDER BY total_sent DESC;

-- Initialize default notification templates
INSERT INTO notification_templates (
    template_code, template_name, subject_template,
    body_template_text, body_template_html,
    supported_channels, language, category, required_variables
) VALUES
(
    'TRANSACTION_CONFIRMED',
    'Transaction Confirmation',
    'Transaction {{transaction_id}} confirmed',
    'Your transaction of {{amount}} {{currency}} has been confirmed.\n\nTransaction ID: {{transaction_id}}\nStatus: {{status}}',
    '<h2>Transaction Confirmed</h2><p>Your transaction of <strong>{{amount}} {{currency}}</strong> has been confirmed.</p><p><strong>Transaction ID:</strong> {{transaction_id}}<br><strong>Status:</strong> {{status}}</p>',
    ARRAY['Email', 'SMS', 'WebSocket'],
    'en',
    'transaction',
    ARRAY['transaction_id', 'amount', 'currency', 'status']
),
(
    'SETTLEMENT_COMPLETED',
    'Settlement Completed',
    'Settlement completed for window {{window_id}}',
    'Settlement has been completed successfully.\n\nWindow: {{window_id}}\nAmount: {{amount}} {{currency}}\nCompleted at: {{completed_at}}',
    '<h2>Settlement Completed</h2><p>Settlement has been completed successfully.</p><p><strong>Window:</strong> {{window_id}}<br><strong>Amount:</strong> {{amount}} {{currency}}<br><strong>Completed:</strong> {{completed_at}}</p>',
    ARRAY['Email', 'WebSocket'],
    'en',
    'settlement',
    ARRAY['window_id', 'amount', 'currency', 'completed_at']
),
(
    'COMPLIANCE_ALERT',
    'Compliance Alert',
    'URGENT: Compliance alert for transaction {{transaction_id}}',
    'COMPLIANCE ALERT\n\nTransaction: {{transaction_id}}\nReason: {{reason}}\nAction Required: {{action_required}}',
    '<h2 style="color:red;">COMPLIANCE ALERT</h2><p><strong>Transaction:</strong> {{transaction_id}}<br><strong>Reason:</strong> {{reason}}<br><strong>Action Required:</strong> {{action_required}}</p>',
    ARRAY['Email', 'SMS', 'WebSocket'],
    'en',
    'compliance',
    ARRAY['transaction_id', 'reason', 'action_required']
)
ON CONFLICT DO NOTHING;

-- Initialize mock email provider
INSERT INTO email_provider_config (
    provider_name, is_active, is_default,
    from_email, from_name, rate_limit_per_second, daily_quota
) VALUES (
    'Mock', true, true,
    'noreply@deltran.example.com', 'DelTran Platform',
    100, 10000
) ON CONFLICT DO NOTHING;

-- Initialize mock SMS provider
INSERT INTO sms_provider_config (
    provider_name, is_active, is_default,
    from_number, rate_limit_per_second, daily_quota
) VALUES (
    'Mock', true, true,
    '+1234567890', 50, 5000
) ON CONFLICT DO NOTHING;

-- Grant permissions
GRANT SELECT, INSERT, UPDATE ON notification_subscriptions TO deltran;
GRANT SELECT, INSERT, UPDATE ON notifications TO deltran;
GRANT SELECT, INSERT, UPDATE ON websocket_connections TO deltran;
GRANT SELECT, INSERT ON websocket_messages TO deltran;
GRANT SELECT ON notification_templates TO deltran;
GRANT SELECT, UPDATE ON notification_rate_limits TO deltran;
GRANT SELECT, INSERT ON notification_delivery_stats TO deltran;
GRANT SELECT ON email_provider_config TO deltran;
GRANT SELECT ON sms_provider_config TO deltran;

-- Audit log entry
INSERT INTO audit_log (entity_type, action, changes)
VALUES ('database', 'MIGRATION_007_NOTIFICATION_ENGINE', '{"message": "Notification engine schema created successfully"}'::jsonb);

COMMIT;
