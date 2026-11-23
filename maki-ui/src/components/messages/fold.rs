use std::fmt::Write;
use std::ops::Range;

use maki_agent::ToolOutput;
use maki_agent::tools::{
    BASH_TOOL_NAME, EDIT_TOOL_NAME, MULTIEDIT_TOOL_NAME, QUESTION_TOOL_NAME, READ_TOOL_NAME,
    WRITE_TOOL_NAME,
};
use ratatui::text::{Line, Span};

use crate::animation::spinner_str;
use crate::components::tool_display::{TOOL_INDICATOR, append_right_info, header_label};
use crate::components::{DisplayMessage, DisplayRole, ToolStatus};
use crate::theme;

const EDIT_LINES_TOOL_NAME: &str = "edit_lines";
const INSERT_LINES_TOOL_NAME: &str = "insert_lines";
const FOLD_EXPANDED_MARK: &str = "− ";
const FOLD_SEPARATOR: &str = ", ";

pub(super) struct Fold {
    pub first_msg: usize,
    pub end_msg: usize,
    pub summary_seg: usize,
    pub spacer_seg: usize,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct FoldSummary {
    pub commands: usize,
    pub edits: usize,
    pub thoughts: usize,
    pub reads: usize,
    pub tools: usize,
    pub instructions: usize,
    pub finished: usize,
    pub running: usize,
    pub failed: bool,
    pub first_busy: Option<usize>,
    pub usage: Option<usize>,
    /// A reasoning stream that has not become a message yet.
    pub live_thought: bool,
}

pub(super) fn is_foldable(msg: &DisplayMessage) -> bool {
    match &msg.role {
        DisplayRole::Thinking => true,
        DisplayRole::Tool(tool) => {
            tool.name.as_ref() != QUESTION_TOOL_NAME && msg.background.is_none()
        }
        _ => false,
    }
}

/// The maximal contiguous runs of foldable messages, empty in full mode.
pub(super) fn fold_plan(messages: &[DisplayMessage], compact: bool) -> Vec<Range<usize>> {
    let mut runs = Vec::new();
    if !compact {
        return runs;
    }
    let mut start = None;
    for (i, msg) in messages.iter().enumerate() {
        if is_foldable(msg) {
            start.get_or_insert(i);
        } else if let Some(from) = start.take() {
            runs.push(from..i);
        }
    }
    if let Some(from) = start {
        runs.push(from..messages.len());
    }
    runs
}

pub(super) fn summarize_fold(messages: &[DisplayMessage]) -> FoldSummary {
    let mut summary = FoldSummary::default();
    let mut instruction_paths: Vec<&str> = Vec::new();
    for (i, msg) in messages.iter().enumerate() {
        match &msg.role {
            DisplayRole::Thinking => summary.thoughts += 1,
            DisplayRole::Tool(tool) => {
                if let Some(blocks) = msg
                    .tool_output
                    .as_deref()
                    .and_then(ToolOutput::instructions)
                {
                    for block in blocks {
                        if !instruction_paths.contains(&block.path.as_str()) {
                            instruction_paths.push(block.path.as_str());
                        }
                    }
                }
                match tool.status {
                    ToolStatus::InProgress => {
                        summary.running += 1;
                        summary.first_busy.get_or_insert(i);
                    }
                    ToolStatus::Success | ToolStatus::Error => {
                        summary.finished += 1;
                        summary.failed |= tool.status == ToolStatus::Error;
                        count_tool(&mut summary, &tool.name);
                    }
                }
            }
            _ => {}
        }
        if msg.turn_usage.is_some() {
            summary.usage = Some(i);
        }
    }
    summary.instructions = instruction_paths.len();
    summary.finished += summary.thoughts;
    summary
}

fn count_tool(summary: &mut FoldSummary, name: &str) {
    match name {
        BASH_TOOL_NAME => summary.commands += 1,
        WRITE_TOOL_NAME
        | EDIT_TOOL_NAME
        | MULTIEDIT_TOOL_NAME
        | EDIT_LINES_TOOL_NAME
        | INSERT_LINES_TOOL_NAME => summary.edits += 1,
        READ_TOOL_NAME => summary.reads += 1,
        _ => summary.tools += 1,
    }
}

/// The summary row's body, gutter marker, and whether the gutter spins.
pub(super) fn summary_line(
    summary: &FoldSummary,
    messages: &[DisplayMessage],
    expanded: bool,
    width: u16,
) -> (Vec<Line<'static>>, Span<'static>, bool) {
    let palette = theme::current();
    let (gutter, spins) = if expanded {
        (Span::styled(FOLD_EXPANDED_MARK, palette.tool_dim), false)
    } else if summary.failed {
        (Span::styled(TOOL_INDICATOR, palette.tool_error), false)
    } else if summary.running > 0 || summary.live_thought {
        (Span::styled(spinner_str(0), palette.spinner), true)
    } else {
        (Span::styled(TOOL_INDICATOR, palette.tool_success), false)
    };
    let body = if summary.finished == 0 && summary.running > 0 {
        summary
            .first_busy
            .and_then(|i| messages.get(i))
            .and_then(header_label)
            .unwrap_or_default()
    } else {
        category_text(summary)
    };
    let usage = summary
        .usage
        .and_then(|i| messages.get(i))
        .and_then(|msg| msg.turn_usage.as_deref());
    let mut line = Line::from(Span::styled(body, palette.tool));
    append_right_info(&mut line, None, usage, None, width);
    (vec![line], gutter, spins)
}

fn category_text(summary: &FoldSummary) -> String {
    let mut out = String::new();
    push_count(&mut out, summary.commands, "command", "commands");
    push_count(&mut out, summary.edits, "edit", "edits");
    push_count(&mut out, summary.thoughts, "thought", "thoughts");
    push_count(&mut out, summary.reads, "read", "reads");
    push_count(&mut out, summary.tools, "tool", "tools");
    push_count(
        &mut out,
        summary.instructions,
        "instruction",
        "instructions",
    );
    out
}

fn push_count(out: &mut String, count: usize, one: &str, many: &str) {
    if count == 0 {
        return;
    }
    if !out.is_empty() {
        out.push_str(FOLD_SEPARATOR);
    }
    let _ = write!(out, "{count} {}", if count == 1 { one } else { many });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{BackgroundBadge, ToolRole};
    use maki_agent::{InstructionBlock, TextOutput};
    use std::sync::Arc;
    use test_case::test_case;

    const HEADER: &str = "run";
    const USAGE: &str = "10 tokens";

    #[derive(Clone, Copy)]
    enum Kind {
        Thinking,
        Tool,
        Bash,
        Question,
        Background,
        Assistant,
        User,
        Notice,
    }

    fn message(kind: Kind) -> DisplayMessage {
        let role = |name: &str, status| {
            DisplayRole::Tool(Box::new(ToolRole {
                id: name.into(),
                status,
                name: Arc::from(name),
            }))
        };
        let mut msg = match kind {
            Kind::Thinking => DisplayMessage::new(DisplayRole::Thinking, String::from("thought")),
            Kind::Tool => {
                DisplayMessage::new(role(READ_TOOL_NAME, ToolStatus::Success), String::new())
            }
            Kind::Bash => {
                DisplayMessage::new(role(BASH_TOOL_NAME, ToolStatus::Success), String::new())
            }
            Kind::Question => {
                DisplayMessage::new(role(QUESTION_TOOL_NAME, ToolStatus::Success), String::new())
            }
            Kind::Background => {
                let mut msg =
                    DisplayMessage::new(role(BASH_TOOL_NAME, ToolStatus::Success), String::new());
                msg.background = Some(BackgroundBadge {
                    id: "task".into(),
                    started_at: None,
                    outcome: None,
                });
                msg
            }
            Kind::Assistant => DisplayMessage::new(DisplayRole::Assistant, String::from("answer")),
            Kind::User => DisplayMessage::new(DisplayRole::User, String::from("prompt")),
            Kind::Notice => DisplayMessage::new(DisplayRole::Notice, String::from("notice")),
        };
        if matches!(
            kind,
            Kind::Tool | Kind::Bash | Kind::Question | Kind::Background
        ) {
            msg.text = String::from(HEADER);
        }
        msg
    }

    fn tool(name: &str, status: ToolStatus) -> DisplayMessage {
        DisplayMessage::new(
            DisplayRole::Tool(Box::new(ToolRole {
                id: name.into(),
                status,
                name: Arc::from(name),
            })),
            String::from(HEADER),
        )
    }

    fn thinking() -> DisplayMessage {
        DisplayMessage::new(DisplayRole::Thinking, String::from("thought"))
    }

    fn instruction_tool(path: &str) -> DisplayMessage {
        let mut msg = tool(READ_TOOL_NAME, ToolStatus::Success);
        msg.tool_output = Some(Arc::new(ToolOutput::Plain(TextOutput {
            text: String::from("ok"),
            instructions: Some(vec![InstructionBlock {
                path: path.into(),
                content: String::from("instructions"),
            }]),
            state: None,
            loaded_tools: Vec::new(),
        })));
        msg
    }

    fn ranges(messages: &[DisplayMessage], compact: bool) -> Vec<(usize, usize)> {
        fold_plan(messages, compact)
            .into_iter()
            .map(|range| (range.start, range.end))
            .collect()
    }

    #[test_case(&[Kind::Thinking, Kind::Tool, Kind::Bash], &[(0, 3)] ; "consecutive_activity_shares_one_run")]
    #[test_case(&[Kind::Assistant, Kind::Tool], &[(1, 2)] ; "assistant_text_bounds_a_run")]
    #[test_case(&[Kind::User, Kind::Tool], &[(1, 2)] ; "user_message_bounds_a_run")]
    #[test_case(&[Kind::Tool, Kind::Notice, Kind::Tool], &[(0, 1), (2, 3)] ; "notice_splits_the_run")]
    #[test_case(&[Kind::Tool, Kind::Question, Kind::Tool], &[(0, 1), (2, 3)] ; "question_stays_visible_and_splits")]
    #[test_case(&[Kind::Tool, Kind::Background, Kind::Tool], &[(0, 1), (2, 3)] ; "background_task_stays_visible_and_splits")]
    #[test_case(&[Kind::Thinking], &[(0, 1)] ; "single_item_still_folds")]
    #[test_case(&[], &[] ; "no_messages")]
    fn fold_plan_groups_maximal_runs(kinds: &[Kind], expected: &[(usize, usize)]) {
        let messages: Vec<_> = kinds.iter().copied().map(message).collect();
        assert_eq!(ranges(&messages, true), expected);
    }

    #[test]
    fn fold_plan_is_empty_in_full_mode() {
        let messages = vec![tool(BASH_TOOL_NAME, ToolStatus::Success), thinking()];
        assert!(ranges(&messages, false).is_empty());
    }

    #[test]
    fn summarize_counts_finished_work_in_category_order() {
        let messages = vec![
            tool(BASH_TOOL_NAME, ToolStatus::Success),
            tool(BASH_TOOL_NAME, ToolStatus::Success),
            tool(WRITE_TOOL_NAME, ToolStatus::Success),
            thinking(),
            thinking(),
            thinking(),
            tool(READ_TOOL_NAME, ToolStatus::Success),
            tool(READ_TOOL_NAME, ToolStatus::Success),
            tool(READ_TOOL_NAME, ToolStatus::Success),
            tool(READ_TOOL_NAME, ToolStatus::Success),
            tool("glob", ToolStatus::Success),
            tool("task", ToolStatus::Success),
        ];

        let summary = summarize_fold(&messages);

        assert_eq!(summary.commands, 2);
        assert_eq!(summary.edits, 1);
        assert_eq!(summary.thoughts, 3);
        assert_eq!(summary.reads, 4);
        assert_eq!(summary.tools, 2);
        assert_eq!(
            category_text(&summary),
            "2 commands, 1 edit, 3 thoughts, 4 reads, 2 tools"
        );
    }

    #[test]
    fn summarize_uses_singular_forms() {
        let messages = vec![
            tool(BASH_TOOL_NAME, ToolStatus::Success),
            tool(READ_TOOL_NAME, ToolStatus::Success),
        ];
        assert_eq!(
            category_text(&summarize_fold(&messages)),
            "1 command, 1 read"
        );
    }

    #[test]
    fn summarize_counts_only_finished_tools() {
        let messages = vec![
            tool(READ_TOOL_NAME, ToolStatus::Success),
            tool("glob", ToolStatus::InProgress),
            tool("task", ToolStatus::InProgress),
        ];

        let summary = summarize_fold(&messages);

        assert_eq!(summary.reads, 1);
        assert_eq!(summary.tools, 0);
        assert_eq!(summary.running, 2);
        assert_eq!(summary.finished, 1);
        assert_eq!(summary.first_busy, Some(1));
    }

    #[test]
    fn summarize_counts_distinct_instruction_paths() {
        let messages = vec![
            instruction_tool("AGENTS.md"),
            instruction_tool("AGENTS.md"),
            instruction_tool("CLAUDE.md"),
        ];
        assert_eq!(summarize_fold(&messages).instructions, 2);
        assert!(category_text(&summarize_fold(&messages)).ends_with("2 instructions"));
    }

    fn line_text(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    #[test]
    fn summary_marks_success_with_a_green_dot() {
        let messages = vec![
            tool(BASH_TOOL_NAME, ToolStatus::Success),
            tool(READ_TOOL_NAME, ToolStatus::Success),
        ];
        let (lines, gutter, spins) = summary_line(&summarize_fold(&messages), &messages, false, 80);

        assert_eq!(gutter.content, TOOL_INDICATOR);
        assert_eq!(gutter.style, theme::current().tool_success);
        assert!(!spins);
        assert!(line_text(&lines[0]).contains("1 command, 1 read"));
    }

    #[test]
    fn summary_is_expanded_minus_with_counts() {
        let messages = vec![tool(BASH_TOOL_NAME, ToolStatus::Success)];
        let (lines, gutter, spins) = summary_line(&summarize_fold(&messages), &messages, true, 80);

        assert_eq!(gutter.content, FOLD_EXPANDED_MARK);
        assert!(!spins);
        assert!(line_text(&lines[0]).contains("1 command"));
    }

    #[test]
    fn summary_shows_the_first_busy_header_before_any_finish() {
        let messages = vec![tool(BASH_TOOL_NAME, ToolStatus::InProgress)];
        let (lines, _, spins) = summary_line(&summarize_fold(&messages), &messages, false, 80);

        assert!(spins);
        assert_eq!(line_text(&lines[0]), format!("{BASH_TOOL_NAME}> {HEADER}"));
    }

    #[test]
    fn summary_shows_spinner_and_counts_after_a_finish() {
        let messages = vec![
            tool(READ_TOOL_NAME, ToolStatus::Success),
            tool("glob", ToolStatus::InProgress),
        ];
        let (lines, _, spins) = summary_line(&summarize_fold(&messages), &messages, false, 80);
        let text = line_text(&lines[0]);

        assert!(spins);
        assert!(text.contains("1 read"));
        assert!(!text.contains("glob"));
    }

    #[test]
    fn summary_marks_a_failure() {
        let messages = vec![
            tool(BASH_TOOL_NAME, ToolStatus::Error),
            tool(READ_TOOL_NAME, ToolStatus::Success),
        ];
        let (lines, gutter, spins) = summary_line(&summarize_fold(&messages), &messages, false, 80);

        assert_eq!(gutter.content, TOOL_INDICATOR);
        assert_eq!(gutter.style, theme::current().tool_error);
        assert!(!spins);
        assert!(line_text(&lines[0]).contains("1 command, 1 read"));
    }

    #[test]
    fn summary_right_aligns_the_turn_usage() {
        let mut messages = vec![tool(READ_TOOL_NAME, ToolStatus::Success)];
        messages[0].turn_usage = Some(USAGE.into());

        let (lines, ..) = summary_line(&summarize_fold(&messages), &messages, false, 80);
        let text = line_text(&lines[0]);

        assert!(text.starts_with("1 read"));
        assert!(text.ends_with(USAGE));
        assert!(text[..text.len() - USAGE.len()].ends_with(' '));
    }
}
