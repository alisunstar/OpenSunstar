## Description: <br>
Product analytics with your AI agent: set up consent-based tracking, read funnels, paths, retention, experiments, and context, then recommend the smallest growth action using the official Agent Analytics CLI. <br>

This skill is ready for commercial/non-commercial use. <br>

## Publisher: <br>
[dannyshmueli](https://clawhub.ai/user/dannyshmueli) <br>

### License/Terms of Use: <br>
MIT-0 <br>


## Use Case: <br>
Developers, product engineers, and growth teams use this skill to add consent-based Agent Analytics tracking, query funnels, paths, retention, experiments, sessions, and context, then choose a narrow growth action. It is intended for products the user owns or manages. <br>

### Deployment Geography for Use: <br>
Global <br>

## Known Risks and Mitigations: <br>
Risk: The skill runs an external analytics CLI through npx and uses browser-based account approval. <br>
Mitigation: Use the pinned Agent Analytics CLI command path, approve the account in the browser, and verify the active identity before setup or analytics reads. <br>
Risk: Local auth configuration can be stored in the workspace. <br>
Mitigation: Keep .openclaw/agent-analytics/config.json out of git and use a dedicated config directory for managed runtimes. <br>
Risk: Tracking changes may collect events that do not match the user's product goals. <br>
Mitigation: Confirm the project goal before storing context, inspect the product workflow, add only the smallest meaningful event set, and review tracker events before deployment. <br>


## Reference(s): <br>
- [Agent Analytics ClawHub page](https://clawhub.ai/dannyshmueli/skills/agent-analytics) <br>
- [Agent Analytics homepage](https://agentanalytics.sh) <br>
- [Projects, surfaces, and portfolios guide](https://docs.agentanalytics.sh/guides/projects-surfaces-portfolios/) <br>
- [setup-auth.md](references/setup-auth.md) <br>
- [product-analytics-operating-model.md](references/product-analytics-operating-model.md) <br>
- [growth-recipes.md](references/growth-recipes.md) <br>


## Skill Output: <br>
**Output Type(s):** [Guidance, Shell commands, Code, Configuration, Markdown] <br>
**Output Format:** [Markdown guidance with inline shell commands, code snippets, and analytics recommendations] <br>
**Output Parameters:** [1D] <br>
**Other Properties Related to Output:** [Uses npx and the pinned @agent-analytics/cli@0.5.34 command path; may propose tracker and event instrumentation for user review.] <br>

## Skill Version(s): <br>
4.0.34 (source: server evidence release.version and SKILL.md frontmatter) <br>

## Ethical Considerations: <br>
Users should evaluate whether this skill is appropriate for their environment, review any generated or modified files before relying on them, and apply their organization's safety, security, and compliance requirements before deployment. <br>
