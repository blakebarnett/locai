import React, { useState } from 'react';
import { Handle, Position } from 'reactflow';
import type { NodeProps } from 'reactflow';
import type { Memory } from '../../types/api';
import { useTooltipPosition } from '../../hooks/useTooltipPosition';

interface MemoryNodeData {
  id: string;
  type: 'memory';
  content: Memory;
  centrality?: number;
  degree: number;
}

const MemoryNode: React.FC<NodeProps<MemoryNodeData>> = ({ data, selected }) => {
  const memory = data.content;
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
  
  const getMemoryTypeClass = (type: string) => {
    switch (type) {
      case 'Fact':
        return 'memory-node-fact';
      case 'Episodic':
        return 'memory-node-episodic';
      case 'Semantic':
        return 'memory-node-semantic';
      default:
        return 'memory-node-fact';
    }
  };

  const getMemoryTypeBorderColor = (type: string) => {
    switch (type) {
      case 'Fact':
        return '#60a5fa';
      case 'Episodic':
        return '#4ade80';
      case 'Semantic':
        return '#c084fc';
      default:
        return '#60a5fa';
    }
  };

  const getMemoryTypeTextColor = (type: string) => {
    switch (type) {
      case 'Fact':
        return '#dbeafe';
      case 'Episodic':
        return '#dcfce7';
      case 'Semantic':
        return '#f3e8ff';
      default:
        return '#dbeafe';
    }
  };

  const getPriorityIndicator = (priority: string) => {
    switch (priority) {
      case 'High':
        return '●';
      case 'Medium':
        return '◐';
      case 'Low':
        return '○';
      default:
        return '○';
    }
  };

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric'
    });
  };

  const nodeSize = getNodeSize(degree);

  return (
    <div 
      ref={nodeRef}
      className={`memory-node ${getMemoryTypeClass(memory.memory_type)} ${selected ? 'selected' : ''}`}
      style={{
        width: `${nodeSize}px`,
        height: `${Math.max(nodeSize * 0.4, 40)}px`,
        fontSize: `${Math.max(nodeSize * 0.08, 9)}px`,
        border: `2px solid ${getMemoryTypeBorderColor(memory.memory_type)}`,
        background: 'transparent',
        color: getMemoryTypeTextColor(memory.memory_type),
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
      
      <div className="memory-node-header">
        <span className="memory-node-type" style={{ color: getMemoryTypeTextColor(memory.memory_type) }}>{memory.memory_type}</span>
        <span className="memory-node-priority">
          {getPriorityIndicator(memory.priority)}
        </span>
      </div>

      <div className="memory-node-content">
        {memory.content.length > (nodeSize / 3) 
          ? `${memory.content.substring(0, Math.floor(nodeSize / 3))}...` 
          : memory.content
        }
      </div>

      {degree > 0 && (
        <div className="memory-node-degree" style={{ 
          textShadow: '0 1px 2px rgba(0, 0, 0, 0.8)',
          color: getMemoryTypeTextColor(memory.memory_type),
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
          {memory.memory_type} Memory
        </div>
        <div className="tooltip-content">
          {memory.content}
        </div>
        <div className="tooltip-meta">
          <div>Priority: {memory.priority}</div>
          <div>Connections: {degree}</div>
          <div>Created: {formatDate(memory.created_at)}</div>
          {memory.updated_at !== memory.created_at && (
            <div>Updated: {formatDate(memory.updated_at)}</div>
          )}
          {data.centrality && (
            <div>Centrality: {data.centrality.toFixed(3)}</div>
          )}
          {memory.metadata && Object.keys(memory.metadata).length > 0 && (
            <div>
              Metadata: {Object.entries(memory.metadata)
                .slice(0, 2)
                .map(([k, v]) => `${k}: ${v}`)
                .join(', ')}
            </div>
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

export default MemoryNode; 