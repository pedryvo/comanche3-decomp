use crate::{
    flight::Flight,
    mission::{Instruction, Program},
    trigger_runtime::{TriggerHost, TriggerRuntime},
};

const WAYPOINT_RADIUS: f32 = 12.0;
const WAYPOINT_OFFSETS: [f32; 3] = [45.0, 90.0, 135.0];
const BRIEFINGS: [&str; 4] = [
    "DECOLE E PASSE PELO PRIMEIRO MARCADOR",
    "SIGA EM FRENTE ATE O SEGUNDO MARCADOR",
    "PASSE PELO ULTIMO MARCADOR",
    "TREINAMENTO CONCLUIDO",
];

const ROUTE: &[u8] = b"; Trigger Data BEGINS
if
reached, 1
then
advance, 1
if
reached, 2
then
advance, 2
if
reached, 3
then
advance, 3
<
; Trigger Data ENDS
";

pub(crate) struct Training<'program> {
    runtime: TriggerRuntime<'program>,
    waypoints: [(f32, f32); WAYPOINT_OFFSETS.len()],
    visited: usize,
}

impl<'program> Training<'program> {
    pub(crate) fn program() -> Result<Program, String> {
        Program::parse(ROUTE)
    }

    pub(crate) fn new(program: &'program Program, flight: &mut Flight) -> Self {
        let waypoints = WAYPOINT_OFFSETS.map(|offset| (flight.x, flight.z + offset));
        (flight.waypoint_x, flight.waypoint_z) = waypoints[0];
        flight.complete = false;
        Self {
            runtime: TriggerRuntime::new(program),
            waypoints,
            visited: 0,
        }
    }

    pub(crate) fn tick(&mut self, flight: &mut Flight) -> Result<(), String> {
        let mut host = RouteHost {
            flight,
            waypoints: &self.waypoints,
            visited: &mut self.visited,
        };
        self.runtime.tick(&mut host).map(|_| ())
    }

    pub(crate) fn message(&self) -> &str {
        BRIEFINGS[self.visited]
    }

    pub(crate) fn progress(&self) -> (usize, usize) {
        (self.visited, self.waypoints.len())
    }
}

struct RouteHost<'a> {
    flight: &'a mut Flight,
    waypoints: &'a [(f32, f32)],
    visited: &'a mut usize,
}

impl RouteHost<'_> {
    fn waypoint_index(&self, instruction: &Instruction) -> Result<usize, String> {
        let [number] = instruction.args.as_slice() else {
            return Err("expected one waypoint number".into());
        };
        number
            .parse::<usize>()
            .ok()
            .and_then(|number| number.checked_sub(1))
            .filter(|&index| index < self.waypoints.len())
            .ok_or_else(|| format!("unknown waypoint: {number}"))
    }
}

impl TriggerHost for RouteHost<'_> {
    fn predicate(&mut self, instruction: &Instruction) -> Result<bool, String> {
        if instruction.opcode != "reached" {
            return Err(format!(
                "unknown training condition: {}",
                instruction.opcode
            ));
        }
        let index = self.waypoint_index(instruction)?;
        let (x, z) = self.waypoints[index];
        Ok(index == *self.visited
            && self.flight.is_airborne()
            && (self.flight.x - x).hypot(self.flight.z - z) <= WAYPOINT_RADIUS)
    }

    fn action(&mut self, instruction: &Instruction) -> Result<(), String> {
        if instruction.opcode != "advance" {
            return Err(format!("unknown training action: {}", instruction.opcode));
        }
        let index = self.waypoint_index(instruction)?;
        if index != *self.visited {
            return Err(format!("waypoint {} is out of sequence", index + 1));
        }

        *self.visited += 1;
        if let Some(&next) = self.waypoints.get(*self.visited) {
            (self.flight.waypoint_x, self.flight.waypoint_z) = next;
        } else {
            self.flight.complete = true;
        }
        Ok(())
    }
}
