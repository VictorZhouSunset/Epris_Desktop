# Epris V0.5 Step-by-step 开发计划（Windows 优先）

目标：把 V0 从“本地闭环可用”推进到“可以发给朋友试用 + 客户端自动更新 + 基础沙盒更严格”。

---

## 0.5 验收标准（Definition of Done）

1. 你发布 v0.5.0 安装包给朋友；之后发布 v0.5.1 到 GitHub Releases。
2. v0.5.0 客户端能检测到更新并升级到 v0.5.1。
3. 更新过程可见状态与错误信息；失败不影响打开旧版本与旧项目。
4. 用户项目数据不会因更新丢失或被覆盖。
5. OpenCode/AI 只能读写“当前项目子文件夹”（projectRoot），不能越界到其他项目或总目录。

---

## 目录与边界约定

- **App 数据根目录**：放在 OS 的应用数据目录（Tauri `appDataDir`），避免更新时被覆盖。
- **总文件夹（projectsRoot）**：存放项目列表、日志索引、缓存。
- **项目目录（projectRoot）**：每个项目一个子文件夹，包含 Remotion 工程与 node_modules。
- **会话边界**：所有 AI/预览/渲染/编译都绑定到某个 projectRoot。

建议目录结构：

```
<appDataDir>/
  epris/
    projects/
      <projectId-1>/
        (remotion project...)
      <projectId-2>/
    registry.json (或 registry.sqlite)
    logs/
      <projectId>/<sessionId>/...
    cache/
      browsers/ (若需要)
```

---

## 里程碑与步骤

### Step 0：版本与可复现基线（0.5.0 前置）

**目标**：确保两台机器构建与渲染结果一致，避免朋友机器“依赖漂移”。

**任务**
- 锁定关键依赖版本：Remotion 及 `@remotion/*` 版本完全一致，去掉 `^`。
- 锁定 Node 版本（例如 `.nvmrc` / `.tool-versions` / 文档）。
- 项目元数据增加 `schemaVersion`，为未来迁移做准备。

**产出物**
- `docs/versioning.md`（版本策略与兼容说明）
- `package.json` 版本锁定提交

**验收**
- 两台 Windows 机器：`pnpm i` + 一次 smoke render 输出一致。

---

### Step 1：接入 Tauri Updater（客户端）

**目标**：客户端具备“检查更新/下载/安装”的能力，更新源为 GitHub Releases + `latest.json`。

**任务**
- 安装/启用 Updater 插件。
- 配置 endpoints 指向：
  - `https://github.com/<org>/<repo>/releases/latest/download/latest.json`
- Windows 安装模式建议 `passive`，减少强打断。
- 开启 `bundle.createUpdaterArtifacts = true` 生成更新产物。

**产出物**
- `src-tauri/tauri.conf.json`（或等价配置）
- Updater 公钥配置（pubkey）

**验收**
- 本地 release build 成功且启动无 updater 配置报错。

---

### Step 2：生成签名密钥与本地验证

**目标**：更新包可被签名并被客户端校验。

**任务**
- 用 Tauri CLI 生成 updater 签名密钥对。
- 公钥写入 `plugins.updater.pubkey`。
- 本地构建一次，验证更新产物生成与签名文件存在。

**产出物**
- `pubkey` 写入配置
- 密钥管理说明（不要把私钥提交到仓库）

**验收**
- 本地生成 updater artifacts；客户端校验通过（可先在 dev 日志里确认）。

---

### Step 3：Capabilities 收紧（前端权限面）

**目标**：前端无法任意执行系统命令；只能调用你明确开放的后端命令。

**任务**
- 为主窗口定义 capability：仅授予必需权限。
- 避免 `shell:allow-all` / `process:allow-all`。
- Updater 插件按需加入 `updater:default`。

**产出物**
- `src-tauri/capabilities/*.json`
- 安全基线文档：`docs/security-baseline.md`

**验收**
- 前端无法直接执行任意命令；只能通过你定义的 `invoke` API。

---

### Step 4：更新 UI 状态机（体验层）

**目标**：用户能看到更新状态；失败不影响主流程。

**任务**
- 在设置页或顶部菜单加：
  - 当前版本号
  - “检查更新”按钮
  - 状态显示：checking / available / downloading / installing / done / error
- 错误处理：网络失败、签名失败、下载失败、安装失败。
- 所有错误写入日志。

**产出物**
- `src/ui/update/UpdatePanel.tsx`（示例命名）
- `src/lib/updater.ts`（封装 updater 调用）

**验收**
- 断网/返回 404 时 UI 给出明确错误且不崩溃。

---

### Step 5：GitHub Actions 自动打包与发布 Release（流水线）

**目标**：打 tag/推送 release 分支即可自动构建并生成 Release 资产 + `latest.json`。

**任务**
- 使用 `tauri-apps/tauri-action` 构建 Windows 安装包。
- 配置环境变量：
  - `TAURI_SIGNING_PRIVATE_KEY`
  - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
- `uploadUpdaterJson: true` 自动上传 `latest.json`。
- Release 流程建议：先 draft，验收后再 publish。

**产出物**
- `.github/workflows/release.yml`
- GitHub Secrets 配置说明：`docs/release-secrets.md`

**验收**
- CI 跑完后 Release 中包含：安装包、签名相关文件、`latest.json`。

---

### Step 6：端点可用性与版本演练（更新闭环）

**目标**：真实演练一次 0.5.0 -> 0.5.1 更新。

**任务**
- 发布 v0.5.0（draft -> publish）。
- 安装 v0.5.0。
- 发布 v0.5.1。
- 在 v0.5.0 客户端检查更新并升级。

**产出物**
- `docs/update-drill.md`（演练记录：发生了什么、踩坑与修复）

**验收**
- v0.5.0 能升级到 v0.5.1，升级后能打开旧项目。

---

### Step 7：项目数据落盘位置确认（避免更新覆盖）

**目标**：用户项目与日志永远不放在安装目录。

**任务**
- 将 projectsRoot 定位到 `appDataDir`。
- 如果当前实现用的是“用户随便选的一个总目录”：
  - 允许，但要在文档中明确推荐路径，并保证 updater 不触碰。
- 为总目录加入：
  - `registry.json`（项目列表）
  - `logs/` 统一落盘

**产出物**
- `src-tauri/src/storage.rs`（示例）
- `docs/data-layout.md`

**验收**
- 更新后 projectsRoot 不变；项目不丢失。

---

### Step 8：AI CLI 沙盒收紧到 projectRoot（V0.5 关键）

**目标**：AI 只在“当前项目子文件夹”工作，不能跨项目、不能写 node_modules。

**任务（按优先级）**
1) **进程边界**
- 启动 OpenCode 时：`cwd = projectRoot`。
- 不把 projectsRoot 作为可写路径暴露给 agent。

2) **工具边界（强制）**
- 你提供给 agent 的文件读写工具全部走后端 gate：
  - 路径 `canonicalize`
  - 必须 `startsWith(projectRoot)`
  - 禁止写入：`**/node_modules/**`、`**/.git/**`（按你的策略）

3) **审计日志**
- 记录每次：prompt、改动文件列表、tsc 错误、渲染错误、越界拒绝。

**产出物**
- `src-tauri/src/sandbox.rs`（路径白名单/黑名单）
- `docs/sandbox.md`

**验收**
- 恶意路径（如 `../other-project/...`）写入被拒绝且有日志。
- node_modules 写入被拒绝。

---

### Step 9：稳定性补齐（朋友试用的必需细节）

**目标**：朋友机器上“装完就能用”。

**任务**
- 首次启动向导：创建/选择 projectsRoot，创建 demo 项目。
- 预检：Node 可用、依赖齐全。
- Remotion 渲染依赖：首次渲染若需要下载浏览器，显示进度并缓存到 appData。
- Crash 保护：后台进程异常退出时，UI 给出可恢复提示。

**产出物**
- `docs/first-run.md`
- 一键诊断日志导出按钮

**验收**
- 新机器从 0 到完成一次 smoke render ≤ 5–10 分钟（取决于下载）。

---

## 推荐的发布节奏（10 天）

- D1：Step 0
- D2–D3：Step 1–2
- D4：Step 3–4
- D5–D6：Step 5
- D7：Step 6
- D8：Step 7
- D9：Step 8
- D10：Step 9 + Bugfix

---

## 风险清单（提前规避）

1. **把 projectsRoot 放在安装目录旁**：Updater 更新可能覆盖/权限冲突。必须放 appDataDir 或用户自选目录。
2. **node_modules 被写入或进入版本历史**：会导致 diff 爆炸与不可复现。必须在写工具与日志/版本系统里忽略。
3. **前端权限过大**：一旦前端可直接 shell，沙盒等于没有。必须靠 capabilities 收紧。
4. **依赖漂移**：Remotion 与 renderer/player 版本不一致会出现奇怪错误。必须锁版本。

---

## 附：最小清单（你可以按这个顺序打勾）

- [ ] 锁定 Remotion 依赖版本
- [ ] Updater 插件接入 + endpoints 配置
- [ ] 生成签名 key，公钥进配置，私钥进 CI secrets
- [ ] Capabilities 收紧：禁用 shell/process allow-all
- [ ] 更新 UI：检查/下载/安装状态
- [ ] GitHub Actions：自动构建 release + 上传 latest.json
- [ ] 演练 0.5.0 -> 0.5.1
- [ ] projectsRoot 落盘到 appDataDir
- [ ] projectRoot 沙盒：路径白名单 + 禁写 node_modules
- [ ] 首次启动向导 + 诊断日志导出

