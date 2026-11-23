# Tasks

## 1. Compact transcript setting

- [x] 1.1 Add `UiFileConfig.compact_transcript: Option<bool>` and `UiConfig.compact_transcript: bool` with `false` as the default; verify default and parsing with `compact_transcript` tests in `cargo nextest run -p maki-config`
- [x] 1.2 Generate config docs for `ui.compact_transcript` and verify `just gen-docs-check` passes

## 2. Fold plan and summary

- [x] 2.1 Add fold plan types and a `fold_plan` pass that groups maximal runs of `Thinking` and foldable `Tool` messages; verify tests cover assistant text, user messages, notices, `question`, background tasks, single-item runs, and full-mode empty plans
- [x] 2.2 Add `summarize_fold` with command/edit/thought/read/tool/instruction categories, finished-only counting, distinct instruction paths, and correct singular/plural forms; verify table tests for mixed, singular, running, and duplicate-instruction cases
- [x] 2.3 Add the summary line builder with maki's green dot, `−`, a red dot, busy status, right-aligned turn usage, and the tool row background; verify rendered tests for collapsed, expanded, running-before-finish, error, usage, and background cases
- [x] 2.4 Add `Segment.fold` and make a summary segment identifiable as clickable and searchable; verify the segment carries the fold key while normal segments do not

## 3. Cache integration and live updates

- [x] 3.1 Implement the compact incremental cache path: collapsed folds push one summary and skip hidden segments, spacers, and instruction reservations; expanded folds push normal rows; verify compact and full render the same content when all folds are expanded
- [x] 3.2 Patch the summary on `tool_start`, `tool_done`, annotations, usage stamps, and background changes, and skip summary rebuilds from `tool_output`; verify counts and states update once per event class
- [x] 3.3 Handle folds that grow while live: extending the last collapsed or expanded fold, and opening a new fold after a boundary; verify new rows appear in expanded folds and counts update in collapsed ones
- [x] 3.4 Invalidate from the fold start when a folded tool gains a background badge or a boundary changes; verify a moved task splits the run and stays visible
- [x] 3.5 Clear fold and expansion state on `load_messages`/`replace`; verify reloaded sessions fold correctly and a default panel keeps today's segments
- [x] 3.6 Fold a streaming thought into its run's summary instead of drawing a separate reasoning block, keeping raw rows only in full mode or an expanded run; verify the summary live-updates, the live row disappears, and a reset drops the live marker

## 4. Expansion, collapse, and scroll

- [x] 4.1 Add `SegmentCache::truncate_for_rebuild` and implement click-to-expand/collapse through `Segment.fold`; verify clicking a summary shows the same rows full mode renders, and clicking again hides them
- [x] 4.2 Keep the clicked summary on its row and disable auto-scroll when expanding at the bottom; verify an at-bottom expanded run leaves the summary visible and a scrolled-up run keeps its position
- [x] 4.3 Preserve existing per-row behavior inside expanded folds: thinking tails, tool output truncation, tool clicks, and instruction sub-rows; verify each still toggles independently

## 5. Search over folded activity

- [x] 5.1 Include hidden message text, full tool output, and instruction text in a folded summary's search corpus; verify search finds text that exists only inside a folded tool
- [x] 5.2 Expand a folded run when its summary is selected from search and scroll to the run; verify the selected match becomes visible and full-mode search results are unchanged

## 6. App wiring

- [x] 6.1 Build every chat's panel from `UiConfig.compact_transcript`, including subagent tabs and later chats; verify a config-built compact app and a default app produce identical provider messages and inputs
- [x] 6.2 Remove the `/transcript` command, state file, flash message, and splash tip; verify the palette and generated docs no longer list them

## 7. Integration verification

- [x] 7.1 Verify compact mode is display-only: the same messages and tool definitions reach the provider and the saved session output is identical to full mode
- [x] 7.2 Run `just check`, `just lint`, and the affected `maki-ui` and `maki-config` tests; fix failures before handoff
- [x] 7.3 Run `just test` and confirm the workspace suite passes with no new flaky tests

## Workflow follow-up

- Implement from these tasks with the OpenSpec apply workflow; explore mode does not implement.
- Archive the change after review and verification.
- Verify the archived spec and tasks state.
