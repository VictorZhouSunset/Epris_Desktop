## 讨论总结

### 目标产品形态

- 桌面应用 Epris：本地生成与编辑 Remotion 动画视频，隐藏代码与 IDE。
- V0 极简：只有预览窗口 + 一个 prompt 输入框，用户通过连续 prompt 迭代动画，支持导出。
- 后续支持：对象选中 + 关键帧时间轴编辑（专业视频工具式 UI）。
- 产品核心：双agent模式，Coding Agent Workflow负责生成视频代码，UI-Config Agent Workflow负责从代码中提取视频元素编辑配置并应用于UI。

### 核心架构结论

- **实时预览回路必须不依赖 AI**：任何 UI 调参/关键帧编辑都要通过本地数据（inputProps/editorData）驱动 React 重渲染实现即时变化。
- **AI（OpenCode）只负责“改代码”链路**：prompt -> OpenCode 改 Remotion 工程代码 -> 预览刷新；以及渲染失败后自动修复。
- **对象选中需要 overlay 交互层**：因为视频画面本身不可直接“点选组件”，需要一层透明 overlay 来画 bounding boxes/处理命中。

### 工具/组件选择（方向性）

- 桌面壳：Tauri。
- 代码 Agent 引擎：选 anomalyco 的 OpenCode（MIT，可商用分发），用 headless server 接入。
- 预览：@remotion/player（嵌入式预览）。
- 导出/冒烟：@remotion/renderer（renderStill/renderFrames/renderMedia）。
- 画布交互：daybrush/moveable。
- overlay 承载：先用纯 DOM（绝对定位 div），后续可换 react-konva/fabric。
- 时间轴：react-timeline-editor（或 timeline-editor-react）。
- 代码分析/改写（后置增强）：ts-morph（精确类型感知改写）+ jscodeshift（批量 codemod）。
- Skills：superpowers（计划/执行流程技能）+ planning-with-files（计划/进度落盘复读）用于后台治理与开发流程。

### 更新与分发

- Tauri Updater 可用 GitHub public Releases + latest.json（静态 JSON）实现自动更新（不必先做服务器）。
- 但如果你要“登录后才能用”，需要后端做身份与权限校验（至少做到：未登录不能进入主功能）。

### 登录/订阅/权限：是否需要 Supabase

- 你明确想“用户登录后才能使用”，因此需要一个后端。
- 最薄方案：Supabase（Google Auth + DB + Edge Functions + Storage）。
  - 先不做 LLM 代理，不碰用户 prompt；只做登录、白名单、功能开关、（后续）订阅权益。
  - 更新包依旧放 GitHub Releases；Supabase 可用于 gating 与下发配置。

---

## 版本迭代计划（从试用到专业编辑器）

### V0 内测（本地闭环跑通）

**用户体验**

- Windows 可安装运行。
- 仅：预览窗口 + 单 prompt 输入框。
- prompt -> 生成/修改动画 -> 预览自动刷新。
- 支持导出视频。

**必须实现的系统能力**

- Workspace 初始化（Remotion 模板、目录结构）。
- 后台服务：
  - OpenCode headless（会话/工具调用）。
  - 预览服务（Player 或 dev server）。
  - 渲染 worker（renderer）。
- Gate：tsc + smoke render（少量帧）失败自动修复再重试。
- 版本git DAG：每次改动有历史记录。
- 日志：prompt/改动文件/错误栈落盘。

**服务端开发**

- 无。

---

### V0.5 试用分发（自动更新 + 基础稳定性）

**用户体验**

- 你把安装包发给 5 个朋友；之后客户端自动检查更新。

**必须实现的系统能力**

- Tauri Updater：GitHub Releases + latest.json。
- CI/CD：GitHub Actions 自动打包并发布 release。
- 更严格的沙盒：限制 OpenCode 只能读写 workspace。

**服务端开发**

- 无（仍可先不做登录）。

---

### V1 登录门槛版（登录后才能用）

**用户体验**

- 启动进入登录页：Google 登录。
- 未登录不能进入生成界面。
- 登录后进入 V0 功能（预览 + prompt + 导出）。

**必须实现的系统能力**

- 本地 token/session 管理。
- 远程配置拉取：允许的功能开关、渠道（beta/stable）、（可选）白名单。

**服务端开发（Supabase）**

- Auth：Google OAuth。
- DB：users 表 + allowlist/entitlements 表（最薄）。
- Edge Function：/me（返回用户是否允许使用、功能开关）。

**更新策略**

- 更新包仍放 GitHub public。
- 客户端更新不强制鉴权（第一版可接受“拿到 app 就能更新”，但没有登录用不了功能）。

---

### V1.5 稳定性治理（后台技能化，不增加 UI 复杂度）

**用户体验**

- 无明显界面变化，整体更稳。

**必须实现的系统能力**

- 后台引入 superpowers 的“先写文件级计划再执行”作为安全刹车。
- planning-with-files 在 workspace 内维护：实现计划、进度、已知坑、上次失败摘要。
- 变更摘要：展示改动文件列表/渲染状态（不展示代码）。

**服务端开发**

- 可选：记录匿名 crash/导出失败计数（便于你迭代）。

---

### V2 交互编辑起步（对象选中 + 拖拽，但关键帧最小化）

**用户体验**

- 预览画面可点选对象。
- 选中对象出现控制框，可拖拽移动/缩放/旋转。
- 右侧出现对象列表与基础参数（但时间轴可先只支持“单关键帧/起始值”）。

**必须实现的系统能力**

- overlay 交互层（DOM overlay + moveable）。
- 运行时可编辑数据：editorData（挂在 inputProps 里）。
- Remotion 侧解释：组件从 editorData 读取 transform/opacity 等。
- OpenCode 改代码时必须保留 editor runtime 约定（不能破坏 editorData 解释器接口）。

**服务端开发**

- 无新增。

---

### V3 专业编辑器骨架（关键帧时间轴）

**用户体验**

- 选中对象后，预览下方显示该对象轨道时间轴。
- 可增删关键帧、拖动关键帧、设置插值曲线。
- 拖动时间轴时预览同步 seek。

**必须实现的系统能力**

- timeline 组件集成。
- 数据模型：tracks/keyframes/curves（仍然存 editorData）。
- valueAtFrame 解释器（把 keyframes 转换为每帧值）。
- 性能优化：关键帧编辑时避免整页卡顿。

**服务端开发**

- 可选：云端同步项目元数据（仅当你想多设备）。

---

### V4 参数抽取与面板（你的“第二个 workflow”落地）

**用户体验**

- 右侧参数更丰富：搜索、分组、范围滑块、颜色等。
- 用户可用自然语言请求“暴露某参数”。

**必须实现的系统能力**

- 组合策略：
  - 首先要求 Coding Agent 维护 Zod schema + defaultProps。
  - 引入 ts-morph/jscodeshift：自动提升常量为 prop 并改写引用。
- UI：从 schema 派生控件。

**服务端开发**

- 若要订阅差异化：开始引入 Stripe（或先只做套餐字段）。

---

### V5 订阅与托管 LLM（可选，商业化升级）

**用户体验**

- 订阅用户免配置 key（或仍支持 BYOK）。
- 多 agent 多模型路由由你控制。

**必须实现的系统能力**

- LLM Proxy：限流、配额、审计、成本上限。
- 模型路由策略（规划/改写/修复/渲染诊断用不同模型）。

**服务端开发（Supabase + 额外服务）**

- Stripe webhook + entitlements。
- Proxy 服务（可以单独部署，不建议放 Edge Function）。

---

## 你当前阶段的建议路径

- 先做：V0 -> V0.5（给朋友试用 + 自动更新）。
- 然后立刻切到：V1（Google 登录 gating）。
- 再推进：V2/V3（对象选中 + 关键帧时间轴），这是你产品核心。
- Props 抽取（V4）与订阅托管（V5）都可以后置。
