# Steps 8-9.5 完整规划（调整顺序 + 版本历史 UI）

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

#### 7. **自动清理策略（保留重要快照）**

```rust
fn cleanup_old_snapshots(workspace_path: &str) -> Result<(), String> {
    // 清理规则：
    // - 保留所有手动保存的快照
    // - 自动快照只保留最近 10 个
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
- 创建 12 个自动快照后，未被引用的旧快照自动删除（保留 10 个）
- 手动快照永不自动删除

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

---

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

## 总结：步骤依赖关系

```
Step 8: Snapshot 系统（基础设施）
  ↓
Step 8.5: Gate（依赖 rollback）
  ↓
Step 9: Export
  ↓
Step 9.5: 版本历史 UI（依赖 Step 8 的所有 API）
```

这样调整后，逻辑清晰，且 UI 可以在基础功能完成后作为增强功能逐步实现！
