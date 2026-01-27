import { AbsoluteFill, interpolate, useCurrentFrame } from 'remotion';

export const Main: React.FC = () => {
	const frame = useCurrentFrame();
	const opacity = interpolate(frame, [0, 30], [0, 1], {
		extrapolateRight: 'clamp',
	});

	return (
		<AbsoluteFill
			style={{
				backgroundColor: '#0f172a',
				justifyContent: 'center',
				alignItems: 'center',
				color: 'white',
				fontSize: '80px',
				fontWeight: 'bold',
				fontFamily: 'sans-serif',
			}}
		>
			<div style={{ opacity }}>Hello Epris</div>
		</AbsoluteFill>
	);
};
