import React, { useState } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ThemeProvider, useTheme } from './contexts/ThemeContext';
import D3GraphVisualization from './components/D3GraphVisualization';
import ThemeToggle from './components/ThemeToggle';

// Create a client for React Query
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 5 * 60 * 1000, // 5 minutes
      gcTime: 10 * 60 * 1000, // 10 minutes (renamed from cacheTime in v5)
    },
  },
});

const AppContent: React.FC = () => {
  const { colors } = useTheme();

  return (
    <div 
      className="w-screen h-screen"
      style={{ backgroundColor: colors.bg.secondary }}
    >
      <header 
        className="h-20 flex items-center justify-between px-6 border-b"
        style={{ 
          backgroundColor: colors.bg.primary, 
          borderColor: colors.border.primary 
        }}
      >
        <div className="flex items-center space-x-4">
          <div className="flex items-center space-x-2">
            <div 
              className="w-8 h-8 rounded-lg flex items-center justify-center"
              style={{ backgroundColor: colors.border.accent }}
            >
              <span className="text-white font-bold text-lg">A</span>
            </div>
            <h1 
              className="text-2xl font-bold"
              style={{ color: colors.text.primary }}
            >
              Locai Memory Explorer
            </h1>
          </div>
        </div>
        
        <div className="flex items-center space-x-4">
          <div 
            className="text-sm"
            style={{ color: colors.text.muted }}
          >
            Real-time Memory Visualization
          </div>
          <ThemeToggle />
        </div>
      </header>
      
      <main className="h-[calc(100vh-80px)]">
        <D3GraphVisualization />
      </main>
    </div>
  );
};

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <AppContent />
      </ThemeProvider>
    </QueryClientProvider>
  );
}

export default App;
