# Growth recipes

Use this reference for broad product-growth strategy, experiment framing, and structured diagnosis. These frameworks are recipes for the main skill, not separate skills.

## Core analytics loop

1. Resolve project and context.
2. Define the product question and activation or business goal event.
3. Discover real events and properties before filtering.
4. Diagnose leakage or movement with fixed Agent Analytics commands.
5. Segment the largest practical driver.
6. Recommend one narrow experiment or one readiness fix.
7. State how the result will be measured.

## AARRR

Use AARRR to keep lifecycle diagnosis complete:

- Acquisition: qualified visitors by source, campaign, entry page, or partner surface.
- Activation: first value, setup completion, project creation, first useful event, or product-specific activation.
- Retention: cohort return to the activated behavior at the same cohort age.
- Referral: invites, shared links, forwarded reports, embedded widgets, or qualified outbound loops.
- Revenue: trial-to-paid, checkout completion, subscription start, expansion, or payback.

Do not optimize acquisition volume when activation or retained activated users are the constraint.

## Bullseye

Use Bullseye for channel bets:

1. Outer ring: list plausible channels without judging too early.
2. Middle ring: select channels that can plausibly deliver activated users, not just signups.
3. Inner ring: run small tests with a defined input metric, activation metric, guardrail, and stopping rule.

Rank channels by activated users, retained activated users, revenue, payback, or qualified downstream behavior when available.

## AIDA

Use AIDA for landing page and funnel copy:

- Attention: does the entry page make the right user stop?
- Interest: does it connect to their problem and current workflow?
- Desire: does it show credible value, proof, and stakes?
- Action: is the next step clear, low-friction, and measurable?

Map each stage to tracked events or page behavior before recommending copy tests.

## STP

Use STP to avoid generic experiments:

- Segmentation: split by user job, source, company size, runtime, plan, role, or intent.
- Targeting: pick the segment where a change can plausibly improve the goal event.
- Positioning: state the promise, proof, and comparison that should make that segment act.

Do not average away a segment-specific activation bottleneck.

## JTBD

Use Jobs To Be Done when the funnel says what happened but not why:

- Situation: what triggered the visit or setup attempt?
- Motivation: what progress does the user want?
- Anxiety: what risk or confusion blocks action?
- Alternative: what workaround or competitor are they using?
- Outcome: what first value proves progress?

Turn JTBD hypotheses into measurable copy, onboarding, or event changes.

## 4Ps

Use the 4Ps for offer and go-to-market diagnosis:

- Product: value delivered, activation path, proof, and usability.
- Price: packaging, trial, plan clarity, checkout friction, and payback expectation.
- Place: channels, surfaces, integrations, marketplaces, docs, and partner paths.
- Promotion: message, creative, content, comparison pages, launch moments, and campaigns.

Tie each P to an observable event, property, or experiment guardrail.

## Funnel readout recipe

Use `funnel` for ordered leakage. Include population, window, identity basis, step names, counts, rates, strict survivors, largest absolute loss, largest relative loss, and a caveat. Use `paths` to understand session-local detours and `breakdown` to find where the leak is concentrated.

## Retention recipe

Use cohorts, not blended activity. Define the activated cohort, return behavior, interval, cohort ages, and right-censoring. Compare cohorts at the same age. Prefer retained activated users over raw active users.

## Experiment recipe

Read against the business goal event, not exposure count. Include exposure and conversion counts, guardrails, sample size, uncertainty, practical significance, and whether there is enough evidence to keep running, change, stop, or complete.

## Copyable prompt

```text
Use Agent Analytics to diagnose where <project> loses users before activation. Start from project context, use configured activation events as source of truth, discover real events and properties, run funnel and paths, break down the largest leak, inspect representative journeys/events only if useful, then recommend one narrow experiment or the readiness fix that blocks readout. Use the pinned official CLI path and answer with diagnosis, metric definition, evidence, segment/surface, caveat, and one next action.
```
