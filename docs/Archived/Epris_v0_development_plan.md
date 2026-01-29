## V0 目标与边界

### V0 要达到的体验

- Windows 可安装运行。
- 应用内单窗口（不打开浏览器）。
- 左侧/主区域：内嵌 Remotion Player 预览。
- 下方：一个 prompt 输入框（回车发送）。
- 用户输入 prompt -> 后台 Agent 改 workspace 的 Remotion 代码 -> App 内预览刷新并展示新动画。

### V0 不做

- props UI、对象列表、时间轴、关键帧编辑
- 登录/订阅
- 多项目管理（先单项目/单 workspace）

---

## 总体架构（方式1）

### 组件与进程

- **Tauri App（前端 React）**
  - 包含 `@remotion/player` 组件
  - prompt 输入框
  - 显示渲染/改码状态（简易 toast 或状态条）

- **Workspace（本地目录）**
  - Remotion 工程（composition + 组件）
  - `.opencode/`（命令与规则，可选）
  - `logs/`（prompt、错误、改动摘要）

- **后台服务（由 Tauri Rust side 启动/管理）**
  1. OpenCode headless server（HTTP）
  2. Bundler/Preview Builder（用于让 Player 能加载 workspace 最新代码）

### 关键设计点：Player 如何加载“被改写的工程代码”

- V0 采用最简单可行策略：
  - 后台把 workspace 的 Remotion 工程 **bundle** 成一份可被 Player 引入的 JS（或一个本地 dev server URL）
  - Agent 每次改完代码后：触发一次 **re-bundle**
  - 前端收到“bundle 完成”事件后：重新加载 composition（整块重载 Player 或刷新 key）

---

## 开发顺序（作业式）

每一步都建议你用 `planning-with-files` + `superpowers` 风格驱动 Antigravity：

- 先写文件级计划（改哪些文件、怎么验证），写入 `task_plan.md`
- 实现后把验证结果写入 `progress.md`
- 任何坑写入 `findings.md`

---

### Step 1 — 初始化仓库与“持久计划系统”

**要做的事**

1. private repo：`epris-desktop`
2. 建 `/.agent/skills/` 并放入：
   - superpowers（至少放 writing-plans + executing-plans 的部分）

**产出**

- 初始 commit：`chore: init planning files and skills`

---

### Step 2 — 生成 workspace-template（Remotion 最小工程）

**要做的事**

1. 添加 `/workspace-template/`：
   - `src/Root.tsx` + 1 个 Composition（简单方块/文字动画）
   - `package.json`、lockfile
2. 在 template 里提供一个固定入口，例如 composition id = `Main`，尺寸 1920x1080，duration 150。

**Benchmark**

- 在命令行能本地安装依赖并成功构建（先不要求预览）。

---

### Step 3 — Tauri + React UI 骨架（嵌入 Player）

**要做的事**

1. 创建 `/apps/epris-tauri/`：Tauri + React
2. UI 只做：
   - 一个 Player 容器
   - 一个 prompt 输入框
   - 一个状态区（Idle / Coding / Bundling / Error）
3. Player 先加载“内置 demo composition”（先不接 workspace）

**Benchmark**

- `pnpm dev` 启动后，桌面窗口内能播放 demo。

---

### Step 4 — Workspace 生命周期（创建/打开/清理）

**要做的事**

1. App 启动时：
   - 在用户目录创建 workspace（复制 workspace-template）
   - 写入 `logs/` 目录
2. Rust side 管理：
   - workspace 路径
   - 子进程生命周期（后续 OpenCode）

**Benchmark**

- 删除 workspace 后重启 App，会自动重新生成。
- 关闭 App 后不会遗留锁文件/占用。

---

### Step 4.5 — workspace bootstrap（首次运行自动装依赖 + 可复现）

> 否则你 Step5 的 `pnpm dev` 很容易第一次就失败。

**要做的事**

1. workspace 创建完成后，执行一次 bootstrap（只在首次/依赖缺失时）
   - `pnpm install`（在 workspace 目录）
2. 缓存策略（V0 简化）
   - 允许直接在 workspace 内生成 `node_modules`
   - 失败时把 stdout/stderr 写入 `workspace/logs/bootstrap.log`
3. 将“bootstrap 是否完成”的状态落盘
   - 例如 `workspace/.epris/bootstrap.json`（记录时间、pnpm lock hash）

**Benchmark**

- 删除 workspace 后重启 App：自动复制模板 + 自动 install + 预览能启动
- install 失败：UI 显示“依赖安装失败”，并在 logs 里能看到完整错误

---

### Step 5 — App 内嵌预览（Vite + iframe：加载 workspace 的 preview 页面）

**目标：** 只提供一个播放器，不引入 Remotion Studio UI；同时让 OpenCode 改 workspace 代码后，预览能快速刷新。

**方案：**

- workspace 自己跑一个极简 preview web app（Vite + React + `@remotion/player`）
- App 预览区域用 iframe 加载 `http://127.0.0.1:<port>/`
- 代码变更后依赖 Vite HMR 自动刷新；OpenCode 完成后再做一次 iframe reload 兜底

> 注意：这依然是"嵌进桌面 App 里"，不会打开外部浏览器。iframe 加载完整页面不会有 CORS 问题（不是跨域 import），主要需要处理的是 Tauri CSP 对 iframe 和 WebSocket 的限制。

---

#### 5.1 workspace-template：新增 Vite preview app

**要做的事**

1. 在 `workspace-template/` 中配置 Vite + React
   - `src/index.tsx` 渲染 `<Player />`
   - `src/Composition.tsx` 导出 `Main`（被 Player 引用）
   - `vite.config.ts` 固定 host/port，并显式配置 HMR

2. 预览入口（示例）

```tsx
// workspace-template/src/index.tsx
import React from "react";
import ReactDOM from "react-dom/client";
import { Player } from "@remotion/player";
import { Main } from "./Composition";

const Root = () => (
  <div style={{ width: "100vw", height: "100vh", margin: 0 }}>
    <Player
      component={Main}
      durationInFrames={150}
      fps={30}
      compositionWidth={1920}
      compositionHeight={1080}
      style={{ width: "100%", height: "100%" }}
      controls
    />
  </div>
);

ReactDOM.createRoot(document.getElementById("root")!).render(<Root />);
```

3. Vite 配置（关键：host/port + HMR）

```ts
// workspace-template/vite.config.ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  server: {
    host: "127.0.0.1",
    port: 3030,
    strictPort: true, // 端口被占用时直接报错，不静默换端口
    hmr: {
      protocol: "ws",
      host: "127.0.0.1",
      port: 3030,
    },
  },
});
```

4. package.json scripts

```json
{
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview --host 127.0.0.1 --port 3030"
  }
}
```

---

#### 5.2 App：启动 workspace dev server + iframe 预览

**要做的事**

1. App 启动时 spawn 预览 server
   - 在 workspace 路径执行：`pnpm dev`
   - **就绪检测**：等待端口 3030 可用后再显示 iframe（或先显示 Loading）

2. App 预览区使用 iframe
   - `<iframe src="http://127.0.0.1:3030/" />`
   - 提供 Reload Preview 按钮：改变 iframe 的 `key` 强制重建

3. OpenCode 改完代码后调用一次 Reload 作为兜底（即使 HMR 正常也没坏处）

---

#### 5.3 Tauri 配置：CSP 注意事项

Vite HMR 会用 `ws://127.0.0.1:3030`，iframe 会加载 `http://127.0.0.1:3030`。

**当前配置 `"csp": null`** 表示无 CSP 限制，iframe 和 HMR 应该都能正常工作。（V0 为了快速验证先 csp: null，V0.5 版本收紧为白名单）

如果后续启用 CSP，需要确保：

- `frame-src` 允许 `http://127.0.0.1:*`
- `connect-src` 允许 `ws://127.0.0.1:*` 和 `http://127.0.0.1:*`

---

**Benchmark**

- 手动编辑 workspace 中 `Main` 的文字或颜色并保存
- App 内 iframe 预览自动刷新（HMR）看到变化
- 若 HMR 因环境问题未触发，点击 Reload Preview 能看到变化

---

### Step 6 — 接入 OpenCode headless（仅改代码，不暴露 IDE）

**要做的事**

1. App 启动时启动 OpenCode server

- spawn：`opencode serve --hostname 127.0.0.1 --port 4096`
- 启动后做一次健康检查（例如请求 `/global/health` 或打开 `/doc` 验证 server 活着）

2. App 发送 prompt 的调用链（推荐：Rust side 调 OpenCode，绕开 CORS）

- 前端：prompt 输入 -> 调用 Tauri command `agent_apply_prompt(prompt)`
- Rust：负责向 `http://127.0.0.1:4096` 发 session/message 请求
  - 首次：创建 session，保存 sessionId
  - 后续：复用 sessionId 发送 message

3. Prompt 模板（V0 先写死）

- System/Instruction 里明确：
  - workspace 路径（只允许修改 workspace 内文件）
  - 目标：修改 Remotion 动画以满足用户描述
  - 不允许破坏 `/preview` 页面入口与 `Main` composition id（这是 V0 的稳定锚点）
  - 修改后必须确保能启动预览（可选：跑一次 typecheck/build）

4. OpenCode 完成后的动作

- 记录：把本次 prompt、改动文件列表、关键输出写入 `workspace/logs/`
- 刷新：触发 iframe reload（最简单：重建 iframe key）

**Benchmark**

- 输入 prompt：`生成一个蓝色圆从左到右移动（2 秒），背景白色`
- 在合理时间内（例如 30 秒），App 内预览窗口能看到变化

---

### Step 7 — 增量修改（保持同一 session + 继续 prompt）

**要做的事**

1. 保存 sessionId（按 workspace 粒度）

- sessionId 写在 App 内存中即可；可选落盘到 `workspace/.epris/session.json`

2. 后续 prompt 复用 sessionId

- 让 OpenCode 在已有代码基础上修改，而不是重新生成一套新工程
- 同时继续要求：
  - 保持 `Main` composition id 不变
  - 保持 `/preview` 页面入口不变

3. 增量修改后的刷新

- OpenCode 完成 -> iframe reload

**Benchmark**

- 第一次 prompt 生成动画
- 第二次 prompt：`把移动速度加快一倍，并且把圆改成红色`
- 结果是同一个动画“变快且变红”，而不是完全换成不相关的结构

---

## Step 8 — Snapshot 系统与双轨保存（基础设施，必须在 Gate 之前）

### 要做的事

#### 1. **快照数据结构（支持 DAG）**

```rust
#[derive(Serialize, Deserialize)]
struct SnapshotMetadata {
    id: String,              // UUID
    name: String,            // 用户可编辑，默认为时间戳
    description: String,     // 用户可编辑，默认空
    timestamp: String,       // ISO 8601
    parent_id: Option<String>, // 父快照 ID（支持分支）
    is_manual: bool,         // true=手动保存, false=自动保存
    prompt: Option<String>,  // 触发的 prompt（自动保存时有值）
    session_id: String,
    gate_result: Option<GateResult>, // Gate 完成后更新
}
```

**目录结构：**

```
workspace/.epris/history/
├── <snapshot-id-1>/
│   ├── meta.json          // SnapshotMetadata
│   ├── src/               // 快照内容
│   └── public/
├── <snapshot-id-2>/
│   └── ...
└── graph.json             // DAG 关系图（可选，也可从 meta 重建）
```

---

#### 2. **白名单策略（只快照可修改内容）**

```rust
const SNAPSHOT_WHITELIST: &[&str] = &[
    "src",        // 所有源码
    "public"      // 静态资源
    // ❌ 不快照配置文件（禁止修改）
];
```

---

#### 3. **两种保存机制**

**A. 自动保存（每次 prompt 前）**

```rust
#[tauri::command]
fn auto_save_snapshot(
    workspace_path: String,
    prompt: String,
    parent_id: String,  // 当前 HEAD 快照 ID
    session_id: String,
) -> Result<String, String> {
    let snapshot_id = Uuid::new_v4().to_string();
    let snapshot_meta = SnapshotMetadata {
        id: snapshot_id.clone(),
        name: format!("Auto {}", chrono::Utc::now().format("%H:%M:%S")),
        description: String::new(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        parent_id: Some(parent_id),
        is_manual: false,
        prompt: Some(prompt),
        session_id,
        gate_result: None,
    };

    create_snapshot_with_metadata(workspace_path, snapshot_meta)
}
```

**B. 手动保存（用户主动）**

```rust
#[tauri::command]
fn manual_save_snapshot(
    workspace_path: String,
    name: String,           // 用户输入
    description: String,    // 用户输入
    parent_id: String,
) -> Result<String, String> {
    let snapshot_id = Uuid::new_v4().to_string();
    let snapshot_meta = SnapshotMetadata {
        id: snapshot_id.clone(),
        name,
        description,
        timestamp: chrono::Utc::now().to_rfc3339(),
        parent_id: Some(parent_id),
        is_manual: true,
        prompt: None,
        session_id: String::new(),
        gate_result: None,
    };

    create_snapshot_with_metadata(workspace_path, snapshot_meta)
}
```

---

#### 4. **HEAD 指针管理**

```rust
// workspace/.epris/HEAD 文件：记录当前指向的快照 ID
#[tauri::command]
fn get_current_head(workspace_path: String) -> Result<String, String> {
    let head_path = Path::new(&workspace_path).join(".epris/HEAD");
    std::fs::read_to_string(head_path)
        .map_err(|_| "root".to_string()) // 初始状态返回 "root"
}

#[tauri::command]
fn set_head(workspace_path: String, snapshot_id: String) -> Result<(), String> {
    let head_path = Path::new(&workspace_path).join(".epris/HEAD");
    std::fs::write(head_path, snapshot_id)
        .map_err(|e| e.to_string())
}
```

---

#### 5. **跳转到快照（Checkout）**

```rust
#[tauri::command]
fn checkout_snapshot(
    workspace_path: String,
    snapshot_id: String,
) -> Result<(), String> {
    // 1. 加载快照内容到 workspace
    restore_snapshot(&workspace_path, &snapshot_id)?;

    // 2. 更新 HEAD
    set_head(workspace_path.clone(), snapshot_id)?;

    // 3. 触发预览刷新
    // (通过事件通知前端 reload iframe)

    Ok(())
}
```

---

#### 6. **删除快照及子树**

```rust
#[tauri::command]
fn delete_snapshot_tree(
    workspace_path: String,
    snapshot_id: String,
) -> Result<Vec<String>, String> {
    // 1. 查找所有子节点（DFS）
    let descendants = find_all_descendants(&workspace_path, &snapshot_id)?;

    // 2. 删除所有快照目录
    for id in &descendants {
        let snapshot_dir = Path::new(&workspace_path)
            .join(".epris/history")
            .join(id);
        std::fs::remove_dir_all(snapshot_dir)?;
    }

    // 3. 如果 HEAD 指向被删除的快照，回退到最近的祖先
    let current_head = get_current_head(workspace_path.clone())?;
    if descendants.contains(&current_head) {
        let parent = find_nearest_ancestor(&workspace_path, &snapshot_id)?;
        set_head(workspace_path, parent)?;
    }

    Ok(descendants)
}
```

---

#### 7. **自动清理策略（保留重要快照）**（暂时忽略）

```rust
fn cleanup_old_snapshots(workspace_path: &str) -> Result<(), String> {
    // 清理规则：
    // - 暂时应该保留所有快照
    // - 删除时保证不破坏 DAG 连通性

    let all_snapshots = list_all_snapshots(workspace_path)?;
    let mut auto_snapshots: Vec<_> = all_snapshots.iter()
        .filter(|s| !s.is_manual)
        .collect();

    auto_snapshots.sort_by_key(|s| &s.timestamp);

    if auto_snapshots.len() > 10 {
        let to_delete = auto_snapshots.len() - 10;
        for snapshot in auto_snapshots.iter().take(to_delete) {
            // 只删除没有子节点的自动快照
            if !has_children(workspace_path, &snapshot.id)? {
                delete_snapshot(workspace_path, &snapshot.id)?;
            }
        }
    }

    Ok(())
}
```

---

### Benchmark

- 发送 prompt → 自动创建快照（名称："Auto 10:30:15"）
- 手动点击 "Save Version" → 弹窗输入名称/描述 → 创建快照

---

## Step 8.5 — Gate：安全检测 + typecheck + smoke + 自动修复

### 要做的事

#### 1. **🔒 安全检测**（在 Gate 之前执行）

```rust
async fn send_prompt(...) -> Result<PromptResponse, String> {
    // Step 1: 自动保存快照
    let parent_id = get_current_head(workspace_path.clone())?;
    let snapshot_id = auto_save_snapshot(
        workspace_path.clone(),
        prompt.clone(),
        parent_id,
        session_id.clone()
    )?;

    // Step 2: OpenCode 修改代码
    // ...

    // Step 2.5: 🔒 安全检测
    let forbidden_files = check_forbidden_files_modified(
        &workspace_path,
        &snapshot_id
    )?;

    if !forbidden_files.is_empty() {
        // 立即回滚到快照
        checkout_snapshot(workspace_path.clone(), snapshot_id.clone())?;

        // 记录安全日志
        log_security_violation(&workspace_path, &forbidden_files, &prompt)?;

        return Ok(PromptResponse {
            success: false,
            message: format!(
                "🔒 Security: AI attempted to modify protected files: {}. Changes reverted.",
                forbidden_files.join(", ")
            ),
            gate_result: None,
            snapshot_id: Some(snapshot_id),
        });
    }

    // Step 3: Gate 验证
    let gate_result = gate_loop(...).await?;

    // Step 4: 更新快照 gate_result
    update_snapshot_gate_result(workspace_path, snapshot_id.clone(), gate_result.clone())?;

    // Step 5: 更新 HEAD
    set_head(workspace_path, snapshot_id.clone())?;

    Ok(PromptResponse { ... })
}
```

**禁止文件列表：**

```rust
const FORBIDDEN_FILES: &[&str] = &[
    "package.json",
    "pnpm-lock.yaml",
    "tsconfig.json",
    "vite.config.ts",
    "index.html"
];
```

---

#### 2. **Gate 验证流程**

1. `pnpm run typecheck`
2. `pnpm run smoke:0`（frame 0 → `out/smoke-0.png`）
3. `pnpm run smoke:mid`（frame 75 → `out/smoke-mid.png`）

**失败处理：**

- 错误摘要截断到 2000 字符
- 回灌 OpenCode："只修复错误，不重构无关代码"
- 等待 30 秒
- 最多重试 2 轮

**成功后：**

- iframe reload
- 写入 `workspace/logs/gate.jsonl`

---

### Benchmark

- AI 尝试修改 `package.json` → 自动回滚到快照 + UI 显示 "🔒 Security blocked"
- 人为制造 TS 错误 → Gate 失败 → 自动修复 → 预览恢复
- 连续失败 2 次 → UI 显示 "Gate Error" + "View Logs" 按钮

---

## Step 9 — 导出视频（统一输出到 out/）

### 要做的事

1. **导出脚本**

   ```json
   {
     "scripts": {
       "export": "pnpm exec remotion render src/Root.tsx Main out/video.mp4 --codec=h264"
     }
   }
   ```

2. **Export 按钮 + Mutex**
   - 并发保护：`ExportState` mutex
   - 确保所有错误路径都重置 flag

3. **UI 状态**
   - `idle` → `exporting` → `done` (显示 "Open Folder") / `error`

4. **错误处理**
   - 所有错误 → `workspace/logs/export.log`

### Benchmark

- 点击 Export → `out/video.mp4` 生成
- 快速连续点击 → 第二次显示 "already in progress"
- 导出失败后再次尝试 → 不被 mutex 卡住

---

## Step 9.5 — 版本历史可视化 UI（Git-like DAG）

### 要做的事

#### 1. **UI 设计（单独页面或侧边栏）**

**视觉元素：**

```
              ○ ← 淡紫色呼吸小圆（未保存状态，无改动时无法保存；有改动时可保存，新出现的矩形框就会在相同位置）
              │   - 轻微阴影
              │   - 呼吸动画（scale 0.95-1.05, 2s循环）
        ┌─────────────┐
        │  Auto 10:30 │  ← 矩形框（自动保存）
        │             │
        └─────────────┘
              │
              │
        ┌─────────────┐
        │  Manual 0   │
        │  "初始版本" │
        └─────────────┘
              │
              ● ← 实心黑点（根节点）

有改动，且保存后：

              ○ ← 淡紫色呼吸小圆
              │
              │
        ┌─────────────┐
        │  Manual 1   │  ← 矩形框（手动保存）
        │  "UI优化"   │
        └─────────────┘
              │
              │
        ┌─────────────┐
        │  Auto 10:30 │  ← 矩形框（自动保存）
        │             │
        └─────────────┘
              │
              │
        ┌─────────────┐
        │  Manual 0   │
        │  "初始版本" │
        └─────────────┘
              │
              ● ← 实心黑点（根节点）
```

**布局：**

- 垂直方向：时间从下到上（最新在上方）
- 水平方向：分支横向展开（DAG）
- Canvas 渲染（推荐 Konva.js 或 React Flow）

---

#### 2. **交互功能**

**A. 手动保存**

- 按钮："Save Version"
- 点击后弹窗：
  ```tsx
  <Dialog>
    <Input label="Version Name" defaultValue={`Manual ${count}`} />
    <Textarea label="Description (optional)" />
    <Button onClick={handleSave}>Save</Button>
  </Dialog>
  ```

**B. 右键菜单**

- 右键快照矩形 → 显示菜单：
  - "Checkout"：跳转到此快照
  - "Rename"：编辑名称
  - "Edit Description"：编辑描述
  - "Delete (and children)"：删除快照及子树

**C. 删除确认**

```tsx
<ConfirmDialog
  title="Delete Snapshot Tree?"
  message={`This will delete "${snapshot.name}" and ${childCount} descendent(s). This action cannot be undone.`}
  onConfirm={() => deleteSnapshotTree(snapshot.id)}
/>
```

**D. Checkout**

- 点击快照 → 显示确认框（如果有未保存修改）
- 确认后 → 调用 `checkout_snapshot` → iframe reload
- 小圆移动到新的 HEAD 位置上方

## **E. 用户手动调整版本在DAG中的位置**

#### 3. **实时状态追踪**

```typescript
// 前端状态
const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false);
const [currentHead, setCurrentHead] = useState<string>("root");

// 监听 workspace 变化（文件 watcher）
useEffect(() => {
  const unsubscribe = listen("workspace:file-changed", () => {
    setHasUnsavedChanges(true);
  });
  return unsubscribe;
}, []);

// 保存后重置状态
const handleSave = async () => {
  const snapshotId = await invoke("manual_save_snapshot", { ... });
  setCurrentHead(snapshotId);
  setHasUnsavedChanges(false);
};
```

---

#### 4. **视觉效果**

**呼吸动画（Framer Motion）：**

```tsx
<motion.circle
  r={8}
  fill="#A78BFA" // 淡紫色
  animate={{
    scale: [0.95, 1.05, 0.95],
    opacity: [0.7, 1, 0.9],
  }}
  transition={{
    duration: 2,
    repeat: Infinity,
    ease: "easeInOut",
  }}
  style={{
    filter: "drop-shadow(0 0 8px rgba(167, 139, 250, 0.6))",
  }}
/>
```

**连接线（Canvas）：**

```typescript
function drawConnection(parent: Node, child: Node) {
  ctx.strokeStyle = "#64748B"; // slate-500
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.moveTo(parent.x, parent.y);
  ctx.lineTo(child.x, child.y);
  ctx.stroke();
}
```

---

#### 5. **Rust Backend 支持**

```rust
#[tauri::command]
fn get_snapshot_dag(workspace_path: String) -> Result<SnapshotDAG, String> {
    let history_dir = Path::new(&workspace_path).join(".epris/history");
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for entry in std::fs::read_dir(history_dir)? {
        let meta_path = entry?.path().join("meta.json");
        let meta: SnapshotMetadata = serde_json::from_str(
            &std::fs::read_to_string(meta_path)?
        )?;

        nodes.push(meta.clone());

        if let Some(parent_id) = &meta.parent_id {
            edges.push(Edge {
                from: parent_id.clone(),
                to: meta.id.clone(),
            });
        }
    }

    Ok(SnapshotDAG { nodes, edges })
}

#[tauri::command]
fn update_snapshot_metadata(
    workspace_path: String,
    snapshot_id: String,
    name: String,
    description: String,
) -> Result<(), String> {
    let meta_path = Path::new(&workspace_path)
        .join(".epris/history")
        .join(&snapshot_id)
        .join("meta.json");

    let mut meta: SnapshotMetadata = serde_json::from_str(
        &std::fs::read_to_string(&meta_path)?
    )?;

    meta.name = name;
    meta.description = description;

    std::fs::write(meta_path, serde_json::to_string_pretty(&meta)?)
        .map_err(|e| e.to_string())
}
```

---

### Benchmark

- 初始状态：黑点 + 紫色小圆（未保存）
- 发送 prompt → 小圆向上移动 + 创建自动快照矩形
- 点击 "Save Version" → 输入名称 "功能完成" + 描述 → 创建矩形框
- 再次修改 → 小圆上移（距离最近快照）
- 右键快照 → Checkout → 预览跳转到该版本
- 右键快照 → Delete → 确认 → 该快照及子树消失
- DAG 分支可视化：从某个快照 checkout 后修改 → 形成新分支

---

### Step 10 — Windows 可安装运行（最小交付闭环）

**要做的事**

1. 一些补充工作（如果之前没有）：
   - 在log里加上每一项的花费时间
   - 对于Open Folder（在export之后），确保能打开对应的output文件夹
2. `tauri build` 产出安装包
3. 在“非开发环境”机器上验证（至少做到：能启动 + 能预览 + 能 prompt 改码 + 能导出）
   - 如果 V0 仍依赖用户机器 Node/pnpm/ffmpeg：在验证文档里明确写前置条件
4. 将安装包放到 public releases repo（V0 内测先手动分发即可）

**Benchmark**

- 朋友机器（满足前置条件）安装后：打开 App -> 看到预览 -> 输入 prompt -> 预览变化 -> 导出成功

---

## V0 的 Benchmarks 汇总（验收清单）

- [ ] App 内（非浏览器）可播放 Remotion Player
- [ ] App 启动自动创建 workspace
- [ ] App 能加载 workspace 的 composition 并在代码变化后刷新
- [ ] prompt -> OpenCode 改 workspace -> 预览出现变化
- [ ] 同一 session 下支持连续 prompt 增量修改
- [ ] 有最小验证与自动修复循环
- [ ] 所有关键过程有日志可复现（prompt、改动摘要、错误栈）

---

## 你在 Antigravity 的工作方式（固定套路）

每个 Step 开工 prompt（建议复制使用）：

1. "先读 task_plan.md/findings.md/progress.md。为 Step X 写文件级计划（含验证命令），写入 task_plan.md，然后停下。"
2. 你确认后："按计划实现，每完成一个小任务就跑验证并把结果写入 progress.md。任何坑写入 findings.md。"

---
