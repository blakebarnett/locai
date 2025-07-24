import React, { useCallback, useEffect, useMemo } from 'react';
import ReactFlow, {
  addEdge,
  useNodesState,
  useEdgesState,
  useReactFlow,
  Controls,
  MiniMap,
  Background,
  BackgroundVariant,
} from 'reactflow';
import type { Node, Edge, Connection, NodeTypes, EdgeTypes } from 'reactflow';
import 'reactflow/dist/style.css';

import { useGraphStore } from '../stores/graphStore';
import { saveNodePositions } from '../utils/layoutCache';
import MemoryNode from './nodes/MemoryNode';
import EntityNode from './nodes/EntityNode';
import GraphControls from './GraphControls';
import AnalyticsPanel from './AnalyticsPanel';
import ToastContainer from './ToastContainer';
import NodeDetailsPanel from './NodeDetailsPanel';

// Define nodeTypes outside component to prevent re-creation
const nodeTypes: NodeTypes = {
  memoryNode: MemoryNode,
  entityNode: EntityNode,
};

const GraphVisualization: React.FC = () => {
  const {
    nodes: storeNodes,
    edges: storeEdges,
    isLoading,
    error,
    connectionState,
    selectedNodes,
    selectedEdges,
    showLabels,
    layoutType,
    selectNode,
    selectEdge,
    clearSelection,
    initializeWebSocket,
    loadAllData,
    loadMetrics,
    applyLayout,
    toasts,
    removeToast,
  } = useGraphStore();

  const { fitView } = useReactFlow();

  // Convert store nodes/edges to React Flow format
  const reactFlowNodes: Node[] = useMemo(() => 
    storeNodes.map(node => ({
      ...node,
      selected: selectedNodes.includes(node.id),
    })), 
    [storeNodes, selectedNodes]
  );

  const reactFlowEdges: Edge[] = useMemo(() => 
    storeEdges.map(edge => ({
      ...edge,
      selected: selectedEdges.includes(edge.id),
      label: showLabels ? edge.label : undefined,
    })), 
    [storeEdges, selectedEdges, showLabels]
  );

  const [nodes, setNodes, onNodesChange] = useNodesState(reactFlowNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(reactFlowEdges);

  // Custom nodes change handler to save positions
  const handleNodesChange = useCallback((changes: any[]) => {
    onNodesChange(changes);
    
    // Check if any position changes occurred
    const hasPositionChanges = changes.some(change => 
      change.type === 'position' && change.position
    );
    
    if (hasPositionChanges) {
      // Save positions after a short delay to avoid excessive saves
      setTimeout(() => {
        const currentNodes = nodes.map(node => {
          const change = changes.find(c => c.id === node.id && c.type === 'position');
          if (change && change.position) {
            return {
              ...node,
              position: change.position,
            };
          }
          return node;
        });
        
        // Convert to store format and save
        const storeNodes = currentNodes.map(node => ({
          id: node.id,
          type: node.type as 'memoryNode' | 'entityNode',
          position: node.position,
          data: node.data,
        }));
        
        saveNodePositions(storeNodes, layoutType);
      }, 100);
    }
  }, [onNodesChange, nodes, layoutType]);

  // Update React Flow nodes/edges when store changes
  useEffect(() => {
    setNodes(reactFlowNodes);
  }, [reactFlowNodes, setNodes]);

  useEffect(() => {
    setEdges(reactFlowEdges);
  }, [reactFlowEdges, setEdges]);

  // Initialize WebSocket and load initial data
  useEffect(() => {
    initializeWebSocket();
    loadAllData(); // Load real data instead of demo data
  }, [initializeWebSocket]);

  // Load metrics separately to avoid dependency issues
  useEffect(() => {
    loadMetrics();
  }, [loadMetrics]);

  // Keyboard shortcuts for layout management
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      // Only trigger if no input field is focused
      if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) {
        return;
      }

      switch (event.key.toLowerCase()) {
        case 'r':
          if (event.ctrlKey || event.metaKey) {
            event.preventDefault();
            applyLayout(800, 600);
          }
          break;
        case 'escape':
          clearSelection();
          break;
        case 'f':
          event.preventDefault();
          fitView({ duration: 800 });
          break;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [applyLayout, clearSelection, fitView]);

  // Handle node selection
  const onNodeClick = useCallback((event: React.MouseEvent, node: Node) => {
    event.stopPropagation();
    selectNode(node.id);
  }, [selectNode]);

  // Handle edge selection
  const onEdgeClick = useCallback((event: React.MouseEvent, edge: Edge) => {
    event.stopPropagation();
    selectEdge(edge.id);
  }, [selectEdge]);

  // Handle pane click (clear selection)
  const onPaneClick = useCallback(() => {
    clearSelection();
  }, [clearSelection]);

  // Handle connection creation (for future use)
  const onConnect = useCallback((params: Connection) => {
    setEdges((eds) => addEdge(params, eds));
  }, [setEdges]);

  // Auto-fit view when nodes change
  useEffect(() => {
    if (nodes.length > 0) {
      setTimeout(() => fitView({ padding: 0.1 }), 100);
    }
  }, [nodes.length, fitView]);

  // Note: Removed automatic layout application to prevent infinite loops
  // Users can manually apply layouts using the "Apply Layout" button

  if (error) {
    return (
      <div className="flex items-center justify-center h-full bg-red-50">
        <div className="text-center">
          <div className="text-red-600 text-lg font-semibold mb-2">
            Error loading graph
          </div>
          <div className="text-red-500">{error}</div>
        </div>
      </div>
    );
  }

  return (
    <div className="w-full h-full relative">
      {/* Connection status indicator */}
      <div className={`connection-status ${
        connectionState === 'connected' ? 'status-connected' : 'status-disconnected'
      }`}>
        {connectionState === 'connected' && '● Connected'}
        {connectionState === 'connecting' && '◐ Connecting...'}
        {connectionState === 'disconnected' && '○ Disconnected'}
        {connectionState === 'error' && '✕ Connection Error'}
      </div>

      {/* Loading overlay */}
      {isLoading && (
        <div className="loading-overlay">
          <div className="loading-spinner"></div>
        </div>
      )}

      {/* Graph Controls */}
      <GraphControls />

      {/* Analytics Panel */}
      <AnalyticsPanel />

      {/* Node Details Panel */}
      <NodeDetailsPanel />

      {/* React Flow */}
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={handleNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onNodeClick={onNodeClick}
        onEdgeClick={onEdgeClick}
        onPaneClick={onPaneClick}
        nodeTypes={nodeTypes}
        fitView
        attributionPosition="bottom-left"
        className="bg-gray-900"
      >
        <Controls />
        <MiniMap 
          nodeColor={(node) => {
            if (node.type === 'memoryNode') {
              // Get memory type from node data for coloring
              const memoryType = node.data?.content?.memory_type;
              switch (memoryType) {
                case 'Fact': return '#60a5fa';
                case 'Episodic': return '#4ade80';
                case 'Semantic': return '#c084fc';
                default: return '#60a5fa';
              }
            }
            if (node.type === 'entityNode') return '#fbbf24';
            return '#94a3b8';
          }}
          className="bg-gray-900 border border-gray-600"
        />
        <Background 
          variant={BackgroundVariant.Dots} 
          gap={24} 
          size={2}
          color="#1f2937"
        />
      </ReactFlow>

      {/* Toast Notifications */}
      <ToastContainer toasts={toasts} onRemoveToast={removeToast} />
    </div>
  );
};

export default GraphVisualization; 