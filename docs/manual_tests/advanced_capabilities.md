# Epris Advanced Capabilities Test Suite

This document contains complex test prompts designed to push the limits of the AI's understanding of Remotion, React, and specialized Skill libraries.

## 1. Advanced Typography & Text Effects

_Focus: Word-level timing, specialized animations, and layout precision._

### A. Word-by-Word Highlight

- **EN**: "Create a 5-second animation where a quote from Steve Jobs appears. Each word should light up in a vibrant neon cyan color exactly when it's 'spoken' (spaced out evenly), while previous words dim to a subtle grey. Use a smooth spring transition for the color change."
- **CN**: "创建一个5秒的动画，显示史蒂夫·乔布斯的一段名言。每个单词在被‘念到’时（均匀间隔）应亮起鲜艳的霓虹青色，而之前的单词则变暗为淡灰色。使用平滑的弹簧（spring）过渡来实现颜色变化。"

### B. Typewriter with Terminal Aesthetics

- **EN**: "Implement a terminal-style typewriter effect. The text should be 'Initializing Epris System...' followed by a scrolling list of mock files. Add a blinking underscore cursor at the end of the text that follows the typing progress. Use a monospace font and glassmorphism for the background container."
- **CN**: "实现一个终端风格的打字机效果。文本内容为‘Initializing Epris System...’，随后显示一段模拟文件列表的滚动。在文字末尾添加一个随打字进度移动的闪烁下划线光标。使用等宽字体，并为背景容器使用玻璃拟态（glassmorphism）设计。"

---

## 2. Physics & Dynamic Motion

_Focus: Spring physics, chaining, and coordinate math._

### A. Elastic Menu Reveal

- **EN**: "Create a vertical side menu that slides in from the left with a heavy elastic bounce (overshoot). Once the menu is open, make 5 menu items pop in one-by-one with a staggering delay, each using a unique spring configuration to feel 'rubbery'."
- **CN**: "创建一个垂直侧边栏菜单，从左侧滑入并带有强烈的弹性回弹（带超调）。菜单打开后，让5个菜单项以交错延迟的方式逐个弹出，每个菜单项使用独特的弹簧配置，使其具有‘橡胶般’的质感。"

### B. Orbiting Particles

- **EN**: "Render a central 'Sun' icon. Have 3 smaller 'Planet' icons orbit around it at different speeds and distances. Use Math.sin and Math.cos to calculate positions, and apply a motion blur trail effect to the planets using CSS filters."
- **CN**: "渲染一个中心的‘太阳’图标。让3个较小的‘行星’图标以不同的速度和距离绕其旋转。使用 Math.sin 和 Math.cos 计算位置，并利用 CSS 滤镜为行星添加动态模糊的拖尾效果。"

---

## 3. Cinematic Scenes & Transitions

_Focus: Using @remotion/transitions and scene management._

### A. Professional Scene Wipe

- **EN**: "Create two scenes. Scene 1 shows a high-tech city background (using a gradient). Scene 2 shows a serene forest. Implement a custom 'Slide and Blur' transition between them at the 2-second mark using the @remotion/transitions library if available, or manual interpolation of blur and X position."
- **CN**: "创建两个场景。场景1显示高科技城市背景（使用渐变），场景2显示宁静的森林。在2秒处，使用 @remotion/transitions 库（如果可用）或手动对模糊度和 X 坐标进行插值，实现一个自定义的‘滑动并模糊’过渡效果。"

---

## 4. Data Visualization & State

_Focus: Mapping data to visual properties._

### A. Dynamic Bar Chart

- **EN**: "Define an array of 5 data points representing 'Usage Stats'. Render a bar chart where each bar's height is determined by the data. When the animation starts, each bar should grow from the bottom to its target height at a slightly different starting frame, with a bouncy settle."
- **CN**: "定义一个包含5个数据点的数组，代表‘使用统计’。渲染一个柱状图，每个柱子的高度由数据决定。动画开始时，每个柱子应从底部增长到目标高度，起始帧略有不同，并带有弹性的稳定效果。"

---

## 5. Component Architecture & "Smart" Rules

_Focus: Adhering to REMOTION.md and clean code._

### A. Reusable Glass Card

- **EN**: "Create a reusable 'FeatureCard' component that takes an icon, title, and description as props. Render 3 of these cards in a responsive flexbox layout. Ensure the code is clean, types are strictly defined, and the principal logic remains in Composition.tsx as per instructions."
- **CN**: "创建一个可复用的‘FeatureCard’组件，接受图标、标题和描述作为 props。在响应式 flexbox 布局中渲染3个这样的卡片。确保代码整洁、类型定义严格，并根据指令将核心逻辑保留在 Composition.tsx 中。"

---

## 6. Stress Test: The "Masterpiece"

- **EN**: "Create an 'Intro' sequence for a tech vlog. It should feature: 1. A background video placeholder (using `staticFile`), 2. An animated logo that assembles from fragments, 3. Dynamic lower-third text that appears when a 'speaker' is active, 4. A subtle overlay of digital floating code particles. Everything must be perfectly timed to a 30fps clock."
- **CN**: "为一个科技视频博客（Vlog）制作一个‘开场’序列。包含：1. 背景视频占位符（使用 `staticFile`），2. 一个由碎片汇聚而成的动画 Logo，3. 在‘演讲者’活跃时出现的动态人名条，4. 叠加一层细微的浮动数字代码粒子。所有元素必须严格按照 30fps 时钟精准对齐。"

---

## 7. Complex Media & Captions

_Focus: Video synchronized with external assets and timing._

### A. Narrative Captions

- **EN**: "Load a background video using staticFile. Add a caption overlay at the bottom. Use a Mock JSON object for timestamps and have the captions change color or scale slightly as they become active. Ensure the timing is perfectly synced with the frame count."
- **CN**: "使用 staticFile 加载一段背景视频。在底部添加字幕叠加层。使用模拟的 JSON 对象提供时间戳，并在字幕激活时使其变色或略微缩放。确保时间控制与帧数（frame count）完美同步。"

### B. Audio Visualizer (Mock)

- **EN**: "Create an 'Audio Visualizer' component. Since we don't have real-time audio analysis, use a random seed based on `frame` to animate 20 vertical bars that dance to a 'fake' beat. Apply a color gradient (purple to pink) that shifts as the bars move."
- **CN**: "创建一个‘音频可视化（Audio Visualizer）’组件。由于没有实时音频分析，请使用基于 `frame` 的随机种子来驱动20个垂直条，使其伴随‘虚拟’节拍跳动。应用一个随条形移动而变化的渐变色（紫色到粉色）。"
