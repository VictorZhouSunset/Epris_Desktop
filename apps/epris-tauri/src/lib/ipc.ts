import { invoke } from '@tauri-apps/api/core';

export const TAURI_COMMANDS = {
  GET_WORKSPACE_CONFIG: 'get_workspace_config',
  GET_PROJECTS_OVERVIEW: 'get_projects_overview',
  GET_ACTIVE_PROJECT_CONFIG: 'get_active_project_config',
  GET_DEFAULT_PROJECTS_ROOT: 'get_default_projects_root',
  PICK_PROJECTS_ROOT: 'pick_projects_root',
  SET_PROJECTS_ROOT: 'set_projects_root',
  CREATE_PROJECT: 'create_project',
  SET_ACTIVE_PROJECT: 'set_active_project',
  DELETE_PROJECT: 'delete_project',
  RENAME_PROJECT: 'rename_project',
  START_PREVIEW_SERVER: 'start_preview_server',
  STOP_PREVIEW_SERVER: 'stop_preview_server',
  START_OPENCODE: 'start_opencode',
  STOP_OPENCODE: 'stop_opencode',
  CANCEL_CURRENT_RUN: 'cancel_current_run',
  SEND_PROMPT: 'send_prompt',
  CLEAR_SESSION: 'clear_session',
  RUN_GATE: 'run_gate',
  STOP_GATE_VALIDATION: 'stop_gate_validation',
  STOP_PROVIDER_CLI: 'stop_provider_cli',
  EXPORT_VIDEO: 'export_video',
  DELETE_SNAPSHOT_TREE: 'delete_snapshot_tree',
  GET_DAG_HEAD: 'get_dag_head',
  SAVE_DAG_HEAD: 'save_dag_head',
  AUTO_SAVE_SNAPSHOT: 'auto_save_snapshot',
  AUTO_SAVE_CHECKPOINT: 'auto_save_checkpoint',
  MANUAL_SAVE_SNAPSHOT: 'manual_save_snapshot',
  CHECKOUT_SNAPSHOT: 'checkout_snapshot',
  GET_SNAPSHOT_DAG: 'get_snapshot_dag',
  LIST_SNAPSHOTS: 'list_snapshots',
  UPDATE_SNAPSHOT_METADATA: 'update_snapshot_metadata',
  GET_GATE_HISTORY: 'get_gate_history',
  CHECK_UNSAVED_CHANGES: 'check_unsaved_changes',
  CLEAR_SNAPSHOT_HISTORY: 'clear_snapshot_history',
  SAVE_SNAPSHOT_LAYOUT: 'save_snapshot_layout',
  GREET: 'greet',
  GET_GEMINI_AUTH_STATUS: 'get_gemini_auth_status',
  OPEN_GEMINI_LOGIN: 'open_gemini_login',
  SET_GEMINI_API_KEY: 'set_gemini_api_key',
  GET_ENVIRONMENT_STATUS: 'get_environment_status',
  INSTALL_MISSING_DEPENDENCIES: 'install_missing_dependencies',
  RESET_USER_DATA: 'reset_user_data',
  GET_VIDEO_CONFIG: 'get_video_config',
  SET_VIDEO_CONFIG: 'set_video_config',
  LIST_ASSETS: 'list_assets',
  UPLOAD_ASSET: 'upload_asset',
  DELETE_ASSET: 'delete_asset',
  RENAME_ASSET: 'rename_asset',
  GET_STT_STATUS: 'get_stt_status',
  INSTALL_WHISPERCPP: 'install_whispercpp',
  TRANSCRIBE_WHISPERCPP: 'transcribe_whispercpp',
  CANCEL_STT: 'cancel_stt',
} as const;

export type CommandKey = keyof typeof TAURI_COMMANDS;

export class IpcService {
  /**
   * Strictly typed invoke wrapper.
   * Leverages TS Literal Types via CommandKey for safety.
   */
  static async call<T>(key: CommandKey, args?: Record<string, unknown>): Promise<T> {
    const command = TAURI_COMMANDS[key];
    console.log(`[Epris IPC] Calling ${command}`, args);
    try {
      const result = await invoke<T>(command, args);
      console.log(`[Epris IPC] Success ${command}`, result);
      return result;
    } catch (err) {
      console.error(`[Epris IPC] Error ${command}`, err);
      throw err;
    }
  }
}
