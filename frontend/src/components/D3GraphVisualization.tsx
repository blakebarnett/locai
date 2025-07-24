import React, { useEffect, useRef, useCallback, useMemo } from 'react';
import * as d3 from 'd3';
import { useGraphStore } from '../stores/graphStore';
import { useTheme } from '../contexts/ThemeContext';
import { saveNodePositions, hasLayoutCache } from '../utils/layoutCache';
import { saveZoomTransform, loadZoomTransform, hasZoomCache } from '../utils/zoomCache';
import ToastContainer from './ToastContainer';
import AnalyticsPanel from './AnalyticsPanel';
import NodeDetailsPanel from './NodeDetailsPanel';
import RealtimeEventFeed from './RealtimeEventFeed';
import GraphControls from './GraphControls';


interface D3Node extends d3.SimulationNodeDatum {
  id: string;
  type: 'memoryNode' | 'entityNode';
  data: {
    id: string;
    type: 'memory' | 'entity';
    content: any;
    centrality?: number;
    smartConnectivity?: number;
  };
  selected?: boolean;
  radius?: number;
  color?: string;
}

interface D3Link extends d3.SimulationLinkDatum<D3Node> {
  id: string;
  source: D3Node | string;
  target: D3Node | string;
  type: string;
  label?: string;
  data: any;
  selected?: boolean;
  opacity?: number;
  strokeWidth?: number;
  color?: string;
  importance?: string;
}

const D3GraphVisualization: React.FC = () => {
  const svgRef = useRef<SVGSVGElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const simulationRef = useRef<d3.Simulation<D3Node, D3Link> | null>(null);
  const transformRef = useRef<d3.ZoomTransform>(d3.zoomIdentity);
  const zoomBehaviorRef = useRef<d3.ZoomBehavior<SVGSVGElement, unknown> | null>(null);
  const isInitialLoadRef = useRef(true);
  const lastNodeCountRef = useRef(0);
  const lastEdgeCountRef = useRef(0);

  const {
    nodes: storeNodes,
    edges: storeEdges,
    isLoading,
    error,
    connectionState,
    selectedNodes,
    selectedEdges,
    showLabels,
    nodeSize,
    layoutType,
    dataSource,
    activeFilters,
    selectNode,
    selectEdge,
    clearSelection,
    initializeWebSocket,
    loadAllData,
    loadMetrics,
    updateNode,
    applyLayout,
    toasts,
    removeToast,
    loadDemoData,
  } = useGraphStore();
  const { colors } = useTheme();

  // Convert store data to D3 format
  const d3Data = useMemo(() => {
    // Enhanced connectivity calculation with relationship type weighting
    const calculateSmartConnectivity = (nodeId: string, edges: any[]) => {
      const connections = edges.filter(edge => 
        edge.source === nodeId || edge.target === nodeId
      );
      
      let semanticWeight = 0;
      let temporalWeight = 0;
      
      connections.forEach(edge => {
        const relType = edge.type?.toLowerCase() || '';
        
        // Semantic relationships get full weight (important for understanding)
        if (['references', 'mentions', 'relates_to', 'describes', 'contains', 'associated_with'].includes(relType)) {
          semanticWeight += 1.0;
        }
        // Temporal relationships get reduced weight (less important for sizing)
        else if (['temporal_sequence', 'follows', 'precedes', 'before', 'after'].includes(relType)) {
          temporalWeight += 0.2; // Much less weight
        }
        // Unknown relationships get moderate weight
        else {
          semanticWeight += 0.6;
        }
      });
      
      // Calculate smart connectivity: semantic connections matter most
      return Math.round(semanticWeight + Math.min(temporalWeight, 3)); // Cap temporal contribution
    };

    // Calculate improved node sizing with multiple strategies
    const calculateNodeRadius = (smartConnectivity: number, centrality: number = 0) => {
      const minRadius = 15;  // Smaller minimum
      const maxRadius = 45;  // Much smaller maximum to reduce overlap
      
      switch (nodeSize) {
        case 'uniform':
          return 20; // All nodes same size
          
        case 'centrality':
          // Size based on centrality if available, fallback to connectivity
          const centralityValue = centrality || smartConnectivity;
          if (centralityValue === 0) return minRadius;
          const centralityScale = Math.log(centralityValue + 1) / Math.log(10);
          const centralityFactor = Math.min(centralityScale / 2, 1);
          return Math.round(minRadius + (maxRadius - minRadius) * centralityFactor);
          
        case 'degree':
        default:
          // Size based on connectivity (original logic)
          if (smartConnectivity === 0) return minRadius;
          const normalizedConnectivity = Math.log(smartConnectivity + 1) / Math.log(10);
          const scaleFactor = Math.min(normalizedConnectivity / 2, 1);
          return Math.round(minRadius + (maxRadius - minRadius) * scaleFactor);
      }
    };

    // Apply memory type filtering
    const filteredNodes = storeNodes.filter(node => {
      // Always show entity nodes
      if (node.type === 'entityNode') return true;
      
      // For memory nodes, check if memory type filters are active
      if (activeFilters.memoryTypes.length === 0) return true; // No filters = show all
      
      // Check if this memory node's type is in the active filters
      if (node.data.type === 'memory') {
        const memoryType = (node.data.content as any)?.memory_type;
        return activeFilters.memoryTypes.includes(memoryType);
      }
      
      return true; // Fallback
    });

    const nodes: D3Node[] = filteredNodes.map(node => {
      const smartConnectivity = calculateSmartConnectivity(node.id, storeEdges);
      const centrality = node.data.centrality || 0;
      const radius = calculateNodeRadius(smartConnectivity, centrality);
      
      return {
        id: node.id,
        type: node.type,
        data: { ...node.data, smartConnectivity }, // Store for debugging
        selected: selectedNodes.includes(node.id),
        x: node.position.x,
        y: node.position.y,
        radius,
        color: node.type === 'memoryNode' ? colors.node.memory.fact : colors.node.entity,
      };
    });

    // Create a Set of valid node IDs for fast lookup
    const nodeIds = new Set(nodes.map(n => n.id));

    // Enhanced link processing with visual hierarchy
    const links: D3Link[] = storeEdges
      .filter(edge => {
        const hasSource = nodeIds.has(edge.source);
        const hasTarget = nodeIds.has(edge.target);
        return hasSource && hasTarget;
      })
      .map(edge => {
        const relType = edge.type?.toLowerCase() || '';
        
        // Determine visual properties based on relationship type
        let opacity = 0.6;
        let strokeWidth = 2;
        let color = colors.edge.default;
        let importance = 'medium';
        
        if (['references', 'mentions', 'relates_to', 'describes', 'contains'].includes(relType)) {
          // Semantic relationships: prominent
          opacity = 0.8;
          strokeWidth = 2.5;
          color = '#8b5cf6'; // Purple for semantic
          importance = 'high';
        } else if (['temporal_sequence', 'follows', 'precedes'].includes(relType)) {
          // Temporal relationships: subtle
          opacity = 0.3;
          strokeWidth = 1.5;
          color = '#6b7280'; // Gray for temporal
          importance = 'low';
        }
        
        return {
          id: edge.id,
          source: edge.source,
          target: edge.target,
          type: edge.type,
          label: edge.label,
          data: edge.data,
          selected: selectedEdges.includes(edge.id),
          opacity,
          strokeWidth,
          color,
          importance,
        };
      });

    console.log(`Enhanced graph: ${nodes.length} nodes (filtered from ${storeNodes.length}), ${links.length} links`);
    console.log('Memory type filtering:', {
      activeFilters: activeFilters.memoryTypes,
      originalNodes: storeNodes.length,
      filteredNodes: nodes.length,
      memoryNodes: nodes.filter(n => n.type === 'memoryNode').length,
      entityNodes: nodes.filter(n => n.type === 'entityNode').length,
    });
    console.log('Node sizing strategy:', {
      nodeSize,
      distribution: {
        small: nodes.filter(n => n.radius! <= 20).length,
        medium: nodes.filter(n => n.radius! > 20 && n.radius! <= 35).length,
        large: nodes.filter(n => n.radius! > 35).length,
      }
    });
    console.log('Edge importance distribution:', {
      high: links.filter(l => l.importance === 'high').length,
      medium: links.filter(l => l.importance === 'medium').length,
      low: links.filter(l => l.importance === 'low').length,
    });

    return { nodes, links };
  }, [storeNodes, storeEdges, selectedNodes, selectedEdges, colors.node.memory.fact, colors.node.entity, colors.edge.default, activeFilters.memoryTypes, nodeSize]);

  // Create a stable reference for structural data only (no selection state)
  const structuralData = useMemo(() => ({
    nodes: storeNodes,
    edges: storeEdges,
    nodeSize,
    layoutType,
  }), [storeNodes, storeEdges, nodeSize, layoutType]);

  // Use refs for stable handlers to prevent unnecessary re-renders
  const handlersRef = useRef({
    handleNodeClick: (event: MouseEvent, node: D3Node) => {
      event.stopPropagation();
      selectNode(node.id);
    },
    handleLinkClick: (event: MouseEvent, link: D3Link) => {
      event.stopPropagation();
      selectEdge(link.id);
    },
    handleBackgroundClick: () => {
      clearSelection();
    },
  });

  // Track click vs drag state
  const dragStateRef = useRef({
    isDragging: false,
    startPos: { x: 0, y: 0 },
    threshold: 5, // pixels to distinguish click from drag
  });

  // Update handler references when dependencies change
  useEffect(() => {
    handlersRef.current.handleNodeClick = (event: MouseEvent, node: D3Node) => {
      event.stopPropagation();
      selectNode(node.id);
    };
    handlersRef.current.handleLinkClick = (event: MouseEvent, link: D3Link) => {
      event.stopPropagation();
      selectEdge(link.id);
    };
    handlersRef.current.handleBackgroundClick = () => {
      clearSelection();
    };
  }, [selectNode, selectEdge, clearSelection]);

  // Initialize and update D3 visualization
  useEffect(() => {
    if (!svgRef.current || !containerRef.current) return;

    const svg = d3.select(svgRef.current);
    const container = containerRef.current;
    const width = container.clientWidth;
    const height = container.clientHeight;

    // Clear previous content
    svg.selectAll('*').remove();

    // Set up SVG dimensions and dark background
    svg.attr('width', width).attr('height', height)
       .style('background-color', '#0a0a0a');

    // Create main group for zoom/pan
    const g = svg.append('g').attr('class', 'graph-container');

    // Set up zoom behavior
    const zoom = d3.zoom<SVGSVGElement, unknown>()
      .scaleExtent([0.1, 4])
      .on('zoom', (event) => {
        transformRef.current = event.transform;
        g.attr('transform', event.transform);
        
        // Save zoom transform to localStorage (debounced to avoid excessive writes)
        if (dataSource) {
          clearTimeout((zoom as any).__saveTimeout);
          (zoom as any).__saveTimeout = setTimeout(() => {
            saveZoomTransform(event.transform, layoutType, dataSource);
          }, 500);
        }
      });

    zoomBehaviorRef.current = zoom;
    svg.call(zoom);

    // Load cached zoom transform if available
    if (dataSource) {
      const cachedTransform = loadZoomTransform(layoutType, dataSource);
      if (cachedTransform) {
        console.log('Applying cached zoom transform');
        svg.call(zoom.transform, cachedTransform);
        transformRef.current = cachedTransform;
      }
    }

    // Handle background clicks
    svg.on('click', () => handlersRef.current.handleBackgroundClick());

    // Calculate node connectivity for hierarchical clustering (using smart connectivity)
    const nodeConnectivity = new Map<string, number>();
    const { nodes, links } = d3Data;
    
    // Use the smart connectivity we already calculated
    nodes.forEach(node => {
      nodeConnectivity.set(node.id, node.data.smartConnectivity || 0);
    });

    // Identify central nodes (high connectivity) for each type
    const memoryNodes = nodes.filter(n => n.type === 'memoryNode');
    const entityNodes = nodes.filter(n => n.type === 'entityNode');
    
    const centralMemoryNodes = memoryNodes
      .sort((a, b) => (nodeConnectivity.get(b.id) || 0) - (nodeConnectivity.get(a.id) || 0))
      .slice(0, Math.min(5, Math.ceil(memoryNodes.length / 20))); // Top 5 or 5% of memory nodes
    
    const centralEntityNodes = entityNodes
      .sort((a, b) => (nodeConnectivity.get(b.id) || 0) - (nodeConnectivity.get(a.id) || 0))
      .slice(0, Math.min(3, Math.ceil(entityNodes.length / 10))); // Top 3 or 10% of entity nodes

    const centralNodeIds = new Set([
      ...centralMemoryNodes.map(n => n.id),
      ...centralEntityNodes.map(n => n.id)
    ]);

    // Debug: Log central nodes for development
    console.log('Central nodes identified:', {
      memories: centralMemoryNodes.map(n => ({ 
        id: n.id, 
        type: n.data.content.memory_type,
        connections: nodeConnectivity.get(n.id) 
      })),
      entities: centralEntityNodes.map(n => ({ 
        id: n.id, 
        name: n.data.content.properties?.name || n.data.content.name,
        connections: nodeConnectivity.get(n.id) 
      }))
    });

    // Create hierarchical force simulation with increased spacing
    const simulation = d3.forceSimulation<D3Node, D3Link>()
      .force('link', d3.forceLink<D3Node, D3Link>()
        .id(d => d.id)
        .distance(d => {
          const sourceNode = d.source as D3Node;
          const targetNode = d.target as D3Node;
          const link = d as any;
          
          // Distance based on relationship importance and node sizes
          const baseDistance = 60;
          const sourceRadius = sourceNode.radius || 20;
          const targetRadius = targetNode.radius || 20;
          const radiusBuffer = sourceRadius + targetRadius + 20;
          
          // Temporal relationships can be closer (less visual importance)
          if (link.importance === 'low') {
            return Math.max(baseDistance, radiusBuffer * 0.8);
          }
          // Semantic relationships need more space (high visual importance)
          else if (link.importance === 'high') {
            return Math.max(baseDistance * 1.5, radiusBuffer * 1.2);
          }
          
          return Math.max(baseDistance, radiusBuffer);
        })
        .strength(d => {
          const link = d as any;
          // Semantic relationships have stronger attraction
          return link.importance === 'high' ? 0.8 : 0.4;
        })
        .iterations(2))
      .force('charge', d3.forceManyBody()
        .strength(d => {
          const node = d as D3Node;
          const radius = node.radius || 20;
          const connections = nodeConnectivity.get(node.id) || 0;
          
          // Repulsion based on actual node size and connections
          const baseRepulsion = -400;
          const sizeMultiplier = radius / 20; // Scale with actual size
          const connectionMultiplier = 1 + (connections * 0.3);
          
          return baseRepulsion * sizeMultiplier * connectionMultiplier;
        })
        .distanceMax(400)) // Increased for better separation
      .force('center', d3.forceCenter(width * 0.6, height / 2).strength(0.1))
      .force('collision', d3.forceCollide<D3Node>()
        .radius(d => {
          const node = d as D3Node;
          const radius = node.radius || 20;
          return radius + 15; // Good spacing buffer
        })
        .strength(0.9))
      // Type-based clustering with better spacing
      .force('cluster', d3.forceX()
        .x(d => {
          const node = d as D3Node;
          const connections = nodeConnectivity.get(node.id) || 0;
          
          // Highly connected nodes spread across center
          if (connections >= 3) {
            return width * (0.5 + Math.random() * 0.3); // Center area
          }
          
          // Type-based positioning with more spread
          return node.type === 'memoryNode' ? width * 0.45 : width * 0.75;
        })
        .strength(0.2))
      .force('clusterY', d3.forceY()
        .y(d => height / 2 + (Math.random() - 0.5) * height * 0.6)
        .strength(0.1))
      .alphaDecay(0.015) // Balanced decay
      .alphaMin(0.001);

    simulationRef.current = simulation;

    // Create arrow markers for directed edges - smaller arrows
    const defs = svg.append('defs');
    defs.append('marker')
      .attr('id', 'arrowhead')
      .attr('viewBox', '-0 -3 6 6')
      .attr('refX', 20)
      .attr('refY', 0)
      .attr('orient', 'auto')
      .attr('markerWidth', 5)
      .attr('markerHeight', 5)
      .attr('xoverflow', 'visible')
      .append('svg:path')
      .attr('d', 'M 0,-3 L 6,0 L 0,3')
      .attr('fill', colors.edge.default)
      .style('stroke', 'none');

    // Update visualization with data
    updateVisualization();

    function updateVisualization() {
      const { nodes, links } = d3Data;

      // Links
      const linkSelection = g.selectAll<SVGLineElement, D3Link>('.link')
        .data(links, d => d.id);

      linkSelection.exit().remove();

      const linkEnter = linkSelection.enter()
        .append('line')
        .attr('class', 'link')
        .attr('stroke', d => (d as any).color)
        .attr('stroke-width', d => (d as any).strokeWidth)
        .attr('stroke-opacity', d => (d as any).opacity)
        .attr('marker-end', 'url(#arrowhead)')
        .style('cursor', 'pointer')
        .style('stroke-dasharray', d => {
          const link = d as any;
          // Dash temporal relationships to make them less prominent
          return link.importance === 'low' ? '4,4' : 'none';
        })
        .on('click', (event, d) => handlersRef.current.handleLinkClick(event, d));

      const linkUpdate = linkEnter.merge(linkSelection)
        .attr('stroke', d => {
          // Keep original edge colors - selection handled in separate effect
          return (d as any).color;
        })
        .attr('stroke-width', d => {
          // Keep original edge widths - selection handled in separate effect
          return (d as any).strokeWidth;
        })
        .attr('stroke-opacity', d => {
          // Keep original edge opacity - selection handled in separate effect
          return (d as any).opacity;
        })
        .style('stroke-dasharray', d => {
          const link = d as any;
          // Dash temporal relationships to make them less prominent
          return link.importance === 'low' ? '4,4' : 'none';
        });

      // Link labels
      if (showLabels) {
        const labelSelection = g.selectAll<SVGTextElement, D3Link>('.link-label')
          .data(links, d => d.id);

        labelSelection.exit().remove();

        const labelEnter = labelSelection.enter()
          .append('text')
          .attr('class', 'link-label')
          .attr('text-anchor', 'middle')
          .attr('font-size', '12px')
          .attr('fill', colors.text.muted)
          .attr('pointer-events', 'none');

        const labelUpdate = labelEnter.merge(labelSelection);
        labelUpdate.text(d => d.label || '');
      } else {
        g.selectAll('.link-label').remove();
      }

      // Nodes
      const nodeSelection = g.selectAll<SVGGElement, D3Node>('.node')
        .data(nodes, d => d.id);

      nodeSelection.exit().remove();

      const nodeEnter = nodeSelection.enter()
        .append('g')
        .attr('class', 'node')
        .style('cursor', 'pointer')
        .on('click', (event, d) => handlersRef.current.handleNodeClick(event, d));

      // Add glow effect for highly connected nodes
      nodeEnter
        .filter(d => (nodeConnectivity.get(d.id) || 0) >= 3)
        .append('circle')
        .attr('class', 'node-glow')
        .attr('fill', 'none')
        .attr('stroke-width', 6)
        .attr('stroke-opacity', 0.4);

      // Add selection ring (outer ring for double-ring effect)
      nodeEnter.append('circle')
        .attr('class', 'node-selection-ring')
        .attr('fill', 'none')
        .attr('stroke-width', 3)
        .attr('stroke-opacity', 0)
        .attr('stroke', '#60a5fa'); // Blue selection ring

      // Node circles
      nodeEnter.append('circle')
        .attr('class', 'node-circle')
        .style('fill', 'transparent') // Ensure new nodes start transparent
        .attr('fill', 'transparent'); // Ensure new nodes start transparent

      // Node labels (positioned below the node)
      nodeEnter.append('text')
        .attr('class', 'node-label')
        .attr('text-anchor', 'middle')
        .attr('dy', '0.35em')
        .attr('font-size', '11px')
        .attr('font-weight', '600')
        .attr('fill', colors.text.primary)
        .attr('pointer-events', 'none');

      const nodeUpdate = nodeEnter.merge(nodeSelection);

      // Enhanced drag behavior with proper click detection
      const drag = d3.drag<SVGGElement, D3Node>()
        .on('start', (event, d) => {
          // Record start position and time to detect clicks vs drags
          dragStateRef.current.startPos = { x: event.x, y: event.y };
          dragStateRef.current.isDragging = false;
          (d as any).__dragStartTime = Date.now();
          
          // Mark this node as being dragged by user
          (d as any).__lastDragTime = Date.now();
          
          // Only restart simulation if it's completely stopped
          if (!event.active && simulation.alpha() < 0.01) {
            simulation.alphaTarget(0.05).restart();
          }
          // Fix the node position immediately
          d.fx = d.x;
          d.fy = d.y;
        })
        .on('drag', (event, d) => {
          // Check if we've moved enough to consider this a drag
          const dx = event.x - dragStateRef.current.startPos.x;
          const dy = event.y - dragStateRef.current.startPos.y;
          const distance = Math.sqrt(dx * dx + dy * dy);
          
          if (distance > dragStateRef.current.threshold) {
            dragStateRef.current.isDragging = true;
          }
          
          // Update position immediately during drag
          d.fx = event.x;
          d.fy = event.y;
        })
        .on('end', (event, d) => {
          // Calculate total movement and time to determine if this was a click
          const dx = event.x - dragStateRef.current.startPos.x;
          const dy = event.y - dragStateRef.current.startPos.y;
          const totalDistance = Math.sqrt(dx * dx + dy * dy);
          const duration = Date.now() - ((d as any).__dragStartTime || 0);
          
          // Consider it a click if: minimal movement AND short duration
          const isClick = totalDistance < dragStateRef.current.threshold && duration < 300;
          
          if (isClick) {
            // This was a click, not a drag - trigger click handler
            console.log('Node clicked:', d.id);
            event.stopPropagation?.();
            handlersRef.current.handleNodeClick(event, d);
          } else {
            // This was a drag - handle position update
            console.log('Node dragged:', d.id, { distance: totalDistance, duration });
            
            // Keep the node fixed at the dragged position
            d.fx = event.x;
            d.fy = event.y;
            
            // Update the store with the new position immediately
            updateNode(d.id, { 
              position: { x: event.x, y: event.y } 
            });
          }
          
          // Mark drag completion time
          (d as any).__lastDragTime = Date.now();
          
          // Stop simulation targeting
          if (!event.active) {
            simulation.alphaTarget(0);
          }
          
          // Reset drag state
          setTimeout(() => {
            dragStateRef.current.isDragging = false;
          }, 50);
        });

      nodeUpdate.call(drag);

      // Enhanced glow effect for important nodes
      nodeUpdate.select('.node-glow')
        .attr('r', d => {
          const connections = nodeConnectivity.get(d.id) || 0;
          const radius = d.radius || 20;
          // Only show glow for moderately connected nodes (not everything)
          if (connections >= 2 && connections <= 8) {
            return radius + 8;
          }
          return 0;
        })
        .attr('stroke', d => {
          const connections = nodeConnectivity.get(d.id) || 0;
          if (connections >= 2 && connections <= 8) {
            if (d.type === 'memoryNode') {
              const content = d.data.content;
              const memoryType = content.memory_type;
              if (memoryType === 'fact') return '#60a5fa';
              if (memoryType === 'episodic') return '#4ade80';
              if (memoryType === 'semantic') return '#c084fc';
              return '#60a5fa';
            } else {
              return '#fbbf24';
            }
          }
          return 'none';
        })
        .attr('stroke-opacity', 0.4);

      // Update selection rings
      nodeUpdate.select('.node-selection-ring')
        .attr('r', d => {
          if (d.selected) {
            const radius = d.radius || 20;
            return radius + 8; // Outer ring radius
          }
          return 0;
        })
        .attr('stroke-opacity', d => d.selected ? 0.8 : 0);

      // Update node circles - always preserve original colors
      nodeUpdate.select('.node-circle')
        .attr('r', d => d.radius || 20)
        .style('fill', 'transparent')
        .attr('fill', 'transparent')
        .attr('stroke', d => {
          // Always use original semantic colors, never override for selection
          if (d.type === 'memoryNode') {
            const content = d.data.content;
            const memoryType = content.memory_type;
            if (memoryType === 'fact') return '#60a5fa';
            if (memoryType === 'episodic') return '#4ade80';
            if (memoryType === 'semantic') return '#c084fc';
            return '#60a5fa';
          } else {
            return '#fbbf24';
          }
        })
        .attr('stroke-width', d => {
          const connections = nodeConnectivity.get(d.id) || 0;
          const selected = d.selected;
          // Slightly thicker border for selected nodes, plus connection-based thickness
          const baseWidth = connections >= 3 ? 4 : 3;
          return selected ? baseWidth + 1 : baseWidth;
        })
        .attr('opacity', 1.0);

      // Update labels
      nodeUpdate.select('.node-label')
        .attr('y', d => {
          const radius = d.radius || 20;
          return radius + 16;
        })
        .text(d => {
          const content = d.data.content;
          const connections = nodeConnectivity.get(d.id) || 0;
          
          let text = '';
          if (d.type === 'memoryNode') {
            text = String(content.memory_type || 'Memory');
          } else {
            text = String(content.properties?.name || content.name || content.entity_type || 'Entity');
          }
          
          // Dynamic truncation based on node size
          const radius = d.radius || 20;
          const maxLength = Math.max(8, Math.floor(radius / 3));
          
          // Show connection count for highly connected nodes
          if (connections >= 3) {
            text = `${text} (${connections})`;
          }
          
          return text.length > maxLength ? text.substring(0, maxLength) + '...' : text;
        })
        .attr('font-size', d => {
          const radius = d.radius || 20;
          // Scale font size with node radius
          return Math.max(10, Math.min(14, radius / 2.5)) + 'px';
        })
        .attr('font-weight', '600')
        .attr('fill', colors.text.primary)
        .attr('text-shadow', '0 1px 2px rgba(0, 0, 0, 0.8)')
        .attr('stroke', colors.bg.primary)
        .attr('stroke-width', '2px')
        .attr('paint-order', 'stroke');

      // Update simulation data with safety checks
      if (nodes.length > 0) {
        simulation.nodes(nodes);
        const linkForce = simulation.force<d3.ForceLink<D3Node, D3Link>>('link');
        if (linkForce && links.length > 0) {
          linkForce.links(links);
        } else if (linkForce && links.length === 0) {
          linkForce.links([]);
        }
      } else {
        simulation.nodes([]);
        const linkForce = simulation.force<d3.ForceLink<D3Node, D3Link>>('link');
        if (linkForce) {
          linkForce.links([]);
        }
      }

      // Tick function with safety checks
      simulation.on('tick', () => {
        linkUpdate
          .attr('x1', d => {
            const source = d.source as D3Node;
            return source && source.x !== undefined ? source.x : 0;
          })
          .attr('y1', d => {
            const source = d.source as D3Node;
            return source && source.y !== undefined ? source.y : 0;
          })
          .attr('x2', d => {
            const target = d.target as D3Node;
            return target && target.x !== undefined ? target.x : 0;
          })
          .attr('y2', d => {
            const target = d.target as D3Node;
            return target && target.y !== undefined ? target.y : 0;
          });

        if (showLabels) {
          g.selectAll<SVGTextElement, D3Link>('.link-label')
            .attr('x', (d: any) => {
              const source = d.source as D3Node;
              const target = d.target as D3Node;
              if (source && target && source.x !== undefined && target.x !== undefined) {
                return (source.x + target.x) / 2;
              }
              return 0;
            })
            .attr('y', (d: any) => {
              const source = d.source as D3Node;
              const target = d.target as D3Node;
              if (source && target && source.y !== undefined && target.y !== undefined) {
                return (source.y + target.y) / 2;
              }
              return 0;
            });
        }

        nodeUpdate
          .attr('transform', d => {
            const x = d.x !== undefined ? d.x : 0;
            const y = d.y !== undefined ? d.y : 0;
            return `translate(${x},${y})`;
          });
      });

      // Track data changes but preserve existing positions
      const hasNewNodes = nodes.length !== lastNodeCountRef.current;
      const hasNewEdges = links.length !== lastEdgeCountRef.current;
      
      // Store current positions before updating data
      const nodePositions = new Map<string, {x: number, y: number}>();
      if (!isInitialLoadRef.current) {
        simulation.nodes().forEach(node => {
          if (node.x !== undefined && node.y !== undefined) {
            nodePositions.set(node.id, { x: node.x, y: node.y });
          }
        });
      }
      
      // Update simulation data
      simulation.nodes(nodes);
      const linkForce = simulation.force<d3.ForceLink<D3Node, D3Link>>('link');
      if (linkForce) {
        linkForce.links(links);
      }
      
      // Restore positions for existing nodes and set initial positions for new ones
      if (!isInitialLoadRef.current) {
        nodes.forEach(node => {
          const savedPos = nodePositions.get(node.id);
          if (savedPos) {
            // Restore exact position for existing nodes
            node.x = savedPos.x;
            node.y = savedPos.y;
            node.vx = 0; // Stop any velocity to prevent movement
            node.vy = 0;
            // Keep them fixed to prevent simulation from moving them
            node.fx = savedPos.x;
            node.fy = savedPos.y;
            console.log(`Restored cached position for ${node.id}:`, savedPos);
          } else {
            // New node - position near similar nodes or use default
            const similarNodes = nodes.filter(n => 
              n.type === node.type && nodePositions.has(n.id)
            );
            
            if (similarNodes.length > 0) {
              // Position near similar nodes with slight offset
              const avgX = similarNodes.reduce((sum, n) => sum + (n.x || 0), 0) / similarNodes.length;
              const avgY = similarNodes.reduce((sum, n) => sum + (n.y || 0), 0) / similarNodes.length;
              node.x = avgX + (Math.random() - 0.5) * 100;
              node.y = avgY + (Math.random() - 0.5) * 100;
            } else {
              // Use force-based initial positioning (account for left panel)
              const width = container.clientWidth;
              const height = container.clientHeight;
              const visibleCenterX = width * 0.7; // Center of visible area
              node.x = node.type === 'memoryNode' ? width * 0.6 : width * 0.8;
              node.y = height * 0.5 + (Math.random() - 0.5) * 200;
            }
            node.vx = 0;
            node.vy = 0;
            console.log(`Set initial position for new node ${node.id}:`, { x: node.x, y: node.y });
          }
        });
      } else {
        // Initial load - check if nodes already have good positions from store (cached)
        nodes.forEach(node => {
          // If store nodes have specific positions (not random), use them
          const storeNode = storeNodes.find(sn => sn.id === node.id);
          if (storeNode && storeNode.position.x !== undefined && storeNode.position.y !== undefined) {
            // Check if this looks like a cached position (not random 0-500 range)
            if (storeNode.position.x > 600 || storeNode.position.y > 600 || 
                storeNode.position.x % 1 !== 0 || storeNode.position.y % 1 !== 0) {
              node.x = storeNode.position.x;
              node.y = storeNode.position.y;
              node.fx = storeNode.position.x; // Fix position
              node.fy = storeNode.position.y;
              node.vx = 0;
              node.vy = 0;
              console.log(`Applied cached store position for ${node.id}:`, storeNode.position);
            }
          }
        });
      }
      
      // Update counts after processing
      lastNodeCountRef.current = nodes.length;
      lastEdgeCountRef.current = links.length;
      
      // Only restart simulation with appropriate alpha for different scenarios
      if (isInitialLoadRef.current && nodes.length > 0) {
        isInitialLoadRef.current = false;
        
        // Check if nodes have good cached positions (not just random)
        const hasCachedLayout = hasLayoutCache(layoutType);
        const hasGoodPositions = hasCachedLayout || nodes.some(node => {
          // If any node has non-random looking coordinates, assume we have cached positions
          const x = node.x || 0;
          const y = node.y || 0;
          // Random positions are typically 0-500, so check for positions outside that or specific patterns
          return (x > 600 || y > 600 || (x % 1 !== 0) || (y % 1 !== 0));
        });
        
        if (hasGoodPositions) {
          console.log('Using cached positions, minimal simulation restart');
          // Just gentle settling with very low alpha
          simulation.alpha(0.05).restart();
          
          // Save the cached positions immediately to ensure they persist
          const currentNodes = storeNodes.map(storeNode => ({
            ...storeNode,
            position: {
              x: nodes.find(n => n.id === storeNode.id)?.x || storeNode.position.x,
              y: nodes.find(n => n.id === storeNode.id)?.y || storeNode.position.y,
            },
          }));
          saveNodePositions(currentNodes, layoutType);
          
          // Release fixed positions after a short delay to allow gentle settling
          setTimeout(() => {
            nodes.forEach(node => {
              // Only release if this node hasn't been dragged recently
              const lastDragTime = (node as any).__lastDragTime;
              if (!lastDragTime || Date.now() - lastDragTime > 5000) {
                node.fx = null;
                node.fy = null;
              }
            });
          }, 1000);
        } else {
          console.log('No cached positions, full layout restart');
          simulation.alpha(0.8).restart();
          
          // After simulation settles, save the new positions
          setTimeout(() => {
            const finalNodes = storeNodes.map(storeNode => ({
              ...storeNode,
              position: {
                x: nodes.find(n => n.id === storeNode.id)?.x || storeNode.position.x,
                y: nodes.find(n => n.id === storeNode.id)?.y || storeNode.position.y,
              },
            }));
            saveNodePositions(finalNodes, layoutType);
          }, 3000);
        }
      } else if (hasNewNodes || hasNewEdges) {
        // Data change during session - preserve existing positions, gentle restart for new elements
        simulation.alpha(0.1).restart();
        
                 // Save updated positions after brief settling
         setTimeout(() => {
           const currentNodes = storeNodes.map(storeNode => ({
             ...storeNode,
             position: {
               x: nodes.find(n => n.id === storeNode.id)?.x || storeNode.position.x,
               y: nodes.find(n => n.id === storeNode.id)?.y || storeNode.position.y,
             },
           }));
           saveNodePositions(currentNodes, layoutType);
         }, 1000);
      }
    }

    // Update on data changes
    updateVisualization();

    // Handle resize
    const handleResize = () => {
      const newWidth = container.clientWidth;
      const newHeight = container.clientHeight;
      svg.attr('width', newWidth).attr('height', newHeight);
      // Update center force to account for left panel
      simulation.force('center', d3.forceCenter(newWidth * 0.6, newHeight / 2));
      // Only animate resize if not initial load
      if (!isInitialLoadRef.current) {
        simulation.alpha(0.05).restart();
      }
    };

    window.addEventListener('resize', handleResize);

    return () => {
      window.removeEventListener('resize', handleResize);
      simulation.stop();
    };
  }, [structuralData, showLabels, colors.bg.primary, colors.edge.default, colors.node.selected, colors.text.primary]);

  // Handle selection changes without restarting simulation
  useEffect(() => {
    if (!svgRef.current) return;
    
    const svg = d3.select(svgRef.current);
    const g = svg.select('.graph-container');
    
    // Update node selection rings (double-ring effect)
    g.selectAll<SVGGElement, D3Node>('.node')
      .select('.node-selection-ring')
      .attr('r', (d: any) => {
        const selected = selectedNodes.includes(d.id);
        if (selected) {
          const radius = d.radius || 20;
          return radius + 8; // Outer ring radius
        }
        return 0;
      })
      .attr('stroke-opacity', (d: any) => {
        const selected = selectedNodes.includes(d.id);
        return selected ? 0.8 : 0;
      });

    // Update node circles - keep original colors, just adjust thickness
    g.selectAll<SVGGElement, D3Node>('.node')
      .select('.node-circle')
      .attr('stroke-width', (d: any) => {
        const selected = selectedNodes.includes(d.id);
        // Preserve semantic stroke colors, just adjust thickness
        return selected ? 4 : 3;
      });

    // Update edge selection styling with more subtle highlight
    g.selectAll<SVGLineElement, D3Link>('.link')
      .attr('stroke', (d: any) => {
        const selected = selectedEdges.includes(d.id);
        if (selected) {
          return '#60a5fa'; // Blue highlight for selected edges
        }
        return (d as any).color; // Keep original edge color
      })
      .attr('stroke-width', (d: any) => {
        const selected = selectedEdges.includes(d.id);
        return selected ? ((d as any).strokeWidth + 1) : (d as any).strokeWidth;
      })
      .attr('stroke-opacity', (d: any) => {
        const selected = selectedEdges.includes(d.id);
        return selected ? 1.0 : (d as any).opacity;
      });
  }, [selectedNodes, selectedEdges]);

  // Fit to page function - stable to prevent recreation
  const fitToPage = useCallback(() => {
    if (!svgRef.current || !containerRef.current || !zoomBehaviorRef.current) return;
    
    const svg = d3.select(svgRef.current);
    const container = containerRef.current;
    const width = container.clientWidth;
    const height = container.clientHeight;
    
    console.log('🔍 Fitting to page...');
    
    // Use the graph container to get bounds (more reliable than individual nodes)
    const g = svg.select('.graph-container');
    const bounds = (g.node() as SVGGElement)?.getBBox();
    
    if (!bounds || bounds.width <= 0 || bounds.height <= 0) {
      console.log('🔍 No valid bounds found for fit-to-page');
      return;
    }
    
    console.log('Graph bounds:', bounds);
    
    // Calculate scale to fit with padding, accounting for left panel  
    const padding = 50;
    const visibleWidth = width * 0.7; // Slightly more generous for main content area
    const visibleHeight = height;
    
    const scaleX = visibleWidth / (bounds.width + padding * 2);
    const scaleY = visibleHeight / (bounds.height + padding * 2);
    const scale = Math.min(scaleX, scaleY, 2); // Cap at 2x zoom
    
    // Calculate centers
    const graphCenterX = bounds.x + bounds.width / 2;
    const graphCenterY = bounds.y + bounds.height / 2;
    
    // Target center should be in the right part of the screen (avoiding left panel)
    const leftPanelWidth = width * 0.3; // Account for left controls
    const targetCenterX = leftPanelWidth + (width - leftPanelWidth) / 2;
    const targetCenterY = height / 2;
    
    // Calculate translation using the standard D3 approach
    const transform = d3.zoomIdentity
      .translate(targetCenterX, targetCenterY)
      .scale(scale)
      .translate(-graphCenterX, -graphCenterY);
    
    console.log('Applying transform:', { 
      scale: scale.toFixed(2), 
      graphCenter: [graphCenterX.toFixed(0), graphCenterY.toFixed(0)],
      targetCenter: [targetCenterX.toFixed(0), targetCenterY.toFixed(0)],
      finalTranslate: [transform.x.toFixed(0), transform.y.toFixed(0)]
    });
    
    // Apply transform smoothly
    svg.transition()
      .duration(750)
      .call(zoomBehaviorRef.current.transform, transform);
    
    // Save the fit transform - use current values from store
    const currentDataSource = useGraphStore.getState().dataSource;
    const currentLayoutType = useGraphStore.getState().layoutType;
    if (currentDataSource) {
      setTimeout(() => {
        saveZoomTransform(transform, currentLayoutType, currentDataSource);
      }, 800); // After transition completes
    }
    
    console.log(`🔍 Fitted to page successfully`);
  }, []);

  // Track if initial setup is complete to avoid infinite loops
  const initialSetupRef = useRef(false);

  // Auto-apply layout when layout type changes (after initial setup)
  useEffect(() => {
    if (storeNodes.length > 0 && initialSetupRef.current) {
      console.log(`Layout type changed to: ${layoutType}, auto-applying...`);
      // Preserve current zoom/pan by getting container dimensions
      const container = containerRef.current;
      if (container) {
        const width = container.clientWidth;
        const height = container.clientHeight;
        applyLayout(width, height).then(() => {
          // Fit to page after layout is applied
          setTimeout(() => {
            fitToPage();
          }, 100);
        });
      }
    }
  }, [layoutType]);

  // Initialize WebSocket and load data based on cached data source
  useEffect(() => {
    initializeWebSocket();
    
    // Load data based on cached preference if no data is present
    if (storeNodes.length === 0) {
      if (dataSource === 'server') {
        loadAllData();
      } else {
        // Default to demo data for first-time users or if demo is selected
        loadDemoData();
      }
    }
  }, [initializeWebSocket, loadDemoData, loadAllData, storeNodes.length, dataSource]);

  // One-time setup: apply initial layout when data is first loaded
  useEffect(() => {
    if (storeNodes.length > 0 && d3Data.nodes.length > 0 && !initialSetupRef.current) {
      console.log('🚀 Initial data loaded, applying initial layout...');
      initialSetupRef.current = true;
      
      // Apply initial layout with fit-to-page
      const container = containerRef.current;
      if (container) {
        const width = container.clientWidth;
        const height = container.clientHeight;
        applyLayout(width, height).then(() => {
          // Fit to page after layout is applied
          setTimeout(() => {
            fitToPage();
          }, 100);
        });
      }
    }
  }, [storeNodes.length, d3Data.nodes.length]);

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
            applyLayout(containerRef.current?.clientWidth || 800, containerRef.current?.clientHeight || 600).then(() => {
              // Fit to page after layout is applied
              setTimeout(() => {
                fitToPage();
              }, 100);
            });
          }
          break;
        case 'f':
          event.preventDefault();
          console.log('F key pressed - fitting to page');
          fitToPage();
          break;
        case 'escape':
          clearSelection();
          break;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [applyLayout, clearSelection, fitToPage]);

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
    <div className="w-full h-full relative" ref={containerRef}>
      
      {/* Connection status and data source indicator */}
      <div className={`connection-status ${
        connectionState === 'connected' ? 'status-connected' : 'status-disconnected'
      }`}>
        <div>
          {connectionState === 'connected' && '● Connected'}
          {connectionState === 'connecting' && '◐ Connecting...'}
          {connectionState === 'disconnected' && '○ Disconnected'}
          {connectionState === 'error' && '✕ Connection Error'}
          {connectionState === 'demo' && '● Demo Mode'}
        </div>
        {dataSource && (
          <div className="text-xs opacity-75 mt-1">
            Data: {dataSource === 'demo' ? 'Demo' : 'Server'}
          </div>
        )}
      </div>

      {/* Loading overlay */}
      {isLoading && (
        <div className="loading-overlay">
          <div className="loading-spinner"></div>
        </div>
      )}

      {/* Graph Controls */}
      <GraphControls />

      {/* Visual Legend */}
      <div style={{ 
        position: 'absolute', 
        top: '20px', 
        right: '20px', 
        background: colors.bg.primary, 
        border: `1px solid ${colors.border.primary}`,
        borderRadius: '8px',
        padding: '12px',
        maxWidth: '280px',
        fontSize: '12px',
        zIndex: 1000,
        boxShadow: '0 2px 8px rgba(0,0,0,0.2)'
      }}>
        <div style={{ fontWeight: 'bold', marginBottom: '8px', color: colors.text.primary }}>
          Graph Legend
        </div>
        
        <div style={{ marginBottom: '8px' }}>
          <div style={{ fontWeight: '600', color: colors.text.primary, marginBottom: '4px' }}>Node Size:</div>
          <div style={{ color: colors.text.muted, fontSize: '11px', lineHeight: '1.4' }}>
            Based on semantic connections (references, mentions, relates_to). Temporal connections count less.
          </div>
        </div>
        
        <div style={{ marginBottom: '8px' }}>
          <div style={{ fontWeight: '600', color: colors.text.primary, marginBottom: '4px' }}>Edge Types:</div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <div style={{ 
                width: '20px', 
                height: '2px', 
                background: '#8b5cf6', 
                opacity: 0.8 
              }}></div>
              <span style={{ color: colors.text.muted, fontSize: '11px' }}>Semantic (references, mentions)</span>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <div style={{ 
                width: '20px', 
                height: '2px', 
                background: '#6b7280', 
                opacity: 0.3,
                backgroundImage: 'repeating-linear-gradient(90deg, transparent, transparent 2px, #6b7280 2px, #6b7280 4px)'
              }}></div>
              <span style={{ color: colors.text.muted, fontSize: '11px' }}>Temporal (sequence, follows)</span>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <div style={{ 
                width: '20px', 
                height: '2px', 
                background: colors.edge.default, 
                opacity: 0.6 
              }}></div>
              <span style={{ color: colors.text.muted, fontSize: '11px' }}>Other relationships</span>
            </div>
          </div>
        </div>
        
        <div>
          <div style={{ fontWeight: '600', color: colors.text.primary, marginBottom: '4px' }}>Node Colors:</div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <div style={{ 
                width: '12px', 
                height: '12px', 
                border: '2px solid #60a5fa', 
                borderRadius: '50%',
                background: 'transparent'
              }}></div>
              <span style={{ color: colors.text.muted, fontSize: '11px' }}>Fact memories</span>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <div style={{ 
                width: '12px', 
                height: '12px', 
                border: '2px solid #4ade80', 
                borderRadius: '50%',
                background: 'transparent'
              }}></div>
              <span style={{ color: colors.text.muted, fontSize: '11px' }}>Episodic memories</span>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <div style={{ 
                width: '12px', 
                height: '12px', 
                border: '2px solid #c084fc', 
                borderRadius: '50%',
                background: 'transparent'
              }}></div>
              <span style={{ color: colors.text.muted, fontSize: '11px' }}>Semantic memories</span>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <div style={{ 
                width: '12px', 
                height: '12px', 
                border: '2px solid #fbbf24', 
                borderRadius: '50%',
                background: 'transparent'
              }}></div>
              <span style={{ color: colors.text.muted, fontSize: '11px' }}>Entities</span>
            </div>
          </div>
        </div>
      </div>

      {/* Analytics Panel */}
      <AnalyticsPanel />

      {/* Realtime Event Feed */}
      <div style={{ position: 'absolute', bottom: '16px', right: '16px', width: '300px', zIndex: 1000 }}>
        <RealtimeEventFeed />
      </div>
      
      {/* Node Details Panel */}
      <NodeDetailsPanel />

      {/* D3 SVG */}
      <svg 
        ref={svgRef}
        className="w-full h-full"
        style={{ 
          position: 'absolute', 
          top: 0, 
          left: 0,
          backgroundColor: colors.bg.secondary,
          zIndex: 1, // Behind other panels
        }}
      />

      {/* Toast Notifications */}
      <ToastContainer toasts={toasts} onRemoveToast={removeToast} />
    </div>
  );
};

export default D3GraphVisualization; 