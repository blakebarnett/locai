import React from 'react';
import { useGraphStore } from '../stores/graphStore';
import { useTheme } from '../contexts/ThemeContext';
import type { Memory, Entity } from '../types/api';

const NodeDetailsPanel: React.FC = () => {
  const { nodes, selectedNodes, selectNode, clearSelection } = useGraphStore();
  const { colors } = useTheme();

  // Get all selected nodes
  const selectedNodeObjects = selectedNodes.map(id => nodes.find(node => node.id === id)).filter(Boolean);

  if (selectedNodeObjects.length === 0) {
    return (
      <div className="node-details-panel">
        <div className="panel-header">
          <h3>Node Details</h3>
        </div>
        <div className="panel-content">
          <div className="empty-state">
            <div className="empty-icon">📋</div>
            <p>Select a node to view its details</p>
          </div>
        </div>
      </div>
    );
  }

  // Check if we have multiple selections
  const isMultipleSelection = selectedNodeObjects.length > 1;

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleString();
  };

  const formatDateShort = (dateString: string) => {
    return new Date(dateString).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: '2-digit'
    });
  };

  const renderProperties = (properties: Record<string, any>) => {
    return Object.entries(properties).map(([key, value]) => (
      <div key={key} className="property-item">
        <span className="property-label">{key}:</span>
        <span className="property-value">
          {typeof value === 'object' ? JSON.stringify(value) : String(value)}
        </span>
      </div>
    ));
  };

  const renderSingleNodeDetails = (node: any) => {
    const isMemoryNode = node.type === 'memoryNode';
    const content = node.data.content as Memory | Entity;

    return (
      <div className="single-node-details">
        {isMemoryNode ? (
          // Memory Node Details
          <div className="memory-details">
            <div className="content-section">
              <label>Content</label>
              <div className="content-text">{(content as Memory).content}</div>
            </div>
            
            <div className="metadata-grid">
              <div className="metadata-item">
                <label>Type</label>
                <span className="memory-type-tag" data-type={(content as Memory).memory_type}>
                  {(content as Memory).memory_type}
                </span>
              </div>
              
              <div className="metadata-item">
                <label>Priority</label>
                <span className="priority-tag" data-priority={(content as Memory).priority}>
                  {(content as Memory).priority}
                </span>
              </div>
              
              <div className="metadata-item">
                <label>Created</label>
                <span>{formatDate((content as Memory).created_at)}</span>
              </div>
              
              <div className="metadata-item">
                <label>Updated</label>
                <span>{formatDate((content as Memory).updated_at)}</span>
              </div>
            </div>
            
            {(content as Memory).metadata && Object.keys((content as Memory).metadata!).length > 0 && (
              <div className="properties-section">
                <label>Metadata</label>
                <div className="properties-list">
                  {renderProperties((content as Memory).metadata!)}
                </div>
              </div>
            )}
          </div>
        ) : (
          // Entity Node Details
          <div className="entity-details">
            <div className="content-section">
              <label>Name</label>
              <div className="content-text">
                {(content as Entity).name || (content as Entity).properties?.name || 'Unnamed Entity'}
              </div>
            </div>
            
            <div className="metadata-grid">
              <div className="metadata-item">
                <label>Type</label>
                <span className="entity-type-tag">
                  {(content as Entity).entity_type}
                </span>
              </div>
              
              <div className="metadata-item">
                <label>Created</label>
                <span>{formatDate((content as Entity).created_at)}</span>
              </div>
              
              <div className="metadata-item">
                <label>Updated</label>
                <span>{formatDate((content as Entity).updated_at)}</span>
              </div>
            </div>
            
            {(content as Entity).properties && Object.keys((content as Entity).properties).length > 0 && (
              <div className="properties-section">
                <label>Properties</label>
                <div className="properties-list">
                  {renderProperties((content as Entity).properties)}
                </div>
              </div>
            )}
          </div>
        )}
        
        <div className="node-id-section">
          <label>Node ID</label>
          <code className="node-id">{node.id}</code>
        </div>
        
        {node.data.centrality && (
          <div className="centrality-section">
            <label>Centrality Score</label>
            <div className="centrality-bar">
              <div 
                className="centrality-fill" 
                style={{ width: `${Math.min(node.data.centrality * 100, 100)}%` }}
              ></div>
              <span className="centrality-value">
                {(node.data.centrality * 100).toFixed(1)}%
              </span>
            </div>
          </div>
        )}
      </div>
    );
  };

  const handleRowClick = (nodeId: string) => {
    // Clear current selection and select only this node
    clearSelection();
    selectNode(nodeId);
  };

  const renderMultipleNodesTable = () => {
    return (
      <div className="multiple-nodes-table">
        <div className="table-container">
          <table className="nodes-table">
            <thead>
              <tr>
                <th>Type</th>
                <th>Name/Content</th>
                <th>Category</th>
                <th>Priority</th>
                <th>Created</th>
                <th>ID</th>
              </tr>
            </thead>
            <tbody>
              {selectedNodeObjects.map((node: any) => {
                const isMemoryNode = node.type === 'memoryNode';
                const content = node.data.content as Memory | Entity;
                
                return (
                  <tr 
                    key={node.id} 
                    className="table-row-clickable"
                    onClick={() => handleRowClick(node.id)}
                    title="Click to view details for this node"
                  >
                    <td>
                      <span className={`node-type-badge ${isMemoryNode ? 'memory' : 'entity'}`}>
                        {isMemoryNode ? 'Memory' : 'Entity'}
                      </span>
                    </td>
                    <td className="content-cell">
                      {isMemoryNode 
                        ? (content as Memory).content.length > 50 
                          ? `${(content as Memory).content.substring(0, 50)}...`
                          : (content as Memory).content
                        : (content as Entity).name || (content as Entity).properties?.name || 'Unnamed Entity'
                      }
                    </td>
                    <td>
                      {isMemoryNode ? (
                        <span className="memory-type-tag" data-type={(content as Memory).memory_type}>
                          {(content as Memory).memory_type}
                        </span>
                      ) : (
                        <span className="entity-type-tag">
                          {(content as Entity).entity_type}
                        </span>
                      )}
                    </td>
                    <td>
                      {isMemoryNode ? (
                        <span className="priority-tag" data-priority={(content as Memory).priority}>
                          {(content as Memory).priority}
                        </span>
                      ) : (
                        <span className="priority-tag" data-priority="Medium">
                          -
                        </span>
                      )}
                    </td>
                    <td className="date-cell">
                      {formatDateShort(isMemoryNode ? (content as Memory).created_at : (content as Entity).created_at)}
                    </td>
                    <td className="id-cell">
                      <code className="table-node-id">{node.id.substring(0, 8)}...</code>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </div>
    );
  };

  return (
    <div className={`node-details-panel ${isMultipleSelection ? 'multiple-selection' : 'single-selection'}`}>
      <div className="panel-header">
        <h3>
          {isMultipleSelection 
            ? `Selected Nodes (${selectedNodeObjects.length})`
            : 'Node Details'
          }
        </h3>
        {!isMultipleSelection && selectedNodeObjects[0] && (
          <div className="node-type-badge" data-type={selectedNodeObjects[0].type}>
            {selectedNodeObjects[0].type === 'memoryNode' ? 'Memory' : 'Entity'}
          </div>
        )}
      </div>
      
      <div className="panel-content">
        {isMultipleSelection ? renderMultipleNodesTable() : selectedNodeObjects[0] && renderSingleNodeDetails(selectedNodeObjects[0])}
      </div>

      <style>{`
        .node-details-panel {
          position: fixed;
          bottom: 20px;
          left: 20px;
          min-width: 350px;
          max-width: calc(100vw - 40px);
          max-height: 60vh;
          background: ${colors.bg.primary};
          border: 1px solid ${colors.border.primary};
          border-radius: 12px;
          box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
          display: flex;
          flex-direction: column;
          z-index: 1000;
          overflow: hidden;
          transition: all 0.3s ease-in-out;
        }

        .node-details-panel.single-selection {
          width: 350px;
        }

        .node-details-panel.multiple-selection {
          width: min(90vw, 1200px);
          max-height: 70vh;
        }

        .panel-header {
          padding: 16px 20px;
          border-bottom: 1px solid ${colors.border.primary};
          background: ${colors.bg.secondary};
          display: flex;
          justify-content: space-between;
          align-items: center;
        }

        .panel-header h3 {
          margin: 0;
          font-size: 16px;
          font-weight: 600;
          color: ${colors.text.primary};
        }

        .node-type-badge {
          padding: 4px 8px;
          border-radius: 6px;
          font-size: 12px;
          font-weight: 500;
          text-transform: uppercase;
        }

        .node-type-badge[data-type="memoryNode"] {
          background: ${colors.node.memory.fact}20;
          color: ${colors.node.memory.fact};
        }

        .node-type-badge[data-type="entityNode"] {
          background: ${colors.node.entity}20;
          color: ${colors.node.entity};
        }

        .panel-content {
          flex: 1;
          overflow-y: auto;
          padding: 20px;
        }

        .empty-state {
          text-align: center;
          padding: 40px 20px;
          color: ${colors.text.muted};
        }

        .empty-icon {
          font-size: 48px;
          margin-bottom: 16px;
        }

        .content-section {
          margin-bottom: 20px;
        }

        .content-section label {
          display: block;
          font-size: 12px;
          font-weight: 600;
          color: ${colors.text.muted};
          text-transform: uppercase;
          letter-spacing: 0.5px;
          margin-bottom: 8px;
        }

        .content-text {
          background: ${colors.bg.secondary};
          border: 1px solid ${colors.border.primary};
          border-radius: 8px;
          padding: 12px;
          color: ${colors.text.primary};
          line-height: 1.5;
          font-size: 14px;
        }

        .metadata-grid {
          display: grid;
          grid-template-columns: 1fr 1fr;
          gap: 16px;
          margin-bottom: 20px;
        }

        .metadata-item {
          display: flex;
          flex-direction: column;
          gap: 4px;
        }

        .metadata-item label {
          font-size: 12px;
          font-weight: 600;
          color: ${colors.text.muted};
          text-transform: uppercase;
          letter-spacing: 0.5px;
        }

        .metadata-item span {
          color: ${colors.text.primary};
          font-size: 14px;
        }

        .memory-type-tag {
          padding: 4px 8px;
          border-radius: 4px;
          font-size: 12px;
          font-weight: 500;
          width: fit-content;
        }

        .memory-type-tag[data-type="Fact"] {
          background: #3b82f620;
          color: #3b82f6;
        }

        .memory-type-tag[data-type="Episodic"] {
          background: #8b5cf620;
          color: #8b5cf6;
        }

        .memory-type-tag[data-type="Semantic"] {
          background: #10b98120;
          color: #10b981;
        }

        .priority-tag {
          padding: 4px 8px;
          border-radius: 4px;
          font-size: 12px;
          font-weight: 500;
          width: fit-content;
        }

        .priority-tag[data-priority="High"] {
          background: #ef444420;
          color: #ef4444;
        }

        .priority-tag[data-priority="Medium"] {
          background: #f59e0b20;
          color: #f59e0b;
        }

        .priority-tag[data-priority="Low"] {
          background: #6b728020;
          color: #6b7280;
        }

        .entity-type-tag {
          padding: 4px 8px;
          border-radius: 4px;
          font-size: 12px;
          font-weight: 500;
          background: ${colors.node.entity}20;
          color: ${colors.node.entity};
          width: fit-content;
        }

        .properties-section {
          margin-bottom: 20px;
        }

        .properties-section label {
          display: block;
          font-size: 12px;
          font-weight: 600;
          color: ${colors.text.muted};
          text-transform: uppercase;
          letter-spacing: 0.5px;
          margin-bottom: 8px;
        }

        .properties-list {
          background: ${colors.bg.secondary};
          border: 1px solid ${colors.border.primary};
          border-radius: 8px;
          padding: 12px;
        }

        .property-item {
          display: flex;
          justify-content: space-between;
          align-items: start;
          padding: 6px 0;
          border-bottom: 1px solid ${colors.border.primary};
        }

        .property-item:last-child {
          border-bottom: none;
        }

        .property-label {
          font-weight: 500;
          color: ${colors.text.muted};
          margin-right: 12px;
          flex-shrink: 0;
        }

        .property-value {
          color: ${colors.text.primary};
          text-align: right;
          word-break: break-word;
          font-size: 14px;
        }

        .node-id-section {
          margin-bottom: 20px;
        }

        .node-id-section label {
          display: block;
          font-size: 12px;
          font-weight: 600;
          color: ${colors.text.muted};
          text-transform: uppercase;
          letter-spacing: 0.5px;
          margin-bottom: 8px;
        }

        .node-id {
          display: block;
          background: ${colors.bg.secondary};
          border: 1px solid ${colors.border.primary};
          border-radius: 6px;
          padding: 8px 10px;
          font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
          font-size: 12px;
          color: ${colors.text.primary};
          word-break: break-all;
        }

        .centrality-section {
          margin-bottom: 20px;
        }

        .centrality-section label {
          display: block;
          font-size: 12px;
          font-weight: 600;
          color: ${colors.text.muted};
          text-transform: uppercase;
          letter-spacing: 0.5px;
          margin-bottom: 8px;
        }

        .centrality-bar {
          position: relative;
          background: ${colors.bg.secondary};
          border: 1px solid ${colors.border.primary};
          border-radius: 6px;
          height: 24px;
          overflow: hidden;
        }

        .centrality-fill {
          height: 100%;
          background: linear-gradient(90deg, ${colors.node.selected}, ${colors.node.selected}80);
          transition: width 0.3s ease;
        }

        .centrality-value {
          position: absolute;
          top: 50%;
          right: 8px;
          transform: translateY(-50%);
          font-size: 12px;
          font-weight: 500;
          color: ${colors.text.primary};
        }

        .multiple-nodes-table {
          width: 100%;
          height: 100%;
        }

        .table-container {
          overflow-x: auto;
          overflow-y: auto;
          max-height: calc(70vh - 100px);
        }

        .nodes-table {
          width: 100%;
          border-collapse: collapse;
          font-size: 14px;
        }

        .nodes-table th {
          background: ${colors.bg.secondary};
          color: ${colors.text.primary};
          font-weight: 600;
          text-align: left;
          padding: 12px 8px;
          border-bottom: 2px solid ${colors.border.primary};
          position: sticky;
          top: 0;
          z-index: 1;
        }

        .nodes-table td {
          padding: 10px 8px;
          border-bottom: 1px solid ${colors.border.primary};
          vertical-align: top;
        }

        .table-row-clickable {
          cursor: pointer;
          transition: background-color 0.2s ease;
        }

        .table-row-clickable:hover {
          background: ${colors.bg.secondary};
        }

        .table-row-clickable:active {
          background: ${colors.border.primary};
        }

        .content-cell {
          max-width: 300px;
          word-wrap: break-word;
          line-height: 1.4;
        }

        .date-cell {
          white-space: nowrap;
          font-size: 12px;
          color: ${colors.text.muted};
        }

        .id-cell {
          font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
          font-size: 11px;
        }

        .table-node-id {
          background: ${colors.bg.secondary};
          padding: 2px 4px;
          border-radius: 4px;
          font-size: 10px;
          color: ${colors.text.muted};
        }

        .node-type-badge.memory {
          background: ${colors.node.memory.fact}20;
          color: ${colors.node.memory.fact};
          padding: 3px 6px;
          border-radius: 4px;
          font-size: 11px;
          font-weight: 500;
          text-transform: uppercase;
        }

        .node-type-badge.entity {
          background: ${colors.node.entity}20;
          color: ${colors.node.entity};
          padding: 3px 6px;
          border-radius: 4px;
          font-size: 11px;
          font-weight: 500;
          text-transform: uppercase;
        }

        .single-node-details {
          animation: fadeIn 0.2s ease-in-out;
        }

        .multiple-nodes-table {
          animation: slideUp 0.3s ease-in-out;
        }

        @keyframes fadeIn {
          from {
            opacity: 0;
            transform: translateY(10px);
          }
          to {
            opacity: 1;
            transform: translateY(0);
          }
        }

        @keyframes slideUp {
          from {
            opacity: 0;
            transform: translateY(20px);
          }
          to {
            opacity: 1;
            transform: translateY(0);
          }
        }

        @media (max-width: 768px) {
          .node-details-panel {
            position: fixed;
            bottom: 0;
            left: 0;
            right: 0;
            width: 100%;
            max-width: none;
            border-radius: 12px 12px 0 0;
          }

          .node-details-panel.multiple-selection {
            max-height: 80vh;
          }

          .nodes-table {
            font-size: 12px;
          }

          .nodes-table th,
          .nodes-table td {
            padding: 8px 4px;
          }

          .content-cell {
            max-width: 200px;
          }
        }
      `}</style>
    </div>
  );
};

export default NodeDetailsPanel; 