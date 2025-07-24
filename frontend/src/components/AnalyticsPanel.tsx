import React, { useState, useEffect } from 'react';
import { useGraphStore } from '../stores/graphStore';
import { useTheme } from '../contexts/ThemeContext';

const AnalyticsPanel: React.FC = () => {
  const { metrics, nodes, edges, loadMetrics } = useGraphStore();
  const { colors } = useTheme();
  const [isExpanded, setIsExpanded] = useState(false);

  useEffect(() => {
    // Refresh metrics every 30 seconds
    const interval = setInterval(loadMetrics, 30000);
    return () => clearInterval(interval);
  }, [loadMetrics]);

  const currentStats = {
    nodeCount: nodes.length,
    edgeCount: edges.length,
    memoryCount: nodes.filter(n => n.data.type === 'memory').length,
    entityCount: nodes.filter(n => n.data.type === 'entity').length,
  };

  return (
    <div 
      className="analytics-panel"
      style={{
        backgroundColor: colors.panel.bg,
        border: `1px solid ${colors.panel.border}`,
        boxShadow: colors.panel.shadow,
      }}
    >
      <div className="flex items-center justify-between mb-4">
        <h3 
          className="text-lg font-semibold"
          style={{ color: colors.text.primary }}
        >
          Analytics
        </h3>
        <button
          onClick={() => setIsExpanded(!isExpanded)}
          className="hover:opacity-80 transition-opacity"
          style={{ color: colors.text.muted }}
        >
          {isExpanded ? '−' : '+'}
        </button>
      </div>

      {/* Current View Stats */}
      <div className="stats-grid">
        <div 
          className="stat-card"
          style={{
            backgroundColor: colors.bg.secondary,
            border: `1px solid ${colors.border.primary}`,
          }}
        >
          <div 
            className="stat-number"
            style={{ color: colors.node.memory.fact }}
          >
            {currentStats.nodeCount}
          </div>
          <div 
            className="stat-label"
            style={{ color: colors.text.muted }}
          >
            Nodes
          </div>
        </div>
        <div 
          className="stat-card"
          style={{
            backgroundColor: colors.bg.secondary,
            border: `1px solid ${colors.border.primary}`,
          }}
        >
          <div 
            className="stat-number"
            style={{ color: colors.node.memory.episodic }}
          >
            {currentStats.edgeCount}
          </div>
          <div 
            className="stat-label"
            style={{ color: colors.text.muted }}
          >
            Edges
          </div>
        </div>
        <div 
          className="stat-card"
          style={{
            backgroundColor: colors.bg.secondary,
            border: `1px solid ${colors.border.primary}`,
          }}
        >
          <div 
            className="stat-number"
            style={{ color: colors.node.memory.semantic }}
          >
            {currentStats.memoryCount}
          </div>
          <div 
            className="stat-label"
            style={{ color: colors.text.muted }}
          >
            Memories
          </div>
        </div>
        <div 
          className="stat-card"
          style={{
            backgroundColor: colors.bg.secondary,
            border: `1px solid ${colors.border.primary}`,
          }}
        >
          <div 
            className="stat-number"
            style={{ color: colors.node.entity }}
          >
            {currentStats.entityCount}
          </div>
          <div 
            className="stat-label"
            style={{ color: colors.text.muted }}
          >
            Entities
          </div>
        </div>
      </div>

      {isExpanded && metrics && (
        <>
          {/* Global Graph Metrics */}
          <div className="mb-4">
            <h4 
              className="text-md font-semibold mb-2"
              style={{ color: colors.text.primary }}
            >
              Global Metrics
            </h4>
            <div className="space-y-2 text-sm">
              <div className="flex justify-between">
                <span style={{ color: colors.text.secondary }}>Total Memories:</span>
                <span 
                  className="font-medium"
                  style={{ color: colors.text.primary }}
                >
                  {metrics.memory_count}
                </span>
              </div>
              <div className="flex justify-between">
                <span style={{ color: colors.text.secondary }}>Total Entities:</span>
                <span 
                  className="font-medium"
                  style={{ color: colors.text.primary }}
                >
                  {metrics.entity_count}
                </span>
              </div>
              <div className="flex justify-between">
                <span style={{ color: colors.text.secondary }}>Total Relationships:</span>
                <span 
                  className="font-medium"
                  style={{ color: colors.text.primary }}
                >
                  {metrics.relationship_count}
                </span>
              </div>
              <div className="flex justify-between">
                <span style={{ color: colors.text.secondary }}>Average Degree:</span>
                <span 
                  className="font-medium"
                  style={{ color: colors.text.primary }}
                >
                  {metrics.average_degree.toFixed(2)}
                </span>
              </div>
              <div className="flex justify-between">
                <span style={{ color: colors.text.secondary }}>Graph Density:</span>
                <span 
                  className="font-medium"
                  style={{ color: colors.text.primary }}
                >
                  {(metrics.density * 100).toFixed(2)}%
                </span>
              </div>
              <div className="flex justify-between">
                <span style={{ color: colors.text.secondary }}>Components:</span>
                <span 
                  className="font-medium"
                  style={{ color: colors.text.primary }}
                >
                  {metrics.connected_components}
                </span>
              </div>
            </div>
          </div>

          {/* Central Memories */}
          {metrics.central_memories.length > 0 && (
            <div className="mb-4">
              <h4 
                className="text-md font-semibold mb-2"
                style={{ color: colors.text.primary }}
              >
                Most Central Memories
              </h4>
              <div className="space-y-2">
                {metrics.central_memories.slice(0, 3).map((memory) => (
                  <div
                    key={memory.id}
                    className="p-2 rounded text-xs"
                    style={{
                      backgroundColor: colors.bg.tertiary,
                      border: `1px solid ${colors.border.primary}`,
                    }}
                  >
                    <div 
                      className="font-medium truncate"
                      style={{ color: colors.text.primary }}
                    >
                      {memory.content.substring(0, 50)}...
                    </div>
                    <div style={{ color: colors.text.secondary }}>
                      Score: {memory.centrality_score.toFixed(3)}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Central Entities */}
          {metrics.central_entities.length > 0 && (
            <div className="mb-4">
              <h4 
                className="text-md font-semibold mb-2"
                style={{ color: colors.text.primary }}
              >
                Most Central Entities
              </h4>
              <div className="space-y-2">
                {metrics.central_entities.slice(0, 3).map((entity) => (
                  <div
                    key={entity.id}
                    className="p-2 rounded text-xs"
                    style={{
                      backgroundColor: colors.bg.tertiary,
                      border: `1px solid ${colors.border.primary}`,
                    }}
                  >
                    <div 
                      className="font-medium"
                      style={{ color: colors.text.primary }}
                    >
                      {entity.name}
                    </div>
                    <div style={{ color: colors.text.secondary }}>
                      Score: {entity.centrality_score.toFixed(3)}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Refresh Button */}
          <button
            onClick={loadMetrics}
            className="w-full px-3 py-2 rounded-md text-sm hover:opacity-80 transition-opacity"
            style={{
              backgroundColor: colors.border.accent,
              color: colors.text.inverse,
            }}
          >
            Refresh Metrics
          </button>
        </>
      )}
    </div>
  );
};

export default AnalyticsPanel; 