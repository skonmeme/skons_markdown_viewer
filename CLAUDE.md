# CLAUDE.md

Behavioral guidelines to reduce common LLM coding mistakes. Merge with project-specific instructions as needed.

**Tradeoff:** These guidelines bias toward caution over speed. For trivial tasks, use judgment.

## 1. Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:
- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them - don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

## 2. Simplicity First

**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

## 3. Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:
- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it - don't delete it.

When your changes create orphans:
- Remove imports/variables/functions that YOUR changes made unused.
- Don't remove pre-existing dead code unless asked.

The test: Every changed line should trace directly to the user's request.

## 4. Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:
- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:
```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

Strong success criteria let you loop independently. Weak criteria ("make it work") require constant clarification.

---

**These guidelines are working if:** fewer unnecessary changes in diffs, fewer rewrites due to overcomplication, and clarifying questions come before implementation rather than after mistakes.

---

## Specification process

`spec/STANDARD.md` (STANDARD) is the only specification of this project. This section repeats STANDARD §11.3 so that it is always followed. If the two differ, STANDARD is correct; whenever §11.3 changes, this section is changed with it.

STANDARD is changed only through specification requests. The requester asks for changes and makes the decisions; Claude carries out the four steps below. No work is done on the `main` and `develop` branches; all work for a request is done on the branch that holds the request.

**Step 1. Managing a specification request**

1. When the requester asks for a request, Claude creates `spec/requests/NNN-title.md` on the branch that is currently checked out. `NNN` is the next three-digit number across `spec/requests/` and `spec/merged/`, and `title` is a short English name.
2. Claude never creates branches. If the current branch is `main` or `develop`, Claude does not create the request and asks the requester to change the branch.
3. A branch may hold several requests when needed.
4. A request has these sections:

| Section | Content |
|---|---|
| Request | The requester's original request, verbatim |
| Decisions | For each decision: its content (concrete values, formulas, behavior), its source (the requester's words, verbatim) and its reason. A proposal by Claude that the requester has not explicitly approved is marked as such. |
| Resulting specification | What STANDARD will contain as a result of the decisions |
| Impact | Changes visible to users, and the sections and figures of STANDARD, source code, tests and documents that change |
| Open questions | Points that have not been decided yet |

5. Whenever a decision is made or changed in the conversation, Claude updates the request at once. Decisions, calculated results and proposals are never kept only in the conversation. When a decision changes, the earlier statement stays and the new one is added.
6. A request is committed only when the requester says so, or at the start of step 2.

**Step 2. Updating STANDARD**

1. Claude updates STANDARD only when the requester explicitly instructs it. If the request still has open questions, Claude asks the requester first.
2. If the request has uncommitted changes, Claude commits the request first. The instruction to update STANDARD also counts as the instruction to commit the request.
3. Every decision of the request is reflected, and nothing unrelated to the decisions is changed.
4. STANDARD is written in English, and its figures in `spec/STANDARD/` are changed to match.
5. If a decision conflicts with another part of STANDARD, Claude stops and asks the requester.

**Step 3. Implementation and follow-up work**

1. Step 3 may start right after the update of STANDARD has been committed. Steps 2 and 3 are never done at the same time.
2. The source code and tests are always made to match STANDARD. If the implementation turns out to need a change of STANDARD, Claude asks the requester and continues only after the answer.
3. `cargo test --workspace` must pass.
4. The documents are brought in line with STANDARD: `README.md`, `doc/ARCHITECTURE.md` and `doc/ARCHITECTURE/`, `doc/DEVELOPMENT/` (a topic to discuss is added, and a resolved topic is deleted), the Help (`packaging/help/`), and this file when STANDARD §11.3 changes.
5. When everything is done, the request is moved to `spec/merged/`.

**Step 4. Commits**

1. A request is committed only as described in step 1.
2. The results of steps 2 and 3 are committed automatically on the working branch if no problem occurs. The commit of step 2 is made before step 3 starts. If the build or a test fails, or a conflict within STANDARD appears, nothing is committed and the requester is told.
3. Commits are made only on the working branch. Pushing, merging into `develop` and pull requests to `main` happen only when the requester instructs them (STANDARD §11.4).
4. Commit messages have this form:

| Work | Commit message |
|---|---|
| Adding a request | `spec: add request NNN-title` |
| Updating a request | `spec: <change> to request NNN-title`, for example `spec: add decisions to request 002-package` |
| Updating STANDARD | `spec: update STANDARD.md from request NNN-title` |
| Implementation | `Implement STANDARD from request NNN-title` |
| Documents | `docs: <change>` |
| Moving a request | `spec: move request NNN-title to merged` |
