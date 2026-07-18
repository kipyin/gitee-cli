# README: platform limits + beyond-parity docs

Status: ready-for-agent

## Context
Prevents duplicate feature requests and sets expectations vs gh.

## Scope
- README section "与 gh 的差异": list spec.md's platform-blocked items
  (no Gitee Go public API → no workflow/run/checks; no issue transfer/pin/lock/delete;
  no repo archive; no codespaces/projects).
- Section "Gitee 特色": pr test (审查/测试双轨), issue priority/security_hole,
  star/watch, milestone, webhook — as those tickets land (mark 未实现 items as planned).
- Keep in sync with spec.md; single short section, not a full comparison essay.

## Acceptance
- README renders on gitee.com; blocked list matches spec.md exactly.
