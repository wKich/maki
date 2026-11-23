# Proposal

## Why

Long tool-heavy turns bury the assistant's answer and the user's own prompts under
dozens of one-line tool rows. OpenCode v2 solved this by making Low transcript
verbosity the default: each run of thinking and tool calls collapses into one
activity summary. Maki still renders every thinking block and tool call as its own
row, so scanning a long session costs more effort the longer the agent works.

This is a display change only. It does not alter the model context, session files,
or token usage; it makes the existing transcript readable.

## What Changes

- Add an opt-in compact transcript mode. In compact mode, every maximal run of
  `Thinking` and `Tool` messages folds into one summary segment:
  `● 3 thoughts, 8 reads, 12 tools`.
- Runs end at user messages, assistant text, notices, compaction, errors, done
  messages, `question` tool calls, and moved/background `task` rows.
- The summary counts finished work only: commands (`bash`), edits (write/edit
  family), thoughts, reads (`read`), tools (everything else), and distinct loaded
  instruction files. Zero categories are omitted.
- A group with nothing finished yet shows a spinner and the first busy row's
  header instead of counts. A finished group shows maki's green dot, any failure
  a red dot; an expanded group shows `−`.
- Clicking the summary expands it to today's rows and clicking again collapses it.
  Expanding or collapsing keeps the summary row anchored; expansion at the bottom
  unpins auto-scroll.
- Moved/background `task` rows stay visible and split runs. `question` rows stay
  visible and end runs.
- Add `ui.compact_transcript = true`, off by default. It is a config-only,
  set-once density preference like `ui.thinking_lines` and
  `ui.tool_output_lines`; there is no command and no saved state.
- Search includes folded rows. Selecting a hit expands the fold and navigates to
  the summary; exact hidden-row targeting remains a follow-up.
- Display-only: no changes to the agent, request history, session serialization,
  print/headless output, or ACP.

## Capabilities

### New Capabilities

- `transcript`: compact activity folding, summary counts and states, expansion,
  search over folded rows, and the compact transcript setting.

### Modified Capabilities

<!-- None. -->

## Impact

- `maki-ui`: `MessagesPanel` fold plan and segment cache, summary rendering, click
  and scroll handling, search corpus, and config plumbing.
- `maki-config`: `ui.compact_transcript` and file parsing.
- Docs, the generated config reference, and TUI tests.
- No new dependencies; no agent, provider, or session-format changes.
