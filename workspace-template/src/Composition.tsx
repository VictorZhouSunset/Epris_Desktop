import { AbsoluteFill, interpolate, useCurrentFrame } from 'remotion';
import { EprisGroup } from './epris';

export type MainProps = {
	backgroundColor?: string;
	titleText?: string;
	titleColor?: string;
	titleFontSize?: number;
	titleFontFamily?: string;
};

export const Main: React.FC<MainProps> = ({
	backgroundColor = '#0f172a',
	titleText = 'Hello Epris',
	titleColor = '#ffffff',
	titleFontSize = 80,
	titleFontFamily = 'sans-serif',
}) => {
	const frame = useCurrentFrame();
	const opacity = interpolate(frame, [0, 30], [0, 1], {
		extrapolateRight: 'clamp',
	});

	return (
		<AbsoluteFill>
			<EprisGroup id="background" label="Background" kind="background">
				<AbsoluteFill style={{ backgroundColor }} />
			</EprisGroup>

			<AbsoluteFill style={{ justifyContent: 'center', alignItems: 'center' }}>
				<EprisGroup id="title" label="Title" kind="text">
					<div
						style={{
							opacity,
							color: titleColor,
							fontSize: `${titleFontSize}px`,
							fontWeight: 'bold',
							fontFamily: titleFontFamily,
						}}
					>
						{titleText}
					</div>
				</EprisGroup>
			</AbsoluteFill>
		</AbsoluteFill>
	);
};
