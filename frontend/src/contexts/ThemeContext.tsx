import React, { createContext, useContext, useState, useEffect } from 'react';
import type { ReactNode } from 'react';

export type ThemeMode = 'light' | 'dark';

interface ThemeColors {
  // Background colors
  bg: {
    primary: string;
    secondary: string; 
    tertiary: string;
    overlay: string;
  };
  // Text colors
  text: {
    primary: string;
    secondary: string;
    muted: string;
    inverse: string;
  };
  // Border colors
  border: {
    primary: string;
    secondary: string;
    accent: string;
  };
  // Component specific colors
  panel: {
    bg: string;
    border: string;
    shadow: string;
  };
  node: {
    memory: {
      fact: string;
      episodic: string;
      semantic: string;
    };
    entity: string;
    selected: string;
    hover: string;
  };
  edge: {
    default: string;
    selected: string;
  };
  // Status colors
  status: {
    success: string;
    warning: string;
    error: string;
    info: string;
  };
}

interface ThemeContextValue {
  mode: ThemeMode;
  colors: ThemeColors;
  toggleTheme: () => void;
  setTheme: (mode: ThemeMode) => void;
}

const lightTheme: ThemeColors = {
  bg: {
    primary: '#ffffff',
    secondary: '#f8fafc',
    tertiary: '#f1f5f9',
    overlay: 'rgba(255, 255, 255, 0.9)',
  },
  text: {
    primary: '#1e293b',
    secondary: '#475569',
    muted: '#64748b',
    inverse: '#ffffff',
  },
  border: {
    primary: '#e2e8f0',
    secondary: '#cbd5e1',
    accent: '#3b82f6',
  },
  panel: {
    bg: '#ffffff',
    border: '#e2e8f0',
    shadow: '0 8px 32px rgba(0, 0, 0, 0.1)',
  },
  node: {
    memory: {
      fact: '#3b82f6',
      episodic: '#22c55e',
      semantic: '#a855f7',
    },
    entity: '#f59e0b',
    selected: '#ef4444',
    hover: '#64748b',
  },
  edge: {
    default: '#94a3b8',
    selected: '#3b82f6',
  },
  status: {
    success: '#22c55e',
    warning: '#f59e0b',
    error: '#ef4444',
    info: '#3b82f6',
  },
};

const darkTheme: ThemeColors = {
  bg: {
    primary: '#0f172a',
    secondary: '#1e293b',
    tertiary: '#334155',
    overlay: 'rgba(30, 41, 59, 0.9)',
  },
  text: {
    primary: '#f1f5f9',
    secondary: '#e2e8f0',
    muted: '#94a3b8',
    inverse: '#1e293b',
  },
  border: {
    primary: '#334155',
    secondary: '#475569',
    accent: '#3b82f6',
  },
  panel: {
    bg: '#0f172a',
    border: '#334155',
    shadow: '0 8px 32px rgba(0, 0, 0, 0.3)',
  },
  node: {
    memory: {
      fact: '#3b82f6',
      episodic: '#22c55e',
      semantic: '#a855f7',
    },
    entity: '#f59e0b',
    selected: '#ef4444',
    hover: '#64748b',
  },
  edge: {
    default: '#475569',
    selected: '#3b82f6',
  },
  status: {
    success: '#22c55e',
    warning: '#f59e0b',
    error: '#ef4444',
    info: '#3b82f6',
  },
};

const ThemeContext = createContext<ThemeContextValue | undefined>(undefined);

interface ThemeProviderProps {
  children: ReactNode;
}

export const ThemeProvider: React.FC<ThemeProviderProps> = ({ children }) => {
  const [mode, setMode] = useState<ThemeMode>(() => {
    // Check for saved theme preference or default to 'dark'
    const saved = localStorage.getItem('locai-theme');
    return (saved as ThemeMode) || 'dark';
  });

  const colors = mode === 'light' ? lightTheme : darkTheme;

  const toggleTheme = () => {
    setMode(prev => prev === 'light' ? 'dark' : 'light');
  };

  const setTheme = (newMode: ThemeMode) => {
    setMode(newMode);
  };

  // Save theme preference and apply to document
  useEffect(() => {
    localStorage.setItem('locai-theme', mode);
    document.documentElement.setAttribute('data-theme', mode);
    
    // Apply theme colors as CSS custom properties
    const root = document.documentElement;
    root.style.setProperty('--bg-primary', colors.bg.primary);
    root.style.setProperty('--bg-secondary', colors.bg.secondary);
    root.style.setProperty('--bg-tertiary', colors.bg.tertiary);
    root.style.setProperty('--text-primary', colors.text.primary);
    root.style.setProperty('--text-secondary', colors.text.secondary);
    root.style.setProperty('--text-muted', colors.text.muted);
    root.style.setProperty('--border-primary', colors.border.primary);
    root.style.setProperty('--border-secondary', colors.border.secondary);
    root.style.setProperty('--border-accent', colors.border.accent);
    root.style.setProperty('--panel-bg', colors.panel.bg);
    root.style.setProperty('--panel-border', colors.panel.border);
    root.style.setProperty('--bg-overlay', colors.bg.overlay);
  }, [mode, colors]);

  const value: ThemeContextValue = {
    mode,
    colors,
    toggleTheme,
    setTheme,
  };

  return (
    <ThemeContext.Provider value={value}>
      {children}
    </ThemeContext.Provider>
  );
};

export const useTheme = (): ThemeContextValue => {
  const context = useContext(ThemeContext);
  if (!context) {
    throw new Error('useTheme must be used within a ThemeProvider');
  }
  return context;
}; 