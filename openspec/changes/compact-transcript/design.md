# Design

## Context

See `proposal.md` for motivation and `specs/transcript/spec.md` for the behavior
contract.

The TUI transcript is a flat `Vec<DisplayMessage>` in `MessagesPanel`
(`maki-ui/src/components/messages/mod.rs`). Each message becomes one segment in
`SegmentCache`, plus two reserved segments for a tool's instruction blocks. Roles
are `User`, `Assistant`, `Thinking`, `Tool`, `Notice`, `Error`, and `Done`.

Several paths assume the message-to-segment mapping:

- `tool_start`, `tool_output`, `tool_done`, snapshots, annotations, usage stamps,
  and background badges look up a tool by id and rebuild its segment.
- Thinking and notices expand through their message index.
- Search returns a segment index that `scroll_to_segment` consumes.
- Selection, copy, images, reflow, and scroll anchoring work on segment indexes.

`UiConfig` is resolved once at startup and cloned into each `Chat`. Compact mode
is a startup-only density setting like `ui.thinking_lines` and
`ui.tool_output_lines`: it is read when a panel is built, so changing it takes a
`/reload` or a restart. There is no command and no saved state.

## Goals / Non-Goals

**Goals:**

- Fold runs without changing the message model, agent loop, or session format.
- Keep the incremental segment cache: no per-token work, and no rebuilding the
  whole transcript for routine tool events.
- Preserve scroll anchoring and existing per-row expansion behavior.
- Reuse the current thinking, tool, instructions, and search renderers when a
  fold is expanded.

**Non-Goals:**

- Medium/High verbosity levels, exploration groups, or grouped read rows.
- Folding assistant text, notices, questions, or background task rows.
- Lazy Lua snapshot restore for folded tools.
- Exact hidden-row targeting from search in the first cut.
- A `/transcript` command, saved transcript-mode state, or per-session mode.

## Decisions

### Fold in `MessagesPanel`, not in the message model

The fold is a presentation concern. Keeping every `DisplayMessage` means live
events, history reload, Lua restore state, and expansion keep working unchanged.

Alternatives considered:

- **Event-time aggregation**: accumulate counts and drop the underlying messages.
  Rejected because expansion, snapshots, per-tool clicks, and session restore all
  need the original messages.
- **A full row/timeline model above the cache**: cleaner for future verbosity
  levels, but a wider refactor of scroll, selection, search, and reflow for one
  opt-in mode. Revisit if Medium/High levels are ever wanted.

### Fold plan

Add `UiConfig.compact_transcript: bool` and keep `folds: Vec<Fold>` and
`expanded_folds: HashSet<usize>` in `MessagesPanel`. A fold key is its first
message index, which is stable while messages append.

```text
Fold {
    first_msg: usize,
    end_msg: usize,       // exclusive
    summary_seg: usize,   // cache index when collapsed
    spacer_seg: usize,    // slot before the summary, for truncation
}
```

Foldability: `Thinking` messages and `Tool` messages whose name is not `question`
and whose `background` badge is absent. A fold is a maximal contiguous run of
foldable messages. Everything else is a boundary and renders normally, matching
the spec.

### Cache integration

`SegmentCache.msg_count` keeps its meaning: the number of messages represented,
including hidden ones. In compact mode the incremental loop differs:

- A foldable message that extends the last fold updates that fold. If the fold is
  collapsed, only the summary segment is rebuilt. If expanded, the message gets a
  normal segment.
- A new fold pushes one summary segment, or normal message segments when expanded.
- Hidden messages get no segment, no spacer, and no reserved instruction segments.
- A boundary message closes the open fold and pushes its normal segment.
- A streaming thought joins the run it will extend once flushed: the last fold's
  summary gains the thought and a spinner, or a summary-only live segment stands
  where a new fold will open. Raw reasoning rows are drawn only in full mode or
  inside an expanded run. Flushing or `stream_reset` drops the live marker.

Add `Segment.fold: Option<usize>` so a summary row is clickable and searchable.
`rebuild_tool_segment` first checks whether the tool is hidden in a collapsed
fold. If so it rebuilds the summary; if the tool became a boundary (moved to the
background), it invalidates from the fold start instead. `tool_output` skips the
summary rebuild because output alone changes neither counts nor the stable busy
header; `tool_start`, `tool_done`, annotations, usage stamps, and background
changes do rebuild it.

Expansion and collapse truncate the cache at `spacer_seg`, reset
`SegmentCache.msg_count` to `first_msg`, flip the fold, and let the next `view`
rebuild from there. This keeps the summary segment index stable, so the existing
segment-based scroll position stays on the summary. A new
`SegmentCache::truncate_for_rebuild(seg_len, msg_count)` expresses that operation.

### Counts and summary rendering

`summarize_fold` walks the fold's messages once per summary rebuild:

| Category | Tool names |
| --- | --- |
| commands | `bash` |
| edits | `write`, `edit`, `multiedit`, `edit_lines`, `insert_lines` |
| thoughts | `Thinking` messages |
| reads | `read` |
| tools | every other foldable tool |
| instructions | distinct `InstructionBlock::path` values |

Only tools whose status is no longer `InProgress` count. Categories are emitted in
that fixed order with correct singular/plural forms; zero categories are omitted.

The summary is a normal `Segment` with one line built from the marker, counts, and
optional right-aligned turn usage. It takes the tool row background (`tool_bg`),
like a regular tool call. A running group uses `Segment.gutter` with
`gutter_spins` so the existing spinner tick drives it. Markers are maki's own
green dot (`●`) collapsed, `−` expanded, and a red dot (`●`) when any folded tool
failed. When no item has finished, the
line shows a spinner and the first busy row's header instead of counts, using a
stable label so details arriving do not make it flicker.

Usage comes from the last folded tool that has `turn_usage`, preserving the cost
visibility the tool row provided. Timestamps stay on individual rows and are only
visible when expanded.

### Expansion and scroll

`toggle_at` checks `Segment.fold` before the thinking and tool paths. After the
cache rebuild, the summary occupies the same segment index, so its document row is
unchanged. If `auto_scroll` was pinned to the bottom, expanding disables the pin
and scrolls to the summary, matching OpenCode's fix for summaries jumping away.

### Configuration

Add `UiFileConfig.compact_transcript: Option<bool>` feeding
`UiConfig.compact_transcript: bool`, default `false`. `MessagesPanel::new` copies
it into `compact: bool`; every chat, later chat, and subagent tab clones the same
`UiConfig`.

There is no `/transcript` command and no state file. Compact mode is a set-once
density preference like `ui.thinking_lines` and `ui.tool_output_lines`, changed
in `init.lua` and applied by `/reload` or a restart. A command plus persisted
state was considered and rejected: for a binary density knob it created two
sources of truth and a precedence rule to explain, while per-run expansion
already covers the "show me this run's details" case.

### Search

`segment_search_texts` returns one entry per segment. For a collapsed fold it
concatenates the hidden messages' search text, including full tool output and
instruction blocks, so folded text remains searchable. `scroll_to_segment` detects
a summary segment, expands the fold if needed, rebuilds, and scrolls to the
summary. Exact targeting of the hidden row is left for a follow-up; the proposal
and spec only require navigating to the run.

### Display-only invariant

No agent, provider, or storage-message code changes. The fold plan reads
`DisplayMessage`s after they exist and never mutates them. `history_to_display`
still builds all messages and restore items; compact mode only hides their
segments. Tests assert identical provider history and session output between
modes.

## Risks / Trade-offs

- **Cache invalidation edge cases** → Keep the fold plan derived from messages,
  express all structural changes (append, expand, background move, reload) as
  "invalidate from message N", and cover each with panel tests. A full rebuild is
  the fallback for rare boundary changes.
- **Expanding a very long run renders many rows at once** → Only rebuild on user
  click; keep the existing reflow budget and viewport-limited drawing. A lazy
  snapshot restore can follow if load or expansion cost is measurable.
- **Search builds large strings for folded output** → Built on demand per search,
  not retained, same order of cost as today's search over visible tool output.
- **Summary anchor at the bottom** → Explicitly unpin auto-scroll on expansion;
  test the clicked row stays visible.
- **Moving a tool to the background changes the fold plan mid-turn** → Invalidate
  from that message when the badge appears; the rows on either side become
  separate runs.
- **Folded text is not selectable or copyable** → Accepted; expand the run first.
- **Startup-only setting** → Accepted; consistent with `ui.thinking_lines` and
  `ui.tool_output_lines`. Expanding a run covers seeing its details without a
  global toggle.
- **Category drift when plugins add tools** → Unknown tools fall into `tools`;
  categories are explicit opt-ins, so a new plugin never disappears from counts.

## Migration Plan

Opt-in with `false` as the default, so no migration is required. Users enable
compact mode with `ui.compact_transcript = true` and apply it with `/reload` or a
restart. Rolling back is setting it to `false`; no session or schema data
changes.

## Open Questions

- Exact hidden-row search targeting may need a richer `SearchTarget` returned by
  the search modal. Deferrable without changing the fold approach or specs.
- Whether expanded folds should survive session reload. Expansion is ephemeral in
  the first cut.
- Whether folded tool snapshots should be restored lazily on expansion instead of
  eagerly on session load.
