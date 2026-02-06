import React, { useEffect, useMemo, useState } from 'react';
import { Player } from '@remotion/player';
import { Main } from './Composition';
import { VIDEO_FPS, DURATION_IN_FRAMES, VIDEO_WIDTH, VIDEO_HEIGHT } from './VideoConfig';
import eprisProps from './epris-props.json';
import { listEprisObjects } from './epris/registry';

/**
 * Standalone preview page for embedding in iframe.
 * Renders the Main composition with player controls.
 */
export const Preview: React.FC = () => {
  const initialProps = useMemo<Record<string, unknown>>(() => {
    const v = (eprisProps as any)?.values;
    return v && typeof v === 'object' && !Array.isArray(v) ? (v as Record<string, unknown>) : {};
  }, []);

  const [inputProps, setInputProps] = useState<Record<string, unknown>>(initialProps);

  useEffect(() => {
    window.parent?.postMessage({ type: 'epris:ready' }, '*');

    const onMessage = (event: MessageEvent) => {
      const data = event.data as any;
      if (!data || typeof data !== 'object') return;

      if (data.type === 'epris:setInputProps') {
        const payload = data.payload;
        if (payload && typeof payload === 'object' && !Array.isArray(payload)) {
          setInputProps(payload as Record<string, unknown>);
        }
        return;
      }

      if (data.type === 'epris:getObjectsSnapshot') {
        const objects = listEprisObjects();
        window.parent?.postMessage({ type: 'epris:objectsSnapshot', payload: { objects } }, '*');
      }
    };

    window.addEventListener('message', onMessage);
    return () => window.removeEventListener('message', onMessage);
  }, []);

  return (
    <div
      style={{
        width: '100vw',
        height: '100vh',
        background: '#1e293b',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
      }}
    >
      <Player
        component={Main}
        durationInFrames={DURATION_IN_FRAMES}
        compositionWidth={VIDEO_WIDTH}
        compositionHeight={VIDEO_HEIGHT}
        fps={VIDEO_FPS}
        inputProps={inputProps}
        style={{ width: '100%', height: '100%' }}
        controls
        autoPlay
        loop
      />
    </div>
  );
};

export default Preview;
