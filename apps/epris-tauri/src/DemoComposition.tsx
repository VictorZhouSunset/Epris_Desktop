import { AbsoluteFill, interpolate, useCurrentFrame } from 'remotion';

export const DemoComposition: React.FC = () => {
	const frame = useCurrentFrame();
	const opacity = interpolate(frame, [0, 30], [0, 1], {
		extrapolateRight: 'clamp',
	});

	return (
		<AbsoluteFill
			style={{
				backgroundColor: '#1e293b',
				justifyContent: 'center',
				alignItems: 'center',
				color: 'white',
				fontSize: '60px',
				fontWeight: 'bold',
				fontFamily: 'sans-serif',
			}}
		>
			<div style={{ opacity }}>Epris Preview</div>
		</AbsoluteFill>
	);
};
