//! Whether a coding agent runs this process, from the environment its
//! harness sets. An agent reads every byte it is given, so with no
//! `--format` the text form drops the padding that aligns tables for a
//! person.
//!
//! The variables are those DataDog's pup checks to name the agent in its
//! user agent, `src/useragent.rs` at
//! <https://github.com/DataDog/pup/blob/384a030c6cabd977dddf01d409fb4c6a4f9e8542/src/useragent.rs>,
//! less `CURSOR_TRACE_ID`, which pup notes the Cursor editor sets, so a
//! person's terminal there carries it too.

/// Variables an agent's harness sets to `1` or `true`.
pub const TRUTHY: &[&str] = &[
    "CLAUDECODE",
    "CLAUDE_CODE",
    "CURSOR_AGENT",
    "CODEX",
    "OPENAI_CODEX",
    "OPENCODE",
    "AIDER",
    "CLINE",
    "WINDSURF_AGENT",
    "GITHUB_COPILOT",
    "COPILOT_CLI",
    "AMAZON_Q",
    "AWS_Q_DEVELOPER",
    "GEMINI_CODE_ASSIST",
    "GEMINI_CLI",
    "SRC_CODY",
    "PI_CODING_AGENT",
    "AGENT",
];

/// Variables an agent's harness sets to a session or thread id: any
/// non-empty value counts.
pub const PRESENT: &[&str] = &[
    "CODEX_SESSION_ID",
    "CODEX_THREAD_ID",
    "CODEX_VERSION",
    "CODEX_SANDBOX",
    "CODEX_CI",
    "COPILOT_AGENT_SESSION_ID",
    "DEVIN_SESSION_ID",
];

/// Whether `get` reports an agent's variable: one of [`TRUTHY`] set to `1`
/// or `true` (any case), or one of [`PRESENT`] set to anything.
pub fn detected_by(get: impl Fn(&str) -> Option<String>) -> bool {
    TRUTHY
        .iter()
        .any(|v| get(v).is_some_and(|x| x == "1" || x.eq_ignore_ascii_case("true")))
        || PRESENT
            .iter()
            .any(|v| get(v).is_some_and(|x| !x.is_empty()))
}

/// Whether this process's environment names an agent.
pub fn detected() -> bool {
    detected_by(|v| std::env::var(v).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |k| {
            vars.iter()
                .find(|(n, _)| *n == k)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn claude_code_is_an_agent_and_a_false_marker_is_not() {
        assert!(detected_by(env(&[("CLAUDECODE", "1")])));
        assert!(detected_by(env(&[("GEMINI_CLI", "True")])));
        assert!(!detected_by(env(&[("CLAUDECODE", "0")])));
        assert!(!detected_by(env(&[])));
    }

    #[test]
    fn a_session_id_counts_when_it_is_not_empty() {
        assert!(detected_by(env(&[("DEVIN_SESSION_ID", "run-1")])));
        assert!(!detected_by(env(&[("DEVIN_SESSION_ID", "")])));
        assert!(!detected_by(env(&[("CURSOR_TRACE_ID", "x")])));
    }
}
