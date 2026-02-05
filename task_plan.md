- [x] Confirm current baseline install flow (wizard vs auto-link)
- [x] Stop silent baseline installs in `link_workspace_dependencies`
- [x] Add backend API to compute missing baseline packages + signature
- [x] Add backend API to persist “baseline deps acknowledged” signature
- [x] Add backend API to install baseline deps (workspace + template) when user approves
- [x] Update frontend to prompt once per signature on app update
- [ ] Verify new-project flow: no wizard; shows progress; links deps
- [ ] Verify update flow: asks once when baseline changes; no repeated prompts

---

## V1 Draft — UI Agent (Right Panel Prompt + Prop Controls)

References:
- `docs/plans/2026-02-05-v1-ui-agent-design.md`
- `docs/plans/2026-02-05-v1-ui-agent-implementation-plan.md`

- [ ] Confirm V1 contracts: `src/epris-controls.json` + `src/epris-props.json` (+ `id` regex, + target scope)
- [ ] Persistence: explicit Save (draft in memory; write on Save/export)
- [ ] Add workspace-template stubs (controls/props files + Preview postMessage + Root defaultProps)
- [ ] Add workspace-template Route-B primitives (optional wrappers + registry)
- [ ] Backend IPC: read/write controls + props
- [ ] Frontend: control registry (slider/color/select/toggle/text)
- [ ] Frontend: right panel UI Agent prompt + trigger provider run
- [ ] Preview live update via iframe postMessage
- [ ] Update provider rules (REMOTION.md generator) for prop-first + controls/props contract
- [ ] DAG: Save props creates snapshot; export auto-save & export
- [ ] Manual tests: new project, UI agent prompt, export matches props
