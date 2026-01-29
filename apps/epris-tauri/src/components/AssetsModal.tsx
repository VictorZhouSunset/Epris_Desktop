import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { X, Upload, Trash2, Pencil, FolderOpen, Copy, Check, Music, Film, File, FileText, Image as ImageIcon } from 'lucide-react';
import { IpcService } from '../lib/ipc';
import type { AssetInfo } from '../types/backend';
import { openPath } from '@tauri-apps/plugin-opener';
import { convertFileSrc } from '@tauri-apps/api/core';

interface AssetsModalProps {
  workspacePath: string;
  previewUrl?: string;
  onClose: () => void;
}

function formatBytes(n: number) {
  if (!Number.isFinite(n)) return '';
  if (n < 1024) return `${n} B`;
  const kb = n / 1024;
  if (kb < 1024) return `${kb.toFixed(1)} KB`;
  const mb = kb / 1024;
  return `${mb.toFixed(1)} MB`;
}

function base64FromBytes(bytes: Uint8Array): string {
  let binary = '';
  for (let i = 0; i < bytes.length; i++) binary += String.fromCharCode(bytes[i]);
  return btoa(binary);
}

function WaveformPlaceholder({ label }: { label: string }) {
  return (
    <div className="w-full h-full flex items-center justify-center" aria-label={`Waveform ${label}`}>
      <svg viewBox="0 0 64 64" className="w-10 h-10 text-slate-400" fill="none">
        <path
          d="M8 36c4 0 4-12 8-12s4 24 8 24 4-32 8-32 4 40 8 40 4-28 8-28 4 16 8 16"
          stroke="currentColor"
          strokeWidth="4"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
    </div>
  );
}

export function AssetsModal({ workspacePath, previewUrl, onClose }: AssetsModalProps) {
  const [assets, setAssets] = useState<AssetInfo[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  const load = useCallback(async () => {
    setError(null);
    const items = await IpcService.call<AssetInfo[]>('LIST_ASSETS', { workspacePath });
    setAssets(items);
  }, [workspacePath]);

  useEffect(() => {
    void load();
  }, [load]);

  const assetsFolderPath = useMemo(() => `${workspacePath}\\public\\assets`, [workspacePath]);
  const pathSep = useMemo(() => (workspacePath.includes('\\') ? '\\' : '/'), [workspacePath]);

  const absAssetPath = useCallback(
    (relPath: string) => {
      const rel = relPath.replace(/\\/g, '/');
      const parts = ['public', ...rel.split('/').filter(Boolean)];
      return [workspacePath.replace(/[\\/]+$/g, ''), ...parts].join(pathSep);
    },
    [pathSep, workspacePath],
  );

  const assetUrl = useCallback(
    (relPath: string) => {
      // Prefer serving via the workspace preview server (http://127.0.0.1:PORT/assets/...)
      // because it avoids file protocol / CSP issues in the WebView.
      const base = (previewUrl || '').trim();
      if (base) {
        return `${base.replace(/\/+$/g, '')}/${relPath.replace(/^\/+/, '')}`;
      }
      // Fallback to local file URL for the asset on disk.
      return convertFileSrc(absAssetPath(relPath));
    },
    [absAssetPath, previewUrl],
  );

  const classifyAsset = useCallback((name: string) => {
    const ext = name.split('.').pop()?.toLowerCase() || '';
    const image = new Set(['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp', 'svg', 'avif']);
    const video = new Set(['mp4', 'webm', 'mov', 'mkv', 'avi']);
    const audio = new Set(['mp3', 'wav', 'm4a', 'aac', 'ogg', 'flac']);
    const captions = new Set(['srt', 'vtt', 'ass', 'ssa']);
    if (image.has(ext)) return 'image' as const;
    if (video.has(ext)) return 'video' as const;
    if (audio.has(ext)) return 'audio' as const;
    if (captions.has(ext)) return 'captions' as const;
    return 'file' as const;
  }, []);

  useEffect(() => {
    if (!copied) return;
    const t = window.setTimeout(() => setCopied(null), 1200);
    return () => window.clearTimeout(t);
  }, [copied]);

  const handlePickFiles = useCallback(() => {
    fileInputRef.current?.click();
  }, []);

  const handleUploadFiles = useCallback(
    async (files: FileList | null) => {
      if (!files || files.length === 0) return;
      setBusy(true);
      setError(null);
      try {
        for (const file of Array.from(files)) {
          const buf = await file.arrayBuffer();
          const bytes = new Uint8Array(buf);
          await IpcService.call<AssetInfo>('UPLOAD_ASSET', {
            workspacePath,
            filename: file.name,
            bytesBase64: base64FromBytes(bytes),
          });
        }
        await load();
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        setBusy(false);
        if (fileInputRef.current) fileInputRef.current.value = '';
      }
    },
    [load, workspacePath],
  );

  const handleDelete = useCallback(
    async (a: AssetInfo) => {
      if (!confirm(`Delete "${a.name}"?`)) return;
      setBusy(true);
      setError(null);
      try {
        await IpcService.call<void>('DELETE_ASSET', { workspacePath, relPath: a.rel_path });
        await load();
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        setBusy(false);
      }
    },
    [load, workspacePath],
  );

  const handleRename = useCallback(
    async (a: AssetInfo) => {
      const next = prompt('Rename asset:', a.name);
      if (!next) return;
      setBusy(true);
      setError(null);
      try {
        await IpcService.call<AssetInfo>('RENAME_ASSET', {
          workspacePath,
          relPath: a.rel_path,
          newName: next,
        });
        await load();
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        setBusy(false);
      }
    },
    [load, workspacePath],
  );

  const handleCopyPath = useCallback(async (a: AssetInfo) => {
    try {
      await navigator.clipboard.writeText(a.rel_path);
      setCopied(a.rel_path);
    } catch {
      try {
        // Fallback for older WebView/permissions.
        const el = document.createElement('textarea');
        el.value = a.rel_path;
        el.style.position = 'fixed';
        el.style.left = '-9999px';
        document.body.appendChild(el);
        el.focus();
        el.select();
        document.execCommand('copy');
        document.body.removeChild(el);
        setCopied(a.rel_path);
      } catch {}
    }
  }, []);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/80 backdrop-blur-sm p-4">
      <div className="bg-slate-900 border border-slate-700 rounded-2xl shadow-2xl w-full max-w-2xl overflow-hidden flex flex-col max-h-[90vh]">
        <div className="p-5 border-b border-slate-800 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl bg-indigo-600/20 border border-indigo-500/20 flex items-center justify-center">
              <Upload size={18} className="text-indigo-300" />
            </div>
            <div>
              <div className="text-white font-black">Project Assets</div>
              <div className="text-xs text-slate-500">Stored in `public/assets/` (use `staticFile('assets/...')`)</div>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-2 rounded-xl hover:bg-slate-800 text-slate-400 hover:text-slate-200 transition-colors"
            title="Close"
          >
            <X size={18} />
          </button>
        </div>

        <div className="p-5 flex-1 overflow-y-auto">
          <div className="flex items-center justify-between gap-3 mb-4">
            <div className="text-sm text-slate-300">
              {assets.length} file{assets.length === 1 ? '' : 's'}
            </div>
            <div className="flex items-center gap-2">
              <button
                onClick={async () => {
                  try {
                    await openPath(assetsFolderPath);
                  } catch {}
                }}
                className="px-3 py-2 rounded-xl bg-slate-800/50 border border-slate-700 text-slate-200 hover:bg-slate-800 transition-colors flex items-center gap-2 text-sm"
                title="Open assets folder"
              >
                <FolderOpen size={16} className="text-slate-300" />
                Open Folder
              </button>
              <button
                onClick={handlePickFiles}
                disabled={busy}
                className="px-3 py-2 rounded-xl bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 text-white transition-colors flex items-center gap-2 text-sm font-bold"
                title="Upload files"
              >
                <Upload size={16} />
                Upload
              </button>
              <input
                ref={fileInputRef}
                type="file"
                multiple
                className="hidden"
                onChange={(e) => void handleUploadFiles(e.target.files)}
              />
            </div>
          </div>

          {error && (
            <div className="mb-4 p-3 bg-red-500/10 border border-red-500/20 rounded-xl text-red-400 text-sm">
              {error}
            </div>
          )}

            <div className="space-y-2">
              {assets.length === 0 ? (
                <div className="p-6 bg-slate-950/40 border border-slate-800 rounded-2xl text-slate-500 text-sm">
                  No assets yet. Upload images, audio, or any files you want to reference from the Remotion code.
                </div>
              ) : (
                assets.map((a) => (
                  <div
                    key={a.rel_path}
                    className="p-3 rounded-2xl bg-slate-950/30 border border-slate-800 flex items-center justify-between gap-3"
                  >
                    <div className="min-w-0 flex items-center gap-3">
                      <button
                        type="button"
                        onClick={async () => {
                          try {
                            await openPath(absAssetPath(a.rel_path));
                          } catch {}
                        }}
                        className="w-[88px] h-[88px] rounded-2xl bg-slate-900 border border-slate-800 overflow-hidden flex items-center justify-center shrink-0 hover:border-slate-700 transition-colors"
                        title="Open file"
                      >
                        {(() => {
                          const kind = classifyAsset(a.name);
                          const src = assetUrl(a.rel_path);
                          const fallbackIcon =
                            kind === 'image' ? (
                              <ImageIcon size={18} className="text-slate-400" />
                            ) : kind === 'video' ? (
                              <Film size={18} className="text-slate-400" />
                            ) : kind === 'audio' ? (
                              <Music size={18} className="text-slate-400" />
                            ) : kind === 'captions' ? (
                              <FileText size={18} className="text-slate-400" />
                            ) : (
                              <File size={18} className="text-slate-400" />
                            );

                          if (kind === 'image') {
                            return (
                              <div className="relative w-full h-full flex items-center justify-center">
                                {fallbackIcon}
                                <img
                                  src={src}
                                  alt={a.name}
                                  className="absolute inset-0 w-full h-full object-cover"
                                  onError={(e) => {
                                    (e.currentTarget as HTMLImageElement).style.display = 'none';
                                  }}
                                />
                              </div>
                            );
                          }
                          if (kind === 'video') {
                            return (
                              <div className="relative w-full h-full flex items-center justify-center">
                                {fallbackIcon}
                                <video
                                  src={src}
                                  className="absolute inset-0 w-full h-full object-cover"
                                  muted
                                  playsInline
                                  preload="metadata"
                                  onError={(e) => {
                                    (e.currentTarget as HTMLVideoElement).style.display = 'none';
                                  }}
                                />
                              </div>
                            );
                          }
                          if (kind === 'audio') {
                            return <WaveformPlaceholder label={a.rel_path} />;
                          }
                          return fallbackIcon;
                        })()}
                      </button>
                      <div className="min-w-0">
                        <div className="text-slate-200 font-bold truncate">{a.name}</div>
                        <div className="text-[11px] text-slate-500 truncate">
                          {a.rel_path} • {formatBytes(a.size_bytes)}
                        </div>
                      </div>
                    </div>
                    <div className="flex items-center gap-1">
                      <button
                        onClick={() => void handleCopyPath(a)}
                        disabled={busy}
                        className="p-2 rounded-xl hover:bg-slate-800 text-slate-500 hover:text-slate-200 transition-colors disabled:opacity-50"
                        title="Copy relative path (assets/...)"
                      >
                        {copied === a.rel_path ? <Check size={16} /> : <Copy size={16} />}
                      </button>
                      <button
                        onClick={() => void handleRename(a)}
                        disabled={busy}
                        className="p-2 rounded-xl hover:bg-slate-800 text-slate-500 hover:text-slate-200 transition-colors disabled:opacity-50"
                      title="Rename"
                    >
                      <Pencil size={16} />
                    </button>
                    <button
                      onClick={() => void handleDelete(a)}
                      disabled={busy}
                      className="p-2 rounded-xl hover:bg-red-500/10 text-slate-500 hover:text-red-300 transition-colors disabled:opacity-50"
                      title="Delete"
                    >
                      <Trash2 size={16} />
                    </button>
                  </div>
                  </div>
                ))
            )}
          </div>
        </div>

        <div className="p-5 border-t border-slate-800 bg-slate-900/40 text-xs text-slate-500">
          Tip: Use the copied `assets/...` path in prompts, or in Remotion use `staticFile('assets/{'{'}filename{'}'}')`.
        </div>
      </div>
    </div>
  );
}
