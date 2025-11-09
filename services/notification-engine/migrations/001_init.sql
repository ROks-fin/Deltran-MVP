-- Notification Engine Database Schema

-- Notifications table
CREATE TABLE IF NOT EXISTS notifications (
    id UUID PRIMARY KEY,
    user_id VARCHAR(50) NOT NULL,
    bank_id VARCHAR(50),
    type VARCHAR(20) NOT NULL CHECK (type IN ('email', 'sms', 'websocket', 'push')),
    template_id VARCHAR(50),
    subject VARCHAR(255),
    content TEXT NOT NULL,
    metadata JSONB DEFAULT '{}',
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'sent', 'delivered', 'failed', 'retrying')),
    sent_at TIMESTAMP,
    read_at TIMESTAMP,
    retry_count INT DEFAULT 0,
    error_message TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_notifications_user_id ON notifications(user_id);
CREATE INDEX idx_notifications_bank_id ON notifications(bank_id) WHERE bank_id IS NOT NULL;
CREATE INDEX idx_notifications_status ON notifications(status);
CREATE INDEX idx_notifications_type ON notifications(type);
CREATE INDEX idx_notifications_created_at ON notifications(created_at DESC);

-- User notification preferences
CREATE TABLE IF NOT EXISTS notification_preferences (
    user_id VARCHAR(50) PRIMARY KEY,
    email_enabled BOOLEAN DEFAULT true,
    sms_enabled BOOLEAN DEFAULT true,
    push_enabled BOOLEAN DEFAULT true,
    websocket_enabled BOOLEAN DEFAULT true,
    email_address VARCHAR(255),
    phone_number VARCHAR(20),
    locale VARCHAR(10) DEFAULT 'en',
    timezone VARCHAR(50) DEFAULT 'UTC',
    quiet_hours_start TIME,
    quiet_hours_end TIME,
    preferences JSONB DEFAULT '{}',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Notification templates
CREATE TABLE IF NOT EXISTS notification_templates (
    id VARCHAR(50) PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    type VARCHAR(20) NOT NULL CHECK (type IN ('email', 'sms')),
    subject VARCHAR(255),
    body TEXT NOT NULL,
    locale VARCHAR(10) DEFAULT 'en',
    variables JSONB DEFAULT '[]',
    active BOOLEAN DEFAULT true,
    version INT DEFAULT 1,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_templates_type_locale ON notification_templates(type, locale);
CREATE INDEX idx_templates_active ON notification_templates(active) WHERE active = true;

-- Insert default templates
INSERT INTO notification_templates (id, name, type, subject, body, locale, active) VALUES
('tx_confirm_en', 'Transaction Confirmation', 'email', 'Transaction Confirmed', 
 'Your transaction {{.TransactionID}} for {{.Amount}} {{.Currency}} has been confirmed.', 'en', true),
('tx_confirm_ru', 'Подтверждение транзакции', 'email', 'Транзакция подтверждена',
 'Ваша транзакция {{.TransactionID}} на сумму {{.Amount}} {{.Currency}} подтверждена.', 'ru', true),
('tx_confirm_ar', 'تأكيد المعاملة', 'email', 'تم تأكيد المعاملة',
 'تم تأكيد معاملتك {{.TransactionID}} بمبلغ {{.Amount}} {{.Currency}}.', 'ar', true)
ON CONFLICT (id) DO NOTHING;

-- Create function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Create triggers for updated_at
CREATE TRIGGER update_notifications_updated_at BEFORE UPDATE ON notifications
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_preferences_updated_at BEFORE UPDATE ON notification_preferences
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_templates_updated_at BEFORE UPDATE ON notification_templates
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
