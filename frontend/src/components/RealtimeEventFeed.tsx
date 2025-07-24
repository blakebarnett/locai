import React, { useRef, useEffect } from 'react';
import { useGraphStore } from '../stores/graphStore';
import { useTheme } from '../contexts/ThemeContext';

interface RealtimeEvent {
  id: string;
  type: string;
  timestamp: Date;
  description: string;
  data: any;
}

const EventItem: React.FC<{ event: RealtimeEvent; colors: any }> = ({ event, colors }) => {
  const getEventIcon = (type: string) => {
    switch (type) {
      case 'MemoryCreated': return '📝';
      case 'MemoryUpdated': return '✏️';
      case 'MemoryDeleted': return '🗑️';
      case 'EntityCreated': return '🏷️';
      case 'EntityUpdated': return '🔄';
      case 'EntityDeleted': return '❌';
      case 'RelationshipCreated': return '🔗';
      case 'RelationshipDeleted': return '🔓';
      default: return '📋';
    }
  };

  const getEventColor = (type: string) => {
    switch (type) {
      case 'MemoryCreated':
      case 'EntityCreated': 
      case 'RelationshipCreated': return '#10b981';
      case 'MemoryUpdated':
      case 'EntityUpdated': return '#3b82f6';
      case 'MemoryDeleted':
      case 'EntityDeleted':
      case 'RelationshipDeleted': return '#ef4444';
      default: return colors.text.muted;
    }
  };

  return (
    <div 
      className="event-item"
      style={{
        display: 'flex',
        alignItems: 'flex-start',
        padding: '8px',
        borderBottom: `1px solid ${colors.border.primary}`,
        fontSize: '12px',
        animation: 'fadeInUp 0.3s ease-out'
      }}
    >
      <span 
        className="event-icon"
        style={{
          fontSize: '14px',
          marginRight: '8px',
          marginTop: '2px'
        }}
      >
        {getEventIcon(event.type)}
      </span>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <span 
            className="event-time"
            style={{
              color: colors.text.muted,
              fontSize: '10px',
              fontWeight: '500'
            }}
          >
            {event.timestamp.toLocaleTimeString()}
          </span>
          <span 
            className="event-type"
            style={{
              backgroundColor: getEventColor(event.type),
              color: 'white',
              padding: '2px 6px',
              borderRadius: '10px',
              fontSize: '9px',
              fontWeight: '600',
              textTransform: 'uppercase'
            }}
          >
            {event.type.replace(/([A-Z])/g, ' $1').trim()}
          </span>
        </div>
        <div 
          className="event-description"
          style={{
            color: colors.text.primary,
            marginTop: '4px',
            lineHeight: '1.4',
            wordBreak: 'break-word'
          }}
        >
          {event.description}
        </div>
      </div>
    </div>
  );
};

export const RealtimeEventFeed: React.FC = () => {
  const { realtimeFeed, clearRealtimeFeed } = useGraphStore();
  const { colors } = useTheme();
  const feedRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom when new events arrive
  useEffect(() => {
    if (feedRef.current) {
      feedRef.current.scrollTop = feedRef.current.scrollHeight;
    }
  }, [realtimeFeed]);

  return (
    <div 
      className="realtime-event-feed"
      style={{
        backgroundColor: colors.bg.primary,
        border: `1px solid ${colors.border.primary}`,
        borderRadius: '8px',
        margin: '8px 0',
        height: '300px',
        display: 'flex',
        flexDirection: 'column'
      }}
    >
      <div 
        className="feed-header"
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          padding: '12px 16px',
          borderBottom: `1px solid ${colors.border.primary}`,
          backgroundColor: colors.bg.secondary
        }}
      >
        <h3 
          style={{
            margin: 0,
            color: colors.text.primary,
            fontSize: '14px',
            fontWeight: '600',
            display: 'flex',
            alignItems: 'center'
          }}
        >
          <span style={{ marginRight: '8px' }}>🔴</span>
          Live Updates
          <span 
            style={{
              marginLeft: '8px',
              backgroundColor: colors.border.accent,
              color: 'white',
              padding: '2px 6px',
              borderRadius: '10px',
              fontSize: '10px',
              fontWeight: '500'
            }}
          >
            {realtimeFeed.length}
          </span>
        </h3>
        <button 
          onClick={clearRealtimeFeed}
          style={{
            backgroundColor: 'transparent',
            color: colors.text.muted,
            border: `1px solid ${colors.border.primary}`,
            borderRadius: '4px',
            padding: '4px 8px',
            cursor: 'pointer',
            fontSize: '11px',
            fontWeight: '500'
          }}
        >
          Clear
        </button>
      </div>
      
      <div 
        className="feed-content"
        ref={feedRef}
        style={{
          flex: 1,
          overflowY: 'auto',
          padding: 0
        }}
      >
        {realtimeFeed.length === 0 ? (
          <div 
            style={{
              padding: '24px',
              textAlign: 'center',
              color: colors.text.muted,
              fontSize: '12px'
            }}
          >
            <div style={{ fontSize: '24px', marginBottom: '8px' }}>📡</div>
            <div>Live updates will appear here</div>
            <div style={{ marginTop: '4px', fontSize: '11px' }}>
              Connect to locai-server to see real-time changes
            </div>
          </div>
        ) : (
          realtimeFeed.map((event) => (
            <EventItem key={event.id} event={event} colors={colors} />
          ))
        )}
      </div>
    </div>
  );
};

export default RealtimeEventFeed; 