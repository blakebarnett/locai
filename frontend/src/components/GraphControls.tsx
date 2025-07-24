import React, { useState } from 'react';
import { useGraphStore } from '../stores/graphStore';
import { useTheme } from '../contexts/ThemeContext';
import { clearLayoutCache, hasLayoutCache } from '../utils/layoutCache';

const GraphControls: React.FC = () => {
  const {
    searchQuery,
    activeFilters,
    layoutType,
    showLabels,
    nodeSize,
    showTemporalRelationships,
    dataSource,
    setSearchQuery,
    setFilters,
    setLayoutType,
    setShowLabels,
    setNodeSize,
    setShowTemporalRelationships,
    expandNode,
    selectedNodes,
    applyLayout,
    isLoading,
    loadDemoData,
    loadServerData,
  } = useGraphStore();
  const { colors } = useTheme();

  const [isExpanded, setIsExpanded] = useState(false);

  const handleExpandSelected = () => {
    selectedNodes.forEach(nodeId => {
      expandNode(nodeId, 1);
    });
  };

  const handleApplyLayout = () => {
    applyLayout(800, 600);
  };

  const handleClearCache = () => {
    clearLayoutCache();
    // Trigger a re-layout to show the effect
    applyLayout(800, 600);
  };

  const hasCachedLayout = hasLayoutCache(layoutType);

  return (
    <div 
      className="graph-controls"
      style={{
        backgroundColor: colors.panel.bg,
        border: `1px solid ${colors.panel.border}`,
        boxShadow: colors.panel.shadow,
      }}
    >
      <div className="flex items-center justify-between mb-4">
        <h3 style={{ color: colors.text.primary }}>Graph Controls</h3>
        <button
          onClick={() => setIsExpanded(!isExpanded)}
          className="hover:opacity-80 transition-opacity"
          style={{ color: colors.text.muted }}
        >
          {isExpanded ? '−' : '+'}
        </button>
      </div>

      {/* Data Source Selection */}
      <div className="mb-4">
        <label 
          className="block text-sm font-medium mb-2"
          style={{ color: colors.text.primary }}
        >
          Data Source
        </label>
        <div className="flex gap-2">
          <button
            onClick={loadDemoData}
            disabled={isLoading}
            className="flex-1 py-2 px-3 text-sm rounded transition-colors"
            style={{
              backgroundColor: dataSource === 'demo' ? colors.border.accent : colors.bg.secondary,
              border: `1px solid ${dataSource === 'demo' ? colors.border.accent : colors.border.primary}`,
              color: dataSource === 'demo' ? colors.text.inverse : colors.text.primary,
            }}
          >
            {dataSource === 'demo' && '✓ '}Demo Data
          </button>
          <button
            onClick={loadServerData}
            disabled={isLoading}
            className="flex-1 py-2 px-3 text-sm rounded transition-colors"
            style={{
              backgroundColor: dataSource === 'server' ? colors.border.accent : colors.bg.secondary,
              border: `1px solid ${dataSource === 'server' ? colors.border.accent : colors.border.primary}`,
              color: dataSource === 'server' ? colors.text.inverse : colors.text.primary,
            }}
          >
            {dataSource === 'server' && '✓ '}Server Data
          </button>
        </div>
      </div>

      {/* Search */}
      <div className="mb-4">
        <label 
          className="block text-sm font-medium mb-2"
          style={{ color: colors.text.primary }}
        >
          Search
        </label>
        <input
          type="text"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          placeholder="Search memories and entities..."
          className="graph-controls-input"
          style={{
            backgroundColor: colors.bg.secondary,
            border: `1px solid ${colors.border.primary}`,
            color: colors.text.primary,
          }}
        />
      </div>

      {/* Compact Keyboard Shortcuts - Always Visible */}
      {!isExpanded && (
        <div className="compact-shortcuts mb-4">
          <div className="flex items-center justify-between text-xs mb-2">
            <span style={{ color: colors.text.muted }}>Quick Actions:</span>
          </div>
          <div className="flex flex-wrap gap-2">
            <div className="flex items-center gap-1">
              <kbd 
                className="kbd-shortcut-compact"
                style={{ 
                  backgroundColor: colors.bg.secondary,
                  border: `1px solid ${colors.border.primary}`,
                  color: colors.text.primary
                }}
              >
                Ctrl+R
              </kbd>
              <span style={{ color: colors.text.muted, fontSize: '10px' }}>Reset</span>
            </div>
            <div className="flex items-center gap-1">
              <kbd 
                className="kbd-shortcut-compact"
                style={{ 
                  backgroundColor: colors.bg.secondary,
                  border: `1px solid ${colors.border.primary}`,
                  color: colors.text.primary
                }}
              >
                Esc
              </kbd>
              <span style={{ color: colors.text.muted, fontSize: '10px' }}>Clear</span>
            </div>
          </div>
          <div className="flex flex-wrap gap-2 mt-1">
            <div className="flex items-center gap-1">
              <kbd 
                className="kbd-shortcut-compact"
                style={{ 
                  backgroundColor: colors.bg.secondary,
                  border: `1px solid ${colors.border.primary}`,
                  color: colors.text.primary
                }}
              >
                Drag
              </kbd>
              <span style={{ color: colors.text.muted, fontSize: '10px' }}>Move Nodes</span>
            </div>
                         <div className="flex items-center gap-1">
               <kbd 
                 className="kbd-shortcut-compact"
                 style={{ 
                   backgroundColor: colors.bg.secondary,
                   border: `1px solid ${colors.border.primary}`,
                   color: colors.text.primary
                 }}
               >
                 F
               </kbd>
               <span style={{ color: colors.text.muted, fontSize: '10px' }}>Fit View</span>
             </div>
          </div>
        </div>
      )}

      {isExpanded && (
        <>
          {/* Layout Controls */}
          <div className="mb-4">
            <label 
              className="block text-sm font-medium mb-2"
              style={{ color: colors.text.primary }}
            >
              Layout
            </label>
            <select
              value={layoutType}
              onChange={(e) => setLayoutType(e.target.value as any)}
              className="graph-controls-select"
              style={{
                backgroundColor: colors.bg.secondary,
                border: `1px solid ${colors.border.primary}`,
                color: colors.text.primary,
              }}
            >
              <option value="force">Force Directed</option>
              <option value="hierarchical">Hierarchical</option>
              <option value="circular">Circular</option>
            </select>
            <button
              onClick={handleApplyLayout}
              disabled={isLoading}
              className="w-full mt-2 graph-controls-button"
              style={{
                backgroundColor: isLoading ? colors.text.muted : colors.border.accent,
                color: colors.text.inverse,
              }}
            >
              {isLoading ? 'Applying...' : 'Apply Layout'}
            </button>
            {hasCachedLayout && (
              <button
                onClick={handleClearCache}
                disabled={isLoading}
                className="w-full mt-1 text-xs opacity-75 graph-controls-button"
                style={{
                  backgroundColor: colors.text.muted,
                  color: colors.text.inverse,
                }}
              >
                Clear Cached Layout
              </button>
            )}
          </div>

          {/* Node Size */}
          <div className="mb-4">
            <label 
              className="block text-sm font-medium mb-2"
              style={{ color: colors.text.primary }}
            >
              Node Size
            </label>
            <select
              value={nodeSize}
              onChange={(e) => setNodeSize(e.target.value as any)}
              className="graph-controls-select"
              style={{
                backgroundColor: colors.bg.secondary,
                border: `1px solid ${colors.border.primary}`,
                color: colors.text.primary,
              }}
            >
              <option value="uniform">Uniform</option>
              <option value="centrality">By Centrality</option>
              <option value="degree">By Degree</option>
            </select>
          </div>

          {/* Display Options */}
          <div className="mb-4">
            <label className="flex items-center">
              <input
                type="checkbox"
                checked={showLabels}
                onChange={(e) => setShowLabels(e.target.checked)}
                className="mr-2"
              />
              <span 
                className="text-sm"
                style={{ color: colors.text.primary }}
              >
                Show Edge Labels
              </span>
            </label>
          </div>

          {/* Temporal Relationships Toggle */}
          <div className="mb-4">
            <label className="flex items-center">
              <input
                type="checkbox"
                checked={showTemporalRelationships}
                onChange={(e) => setShowTemporalRelationships(e.target.checked)}
                className="mr-2"
              />
              <span 
                className="text-sm"
                style={{ color: colors.text.primary }}
              >
                Show Temporal Relationships
              </span>
            </label>
            <p 
              className="text-xs mt-1 ml-6"
              style={{ color: colors.text.muted }}
            >
              Mentions, follows, precedes, sequence relationships
            </p>
          </div>

          {/* Memory Type Filters */}
          <div className="mb-4">
            <label 
              className="block text-sm font-medium mb-2"
              style={{ color: colors.text.primary }}
            >
              Memory Types
            </label>
            <div className="space-y-1">
              {['Fact', 'Episodic', 'Semantic'].map(type => (
                <label key={type} className="flex items-center">
                  <input
                    type="checkbox"
                    checked={activeFilters.memoryTypes.includes(type)}
                    onChange={(e) => {
                      const newTypes = e.target.checked
                        ? [...activeFilters.memoryTypes, type]
                        : activeFilters.memoryTypes.filter(t => t !== type);
                      setFilters({ memoryTypes: newTypes });
                    }}
                    className="mr-2"
                  />
                  <span 
                    className="text-sm"
                    style={{ color: colors.text.primary }}
                  >
                    {type}
                  </span>
                </label>
              ))}
            </div>
          </div>

          {/* Actions */}
          <div className="space-y-2">
            <button
              onClick={handleExpandSelected}
              disabled={selectedNodes.length === 0}
              className="graph-controls-button"
              style={{
                backgroundColor: selectedNodes.length === 0 ? colors.text.muted : colors.border.accent,
                color: colors.text.inverse,
              }}
            >
              Expand Selected ({selectedNodes.length})
            </button>
          </div>

          {/* Keyboard Shortcuts Legend */}
          <div className="mt-6 pt-4" style={{ borderTop: `1px solid ${colors.border.primary}` }}>
            <label 
              className="block text-sm font-medium mb-2"
              style={{ color: colors.text.primary }}
            >
              Keyboard Shortcuts
            </label>
            <div className="space-y-1">
              <div className="flex items-center justify-between text-xs">
                <span style={{ color: colors.text.muted }}>Reset Layout</span>
                <kbd 
                  className="kbd-shortcut"
                  style={{ 
                    backgroundColor: colors.bg.secondary,
                    border: `1px solid ${colors.border.primary}`,
                    color: colors.text.primary
                  }}
                >
                  Ctrl+R
                </kbd>
              </div>
              <div className="flex items-center justify-between text-xs">
                <span style={{ color: colors.text.muted }}>Clear Selection</span>
                <kbd 
                  className="kbd-shortcut"
                  style={{ 
                    backgroundColor: colors.bg.secondary,
                    border: `1px solid ${colors.border.primary}`,
                    color: colors.text.primary
                  }}
                >
                  Esc
                </kbd>
              </div>
              <div className="flex items-center justify-between text-xs">
                <span style={{ color: colors.text.muted }}>Fit View</span>
                <kbd 
                  className="kbd-shortcut"
                  style={{ 
                    backgroundColor: colors.bg.secondary,
                    border: `1px solid ${colors.border.primary}`,
                    color: colors.text.primary
                  }}
                >
                  F
                </kbd>
              </div>
              <div className="flex items-center justify-between text-xs">
                <span style={{ color: colors.text.muted }}>Drag Nodes</span>
                <span style={{ color: colors.text.muted, fontSize: '10px' }}>Mouse</span>
              </div>
            </div>
          </div>
        </>
      )}
      
      <style>{`
        .kbd-shortcut {
          padding: 2px 6px;
          border-radius: 4px;
          font-size: 10px;
          font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
          font-weight: 500;
          min-width: 24px;
          text-align: center;
          display: inline-block;
        }

        .kbd-shortcut-compact {
          padding: 1px 4px;
          border-radius: 3px;
          font-size: 9px;
          font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
          font-weight: 500;
          min-width: 18px;
          text-align: center;
          display: inline-block;
        }

        .compact-shortcuts {
          padding: 8px;
          border-radius: 6px;
          background: ${colors.bg.secondary}20;
          border: 1px solid ${colors.border.primary}50;
        }

        .graph-controls-input {
          width: 100%;
          padding: 8px 12px;
          border-radius: 6px;
          font-size: 14px;
        }

        .graph-controls-input:focus {
          outline: none;
          ring: 2px;
          ring-color: ${colors.border.accent};
        }

        .graph-controls-select {
          width: 100%;
          padding: 8px 12px;
          border-radius: 6px;
          font-size: 14px;
        }

        .graph-controls-select:focus {
          outline: none;
          ring: 2px;
          ring-color: ${colors.border.accent};
        }

        .graph-controls-button {
          width: 100%;
          padding: 8px 16px;
          border: none;
          border-radius: 6px;
          font-size: 14px;
          font-weight: 500;
          cursor: pointer;
          transition: opacity 0.2s ease;
        }

        .graph-controls-button:hover {
          opacity: 0.9;
        }

        .graph-controls-button:disabled {
          cursor: not-allowed;
          opacity: 0.6;
        }
      `}</style>
    </div>
  );
};

export default GraphControls; 