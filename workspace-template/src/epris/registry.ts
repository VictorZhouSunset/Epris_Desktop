export type EprisObjectKind =
  | 'group'
  | 'shape'
  | 'text'
  | 'image'
  | 'video'
  | 'background'
  | 'particles'
  | 'other';

export type EprisObjectMeta = {
  id: string;
  label?: string;
  kind?: EprisObjectKind;
  tags?: string[];
};

const objectsById = new Map<string, EprisObjectMeta>();

export function registerEprisObject(meta: EprisObjectMeta) {
  objectsById.set(meta.id, meta);
}

export function unregisterEprisObject(id: string) {
  objectsById.delete(id);
}

export function listEprisObjects(): EprisObjectMeta[] {
  return Array.from(objectsById.values()).sort((a, b) => a.id.localeCompare(b.id));
}

