## Description: <br>
Modelscope Api helps agents search, query, and download ModelScope models, datasets, skills, MCP server configurations, and Studio resources. <br>

This skill is ready for commercial/non-commercial use. <br>

## Publisher: <br>
[chenghd511](https://clawhub.ai/user/chenghd511) <br>

### License/Terms of Use: <br>
MIT-0 <br>


## Use Case: <br>
Developers and agent users use this skill to work with ModelScope catalog APIs, find models, datasets, skills, and MCP servers, and retrieve commands or configuration needed for downloads and client setup. Operations that change remote resources, install components, or handle secrets should be reviewed and explicitly approved before use. <br>

### Deployment Geography for Use: <br>
Global <br>

## Known Risks and Mitigations: <br>
Risk: ModelScope API tokens and other credentials may be exposed if pasted into shared chats or logged shell commands. <br>
Mitigation: Use a least-privilege MODELSCOPE_API_TOKEN environment variable and avoid sharing real tokens in chat transcripts or command history. <br>
Risk: Install, MCP configuration, deploy, delete, or secret-changing actions can modify local or remote resources. <br>
Mitigation: Review the exact target and command, then require explicit user approval before executing any change operation. <br>


## Reference(s): <br>
- [ClawHub skill page](https://clawhub.ai/chenghd511/modelscope-api) <br>
- [Publisher profile](https://clawhub.ai/user/chenghd511) <br>
- [Project homepage](https://github.com/Chenghd511/modelscope-api-skill) <br>
- [Support issues](https://github.com/Chenghd511/modelscope-api-skill/issues) <br>
- [ModelScope access token page](https://www.modelscope.cn/my/overview) <br>


## Skill Output: <br>
**Output Type(s):** [text, markdown, code, shell commands, configuration, guidance] <br>
**Output Format:** [Markdown with JSON, Python, and shell command snippets] <br>
**Output Parameters:** [1D] <br>
**Other Properties Related to Output:** [May include ModelScope API response summaries, MCP client configuration, and token setup guidance.] <br>

## Skill Version(s): <br>
1.0.4 (source: server release metadata and artifact metadata) <br>

## Ethical Considerations: <br>
Users should evaluate whether this skill is appropriate for their environment, review any generated or modified files before relying on them, and apply their organization's safety, security, and compliance requirements before deployment. <br>
