'use client';

import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import { Badge } from '@/components/ui/badge';
import { 
  TrendingUp, 
  TrendingDown, 
  DollarSign, 
  Wallet,
  PieChart,
  Activity
} from 'lucide-react';
import {
  LineChart,
  Line,
  PieChart as RechartsPieChart,
  Cell,
  ResponsiveContainer,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip
} from 'recharts';

interface PortfolioProps {
  portfolio: any;
}

const COLORS = ['#0088FE', '#00C49F', '#FFBB28', '#FF8042', '#8884D8'];

export function PortfolioOverview({ portfolio }: PortfolioProps) {
  if (!portfolio) {
    return (
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        {[1, 2, 3, 4].map((i) => (
          <Card key={i}>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium">Loading...</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="h-8 bg-gray-200 rounded animate-pulse"></div>
            </CardContent>
          </Card>
        ))}
      </div>
    );
  }

  const { summary, positions, wallets } = portfolio;
  const isProfit = !summary.totalPnL.startsWith('-');
  const totalPnLNum = parseFloat(summary.totalPnL);
  const totalPnLPercent = parseFloat(summary.totalPnLPercentage);

  // Prepare data for charts
  const positionData = positions.map((pos: any, index: number) => ({
    name: pos.symbol,
    value: parseFloat(pos.marketValue),
    pnl: pos.pnl.percentage,
    color: COLORS[index % COLORS.length]
  }));

  // Mock historical data - in production, fetch from Convex
  const historicalData = Array.from({ length: 30 }, (_, i) => ({
    date: new Date(Date.now() - (29 - i) * 24 * 60 * 60 * 1000).toLocaleDateString(),
    value: parseFloat(summary.totalValue) * (0.95 + Math.random() * 0.1)
  }));

  return (
    <div className="space-y-6">
      {/* Summary Cards */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Total Value</CardTitle>
            <DollarSign className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">${summary.totalValue}</div>
            <p className="text-xs text-muted-foreground">
              +2.1% from last month
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Total P&L</CardTitle>
            {isProfit ? 
              <TrendingUp className="h-4 w-4 text-green-600" /> : 
              <TrendingDown className="h-4 w-4 text-red-600" />
            }
          </CardHeader>
          <CardContent>
            <div className={`text-2xl font-bold ${isProfit ? 'text-green-600' : 'text-red-600'}`}>
              ${Math.abs(totalPnLNum).toFixed(2)}
            </div>
            <p className={`text-xs ${isProfit ? 'text-green-600' : 'text-red-600'}`}>
              {isProfit ? '+' : ''}{totalPnLPercent.toFixed(2)}% overall
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Positions</CardTitle>
            <PieChart className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{summary.positionCount}</div>
            <p className="text-xs text-muted-foreground">
              Across {wallets.length} wallets
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Active Wallets</CardTitle>
            <Wallet className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{wallets.length}</div>
            <p className="text-xs text-muted-foreground">
              Connected wallets
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Charts Section */}
      <div className="grid gap-6 md:grid-cols-2">
        {/* Portfolio Value Over Time */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Activity className="h-5 w-5" />
              Portfolio Value (30D)
            </CardTitle>
          </CardHeader>
          <CardContent>
            <ResponsiveContainer width="100%" height={300}>
              <LineChart data={historicalData}>
                <CartesianGrid strokeDasharray="3 3" />
                <XAxis 
                  dataKey="date" 
                  fontSize={12}
                  tickFormatter={(value) => value.slice(0, 5)}
                />
                <YAxis 
                  fontSize={12}
                  tickFormatter={(value) => `$${(value / 1000).toFixed(0)}K`}
                />
                <Tooltip 
                  formatter={(value: number) => [`$${value.toFixed(2)}`, 'Value']}
                  labelFormatter={(label) => `Date: ${label}`}
                />
                <Line 
                  type="monotone" 
                  dataKey="value" 
                  stroke="#8884d8" 
                  strokeWidth={2}
                  dot={false}
                />
              </LineChart>
            </ResponsiveContainer>
          </CardContent>
        </Card>

        {/* Position Allocation */}
        <Card>
          <CardHeader>
            <CardTitle>Position Allocation</CardTitle>
          </CardHeader>
          <CardContent>
            {positionData.length > 0 ? (
              <div className="space-y-4">
                <ResponsiveContainer width="100%" height={200}>
                  <RechartsPieChart>
                    <Pie
                      data={positionData}
                      cx="50%"
                      cy="50%"
                      outerRadius={80}
                      dataKey="value"
                    >
                      {positionData.map((entry, index) => (
                        <Cell key={`cell-${index}`} fill={entry.color} />
                      ))}
                    </Pie>
                    <Tooltip formatter={(value: number) => `$${value.toFixed(2)}`} />
                  </RechartsPieChart>
                </ResponsiveContainer>
                
                <div className="space-y-2">
                  {positionData.slice(0, 5).map((position, index) => (
                    <div key={position.name} className="flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        <div 
                          className="w-3 h-3 rounded-full" 
                          style={{ backgroundColor: position.color }}
                        />
                        <span className="text-sm font-medium">{position.name}</span>
                      </div>
                      <div className="flex items-center gap-2">
                        <span className="text-sm">${position.value.toFixed(2)}</span>
                        <Badge 
                          variant={position.pnl >= 0 ? "default" : "destructive"}
                          className="text-xs"
                        >
                          {position.pnl >= 0 ? '+' : ''}{position.pnl.toFixed(1)}%
                        </Badge>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            ) : (
              <div className="flex items-center justify-center h-48 text-muted-foreground">
                No positions to display
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Detailed Positions */}
      <Card>
        <CardHeader>
          <CardTitle>All Positions</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b">
                  <th className="text-left p-2">Token</th>
                  <th className="text-right p-2">Amount</th>
                  <th className="text-right p-2">Value</th>
                  <th className="text-right p-2">Avg Price</th>
                  <th className="text-right p-2">Current Price</th>
                  <th className="text-right p-2">P&L</th>
                  <th className="text-right p-2">Allocation</th>
                </tr>
              </thead>
              <tbody>
                {positions.map((position: any) => (
                  <tr key={position._id} className="border-b hover:bg-muted/50">
                    <td className="p-2">
                      <div className="font-medium">{position.symbol}</div>
                      <div className="text-sm text-muted-foreground">{position.name}</div>
                    </td>
                    <td className="text-right p-2">
                      {parseFloat(position.amount).toFixed(4)}
                    </td>
                    <td className="text-right p-2 font-medium">
                      ${parseFloat(position.marketValue).toFixed(2)}
                    </td>
                    <td className="text-right p-2">
                      ${parseFloat(position.averagePrice).toFixed(6)}
                    </td>
                    <td className="text-right p-2">
                      ${parseFloat(position.currentPrice).toFixed(6)}
                    </td>
                    <td className={`text-right p-2 font-medium ${
                      position.pnl.isProfit ? 'text-green-600' : 'text-red-600'
                    }`}>
                      {position.pnl.isProfit ? '+' : ''}${Math.abs(parseFloat(position.pnl.amount)).toFixed(2)}
                      <div className="text-xs">
                        ({position.pnl.percentage.toFixed(2)}%)
                      </div>
                    </td>
                    <td className="text-right p-2">
                      <div className="flex items-center gap-2">
                        <Progress 
                          value={position.allocation} 
                          className="w-16 h-2" 
                        />
                        <span className="text-xs w-12">
                          {position.allocation.toFixed(1)}%
                        </span>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}