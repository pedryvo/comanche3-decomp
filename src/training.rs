use crate::{
    flight::Flight,
    mission::Instruction,
    training_data::{TrainingData, WAYPOINT_RADIUS},
    trigger_runtime::{TriggerHost, TriggerRuntime},
};

pub(crate) struct Training<'program> {
    runtime: TriggerRuntime<'program>,
    waypoints: Vec<(f32, f32)>,
    visited: usize,
    requires_landing: bool,
}

impl<'program> Training<'program> {
    pub(crate) fn new(data: &'program TrainingData, flight: &mut Flight) -> Self {
        let waypoints: Vec<_> = data
            .offsets
            .iter()
            .map(|&(x, z)| (flight.x + x, flight.z + z))
            .collect();
        (flight.waypoint_x, flight.waypoint_z) = waypoints[0];
        flight.complete = false;
        Self {
            runtime: TriggerRuntime::new(&data.program),
            waypoints,
            visited: 0,
            requires_landing: data.requires_landing,
        }
    }

    pub(crate) fn tick(&mut self, flight: &mut Flight) -> Result<(), String> {
        let mut host = RouteHost {
            flight,
            waypoints: &self.waypoints,
            visited: &mut self.visited,
            requires_landing: self.requires_landing,
        };
        self.runtime.tick(&mut host).map(|_| ())
    }

    pub(crate) fn message(&self) -> &str {
        match self.visited {
            0 => "DECOLE E PASSE PELO PRIMEIRO MARCADOR",
            visited if visited == self.total() => "TREINAMENTO CONCLUIDO",
            _ if self.is_landing() => "FREIE E POUSE JUNTO AO MARCADOR",
            visited if visited + 1 == self.waypoints.len() => "PASSE PELO ULTIMO MARCADOR",
            _ => "SIGA ATE O PROXIMO MARCADOR",
        }
    }

    pub(crate) fn progress(&self) -> (usize, usize) {
        (self.visited, self.total())
    }

    pub(crate) fn is_landing(&self) -> bool {
        self.requires_landing && self.visited == self.waypoints.len()
    }

    fn total(&self) -> usize {
        self.waypoints.len() + usize::from(self.requires_landing)
    }
}

struct RouteHost<'a> {
    flight: &'a mut Flight,
    waypoints: &'a [(f32, f32)],
    visited: &'a mut usize,
    requires_landing: bool,
}

impl RouteHost<'_> {
    fn objective_index(&self, instruction: &Instruction) -> Result<usize, String> {
        let [number] = instruction.args.as_slice() else {
            return Err("expected one waypoint number".into());
        };
        number
            .parse::<usize>()
            .ok()
            .and_then(|number| number.checked_sub(1))
            .filter(|&index| index < self.waypoints.len() + usize::from(self.requires_landing))
            .ok_or_else(|| format!("unknown waypoint: {number}"))
    }
}

impl TriggerHost for RouteHost<'_> {
    fn predicate(&mut self, instruction: &Instruction) -> Result<bool, String> {
        let index = self.objective_index(instruction)?;
        if index != *self.visited {
            return Ok(false);
        }
        let (position, valid_flight) = match instruction.opcode.as_str() {
            "reached" => (self.waypoints.get(index), self.flight.is_airborne()),
            "landed" if self.requires_landing && index == self.waypoints.len() => {
                (self.waypoints.last(), self.flight.is_landed())
            }
            _ => {
                return Err(format!(
                    "unknown training condition: {}",
                    instruction.opcode
                ));
            }
        };
        let &(x, z) = position.ok_or("missing training objective position")?;
        Ok(valid_flight && (self.flight.x - x).hypot(self.flight.z - z) <= WAYPOINT_RADIUS)
    }

    fn action(&mut self, instruction: &Instruction) -> Result<(), String> {
        let index = self.objective_index(instruction)?;
        if index != *self.visited {
            return Err(format!("objective {} is out of sequence", index + 1));
        }
        match instruction.opcode.as_str() {
            "advance" if index < self.waypoints.len() => {
                *self.visited += 1;
                if let Some(&next) = self.waypoints.get(*self.visited) {
                    (self.flight.waypoint_x, self.flight.waypoint_z) = next;
                } else if !self.requires_landing {
                    self.flight.complete = true;
                }
            }
            "finish" if self.requires_landing && index == self.waypoints.len() => {
                *self.visited += 1;
                self.flight.complete = true;
            }
            _ => return Err(format!("unknown training action: {}", instruction.opcode)),
        }
        Ok(())
    }
}
