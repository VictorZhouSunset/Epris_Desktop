import React from 'react';
import { Player } from '@remotion/player';
import { Main } from './Composition';
import { VIDEO_FPS, DURATION_IN_FRAMES, VIDEO_WIDTH, VIDEO_HEIGHT } from './VideoConfig';

/**
 * Standalone preview page for embedding in iframe.
 * Renders the Main composition with player controls.
 */
export const Preview: React.FC = () => {
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
        style={{ width: '100%', height: '100%' }}
        controls
        autoPlay
        loop
      />
    </div>
  );
};

export default Preview;
