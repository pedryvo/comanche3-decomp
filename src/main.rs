mod aircraft;
mod flight;
mod mission;
mod mission_cli;
mod render;
mod terrain;
mod training;
mod trigger_runtime;

use aircraft::Aircraft;
use flight::{Controls, Flight};
use font8x8::{BASIC_FONTS, UnicodeFonts};
use minifb::{Key, KeyRepeat, MouseButton, MouseMode, ScaleMode, Window, WindowOptions};
use render::{ExternalView, Orbit};
use std::path::Path;
use std::time::Instant;
use terrain::Terrain;
use training::Training;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut size = (1280, 720);
    let mut smoke = false;
    let mut demo = false;
    let mut chase = true;
    let resources = Path::new("RESOURCE.RES");
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--1080p" => size = (1920, 1080),
            "--4k" => size = (3840, 2160),
            "--smoke" => smoke = true,
            "--demo" => demo = true,
            "--cockpit" => chase = false,
            "--check-missions" if arguments.len() == 0 => return mission_cli::audit(resources),
            "--trace-mission" => {
                let name = arguments
                    .next()
                    .ok_or("Expected a .mis filename after --trace-mission")?;
                return mission_cli::trace(resources, &name, &arguments.collect::<Vec<_>>());
            }
            "--help" => {
                println!("comanche3-mvp [--1080p | --4k] [--demo | --smoke] [--cockpit]");
                println!("comanche3-mvp --check-missions");
                println!("comanche3-mvp --trace-mission <file.mis> ['condition, arg' ...]");
                return Ok(());
            }
            _ => return Err(format!("Opcao desconhecida: {argument}")),
        }
    }

    let terrain = Terrain::load(resources)?;
    let aircraft = Aircraft::load(resources)?;
    let mut orbit = Orbit::default();
    let mut flight = Flight::new(&terrain);
    let program = Training::program()?;
    let mut training = Training::new(&program, &mut flight);
    let mut frame = vec![0; size.0 * size.1];
    let scale = (size.0 / 640).max(1);

    if smoke {
        check_training_guards(&terrain, &program)?;
        let (mut visited, total) = training.progress();
        for tick in 0..360 {
            flight.step(&terrain, demo_controls(tick as f32 / 60.0), 1.0 / 60.0);
            training.tick(&mut flight)?;
            let (current, _) = training.progress();
            if current > visited {
                println!("Ponto {current}/{total} confirmado");
                visited = current;
            }
        }
        if !flight.complete || visited != total {
            return Err(format!(
                "O voo de teste terminou com {visited}/{total} pontos"
            ));
        }
        let start = Instant::now();
        for tick in 0..10 {
            orbit.rotate(std::f32::consts::TAU / 10.0, 0.05);
            paint(
                &mut frame,
                size,
                scale,
                &terrain,
                &flight,
                &training,
                chase.then_some(ExternalView {
                    aircraft: &aircraft,
                    orbit: &orbit,
                    rotor_angle: tick as f32 * 0.3,
                }),
            );
            std::hint::black_box(&frame);
        }
        let render_elapsed = start.elapsed();
        flight = Flight::new(&terrain);
        training = Training::new(&program, &mut flight);
        training.tick(&mut flight)?;
        if flight.complete || training.progress() != (0, total) {
            return Err("O treinamento manteve o progresso apos reiniciar".into());
        }
        let distinct = frame.iter().any(|pixel| *pixel != frame[0]);
        if !distinct {
            return Err("O renderer produziu uma imagem uniforme".into());
        }
        println!(
            "Treinamento concluido; {}x{}; {:.2} ms/quadro (renderer + HUD, sem janela)",
            size.0,
            size.1,
            render_elapsed.as_secs_f64() * 100.0
        );
        return Ok(());
    }

    let monitor = Window::new(
        "Comanche 3 | MVP",
        size.0.min(1280),
        size.1.min(720),
        WindowOptions {
            resize: true,
            scale_mode: ScaleMode::AspectRatioStretch,
            ..WindowOptions::default()
        },
    );
    let mut window = monitor.map_err(|error| format!("Erro ao abrir a janela: {error}"))?;
    window.set_target_fps(60);
    let mut started = Instant::now();
    let mut previous = started;
    let mut rotor_angle = 0.0;
    let mut mouse_anchor = None::<(f32, f32)>;
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let now = Instant::now();
        if window.is_key_pressed(Key::R, KeyRepeat::No) {
            flight = Flight::new(&terrain);
            training = Training::new(&program, &mut flight);
            started = now;
        }
        if window.is_key_pressed(Key::V, KeyRepeat::No) {
            chase = !chase;
        }
        let down = |key| window.is_key_down(key);
        let controls = if demo {
            demo_controls(now.duration_since(started).as_secs_f32())
        } else {
            Controls {
                accelerate: down(Key::W) || down(Key::Up),
                brake: down(Key::S) || down(Key::Down),
                left: down(Key::A) || down(Key::Left),
                right: down(Key::D) || down(Key::Right),
                climb: down(Key::Space),
                descend: down(Key::LeftShift) || down(Key::RightShift),
            }
        };
        let dt = now.duration_since(previous).as_secs_f32();
        if chase {
            let axis =
                |positive, negative| (i8::from(down(positive)) - i8::from(down(negative))) as f32;
            orbit.rotate(
                axis(Key::L, Key::J) * dt.min(0.1) * 2.0,
                axis(Key::I, Key::K) * dt.min(0.1),
            );
            let cursor = if window.get_mouse_down(MouseButton::Right) {
                window.get_mouse_pos(MouseMode::Discard)
            } else {
                None
            };
            if let (Some((old_x, old_y)), Some((x, y))) = (mouse_anchor, cursor) {
                orbit.rotate((old_x - x) * 0.008, (y - old_y) * 0.006);
            }
            mouse_anchor = cursor;
        } else {
            mouse_anchor = None;
        }
        flight.step(&terrain, controls, dt);
        training.tick(&mut flight)?;
        rotor_angle = (rotor_angle + dt * 24.0).rem_euclid(std::f32::consts::TAU);
        previous = now;
        paint(
            &mut frame,
            size,
            scale,
            &terrain,
            &flight,
            &training,
            chase.then_some(ExternalView {
                aircraft: &aircraft,
                orbit: &orbit,
                rotor_angle,
            }),
        );
        window
            .update_with_buffer(&frame, size.0, size.1)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn check_training_guards(terrain: &Terrain, program: &mission::Program) -> Result<(), String> {
    let mut flight = Flight::new(terrain);
    let mut training = Training::new(program, &mut flight);
    flight.z = flight.waypoint_z;
    training.tick(&mut flight)?;
    if training.progress().0 != 0 || flight.complete {
        return Err("O treinamento aceitou uma passagem no solo".into());
    }

    flight.z += 45.0;
    for _ in 0..10 {
        flight.step(
            terrain,
            Controls {
                climb: true,
                ..Controls::default()
            },
            1.0 / 60.0,
        );
    }
    training.tick(&mut flight)?;
    if !flight.is_airborne() || training.progress().0 != 0 || flight.complete {
        return Err("O treinamento aceitou uma passagem fora de ordem".into());
    }
    Ok(())
}

fn demo_controls(elapsed: f32) -> Controls {
    Controls {
        climb: elapsed < 1.0,
        accelerate: elapsed < 3.5,
        brake: elapsed >= 3.5,
        ..Controls::default()
    }
}

fn paint(
    frame: &mut [u32],
    size: (usize, usize),
    scale: usize,
    terrain: &Terrain,
    flight: &Flight,
    training: &Training<'_>,
    chase: Option<ExternalView<'_>>,
) {
    render::draw(frame, size, terrain, flight, chase);
    let distance = (flight.x - flight.waypoint_x).hypot(flight.z - flight.waypoint_z);
    let (visited, total) = training.progress();
    let objective = if flight.complete {
        format!("TREINAMENTO CONCLUIDO  {visited}/{total}")
    } else {
        format!("PONTO {}/{total}  DIST {:03.0}", visited + 1, distance)
    };
    let telemetry = format!("ALT {:03.0}  VEL {:02.0}", flight.altitude, flight.speed);
    let controls = "ESPACO SUBIR | SHIFT DESCER | WASD VOAR | V CAMERA | R REINICIAR";
    for (line, label) in [
        objective.as_str(),
        training.message(),
        &telemetry,
        controls,
        "IJKL OU BOTAO DIREITO: GIRAR CAMERA",
    ]
    .into_iter()
    .enumerate()
    {
        text(
            frame,
            size,
            8 * scale,
            (8 + line * 12) * scale,
            scale,
            label,
        );
    }
}

fn text(
    frame: &mut [u32],
    (width, height): (usize, usize),
    x: usize,
    y: usize,
    scale: usize,
    label: &str,
) {
    for (column, ch) in label.chars().enumerate() {
        let Some(glyph) = BASIC_FONTS.get(ch) else {
            continue;
        };
        for (row, mask) in glyph.into_iter().enumerate() {
            for bit in 0..8 {
                if mask & (1 << bit) == 0 {
                    continue;
                }
                let left = x + (column * 8 + bit) * scale;
                let top = y + row * scale;
                for py in top..(top + scale).min(height) {
                    for px in left..(left + scale).min(width) {
                        frame[py * width + px] = 0xe7_ffe5;
                    }
                }
            }
        }
    }
}
