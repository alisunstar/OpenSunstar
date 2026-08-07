# Product analytics operating model

Use this reference when setup, instrumentation, project context, or cross-project scope needs more detail.

## Projects, surfaces, and portfolios

- Project: unit of local product learning. It owns events, activation, retention, lifecycle, releases, experiments, goals, and context.
- Surface: a place users encounter or use that project, such as app, docs, blog, pricing, signup, onboarding, subdomain, mobile client, local preview, or deploy preview.
- Portfolio: related projects in one growth system. It supports shared goals, roles, milestones, and configured cross-project identity without collapsing project-local truth.

Subdomains are usually surfaces. Localhost, staging, and previews are setup or QA surfaces. Separate products that need separate activation, lifecycle, or experiments should be separate projects under one portfolio.

Customers define their own project and portfolio goals. Agents can propose goals from the repo and workflow context, but initial durable goals should be confirmed before storing them. Later goal or context updates can follow the user's working agreement with their agent.

## Consent-based instrumentation

Install only for projects the user owns or manages. Treat the base snippet as the start of instrumentation, not the full plan.

Minimum workflow:

1. Classify scope.
2. Ask for or propose the initial project/portfolio goal and get confirmation before storing it as durable context.
3. Inspect product code when available.
4. Login and create or identify the right project.
5. Add the exact tracking snippet returned for that project.
6. Choose the smallest meaningful custom event set tied to the confirmed goal.
7. Explain what each event enables.
8. Verify the first useful event.

Good event candidates: named CTA clicks, signup intent, pricing interactions, checkout progress or completion, install/setup steps, activation milestones, and durable server-side outcomes such as signup, subscription, project creation, installation completion, or first useful product action.

Use `data-aa-event` for named click intent, `data-aa-impression` for meaningful section exposure, `window.aa.track(...)` for computed client state, server-side tracking for durable outcomes, and identify/set calls after auth. Enable scroll depth, forms, downloads, vitals, errors, performance, or SPA tracking only when tied to a concrete decision.

Do not duplicate automatic signals: page views, path, referrer, UTM fields, device/browser, country, sessions, first-touch attribution, or days since first visit.

## Context upkeep

Read context before project-specific analysis. Write context only when durable product truth changed.

Save:

- goals
- activation events
- event-name glossary entries tied to real `event_name` values
- sparse annotations for major landing page, pricing, onboarding, feature, release, or experiment changes

Skip:

- weekly metric values
- temporary spikes
- pasted reports
- raw autoresearch round notes
- PII or secrets
- long notes
- git commit logs
- unsupported guesses

`context set` replaces context; read first, merge carefully, and keep valid entries. Do not invent fields outside goals, activation_events, glossary, and annotations.

## Cross-project identity

For identity-aware reads across related projects, configure both browser and server sides:

- tracker `data-link-domains` so anonymous IDs can move through links
- portfolio membership with `portfolios create` or `portfolios update`

Either side alone is incomplete. Do not claim strict user conversion across projects unless both are configured and the metric definition says so.

## Verification checklist

- Auth is valid.
- Project and allowed origins match the intended product and surface.
- Snippet belongs to the user's project.
- First useful event arrived.
- Context remains compact and durable.
- Final analysis includes decision, metric definition, evidence, caveat, and next action.
