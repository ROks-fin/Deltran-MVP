import React, { useState, useEffect } from 'react';
import {
  Activity,
  TrendingUp,
  Clock,
  CheckCircle,
  DollarSign,
  Zap,
  Globe,
  BarChart3,
  ArrowRight,
} from 'lucide-react';
import { AreaChart, Area, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts';

interface Transaction {
  id: string;
  from: string;
  to: string;
  amount: string;
  currency: string;
  status: 'pending' | 'clearing' | 'settling' | 'completed';
  timestamp: number;
  latency: number;
}

interface Metrics {
  totalVolume: number;
  liquiditySaved: number;
  avgLatency: number;
  successRate: number;
  transactionsCompleted: number;
  savingsPercent: number;
}

function App() {
  const [transactions, setTransactions] = useState<Transaction[]>([]);
  const [metrics, setMetrics] = useState<Metrics>({
    totalVolume: 0,
    liquiditySaved: 0,
    avgLatency: 0,
    successRate: 100,
    transactionsCompleted: 0,
    savingsPercent: 0,
  });

  const [chartData, setChartData] = useState<any[]>([]);
  const [isLive, setIsLive] = useState(false);

  // Simulate real-time updates
  useEffect(() => {
    if (!isLive) return;

    const demoTransactions = [
      {
        id: 'TX-001',
        from: 'Emirates NBD (UAE)',
        to: 'ICICI Bank (India)',
        amount: '10,000',
        currency: 'AED',
      },
      {
        id: 'TX-002',
        from: 'ADCB (UAE)',
        to: 'HDFC Bank (India)',
        amount: '25,000',
        currency: 'AED',
      },
      {
        id: 'TX-003',
        from: 'FAB (UAE)',
        to: 'SBI (India)',
        amount: '50,000',
        currency: 'AED',
      },
      {
        id: 'TX-004',
        from: 'ICICI Bank (India)',
        to: 'Emirates NBD (UAE)',
        amount: '30,000',
        currency: 'INR',
      },
    ];

    let currentIndex = 0;

    const interval = setInterval(() => {
      if (currentIndex >= demoTransactions.length) {
        // Show netting summary
        setTimeout(() => {
          setMetrics({
            totalVolume: 85000,
            liquiditySaved: 35000,
            avgLatency: 850,
            successRate: 100,
            transactionsCompleted: 4,
            savingsPercent: 41.2,
          });
        }, 2000);

        clearInterval(interval);
        return;
      }

      const tx = demoTransactions[currentIndex];
      const newTx: Transaction = {
        ...tx,
        status: 'pending',
        timestamp: Date.now(),
        latency: 0,
      };

      setTransactions((prev) => [...prev, newTx]);

      // Simulate status updates
      setTimeout(() => {
        updateTransactionStatus(newTx.id, 'clearing');
      }, 300);

      setTimeout(() => {
        updateTransactionStatus(newTx.id, 'settling');
      }, 600);

      setTimeout(() => {
        updateTransactionStatus(newTx.id, 'completed');
        setMetrics((prev) => ({
          ...prev,
          transactionsCompleted: prev.transactionsCompleted + 1,
          totalVolume: prev.totalVolume + parseInt(tx.amount.replace(',', '')),
        }));
      }, 900);

      currentIndex++;
    }, 2000);

    return () => clearInterval(interval);
  }, [isLive]);

  const updateTransactionStatus = (id: string, status: Transaction['status']) => {
    setTransactions((prev) =>
      prev.map((tx) =>
        tx.id === id
          ? { ...tx, status, latency: Date.now() - tx.timestamp }
          : tx
      )
    );
  };

  const startDemo = () => {
    setTransactions([]);
    setMetrics({
      totalVolume: 0,
      liquiditySaved: 0,
      avgLatency: 0,
      successRate: 100,
      transactionsCompleted: 0,
      savingsPercent: 0,
    });
    setIsLive(true);
  };

  const getStatusColor = (status: Transaction['status']) => {
    switch (status) {
      case 'pending':
        return 'bg-yellow-500';
      case 'clearing':
        return 'bg-blue-500';
      case 'settling':
        return 'bg-purple-500';
      case 'completed':
        return 'bg-green-500';
      default:
        return 'bg-gray-500';
    }
  };

  const getStatusText = (status: Transaction['status']) => {
    switch (status) {
      case 'pending':
        return 'Funding Confirmed';
      case 'clearing':
        return 'Clearing';
      case 'settling':
        return 'Settling';
      case 'completed':
        return 'Completed';
      default:
        return status;
    }
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-900 via-blue-900 to-slate-900 text-white">
      {/* Header */}
      <div className="border-b border-white/10 bg-black/20 backdrop-blur-sm">
        <div className="container mx-auto px-6 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-3">
              <div className="h-10 w-10 rounded-lg bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center">
                <Zap className="h-6 w-6" />
              </div>
              <div>
                <h1 className="text-2xl font-bold">DelTran Protocol</h1>
                <p className="text-sm text-gray-400">Real-time Cross-Border Settlement</p>
              </div>
            </div>
            <div className="flex items-center space-x-4">
              <div className="flex items-center space-x-2">
                <div className={`h-2 w-2 rounded-full ${isLive ? 'bg-green-500 animate-pulse' : 'bg-gray-500'}`} />
                <span className="text-sm">{isLive ? 'Live Demo' : 'Ready'}</span>
              </div>
              {!isLive && (
                <button
                  onClick={startDemo}
                  className="px-6 py-2 bg-gradient-to-r from-blue-600 to-purple-600 rounded-lg font-semibold hover:from-blue-700 hover:to-purple-700 transition-all"
                >
                  Start Demo
                </button>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="container mx-auto px-6 py-8">
        {/* Key Metrics */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
          <MetricCard
            icon={<DollarSign className="h-6 w-6" />}
            label="Total Volume"
            value={`${metrics.totalVolume.toLocaleString()} AED`}
            trend="+12.5%"
            color="blue"
          />
          <MetricCard
            icon={<TrendingUp className="h-6 w-6" />}
            label="Liquidity Saved"
            value={`${metrics.liquiditySaved.toLocaleString()} AED`}
            trend={`${metrics.savingsPercent.toFixed(1)}%`}
            color="green"
          />
          <MetricCard
            icon={<Clock className="h-6 w-6" />}
            label="Avg Latency"
            value={`${metrics.avgLatency}ms`}
            trend="99.9% faster"
            color="purple"
          />
          <MetricCard
            icon={<CheckCircle className="h-6 w-6" />}
            label="Success Rate"
            value={`${metrics.successRate}%`}
            trend={`${metrics.transactionsCompleted} completed`}
            color="emerald"
          />
        </div>

        {/* Transaction Flow */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-8">
          {/* Active Transactions */}
          <div className="bg-white/5 backdrop-blur-sm rounded-xl p-6 border border-white/10">
            <div className="flex items-center justify-between mb-6">
              <h2 className="text-xl font-bold flex items-center space-x-2">
                <Activity className="h-5 w-5 text-blue-400" />
                <span>Live Transactions</span>
              </h2>
              <span className="text-sm text-gray-400">
                {transactions.length} active
              </span>
            </div>

            <div className="space-y-4 max-h-[500px] overflow-y-auto">
              {transactions.length === 0 ? (
                <div className="text-center py-12 text-gray-500">
                  <Globe className="h-12 w-12 mx-auto mb-4 opacity-50" />
                  <p>Start demo to see live transactions</p>
                </div>
              ) : (
                transactions.map((tx) => (
                  <TransactionCard key={tx.id} transaction={tx} />
                ))
              )}
            </div>
          </div>

          {/* Netting Visualization */}
          <div className="bg-white/5 backdrop-blur-sm rounded-xl p-6 border border-white/10">
            <div className="flex items-center justify-between mb-6">
              <h2 className="text-xl font-bold flex items-center space-x-2">
                <BarChart3 className="h-5 w-5 text-purple-400" />
                <span>Multilateral Netting</span>
              </h2>
            </div>

            {metrics.transactionsCompleted > 0 ? (
              <div className="space-y-6">
                <div>
                  <div className="flex justify-between mb-2">
                    <span className="text-sm text-gray-400">Gross Settlement</span>
                    <span className="font-semibold">85,000 AED</span>
                  </div>
                  <div className="h-4 bg-white/10 rounded-full overflow-hidden">
                    <div className="h-full bg-gradient-to-r from-red-500 to-orange-500 w-full" />
                  </div>
                </div>

                <div>
                  <div className="flex justify-between mb-2">
                    <span className="text-sm text-gray-400">Net Settlement</span>
                    <span className="font-semibold text-green-400">50,000 AED</span>
                  </div>
                  <div className="h-4 bg-white/10 rounded-full overflow-hidden">
                    <div className="h-full bg-gradient-to-r from-green-500 to-emerald-500 w-[59%]" />
                  </div>
                </div>

                <div className="p-4 bg-green-500/10 border border-green-500/20 rounded-lg">
                  <div className="flex items-center justify-between">
                    <span className="text-green-400 font-semibold">Liquidity Saved</span>
                    <span className="text-2xl font-bold text-green-400">
                      35,000 AED (41%)
                    </span>
                  </div>
                  <p className="text-sm text-gray-400 mt-2">
                    Capital freed for lending and investment
                  </p>
                </div>

                <div className="grid grid-cols-2 gap-4 mt-4">
                  <div className="p-3 bg-blue-500/10 border border-blue-500/20 rounded-lg">
                    <div className="text-xs text-gray-400 mb-1">Traditional</div>
                    <div className="text-lg font-bold">2-5 days</div>
                  </div>
                  <div className="p-3 bg-purple-500/10 border border-purple-500/20 rounded-lg">
                    <div className="text-xs text-gray-400 mb-1">DelTran</div>
                    <div className="text-lg font-bold text-purple-400">&lt;1 second</div>
                  </div>
                </div>
              </div>
            ) : (
              <div className="text-center py-12 text-gray-500">
                <TrendingUp className="h-12 w-12 mx-auto mb-4 opacity-50" />
                <p>Netting data will appear after transactions complete</p>
              </div>
            )}
          </div>
        </div>

        {/* Key Differentiators */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          <FeatureCard
            title="Real-time Settlement"
            description="< 1 second end-to-end vs 2-5 days traditional"
            icon={<Zap className="h-8 w-8" />}
            color="yellow"
          />
          <FeatureCard
            title="1:1 Backed Tokens"
            description="Every token backed by real fiat in segregated EMI accounts"
            icon={<CheckCircle className="h-8 w-8" />}
            color="green"
          />
          <FeatureCard
            title="40-60% Liquidity Savings"
            description="Multilateral netting drastically reduces locked capital"
            icon={<TrendingUp className="h-8 w-8" />}
            color="purple"
          />
        </div>
      </div>
    </div>
  );
}

function MetricCard({
  icon,
  label,
  value,
  trend,
  color,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  trend: string;
  color: string;
}) {
  const colorClasses = {
    blue: 'from-blue-500 to-blue-600',
    green: 'from-green-500 to-emerald-600',
    purple: 'from-purple-500 to-purple-600',
    emerald: 'from-emerald-500 to-teal-600',
  };

  return (
    <div className="bg-white/5 backdrop-blur-sm rounded-xl p-6 border border-white/10">
      <div className={`inline-flex p-3 rounded-lg bg-gradient-to-br ${colorClasses[color as keyof typeof colorClasses]} mb-4`}>
        {icon}
      </div>
      <div className="text-sm text-gray-400 mb-1">{label}</div>
      <div className="text-2xl font-bold mb-1">{value}</div>
      <div className="text-sm text-green-400">{trend}</div>
    </div>
  );
}

function TransactionCard({ transaction }: { transaction: Transaction }) {
  return (
    <div className="bg-white/5 backdrop-blur-sm rounded-lg p-4 border border-white/10 hover:border-white/20 transition-all">
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center space-x-3">
          <div
            className={`h-2 w-2 rounded-full ${transaction.status === 'completed' ? 'bg-green-500' : 'bg-blue-500 animate-pulse'}`}
          />
          <span className="font-mono text-sm text-gray-400">{transaction.id}</span>
        </div>
        <span
          className={`px-3 py-1 rounded-full text-xs font-semibold ${transaction.status === 'completed' ? 'bg-green-500/20 text-green-400' : 'bg-blue-500/20 text-blue-400'}`}
        >
          {getStatusText(transaction.status)}
        </span>
      </div>

      <div className="flex items-center justify-between mb-2">
        <div className="flex-1">
          <div className="text-sm font-semibold">{transaction.from}</div>
          <div className="text-xs text-gray-500">Sender</div>
        </div>
        <ArrowRight className="h-5 w-5 text-gray-600 mx-4" />
        <div className="flex-1 text-right">
          <div className="text-sm font-semibold">{transaction.to}</div>
          <div className="text-xs text-gray-500">Receiver</div>
        </div>
      </div>

      <div className="flex items-center justify-between pt-3 border-t border-white/10">
        <div>
          <span className="text-xl font-bold">{transaction.amount}</span>
          <span className="text-sm text-gray-400 ml-2">{transaction.currency}</span>
        </div>
        {transaction.latency > 0 && (
          <div className="text-xs text-gray-500">
            {transaction.latency}ms
          </div>
        )}
      </div>
    </div>
  );
}

function FeatureCard({
  title,
  description,
  icon,
  color,
}: {
  title: string;
  description: string;
  icon: React.ReactNode;
  color: string;
}) {
  const colorClasses = {
    yellow: 'from-yellow-500 to-orange-600',
    green: 'from-green-500 to-emerald-600',
    purple: 'from-purple-500 to-pink-600',
  };

  return (
    <div className="bg-white/5 backdrop-blur-sm rounded-xl p-6 border border-white/10">
      <div className={`inline-flex p-3 rounded-lg bg-gradient-to-br ${colorClasses[color as keyof typeof colorClasses]} mb-4`}>
        {icon}
      </div>
      <h3 className="text-lg font-bold mb-2">{title}</h3>
      <p className="text-sm text-gray-400">{description}</p>
    </div>
  );
}

function getStatusText(status: Transaction['status']): string {
  switch (status) {
    case 'pending':
      return 'Funding Confirmed';
    case 'clearing':
      return 'Clearing';
    case 'settling':
      return 'Settling';
    case 'completed':
      return 'Completed';
    default:
      return status;
  }
}

export default App;
