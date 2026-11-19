use crate::mission::{Connector, Instruction, Program};

const MAX_POINTS: usize = 32;
pub(crate) const WAYPOINT_RADIUS: f32 = 12.0;

#[derive(Debug)]
pub(crate) struct TrainingData {
    pub offsets: Vec<(f32, f32)>,
    pub program: Program,
}

pub(crate) fn parse(data: &[u8]) -> Result<TrainingData, String> {
    let source = std::str::from_utf8(data).map_err(|error| format!("invalid UTF-8: {error}"))?;
    let mut offsets: Vec<(f32, f32)> = Vec::new();
    for (index, raw) in source.lines().enumerate() {
        let number = index + 1;
        if raw.trim_start().starts_with(';')
            && raw.to_ascii_lowercase().contains("trigger data begins")
        {
            break;
        }

        let line = raw.split(';').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        if offsets.len() == MAX_POINTS {
            return Err(format!(
                "line {number}: at most {MAX_POINTS} points are supported"
            ));
        }
        let mut fields = line.split(',').map(str::trim);
        let (Some("point"), Some(x), Some(z), None) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            return Err(format!("line {number}: expected point, dx, dz"));
        };
        let coordinate = |text: &str| {
            text.parse::<f32>()
                .ok()
                .filter(|value| value.is_finite())
                .ok_or_else(|| format!("line {number}: invalid finite coordinate: {text:?}"))
        };
        let (x, z) = (coordinate(x)?, coordinate(z)?);
        if x.hypot(z) <= WAYPOINT_RADIUS {
            return Err(format!(
                "line {number}: point must be more than {WAYPOINT_RADIUS} units from the start"
            ));
        }
        if let Some(other) = offsets.iter().position(|&(other_x, other_z)| {
            (x - other_x).hypot(z - other_z) <= 2.0 * WAYPOINT_RADIUS
        }) {
            return Err(format!(
                "line {number}: point overlaps the radius of point {}",
                other + 1
            ));
        }
        offsets.push((x, z));
    }
    if offsets.is_empty() {
        return Err("training requires at least one point".into());
    }

    let program = Program::parse(data)?;
    if program.rules.len() != offsets.len() {
        return Err(format!(
            "expected {} trigger rules, found {}",
            offsets.len(),
            program.rules.len()
        ));
    }
    for (index, rule) in program.rules.iter().enumerate() {
        let number = index + 1;
        if !matches!(&rule.conditions[..], [(Connector::And, instruction)]
            if matches_instruction(instruction, "reached", number))
        {
            return Err(format!(
                "line {}: rule {number} requires only the condition reached, {number}",
                rule.action.line
            ));
        }
        if !matches_instruction(&rule.action, "advance", number) {
            return Err(format!(
                "line {}: rule {number} requires the action advance, {number}",
                rule.action.line
            ));
        }
    }

    Ok(TrainingData { offsets, program })
}

fn matches_instruction(instruction: &Instruction, opcode: &str, point: usize) -> bool {
    instruction.opcode == opcode
        && matches!(&instruction.args[..], [number] if number.parse::<usize>() == Ok(point))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_offsets_and_retains_source_lines() {
        let data = b"; Comment\r\n\r\npoint, -5.5, 40 ; turn left\r\n; Trigger Data BEGINS\r\nif\r\nreached, 1\r\nthen\r\nadvance, 1\r\n<\r\n; Trigger Data ENDS\r\n";
        let route = parse(data).unwrap();
        assert_eq!(route.offsets, [(-5.5, 40.0)]);
        assert_eq!(route.program.rules[0].action.line, 8);
    }

    #[test]
    fn rejects_invalid_or_overlapping_points() {
        for (source, error) in [
            ("point, NaN, 40", "invalid finite coordinate"),
            ("point, 0, 12", "units from the start"),
            ("point, 0, 40\npoint, 0, 64", "overlaps"),
        ] {
            assert!(parse(source.as_bytes()).unwrap_err().contains(error));
        }
    }

    #[test]
    fn rejects_nonsequential_rules() {
        let data = b"point, 10, 40\n; Trigger Data BEGINS\nif\nreached, 1\nthen\nadvance, 2\n<\n; Trigger Data ENDS\n";
        assert!(parse(data).unwrap_err().contains("advance, 1"));
    }
}
