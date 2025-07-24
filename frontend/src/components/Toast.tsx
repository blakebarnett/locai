import React, { useEffect, useState } from 'react';

export interface ToastProps {
  id: string;
  type: 'info' | 'success' | 'warning' | 'error';
  title: string;
  message?: string;
  duration?: number;
  onClose: (id: string) => void;
}

const Toast: React.FC<ToastProps> = ({ 
  id, 
  type, 
  title, 
  message, 
  duration = 5000, 
  onClose 
}) => {
  const [isVisible, setIsVisible] = useState(false);

  useEffect(() => {
    // Animate in
    setTimeout(() => setIsVisible(true), 10);

    // Auto-close after duration
    const timer = setTimeout(() => {
      setIsVisible(false);
      setTimeout(() => onClose(id), 300); // Wait for animation
    }, duration);

    return () => clearTimeout(timer);
  }, [id, duration, onClose]);

  const getTypeStyles = () => {
    switch (type) {
      case 'success':
        return 'border-green-500 bg-green-900/90 text-green-100';
      case 'warning':
        return 'border-yellow-500 bg-yellow-900/90 text-yellow-100';
      case 'error':
        return 'border-red-500 bg-red-900/90 text-red-100';
      default:
        return 'border-blue-500 bg-blue-900/90 text-blue-100';
    }
  };

  const getIcon = () => {
    switch (type) {
      case 'success':
        return '✓';
      case 'warning':
        return '⚠';
      case 'error':
        return '✕';
      default:
        return 'ℹ';
    }
  };

  return (
    <div
      className={`toast ${getTypeStyles()} ${
        isVisible ? 'toast-visible' : 'toast-hidden'
      }`}
    >
      <div className="flex items-start gap-3">
        <div className="toast-icon">
          {getIcon()}
        </div>
        <div className="flex-1">
          <div className="toast-title">{title}</div>
          {message && <div className="toast-message">{message}</div>}
        </div>
        <button
          onClick={() => {
            setIsVisible(false);
            setTimeout(() => onClose(id), 300);
          }}
          className="toast-close"
        >
          ×
        </button>
      </div>
    </div>
  );
};

export default Toast; 