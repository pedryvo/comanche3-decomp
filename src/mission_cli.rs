use std::{
    collections::{BTreeMap, HashSet},
    path::Path,
};

use crate::{
    mission::{Instruction, Program},
    terrain::ResourceArchive,
    trigger_runtime::{TriggerHost, TriggerRuntime},
};

pub(crate) fn audit(path: &Path) -> Result<(), String> {
    let mut archive = ResourceArchive::open(path).map_err(|err| err.to_string())?;
    let names = archive.mission_names().map_err(|err| err.to_string())?;
    if names.is_empty() {
        return Err("No mission files found in archive".into());
    }
    let mut totals = BTreeMap::<_, (usize, usize, usize)>::new();
    let mut conditions = HashSet::new();
    let mut actions = HashSet::new();
    for name in names {
        let program = read_program(&mut archive, &name)?;
        let group = if name.starts_with("mp") {
            "multiplayer"
        } else {
            "campaign-named"
        };
        let stats = totals.entry(group).or_default();
        stats.0 += 1;
        stats.1 += program.rules.len();
        stats.2 += usize::from(program.rules.is_empty());
        for rule in program.rules {
            conditions.extend(
                rule.conditions
                    .into_iter()
                    .map(|(_, instruction)| instruction.opcode),
            );
            actions.insert(rule.action.opcode);
        }
    }
    for (group, (files, rules, empty)) in totals {
        println!("{group}: {files} files, {rules} rules, {empty} empty blocks");
    }
    println!(
        "Corpus: {} condition names, {} action names",
        conditions.len(),
        actions.len()
    );
    Ok(())
}

pub(crate) fn trace(path: &Path, name: &str, true_conditions: &[String]) -> Result<(), String> {
    let mut archive = ResourceArchive::open(path).map_err(|err| err.to_string())?;
    let program = read_program(&mut archive, name)?;
    let mut facts = HashSet::new();
    for fact in true_conditions {
        let mut parts = fact.split(',').map(str::trim);
        let opcode = parts.next().unwrap_or_default().to_ascii_lowercase();
        let args = parts.map(str::to_owned).collect::<Vec<_>>();
        if !program
            .rules
            .iter()
            .flat_map(|rule| &rule.conditions)
            .any(|(_, instruction)| instruction.opcode == opcode && instruction.args == args)
        {
            return Err(format!("Condition not found in {name}: {fact}"));
        }
        facts.insert((opcode, args));
    }

    println!("{name}: synthetic predicate inputs, provisional OR-of-conjunctions evaluation");
    let mut host = TraceHost { facts };
    let mut runtime = TriggerRuntime::new(&program);
    println!("Tick 1: {} actions", runtime.tick(&mut host)?);
    println!("Tick 2 (same inputs): {} actions", runtime.tick(&mut host)?);
    Ok(())
}

fn read_program(archive: &mut ResourceArchive, name: &str) -> Result<Program, String> {
    let data = archive
        .read(name.as_bytes())
        .map_err(|err| err.to_string())?;
    Program::parse(&data).map_err(|err| format!("{name}: {err}"))
}

struct TraceHost {
    facts: HashSet<(String, Vec<String>)>,
}

impl TriggerHost for TraceHost {
    fn predicate(&mut self, instruction: &Instruction) -> Result<bool, String> {
        Ok(self
            .facts
            .iter()
            .any(|(opcode, args)| *opcode == instruction.opcode && *args == instruction.args))
    }

    fn action(&mut self, instruction: &Instruction) -> Result<(), String> {
        println!("  line {}: {}", instruction.line, instruction.opcode);
        Ok(())
    }
}
