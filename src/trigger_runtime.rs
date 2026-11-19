use crate::mission::{Connector, Instruction, Program, Rule};

pub(crate) trait TriggerHost {
    fn predicate(&mut self, instruction: &Instruction) -> Result<bool, String>;
    fn action(&mut self, instruction: &Instruction) -> Result<(), String>;
}

type Evaluator = fn(&Rule, &mut dyn TriggerHost) -> Result<bool, String>;

pub(crate) struct TriggerRuntime<'program> {
    program: &'program Program,
    fired: Vec<bool>,
    evaluator: Evaluator,
}

impl<'program> TriggerRuntime<'program> {
    pub(crate) fn new(program: &'program Program) -> Self {
        Self::with_evaluator(program, evaluate_groups)
    }

    pub(crate) fn with_evaluator(program: &'program Program, evaluator: Evaluator) -> Self {
        Self {
            program,
            fired: vec![false; program.rules.len()],
            evaluator,
        }
    }

    /// Evaluates pending rules in source order. A failed action remains pending.
    /// Hosts should commit their action effects only when returning success.
    pub(crate) fn tick(&mut self, host: &mut dyn TriggerHost) -> Result<usize, String> {
        let mut count = 0;
        for (rule, fired) in self.program.rules.iter().zip(&mut self.fired) {
            if *fired || !(self.evaluator)(rule, host)? {
                continue;
            }
            let action = &rule.action;
            host.action(action)
                .map_err(|error| contextualize(action, error))?;
            *fired = true;
            count += 1;
        }
        Ok(count)
    }
}

// Provisional policy: OR starts a new conjunction. Repeated constraints
// in c3m2 support this interpretation pending comparison with the DOS game.
fn evaluate_groups(rule: &Rule, host: &mut dyn TriggerHost) -> Result<bool, String> {
    let Some(((first_connector, first), rest)) = rule.conditions.split_first() else {
        return Err(format!("line {}: rule has no condition", rule.action.line));
    };
    if *first_connector != Connector::And {
        return Err(format!("line {}: invalid first connector", first.line));
    }
    let mut evaluate = |instruction: &Instruction| {
        host.predicate(instruction)
            .map_err(|error| contextualize(instruction, error))
    };
    let mut value = evaluate(first)?;
    for (connector, instruction) in rest {
        value = match connector {
            Connector::And => value && evaluate(instruction)?,
            Connector::Or if value => return Ok(true),
            Connector::Or => evaluate(instruction)?,
            Connector::AndNot => value && !evaluate(instruction)?,
        };
    }
    Ok(value)
}

fn contextualize(instruction: &Instruction, error: String) -> String {
    format!(
        "line {} ({}): {error}",
        instruction.line, instruction.opcode
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Host {
        enabled: bool,
        fail_action: Option<&'static str>,
        actions: Vec<String>,
        predicates: Vec<String>,
    }

    impl TriggerHost for Host {
        fn predicate(&mut self, instruction: &Instruction) -> Result<bool, String> {
            self.predicates.push(instruction.opcode.clone());
            Ok(instruction.opcode == "true" || self.enabled)
        }

        fn action(&mut self, instruction: &Instruction) -> Result<(), String> {
            if self.fail_action == Some(instruction.opcode.as_str()) {
                return Err("try again".into());
            }
            self.actions.push(instruction.opcode.clone());
            Ok(())
        }
    }

    fn instruction(opcode: &str, line: usize) -> Instruction {
        Instruction {
            opcode: opcode.to_owned(),
            args: Vec::new(),
            line,
        }
    }

    fn rule(conditions: &[(Connector, &str)], action: &str) -> Rule {
        Rule {
            conditions: conditions
                .iter()
                .enumerate()
                .map(|(index, &(connector, opcode))| (connector, instruction(opcode, index + 1)))
                .collect(),
            action: instruction(action, conditions.len() + 1),
        }
    }

    #[test]
    fn runs_each_matching_rule_once() {
        let program = Program {
            rules: vec![rule(&[(Connector::And, "ready")], "done")],
        };
        let mut runtime = TriggerRuntime::new(&program);
        let mut host = Host::default();
        assert_eq!(runtime.tick(&mut host), Ok(0));
        host.enabled = true;
        assert_eq!(runtime.tick(&mut host), Ok(1));
        assert_eq!(runtime.tick(&mut host), Ok(0));
        assert_eq!(host.actions, ["done"]);
    }

    #[test]
    fn retries_failed_action_without_replaying_successes() {
        let program = Program {
            rules: vec![
                rule(&[(Connector::And, "true")], "first"),
                rule(&[(Connector::And, "true")], "second"),
            ],
        };
        let mut runtime = TriggerRuntime::new(&program);
        let mut host = Host {
            fail_action: Some("second"),
            ..Host::default()
        };
        assert_eq!(
            runtime.tick(&mut host),
            Err("line 2 (second): try again".into())
        );
        host.fail_action = None;
        assert_eq!(runtime.tick(&mut host), Ok(1));
        assert_eq!(host.actions, ["first", "second"]);
    }

    #[test]
    fn true_group_short_circuits_later_conjunctions() {
        let program = Program {
            rules: vec![rule(
                &[
                    (Connector::And, "true"),
                    (Connector::Or, "skipped"),
                    (Connector::AndNot, "true"),
                    (Connector::And, "also skipped"),
                ],
                "done",
            )],
        };
        let mut runtime = TriggerRuntime::new(&program);
        let mut host = Host::default();
        assert_eq!(runtime.tick(&mut host), Ok(1));
        assert_eq!(host.predicates, ["true"]);
    }
    #[test]
    fn false_group_falls_back_to_next_conjunction() {
        let program = Program {
            rules: vec![rule(
                &[
                    (Connector::And, "false"),
                    (Connector::And, "skipped"),
                    (Connector::Or, "true"),
                    (Connector::AndNot, "false"),
                ],
                "done",
            )],
        };
        let mut host = Host::default();
        assert_eq!(TriggerRuntime::new(&program).tick(&mut host), Ok(1));
        assert_eq!(host.predicates, ["false", "true", "false"]);
    }
}
