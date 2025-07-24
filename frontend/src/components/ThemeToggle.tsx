import React from 'react';
import { useTheme } from '../contexts/ThemeContext';

const ThemeToggle: React.FC = () => {
  const { mode, toggleTheme, colors } = useTheme();

  return (
    <button
      onClick={toggleTheme}
      className="theme-toggle"
      style={{
        backgroundColor: colors.panel.bg,
        border: `1px solid ${colors.border.primary}`,
        color: colors.text.primary,
      }}
      title={`Switch to ${mode === 'light' ? 'dark' : 'light'} mode`}
    >
      <div className="theme-toggle-icon">
        {mode === 'light' ? '🌙' : '☀️'}
      </div>
      <div className="theme-toggle-text">
        {mode === 'light' ? 'Dark' : 'Light'}
      </div>
    </button>
  );
};

export default ThemeToggle; 