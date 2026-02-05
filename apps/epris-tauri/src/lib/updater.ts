import { check, type Update } from '@tauri-apps/plugin-updater';
import { invoke } from '@tauri-apps/api/core';

export type UpdatePhase =
  | 'idle'
  | 'checking'
  | 'available'
  | 'downloading'
  | 'installing'
  | 'done'
  | 'error';

export async function appendUpdaterLog(line: string): Promise<void> {
  try {
    await invoke('append_updater_log', { line });
  } catch {
    // Best-effort logging: ignore failures (e.g. command not available in dev).
  }
}

export async function checkForUpdate(): Promise<Update | null> {
  await appendUpdaterLog('check: start');
  const update = await check();
  if (!update) {
    await appendUpdaterLog('check: none');
    return null;
  }
  try {
    const anyUpdate = update as any;
    const keys = Object.keys(anyUpdate || {}).sort();
    await appendUpdaterLog(`check: update keys -> ${keys.join(', ')}`);
    for (const k of ['downloadUrl', 'url', 'path', 'signature', 'date']) {
      const v = anyUpdate?.[k];
      if (v) await appendUpdaterLog(`check: ${k} -> ${String(v)}`);
    }
  } catch {
    // ignore
  }
  await appendUpdaterLog(`check: available -> ${update.version} (current ${update.currentVersion})`);
  return update;
}

export async function downloadAndInstall(update: Update): Promise<void> {
  await appendUpdaterLog(`downloadAndInstall: start -> ${update.version}`);
  await update.downloadAndInstall();
  await appendUpdaterLog(`downloadAndInstall: done -> ${update.version}`);
}
