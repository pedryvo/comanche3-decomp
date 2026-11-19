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
        match self.visited {
            0 => "DECOLE E PASSE PELO PRIMEIRO MARCADOR",
            visited if visited == self.waypoints.len() => "TREINAMENTO CONCLUIDO",
            visited if visited + 1 == self.waypoints.len() => "PASSE PELO ULTIMO MARCADOR",
            _ => "SIGA ATE O PROXIMO MARCADOR",
        }
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
