import { 
  X, 
  CheckCircle2, 
  XCircle, 
  Clock, 
  Trash2, 
  ArrowRight, 
  GitBranch, 
  History, 
  Edit2, 
  Plus
} from 'lucide-react';
import { useSnapshot } from '../hooks/useSnapshot';
import { useCallback, useEffect, useState } from 'react';
import { 
  ReactFlow, 
  Background, 
  Controls, 
  Node, 
  Edge as FlowEdge, 
  Handle, 
  Position,
  NodeProps,
  Panel,
  useNodesState,
  useEdgesState
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';

// Rename local Edge to avoid naming conflict with @xyflow/react's Edge
// import { Edge as BackendEdge } from '../types/backend';

// --- Custom Node Component ---

type SnapshotNodeData = { 
  name: string; 
  id: string; 
  timestamp: string; 
  prompt?: string;
  isHead: boolean;
  isManual: boolean;
  gatePassed?: boolean;
  onCheckout: (id: string) => void;
  onDelete: (id: string) => void;
  onEdit: (data: any) => void;
};

function SnapshotNode({ data }: NodeProps<Node<SnapshotNodeData>>) {
  const date = new Date(data.timestamp).toLocaleTimeString();
  
  return (
    <div className={`group min-w-[300px] max-w-[400px] p-4 rounded-xl border transition-all shadow-lg
      ${data.isHead ? 'bg-indigo-950/40 border-indigo-500 ring-2 ring-indigo-500/20' : 'bg-slate-900 border-slate-700 hover:border-slate-500'}`}>
      
      {/* Target: Input from Parent (Bottom, because time flows up) */}
      <Handle type="target" position={Position.Bottom} className="w-2 h-2 !bg-slate-700 border-none" />
      
      <div className="flex flex-col gap-1">
        <div className="flex items-center justify-between gap-2">
           <div className="flex items-center gap-1.5 overflow-hidden">
             <span className="font-bold text-slate-100 truncate text-sm">{data.name}</span>
             {data.isHead && (
                <span className="px-1.5 py-0.5 bg-indigo-500 text-white text-[8px] font-black rounded uppercase">HEAD</span>
             )}
             {data.gatePassed !== undefined && (
                data.gatePassed ? <CheckCircle2 size={12} className="text-emerald-400 shrink-0" /> : <XCircle size={12} className="text-amber-400 shrink-0" />
             )}
           </div>
           <span className="text-[9px] text-slate-500 font-mono shrink-0">{data.id.substring(0, 6)}</span>
        </div>

        {data.prompt && (
          <div className="mt-2">
            <span className="text-[9px] text-slate-500 uppercase tracking-wider font-bold block mb-0.5">Next Action</span>
            <p className="text-[10px] text-slate-400 italic line-clamp-2 bg-slate-950/50 px-2 py-1 rounded border border-slate-800/30">
              "{data.prompt}"
            </p>
          </div>
        )}

        <div className="flex items-center justify-between mt-3">
          <div className="flex items-center gap-1 text-[9px] text-slate-500">
            <Clock size={10} />
            {date}
          </div>
          
          <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
             <button 
                onClick={(e) => { e.stopPropagation(); data.onCheckout(data.id); }}
                className="p-1 px-2 bg-indigo-600 hover:bg-indigo-500 text-white text-[9px] rounded flex items-center gap-1"
                disabled={data.isHead}
             >
               <ArrowRight size={8} /> Restore
             </button>
             <button 
                onClick={(e) => { e.stopPropagation(); data.onDelete(data.id); }}
                className="p-1 hover:bg-red-500/20 text-slate-500 hover:text-red-400 rounded transition-colors"
                title="Delete recursively"
             >
               <Trash2 size={12} />
             </button>
             <button 
                onClick={(e) => { e.stopPropagation(); data.onEdit(data); }}
                className="p-1 hover:bg-slate-700 text-slate-400 hover:text-slate-200 rounded transition-colors"
             >
               <Edit2 size={12} />
             </button>
          </div>
        </div>
      </div>

      {/* Source: Output to Child (Top, because time flows up) */}
      <Handle type="source" position={Position.Top} className="w-2 h-2 !bg-slate-700 border-none" />
    </div>
  );
}

function CurrentStateNode({ data }: NodeProps<Node<{ label: string; isDirty: boolean }>>) {
  return (
    <div className={`relative flex items-center justify-center w-16 h-16 rounded-full border-4 shadow-[0_0_30px_rgba(139,92,246,0.3)] transition-all duration-500
      ${data.isDirty 
        ? 'bg-purple-600 border-purple-300 animate-pulse' 
        : 'bg-indigo-950 border-indigo-700'
      }`}>
      
      {/* Input from Head (Bottom) */}
      <Handle type="target" position={Position.Bottom} className="w-0 h-0 opacity-0" />
      
      <div className="text-[10px] font-black text-white text-center leading-tight uppercase tracking-widest">
        {data.isDirty ? 'Unsaved' : 'Clean'}
      </div>
    </div>
  );
}

const nodeTypes = {
  snapshot: SnapshotNode,
  currentState: CurrentStateNode,
};

// --- Main History Component ---

interface SnapshotHistoryProps {
  onClose: () => void;
  workspacePath: string | undefined;
  onReloadPreview: () => void;
  hasUnsavedChanges: boolean;
}

export function SnapshotHistory({ onClose, workspacePath, onReloadPreview, hasUnsavedChanges }: SnapshotHistoryProps) {
  const { dag, loading, checkoutSnapshot, deleteSnapshotTree, getHead, manualSaveSnapshot, updateMetadata, clearHistory, saveSnapshotLayout } = useSnapshot(workspacePath, onReloadPreview);
  const [headId, setHeadId] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  
  // React Flow state management for smooth dragging
  const [nodes, setNodes, onNodesChange] = useNodesState<Node>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<FlowEdge>([]);
  
  // Metadata Edit state
  const [editingNode, setEditingNode] = useState<any>(null);
  const [editName, setEditName] = useState('');
  const [editDesc, setEditDesc] = useState('');

  useEffect(() => {
    getHead().then(setHeadId);
  }, [getHead, dag]);

  const handleCheckout = useCallback(async (id: string) => {
    try { await checkoutSnapshot(id); onClose(); } catch (err) { alert('Checkout failed: ' + err); }
  }, [checkoutSnapshot, onClose]);

  const handleDelete = useCallback(async (id: string) => {
    if (!confirm('Are you sure? This will delete this snapshot and ALL its descendants.')) return;
    try { await deleteSnapshotTree(id); } catch (err) { alert('Delete failed: ' + err); }
  }, [deleteSnapshotTree]);

  const handleManualSave = useCallback(async () => {
    if (!workspacePath) return;
    const name = prompt('Enter snapshot name:', `Manual ${new Date().toLocaleTimeString()}`);
    if (!name) return;
    const desc = prompt('Enter description (optional):', '');
    try {
      setIsSaving(true);
      const currentHead = await getHead();
      await manualSaveSnapshot(name, desc || '', currentHead);
    } catch (err) { alert('Save failed: ' + err); } finally { setIsSaving(false); }
  }, [workspacePath, manualSaveSnapshot, getHead]);

  const handleUpdateMetadata = async () => {
    if (!editingNode) return;
    try {
      await updateMetadata(editingNode.id, editName, editDesc);
      setEditingNode(null);
    } catch (err) { alert('Update failed: ' + err); }
  };

  const onNodeDragStop = useCallback((_event: any, node: Node) => {
    if (node.id === 'current-state') return;
    saveSnapshotLayout(node.id, node.position.x, node.position.y);
  }, [saveSnapshotLayout]);

  // --- Graph Construction & Sync ---
  
  useEffect(() => {
    if (!dag) return;

    const flowNodes: Node[] = [];
    const flowEdges: FlowEdge[] = [];

    // Simple Grid Layout Algorithm (Horizontal)
    // 1. Group nodes by depth
    const parentMap = new Map<string, string>();
    dag.edges.forEach(e => parentMap.set(e.to, e.from));

    const getDepth = (id: string): number => {
      let depth = 0;
      let curr: string | undefined = id;
      while (curr && curr !== 'root') {
        curr = parentMap.get(curr);
        depth++;
      }
      return depth;
    };

    // Calculate depths and sort
    const nodesWithDepth = dag.nodes.map(n => ({ ...n, depth: getDepth(n.id) }));
    const depthGroups = new Map<number, typeof nodesWithDepth>();
    
    nodesWithDepth.forEach(n => {
      if (!depthGroups.has(n.depth)) depthGroups.set(n.depth, []);
      depthGroups.get(n.depth)!.push(n);
    });

    // Create graph nodes
    nodesWithDepth.forEach((node) => {
      const idxInGroup = depthGroups.get(node.depth)!.findIndex(n => n.id === node.id);
      const groupSize = depthGroups.get(node.depth)!.length;
      
      // Vertical Layout: Time moves UP (Bottom -> Top)
      const verticalSpacing = 250;
      const horizontalSpacing = 450;
      const xOffset = (idxInGroup - (groupSize - 1) / 2) * horizontalSpacing;
      const yOffset = -node.depth * verticalSpacing;

      // Use stored layout if available
      const position = dag.layout[node.id] || { x: xOffset, y: yOffset };

      flowNodes.push({
        id: node.id,
        type: 'snapshot',
        data: { 
          ...node,
          isManual: node.is_manual,
          isHead: node.id === headId,
          onCheckout: handleCheckout,
          onDelete: handleDelete,
          onEdit: (data: any) => {
            setEditingNode(data);
            setEditName(data.name);
            setEditDesc(data.description || '');
          }
        } as SnapshotNodeData,
        position,
      });
    });

    // --- Current State Node Integration ---
    if (headId) {
      const headNode = flowNodes.find(n => n.id === headId);
      if (headNode) {
        // Calculate position
        const baseX = headNode.position.x;
        const baseY = headNode.position.y;
        
        // If dirty, float up. If clean, sit tight above.
        const floatOffset = hasUnsavedChanges ? 200 : 80;
        
        flowNodes.push({
          id: 'current-state',
          type: 'currentState',
          data: { 
            label: hasUnsavedChanges ? 'Unsaved' : 'Clean',
            isDirty: hasUnsavedChanges,
          },
          position: { x: baseX + 132, y: baseY - floatOffset },
          draggable: false,
        });

        flowEdges.push({
          id: 'e-current',
          source: headId,
          target: 'current-state',
          type: 'straight',
          animated: true,
          style: { stroke: '#8b5cf6', strokeDasharray: hasUnsavedChanges ? '5 5' : '0' },
        });
      }
    }

    dag.edges.forEach((edge, idx) => {
      if (edge.from === 'root') return;
      flowEdges.push({
        id: `e-${idx}`,
        source: edge.from,
        target: edge.to,
        type: 'smoothstep',
        animated: edge.to === headId,
        style: { stroke: edge.to === headId ? '#6366f1' : '#334155', strokeWidth: 2 },
      });
    });

    setNodes(flowNodes);
    setEdges(flowEdges);
  }, [dag, headId, hasUnsavedChanges, handleCheckout, handleDelete, setNodes, setEdges]);

  if (!workspacePath) return null;

  return (
    <div className="fixed inset-0 z-50 flex flex-col bg-slate-950">
      {/* Header Overlay Style */}
      <header className="px-6 py-4 border-b border-white/5 bg-slate-900/80 backdrop-blur flex items-center justify-between absolute top-0 left-0 right-0 z-50 shadow-2xl">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-xl bg-indigo-500/20 flex items-center justify-center text-indigo-400">
             <History size={24} />
          </div>
          <div>
            <h2 className="text-xl font-bold text-slate-100 flex items-center gap-2">
              Snapshot DAG
              <span className="text-[10px] bg-white/10 px-2 py-0.5 rounded-full font-mono text-slate-400 font-normal">
                {dag?.nodes.length || 0} versions
              </span>
            </h2>
            <p className="text-xs text-slate-500">Visual branching history of your animations</p>
          </div>
        </div>

        <div className="flex items-center gap-3">
          <button 
            onClick={handleManualSave}
            disabled={isSaving}
            className="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 text-white rounded-xl text-sm font-bold flex items-center gap-2 transition-all shadow-lg shadow-indigo-500/20 active:scale-95"
          >
            <Plus size={18} strokeWidth={3} />
            Save Current State
          </button>
          <button onClick={onClose} className="p-2.5 hover:bg-white/10 rounded-xl text-slate-400 hover:text-white transition-colors">
            <X size={24} />
          </button>
        </div>
      </header>

      {/* Toolbar */}
      <div className="absolute top-24 right-6 z-40 flex flex-col gap-2">
         <button 
            onClick={() => {
               if(confirm('Are you sure you want to clear the entire history? This cannot be undone.')) {
                 clearHistory();
               }
            }}
            className="p-2 bg-slate-900/80 border border-red-500/30 text-red-400 hover:bg-red-500/10 hover:text-red-300 rounded-lg backdrop-blur flex items-center justify-center transition-all"
            title="Clear History"
         >
            <Trash2 size={16} />
         </button>
      </div>

      {/* React Flow Canvas */}
      <div className="flex-1 w-full h-full pt-20">
        {loading && !dag ? (
           <div className="absolute inset-0 z-10 flex items-center justify-center bg-slate-950/50 backdrop-blur-sm">
             <div className="flex flex-col items-center gap-4">
                <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-indigo-500"></div>
                <span className="text-sm font-medium text-slate-400">Building Version Graph...</span>
             </div>
           </div>
        ) : (
          <ReactFlow
            nodes={nodes}
            edges={edges}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            nodeTypes={nodeTypes}
            onNodeDragStop={onNodeDragStop}
            snapToGrid={true}
            snapGrid={[20, 20]}
            fitView
            fitViewOptions={{ padding: 0.2 }}
            colorMode="dark"
            className="bg-slate-950"
          >
            <Background color="#1e293b" gap={20} size={1} />
            <Controls className="!bg-slate-900 !border-slate-800 !fill-slate-300" />
            <Panel position="bottom-right" className="bg-slate-900/80 p-3 rounded-xl border border-slate-800 backdrop-blur-md text-xs text-slate-400 flex flex-col gap-2">
               <div className="flex items-center gap-2"><div className="w-2 h-2 rounded-full bg-indigo-500" /> Current Work (HEAD)</div>
               <div className="flex items-center gap-2"><div className="w-2 h-2 rounded-full bg-slate-700" /> Saved Snapshots</div>
               <div className="text-[10px] opacity-60 mt-1 border-t border-slate-800 pt-2 italic">Drag background to pan; Roll to zoom</div>
            </Panel>
          </ReactFlow>
        )}
      </div>

      {/* Metadata Edit Modal */}
      {editingNode && (
        <div className="fixed inset-0 z-[60] flex items-center justify-center p-4">
          <div className="absolute inset-0 bg-slate-950/80 backdrop-blur-md" onClick={() => setEditingNode(null)} />
          <div className="relative w-full max-w-md bg-slate-900 border border-slate-700 rounded-3xl p-8 shadow-3xl">
             <h3 className="text-xl font-bold mb-6 flex items-center gap-2">
               <Edit2 size={20} className="text-indigo-400" />
               Edit Snapshot Details
             </h3>
             
             <div className="space-y-5">
               <div>
                 <label className="block text-[10px] uppercase font-black text-slate-500 mb-2 tracking-widest">Snapshot Name</label>
                 <input
                    value={editName}
                    onChange={e => setEditName(e.target.value)}
                    className="w-full bg-slate-950 border border-slate-700 rounded-xl px-4 py-3 text-slate-100 outline-none focus:border-indigo-500 transition-colors"
                    placeholder="e.g. Added Particle System"
                 />
               </div>
               <div>
                 <label className="block text-[10px] uppercase font-black text-slate-500 mb-2 tracking-widest">Description</label>
                 <textarea
                    value={editDesc}
                    onChange={e => setEditDesc(e.target.value)}
                    className="w-full bg-slate-950 border border-slate-700 rounded-xl px-4 py-3 text-slate-100 outline-none focus:border-indigo-500 transition-colors h-32 resize-none"
                    placeholder="Optional details about this version..."
                 />
               </div>
             </div>

             <div className="flex gap-3 mt-8">
               <button 
                  onClick={handleUpdateMetadata}
                  className="flex-1 py-3 bg-indigo-600 hover:bg-indigo-500 text-white rounded-xl font-bold transition-all shadow-lg shadow-indigo-500/10"
               >
                 Save Changes
               </button>
               <button 
                  onClick={() => setEditingNode(null)}
                  className="flex-1 py-3 bg-slate-800 hover:bg-slate-700 text-slate-300 rounded-xl font-bold transition-colors"
               >
                 Cancel
               </button>
             </div>
          </div>
        </div>
      )}

      {/* Empty State */}
      {dag?.nodes.length === 0 && !loading && (
        <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
           <div className="text-center opacity-40">
              <GitBranch size={64} className="mx-auto mb-4" />
              <p className="text-lg">Your graph will grow as you modify</p>
           </div>
        </div>
      )}
    </div>
  );
}
