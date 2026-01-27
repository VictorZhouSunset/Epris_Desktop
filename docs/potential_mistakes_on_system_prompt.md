之前发生过多次这样的问题，请你复核现在是对的。

你现在的 system prompt 实际上被丢了

你在创建 session 时发的是：

client.post(format!("{}/session", base_url))
.json(&json!({ "system": system_prompt }))

但 OpenCode Server 的 POST /session 只接受 { parentID?, title? }，并不包含 system 字段。也就是说你这里的 system_prompt 不会进入会话。

真正支持 system 的是：

POST /session/:id/message 的 body 里可以带 system?（以及 model?, agent?）。

修法（Server 模式）

把 system 从 “create session” 挪到 “send message”。

创建 session：

let res = client.post(format!("{}/session", base_url))
.json(&serde_json::json!({}))
.send().await?;

发第一条消息：

let res = client
.post(format!("{}/session/{}/message", base_url, sid))
.json(&serde_json::json!({
"system": system_prompt,
"parts": [{ "type": "text", "text": prompt }]
}))
.send().await?;

如果你希望“每次请求都强制带规则”，就每次 message 都带上 system（不要只在首次带）。因为你现在还会 Reusing existing session，一旦规则变了、workspace_path 变了、或者你怀疑 server 有缓存行为，只在首次注入会让你以为“又失效了”。

2. 你的模型选择在两条链路里都容易失效
   2.1 Server 模式：你根本没传 model

你发 message 时只发了 parts：

.json(&json!({ "parts": [...] }))

但 server API 支持 model? 字段。
你应该至少这样：

.json(&serde_json::json!({
"system": system_prompt,
"model": desired_model, // 例如 "anthropic/claude-sonnet-4-20250514"
"agent": desired_agent, // 如果你用自定义 agent
"parts": [{ "type": "text", "text": prompt }]
}))

不过有个坑：社区里有人反馈 Server API 在某些情况下会忽略 agent 配置的默认 model，回退到全局默认。
所以更稳的策略是：

server 启动前把默认 model 写进 opencode.json（或你自己的环境配置），让 server 的全局默认就是你想要的；--model 的优先级也高于配置。

同时在 message 里也显式传 model，双保险。

2.2 CLI 模式：你只给 Gemini 拼了 system prompt，没给 OpenCode 传任何 model/system

你的 run_provider_cli 里：

provider_name == "gemini"：你把 OPENCODE_SYSTEM_PROMPT 拼进了 prompt（OK）

否则（比如你用 "opencode" CLI）：你就只 args.push(prompt)，既没 system，也没 --model。

而 OpenCode 的模型选择是 --model / -m（格式 provider/model）。

修法（CLI 模式）：在 provider_name == "opencode" 分支里追加：

args.push("--model".to_string());
args.push(desired_model.to_string()); // 例如 "anthropic/claude-sonnet-4-20250514"

如果你还要强制规则，CLI 模式下也别只靠“拼进用户 prompt”；更推荐用 OpenCode 的 agent/rules 体系（或至少确保每次都 prepend 一段固定指令）。
