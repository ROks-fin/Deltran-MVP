package templates

import (
	"bytes"
	"fmt"
	"html/template"

	"go.uber.org/zap"
)

type Manager struct {
	templates map[string]*template.Template
	logger    *zap.Logger
}

func NewManager(logger *zap.Logger) *Manager {
	mgr := &Manager{
		templates: make(map[string]*template.Template),
		logger:    logger,
	}

	// Load default templates
	mgr.loadDefaults()

	return mgr
}

func (m *Manager) loadDefaults() {
	// Transaction confirmation template
	txTemplate := `
<!DOCTYPE html>
<html>
<head>
	<style>
		.container { max-width: 600px; margin: 0 auto; font-family: Arial, sans-serif; }
		.header { background-color: #2c3e50; color: white; padding: 20px; text-align: center; }
		.content { padding: 20px; background-color: #f8f9fa; }
		.amount { font-size: 24px; font-weight: bold; color: #27ae60; }
	</style>
</head>
<body>
	<div class="container">
		<div class="header"><h1>Transaction Confirmed</h1></div>
		<div class="content">
			<p>Dear Customer,</p>
			<p>Your transaction has been processed successfully.</p>
			<div class="amount">{{.Amount}} {{.Currency}}</div>
			<p><strong>Transaction ID:</strong> {{.TransactionID}}</p>
			<p><strong>Date:</strong> {{.Date}}</p>
		</div>
	</div>
</body>
</html>
`

	tmpl, _ := template.New("transaction_confirmation").Parse(txTemplate)
	m.templates["transaction_confirmation"] = tmpl
}

func (m *Manager) Render(templateName string, data interface{}) (string, error) {
	tmpl, ok := m.templates[templateName]
	if !ok {
		return "", fmt.Errorf("template not found: %s", templateName)
	}

	var buf bytes.Buffer
	if err := tmpl.Execute(&buf, data); err != nil {
		return "", fmt.Errorf("failed to render template: %w", err)
	}

	return buf.String(), nil
}
