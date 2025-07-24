import React, { useState } from 'react';
import { Handle, Position } from 'reactflow';
import type { NodeProps } from 'reactflow';
import type { Entity } from '../../types/api';
import { useTooltipPosition } from '../../hooks/useTooltipPosition';

interface EntityNodeData {
  id: string;
  type: 'entity';
  content: Entity;
  centrality?: number;
  degree: number;
}

const EntityNode: React.FC<NodeProps<EntityNodeData>> = ({ data, selected }) => {
  const entity = data.content;
  const degree = data.degree || 0;
  const [isHovered, setIsHovered] = useState(false);
  const { position, nodeRef, tooltipRef } = useTooltipPosition(isHovered);

  // Calculate node size based on degree (connectivity)
  const getNodeSize = (degree: number) => {
    const minSize = 60;  // Minimum size for unconnected nodes
    const maxSize = 140; // Maximum size for highly connected nodes
    const scaleFactor = Math.min(degree / 10, 1); // Scale based on connections, max at 10 connections
    return minSize + (maxSize - minSize) * scaleFactor;
  };

  const getEntityIcon = (type: string) => {
    switch (type.toLowerCase()) {
      case 'person':
        return '👤';
      case 'place':
      case 'location':
        return '📍';
      case 'organization':
        return '🏢';
      case 'event':
        return '📅';
      case 'concept':
        return '💡';
      case 'technology':
        return '⚙️';
      case 'object':
      case 'item':
        return '📦';
      case 'document':
        return '📄';
      case 'project':
        return '📊';
      case 'task':
        return '✅';
      default:
        return '🔗';
    }
  };

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric'
    });
  };

  const nodeSize = getNodeSize(degree);
  const displayName = entity.name || `${entity.entity_type} entity`;
  const borderColor = '#fbbf24';
  const textColor = '#fef3c7';

  return (
    <div 
      ref={nodeRef}
      className={`entity-node ${selected ? 'selected' : ''}`}
      style={{
        width: `${nodeSize}px`,
        height: `${Math.max(nodeSize * 0.4, 40)}px`,
        fontSize: `${Math.max(nodeSize * 0.08, 9)}px`,
        border: `2px solid ${borderColor}`,
        background: 'transparent',
        color: textColor,
        borderRadius: '8px',
        backdropFilter: 'blur(8px)',
        boxShadow: selected 
          ? `0 0 0 3px rgba(96, 165, 250, 0.3), 0 2px 8px rgba(0, 0, 0, 0.6)` 
          : '0 2px 8px rgba(0, 0, 0, 0.6)',
      }}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      <Handle
        type="target"
        position={Position.Top}
        style={{ 
          width: 6, 
          height: 6, 
          background: '#94a3b8',
          border: 'none',
          borderRadius: '50%'
        }}
      />
      
      <div className="entity-node-header">
        <span className="entity-node-icon">
          {getEntityIcon(entity.entity_type)}
        </span>
        <span className="entity-node-type" style={{ 
          textShadow: '0 1px 2px rgba(0, 0, 0, 0.8)',
          color: textColor,
          opacity: 0.9
        }}>{entity.entity_type}</span>
      </div>

      <div className="entity-node-name" style={{ 
        textShadow: '0 1px 2px rgba(0, 0, 0, 0.8)',
        color: textColor
      }}>
        {displayName.length > (nodeSize / 4) 
          ? `${displayName.substring(0, Math.floor(nodeSize / 4))}...` 
          : displayName
        }
      </div>

      {degree > 0 && (
        <div className="entity-node-degree" style={{ 
          textShadow: '0 1px 2px rgba(0, 0, 0, 0.8)',
          color: textColor,
          opacity: 0.8
        }}>
          {degree} connections
        </div>
      )}

      {/* Hover Tooltip */}
      <div 
        ref={tooltipRef}
        className={`node-tooltip ${isHovered ? 'tooltip-visible' : 'tooltip-hidden'}`}
        style={{
          top: position.top,
          bottom: position.bottom,
          left: position.left,
          right: position.right,
          transform: position.transform,
          marginBottom: position.position === 'top' ? '8px' : undefined,
          marginTop: position.position === 'bottom' ? '8px' : undefined,
          marginRight: position.position === 'left' ? '8px' : undefined,
          marginLeft: position.position === 'right' ? '8px' : undefined,
        }}
      >
        <div className="tooltip-header">
          {entity.entity_type}: {displayName}
        </div>
        <div className="tooltip-content">
          {Object.keys(entity.properties).length > 0 ? (
            <div>
              {Object.entries(entity.properties).map(([key, value]) => (
                <div key={key}>
                  <strong>{key}:</strong> {String(value)}
                </div>
              ))}
            </div>
          ) : (
            <div>No additional properties</div>
          )}
        </div>
        <div className="tooltip-meta">
          <div>Type: {entity.entity_type}</div>
          <div>Connections: {degree}</div>
          <div>Created: {formatDate(entity.created_at)}</div>
          {entity.updated_at !== entity.created_at && (
            <div>Updated: {formatDate(entity.updated_at)}</div>
          )}
          {data.centrality && (
            <div>Centrality: {data.centrality.toFixed(3)}</div>
          )}
        </div>
      </div>

      <Handle
        type="source"
        position={Position.Bottom}
        style={{ 
          width: 6, 
          height: 6, 
          background: '#94a3b8',
          border: 'none',
          borderRadius: '50%'
        }}
      />
    </div>
  );
};

export default EntityNode; 