import { Composition, registerRoot } from 'remotion';
import { Main } from './Composition';
import { COMPOSITION_ID, VIDEO_WIDTH, VIDEO_HEIGHT, VIDEO_FPS, DURATION_IN_FRAMES } from './VideoConfig';
import eprisProps from './epris-props.json';

export const RemotionRoot: React.FC = () => {
	return (
		<>
			<Composition
				id={COMPOSITION_ID}
				component={Main}
				durationInFrames={DURATION_IN_FRAMES}
				fps={VIDEO_FPS}
				width={VIDEO_WIDTH}
				height={VIDEO_HEIGHT}
				defaultProps={(eprisProps as any)?.values ?? {}}
			/>
		</>
	);
};

registerRoot(RemotionRoot);
