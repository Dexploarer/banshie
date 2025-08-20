'use client';

import { useQuery } from 'convex/react';
import { api } from '../../convex/_generated/api';
import { PortfolioOverview } from '@/components/portfolio/PortfolioOverview';
import { TradingInterface } from '@/components/trading/TradingInterface';
import { MarketData } from '@/components/market/MarketData';
import { DCAStrategies } from '@/components/dca/DCAStrategies';
import { AlertsPanel } from '@/components/alerts/AlertsPanel';
import { AISignals } from '@/components/ai/AISignals';
import { Header } from '@/components/layout/Header';
import { Sidebar } from '@/components/layout/Sidebar';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { useState, useEffect } from 'react';

export default function Dashboard() {
  const [activeTab, setActiveTab] = useState('portfolio');
  const [userId, setUserId] = useState<string | null>(null);

  // Mock authentication - in production, use proper auth
  useEffect(() => {
    // Get user from localStorage or auth provider
    const mockUserId = 'user_123' as any; // This would come from authentication
    setUserId(mockUserId);
  }, []);

  // Real-time portfolio data from Convex
  const portfolio = useQuery(
    api.queries.portfolio.getPortfolio,
    userId ? { userId } : 'skip'
  );

  // Real-time price data
  const marketData = useQuery(
    api.queries.prices.getMarketOverview,
    { category: 'all', sortBy: 'volume', limit: 10 }
  );

  // Real-time DCA strategies
  const dcaStrategies = useQuery(
    api.queries.dca.getUserStrategies,
    userId ? { userId } : 'skip'
  );

  // Real-time alerts
  const alerts = useQuery(
    api.queries.alerts.getUserAlerts,
    userId ? { userId } : 'skip'
  );

  // AI signals for premium users
  const aiSignals = useQuery(
    api.queries.ai.getLatestSignals,
    { limit: 5 }
  );

  if (!userId) {
    return (
      <div className="flex h-screen items-center justify-center">
        <div className="text-center">
          <h1 className="text-2xl font-bold mb-4">Solana Trading Dashboard</h1>
          <p>Loading authentication...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-screen bg-background">
      <Sidebar activeTab={activeTab} onTabChange={setActiveTab} />
      
      <div className="flex-1 flex flex-col overflow-hidden">
        <Header user={{ name: 'John Doe', isPremium: true }} />
        
        <main className="flex-1 overflow-x-hidden overflow-y-auto bg-gray-50 p-6">
          <Tabs value={activeTab} onValueChange={setActiveTab}>
            {/* Portfolio Overview */}
            <TabsContent value="portfolio" className="space-y-6">
              <div className="grid gap-6">
                <PortfolioOverview portfolio={portfolio} />
                
                <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
                  <Card>
                    <CardHeader>
                      <CardTitle>Recent Activity</CardTitle>
                    </CardHeader>
                    <CardContent>
                      {/* Recent trades, orders, etc. */}
                      <div className="space-y-2">
                        <p className="text-sm text-muted-foreground">
                          Your recent trading activity will appear here
                        </p>
                      </div>
                    </CardContent>
                  </Card>
                  
                  <Card>
                    <CardHeader>
                      <CardTitle>Performance Metrics</CardTitle>
                    </CardHeader>
                    <CardContent>
                      {portfolio && (
                        <div className="space-y-4">
                          <div className="flex justify-between">
                            <span>Total P&L:</span>
                            <span className={portfolio.summary.totalPnL.startsWith('-') ? 'text-red-600' : 'text-green-600'}>
                              ${portfolio.summary.totalPnL}
                            </span>
                          </div>
                          <div className="flex justify-between">
                            <span>Win Rate:</span>
                            <span>{portfolio.user.stats.successRate.toFixed(1)}%</span>
                          </div>
                          <div className="flex justify-between">
                            <span>Total Trades:</span>
                            <span>{portfolio.user.stats.totalTrades}</span>
                          </div>
                        </div>
                      )}
                    </CardContent>
                  </Card>
                </div>
              </div>
            </TabsContent>

            {/* Trading Interface */}
            <TabsContent value="trading" className="space-y-6">
              <div className="grid gap-6">
                <TradingInterface />
                <MarketData data={marketData} />
              </div>
            </TabsContent>

            {/* DCA Strategies */}
            <TabsContent value="dca" className="space-y-6">
              <DCAStrategies strategies={dcaStrategies} />
            </TabsContent>

            {/* Alerts */}
            <TabsContent value="alerts" className="space-y-6">
              <AlertsPanel alerts={alerts} />
            </TabsContent>

            {/* Market Analysis */}
            <TabsContent value="market" className="space-y-6">
              <div className="grid gap-6">
                <MarketData data={marketData} detailed={true} />
                
                {aiSignals && (
                  <Card>
                    <CardHeader>
                      <CardTitle>🤖 AI Trading Signals</CardTitle>
                    </CardHeader>
                    <CardContent>
                      <AISignals signals={aiSignals} />
                    </CardContent>
                  </Card>
                )}
              </div>
            </TabsContent>

            {/* Analytics */}
            <TabsContent value="analytics" className="space-y-6">
              <div className="grid gap-6">
                <Card>
                  <CardHeader>
                    <CardTitle>Portfolio Analytics</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="h-64 flex items-center justify-center text-muted-foreground">
                      Advanced analytics charts coming soon...
                    </div>
                  </CardContent>
                </Card>
              </div>
            </TabsContent>

            {/* Settings */}
            <TabsContent value="settings" className="space-y-6">
              <Card>
                <CardHeader>
                  <CardTitle>Settings</CardTitle>
                </CardHeader>
                <CardContent>
                  <div className="space-y-4">
                    <div>
                      <h3 className="text-lg font-medium">Trading Preferences</h3>
                      <p className="text-sm text-muted-foreground">
                        Configure your default trading settings
                      </p>
                    </div>
                    
                    <div>
                      <h3 className="text-lg font-medium">Notifications</h3>
                      <p className="text-sm text-muted-foreground">
                        Manage your alert preferences
                      </p>
                    </div>
                    
                    <div>
                      <h3 className="text-lg font-medium">Security</h3>
                      <p className="text-sm text-muted-foreground">
                        API keys and wallet management
                      </p>
                    </div>
                  </div>
                </CardContent>
              </Card>
            </TabsContent>
          </Tabs>
        </main>
      </div>
    </div>
  );
}