import React, { useEffect, useMemo } from 'react';
import type { EprisObjectKind } from './registry';
import { registerEprisObject, unregisterEprisObject } from './registry';

export type EprisGroupProps = {
  id: string;
  label?: string;
  kind?: EprisObjectKind;
  tags?: string[];
  children?: React.ReactNode;
};

export const EprisGroup: React.FC<EprisGroupProps> = ({ id, label, kind = 'group', tags, children }) => {
  const tagsKey = useMemo(() => (tags?.length ? tags.join('|') : ''), [tags]);

  useEffect(() => {
    registerEprisObject({
      id,
      label: label ?? id,
      kind,
      tags,
    });
    return () => unregisterEprisObject(id);
  }, [id, label, kind, tagsKey]);

  return <>{children}</>;
};

