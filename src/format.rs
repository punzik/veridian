use crate::server::LSPServer;
use crate::sources::LSPSupport;
use ropey::Rope;
use std::process::{Command, Stdio};
use tower_lsp::lsp_types::*;

impl LSPServer {
    pub fn formatting(&self, params: DocumentFormattingParams) -> Option<Vec<TextEdit>> {
        let uri = params.text_document.uri;
        let file_id = self.srcs.get_id(&uri).to_owned();
        self.srcs.wait_parse_ready(file_id, false);
        let file = self.srcs.get_file(file_id)?;
        let file = file.read().ok()?;

        if self.format {
            Some(vec![TextEdit::new(
                Range::new(
                    file.text.char_to_pos(0),
                    file.text.char_to_pos(file.text.len_chars() - 1),
                ),
                format_document(&file.text, None)?,
            )])
        } else {
            None
        }
    }

    pub fn range_formatting(&self, params: DocumentRangeFormattingParams) -> Option<Vec<TextEdit>> {
        let uri = params.text_document.uri;
        let file_id = self.srcs.get_id(&uri).to_owned();
        self.srcs.wait_parse_ready(file_id, false);
        let file = self.srcs.get_file(file_id)?;
        let file = file.read().ok()?;

        if self.format {
            let end_slice = file.text.line(params.range.end.line as usize);
            Some(vec![TextEdit::new(
                Range::new(
                    Position::new(params.range.start.line, 0),
                    Position::new(params.range.end.line, end_slice.len_chars() as u64 - 1),
                ),
                format_document(&file.text, Some(params.range))?,
            )])
        } else {
            None
        }
    }
}

pub fn format_document(rope: &Rope, range: Option<Range>) -> Option<String> {
    let mut child = Command::new("verible-verilog-format");
    child
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .stdout(Stdio::piped());
    if let Some(r) = range {
        child
            .arg("--lines")
            .arg(format!("{}-{}", r.start.line + 1, r.end.line + 1));
    }
    dbg!(&child);
    let mut child = child.arg("-").spawn().ok()?;

    rope.write_to(child.stdin.as_mut()?).ok()?;
    let output = child.wait_with_output().ok()?;
    dbg!(&output);
    if output.status.success() {
        let raw_output = String::from_utf8(output.stdout).ok()?;
        Some(raw_output)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use which::which;

    #[test]
    fn test_formatting() {
        let text = r#"
module test;
  logic a;
   logic b;
endmodule"#;
        let text_fixed = r#"
module test;
  logic a;
  logic b;
endmodule
"#;
        let doc = Rope::from_str(&text);
        if which("verible-verilog-format").is_ok() {
            assert_eq!(format_document(&doc, None).unwrap(), text_fixed.to_string());
        }
    }

    #[test]
    fn test_range_formatting() {
        let text = r#"
module test;
  logic a;
   logic b;
endmodule
module test1;
  logic a;
   logic b;
endmodule"#;
        let text_fixed = r#"
module test;
  logic a;
   logic b;
endmodule
module test1;
  logic a;
  logic b;
endmodule
"#;
        let doc = Rope::from_str(&text);
        if which("verible-verilog-format").is_ok() {
            assert_eq!(
                format_document(
                    &doc,
                    Some(Range::new(Position::new(6, 0), Position::new(9, 3)))
                )
                .unwrap(),
                text_fixed.to_string()
            );
        }
    }
}