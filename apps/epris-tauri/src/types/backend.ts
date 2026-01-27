export interface GateResult {
  success: boolean;           // Legacy
  passed: boolean;            // New
  typecheck_passed: boolean;  // Legacy
  typecheck: boolean;         // New
  smoke_0_passed: boolean;    // Legacy
  smoke_0: boolean;           // New
  smoke_mid_passed: boolean;  // Legacy
  smoke_mid: boolean;         // New
  error_output: string;       // Legacy
  error?: string;            // New
  attempts: number;
}

export interface PromptResponse {
  success: boolean;
  message: string;
  gate_result?: GateResult;
  snapshot_id?: string;
  snapshot_timestamp?: string;
}

export interface SnapshotMetadata {
  id: string;
  name: string;
  description: string;
  timestamp: string;
  parent_id?: string;
  is_manual: boolean;
  prompt?: string;
  session_id: string;
  gate_result?: GateResult;
}

export interface Edge {
  from: string;
  to: string;
}

export interface NodePosition {
  x: number;
  y: number;
}

export interface SnapshotDAG {
  nodes: SnapshotMetadata[];
  edges: Edge[];
  layout: Record<string, NodePosition>;
}

export interface ExportResult {
  success: boolean;
  output_path?: string;
  error?: string;
}
