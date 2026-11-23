# Spec Delta

## Purpose

Controls how the TUI transcript presents agent activity, including compact folding
of thinking and tool runs, the summary they render, and how users reveal details.

## ADDED Requirements

### Requirement: Compact transcript setting

The TUI SHALL support a compact transcript presentation, off by default.
`ui.compact_transcript = true` SHALL enable it at startup, and it SHALL apply to
every chat, including subagent tabs.

#### Scenario: Default
- **WHEN** `ui.compact_transcript` is unset or false
- **THEN** every chat renders individual thinking and tool rows

#### Scenario: Configured compact transcript
- **WHEN** `ui.compact_transcript = true`
- **THEN** every chat renders in compact mode from startup

#### Scenario: Subagent tabs follow the setting
- **WHEN** a subagent tab opens while `ui.compact_transcript` is true
- **THEN** that tab renders in compact mode as well

### Requirement: Activity runs fold in compact mode

In `compact` mode, every maximal contiguous run of thinking messages and tool
messages SHALL render as one activity summary row instead of one row per message.
The summary SHALL stand in the same position as the run it replaces.

#### Scenario: Thinking and tools between text
- **WHEN** a turn streams thinking, then two tool calls, then assistant text
- **THEN** the thinking and both tool calls render as one summary row above the
  assistant text

#### Scenario: Consecutive tool calls
- **WHEN** several tool calls appear with no visible message between them
- **THEN** they share one summary row

#### Scenario: Single item
- **WHEN** a run contains one thinking block or one tool call
- **THEN** it still folds into a summary row

#### Scenario: Streaming reasoning
- **WHEN** a thinking block is still streaming and has not become a message yet
- **THEN** it joins the last run's summary with a spinner instead of rendering as
  separate reasoning rows, unless that run is expanded

#### Scenario: Compact mode off
- **WHEN** `ui.compact_transcript` is false
- **THEN** thinking and tool calls render as individual rows as they do today

### Requirement: Visible messages bound runs

User messages, assistant text, notices, compaction notices, errors, done
messages, `question` tool calls, and moved or background `task` rows SHALL remain
visible in compact mode and SHALL end the current activity run.

#### Scenario: Assistant text splits batches
- **WHEN** a tool batch, assistant text, and another tool batch occur in one turn
- **THEN** two summary rows render with the text between them

#### Scenario: Notice splits a run
- **WHEN** a notice such as a compaction result appears between tool calls
- **THEN** the notice stays visible and the tools on either side render in
  separate summary rows

#### Scenario: Question stays visible
- **WHEN** the agent calls the `question` tool
- **THEN** its row stays visible, it is not counted in any summary, and the
  following tools start a new run

#### Scenario: Background task stays visible
- **WHEN** a running `task` call moves to the background
- **THEN** its row stays visible with its badge, and the tools before and after
  it render in separate summary rows

### Requirement: Summary reports finished activity counts

The summary SHALL list counts of finished work in this order: commands, edits,
thoughts, reads, tools, instructions. Categories with a zero count SHALL be
omitted. Singular and plural forms SHALL be used correctly. A tool counts once its
status is no longer in progress, and a thinking block counts once its message
exists or while it is still streaming into its run. Instructions SHALL be counted as distinct loaded files within the run.

#### Scenario: Mixed activity
- **WHEN** a run contains two `bash` calls, one edit, three thinking blocks, four
  `read` calls, and two other tools
- **THEN** the summary reads `2 commands, 1 edit, 3 thoughts, 4 reads, 2 tools`

#### Scenario: Singular forms
- **WHEN** a run contains one command and one read
- **THEN** the summary reads `1 command, 1 read`

#### Scenario: Running work is not counted
- **WHEN** a run contains one finished read and two tools still in progress
- **THEN** the summary counts only the finished read

#### Scenario: Distinct instruction files
- **WHEN** two folded tools load the same instruction file and one folded tool
  loads a different one
- **THEN** the summary reports `2 instructions`

### Requirement: Summary state reflects progress

A collapsed summary SHALL show maki's green dot when every item is finished, a
spinner while work is running, and a red dot when any item failed. When nothing in
the run has
finished, the summary SHALL show a spinner and the first busy row's status instead
of counts. An expanded summary SHALL show `−`.

#### Scenario: Nothing finished yet
- **WHEN** a run contains only tools that are still running
- **THEN** the summary shows a spinner and the first busy row's header, with no
  counts

#### Scenario: Work in progress after a finish
- **WHEN** a run has one finished read and one running tool
- **THEN** the summary shows the read count and a spinner

#### Scenario: Failure
- **WHEN** any tool in the run failed
- **THEN** the collapsed summary shows a red dot

#### Scenario: Expanded
- **WHEN** the user expands the run
- **THEN** the summary shows `−` and keeps its counts

### Requirement: Users expand and collapse a run

Clicking a summary row SHALL expand the run to the individual rows that compact
mode replaced; clicking it again SHALL collapse it. The summary SHALL remain on the
row the user clicked. When the transcript is pinned to the bottom, expanding SHALL
release the pin so the summary does not scroll away.

#### Scenario: Expand
- **WHEN** the user clicks a collapsed summary
- **THEN** the thinking and tool rows appear below it with the same per-row
  collapse behavior they have in full mode

#### Scenario: Collapse
- **WHEN** the user clicks an expanded summary
- **THEN** its rows disappear and only the summary remains

#### Scenario: Summary stays anchored
- **WHEN** the user expands a summary while the transcript is scrolled to the
  bottom
- **THEN** the summary stays on the clicked row and auto-scroll stops following
  the bottom

#### Scenario: New activity joins the run
- **WHEN** new thinking or tool messages arrive for the last run
- **THEN** an expanded run shows them as new rows and a collapsed run updates its
  summary

### Requirement: Search reaches folded activity

Transcript search SHALL match text inside folded rows. Selecting a match that
belongs to a folded run SHALL expand that run and navigate the transcript to it.

#### Scenario: Hidden tool output matches
- **WHEN** the user searches for text that exists only in a folded tool's output
- **THEN** the search results include the match

#### Scenario: Select a folded match
- **WHEN** the user selects a search match inside a folded run
- **THEN** the run expands and the transcript scrolls to that run

### Requirement: Summary keeps turn usage visible

When a folded row carries the turn's usage stamp, the collapsed summary SHALL show
that usage so hiding the row does not hide the turn's cost.

#### Scenario: Usage on the last folded tool
- **WHEN** the turn usage is stamped on a tool inside a collapsed run
- **THEN** the summary shows that usage right-aligned

### Requirement: Compact mode is display-only

Turning on compact mode SHALL NOT change the messages sent to the provider, the
session file contents, headless or print output, or ACP behavior.

#### Scenario: Provider request unchanged
- **WHEN** the same session runs in `full` and `compact` modes
- **THEN** the provider receives the same messages and tool definitions

#### Scenario: Session export unchanged
- **WHEN** a session is saved in compact mode
- **THEN** its stored messages are byte-identical to the same session in full mode
