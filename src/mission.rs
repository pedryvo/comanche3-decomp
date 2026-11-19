/// Supported connectors in the on-disk syntax. Evaluation is handled separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Connector {
    And,
    Or,
    AndNot,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Instruction {
    pub opcode: String,
    pub args: Vec<String>,
    pub line: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Rule {
    /// The initial condition has an implicit `And`.
    pub conditions: Vec<(Connector, Instruction)>,
    pub action: Instruction,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Program {
    pub rules: Vec<Rule>,
}

impl Program {
    pub(crate) fn parse(data: &[u8]) -> Result<Self, String> {
        // Decode each byte without requiring UTF-8 or discarding DOS text.
        let source: String = data.iter().copied().map(char::from).collect();
        let mut lines = source
            .lines()
            .enumerate()
            .map(|(index, line)| (index + 1, line));
        let is_marker = |line: &str, marker| {
            line.trim_start().starts_with(';') && line.to_ascii_lowercase().contains(marker)
        };

        if !lines.any(|(_, line)| is_marker(line, "trigger data begins")) {
            return Err("missing trigger data begin marker".into());
        }

        let mut rules = Vec::new();
        loop {
            let (number, line) = lines
                .find_map(next_statement)
                .ok_or("unexpected end of file while looking for IF or <")?;
            if line == "<" {
                if lines.any(|(_, line)| is_marker(line, "trigger data ends")) {
                    return Ok(Self { rules });
                }
                return Err(format!(
                    "line {number}: missing trigger data end marker after <"
                ));
            }
            if !line.eq_ignore_ascii_case("if") {
                return Err(format!("line {number}: expected IF or <"));
            }

            let mut conditions = Vec::new();
            let mut connector = Connector::And;
            loop {
                let (number, line) = lines
                    .find_map(next_statement)
                    .ok_or("unexpected end of file while looking for a condition")?;
                conditions.push((connector, Instruction::parse(number, line)?));
                let (number, line) = lines
                    .find_map(next_statement)
                    .ok_or("unexpected end of file while looking for THEN or a connector")?;
                connector = match line.to_ascii_lowercase().as_str() {
                    "then" => break,
                    "and" => Connector::And,
                    "or" => Connector::Or,
                    "andn" => Connector::AndNot,
                    _ => return Err(format!("line {number}: expected THEN, AND, OR, or ANDN")),
                };
            }

            let (number, line) = lines
                .find_map(next_statement)
                .ok_or("unexpected end of file while looking for an action")?;
            rules.push(Rule {
                conditions,
                action: Instruction::parse(number, line)?,
            });
        }
    }
}

fn next_statement((number, raw): (usize, &str)) -> Option<(usize, &str)> {
    let line = raw.split(';').next().unwrap_or_default().trim();
    (!line.is_empty()).then_some((number, line))
}

impl Instruction {
    fn parse(number: usize, line: &str) -> Result<Self, String> {
        let mut parts = line.split(',').map(str::trim);
        let opcode = parts.next().unwrap_or_default().to_ascii_lowercase();
        if !opcode
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            || opcode.is_empty()
            || matches!(opcode.as_str(), "if" | "then" | "and" | "or" | "andn")
        {
            return Err(format!(
                "line {number}: expected a condition or action opcode"
            ));
        }
        let args: Vec<String> = parts.map(str::to_owned).collect();
        if args.iter().any(String::is_empty) {
            return Err(format!("line {number}: empty opcode argument"));
        }
        Ok(Self {
            opcode,
            args,
            line: number,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_embedded_nul_and_accepts_mixed_case_connector() {
        let data = b"; Trigger Data BEGINS\r\nif\r\nfarther, 300, \x000\r\nandN\r\nalive, b\r\nthen\r\ntext, 1\r\n<\r\n; Trigger Data ENDS\r\n";
        let program = Program::parse(data).unwrap();
        let conditions = &program.rules[0].conditions;
        assert_eq!(conditions[0].1.args[1].as_bytes(), b"\x000");
        assert_eq!(conditions[1].0, Connector::AndNot);
        assert_eq!(conditions[1].1.args, ["b"]);
        assert_eq!(conditions[1].1.line, 5);
    }
}
