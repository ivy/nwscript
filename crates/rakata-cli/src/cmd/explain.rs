//! `rakata explain CODE`: the long explanation of a diagnostic code.

use super::Ctx;
use crate::Exit;
use rakata::codes;
use rakata::diag::SourceMap;

pub fn run(ctx: &Ctx, code: &str) -> Exit {
    let Some(c) = codes::lookup(code) else {
        eprintln!("error: no diagnostic code `{code}`; codes look like RE0001 or RW0001");
        return Exit::Usage;
    };
    let mut env = ctx.envelope("explain");
    env.result = serde_json::json!({
        "code": c.id,
        "severity": c.severity,
        "summary": c.summary,
        "explanation": c.explanation,
    });
    let text = format!("{}: {}\n\n{}", c.id, c.summary, c.explanation);
    ctx.finish(env, &text, Vec::new(), &SourceMap::new(), Exit::Ok)
}
