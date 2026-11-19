mod aircraft;
mod flight;
mod mission;
mod mission_cli;
mod render;
mod terrain;
mod training;
mod training_data;
mod trigger_runtime;

use aircraft::Aircraft;
use flight::{Controls, Flight};
use font8x8::{BASIC_FONTS, UnicodeFonts};
use minifb::{Key, KeyRepeat, MouseButton, MouseMode, ScaleMode, Window, WindowOptions};
use render::{ExternalView, Orbit};
use std::time::Instant;
use std::{borrow::Cow, path::Path};
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
    let mut training_path = None;
    let mut chase = true;
    let resources = Path::new("RESOURCE.RES");
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--1080p" => size = (1920, 1080),
            "--4k" => size = (3840, 2160),
            "--smoke" => smoke = true,
            "--demo" => demo = true,
            "--training" => {
                training_path = Some(
                    arguments
                        .next()
                        .ok_or("Informe o arquivo apos --training")?,
                );
            }
            "--cockpit" => chase = false,
            "--check-missions" if arguments.len() == 0 => return mission_cli::audit(resources),
            "--trace-mission" => {
                let name = arguments
                    .next()
                    .ok_or("Expected a .mis filename after --trace-mission")?;
                return mission_cli::trace(resources, &name, &arguments.collect::<Vec<_>>());
            }
            "--help" => {
                println!(
                    "comanche3-mvp [--1080p | --4k] [--demo | --smoke] [--cockpit] [--training <arquivo>]"
                );
                println!("comanche3-mvp --check-missions");
                println!("comanche3-mvp --trace-mission <file.mis> ['condition, arg' ...]");
                return Ok(());
            }
            _ => return Err(format!("Opcao desconhecida: {argument}")),
        }
    }

    let terrain = Terrain::load(resources)?;
    let route_source = match training_path.as_deref() {
        None => Cow::Borrowed(include_bytes!("../missions/straight.route").as_slice()),
        Some(path) => Cow::Owned(std::fs::read(path).map_err(|error| format!("{path}: {error}"))?),
    };
    let route = training_data::parse(&route_source).map_err(|error| {
        format!(
            "{}: {error}",
            training_path.as_deref().unwrap_or("straight.route")
        )
    })?;
    let aircraft = Aircraft::load(resources)?;
    let mut orbit = Orbit::default();
    let mut flight = Flight::new(&terrain);
    for (index, &(dx, dz)) in route.offsets.iter().enumerate() {
        let x = flight.x + dx;
        let z = flight.z + dz;
        if !(0.0..terrain.width as f32).contains(&x) || !(0.0..terrain.height as f32).contains(&z) {
            return Err(format!("Ponto {} fora dos limites do terreno", index + 1));
        }
    }
    let mut training = Training::new(&route, &mut flight);
    let mut frame = vec![0; size.0 * size.1];
    let scale = (size.0 / 640).max(1);

    if smoke {
        check_training_guards(&terrain, &route)?;
        let (mut visited, total) = training.progress();
        for tick in 0..9000 * total {
            flight.step(
                &terrain,
                demo_controls(&flight, training.is_landing(), tick as f32 / 60.0),
                1.0 / 60.0,
            );
            training.tick(&mut flight)?;
            let (current, _) = training.progress();
            if current > visited {
                println!("Ponto {current}/{total} confirmado");
                visited = current;
            }
            if flight.complete {
                break;
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
            draw_briefing_panel(&mut frame, size, scale, &training, true);
            std::hint::black_box(&frame);
        }
        let render_elapsed = start.elapsed();
        flight = Flight::new(&terrain);
        training = Training::new(&route, &mut flight);
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
    let mut briefing = true;
    let mut rotor_angle = 0.0;
    let mut mouse_anchor = None::<(f32, f32)>;
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let now = Instant::now();
        let enter = window.is_key_pressed(Key::Enter, KeyRepeat::No);
        if window.is_key_pressed(Key::R, KeyRepeat::No) || (enter && flight.complete) {
            flight = Flight::new(&terrain);
            training = Training::new(&route, &mut flight);
            briefing = true;
            started = now;
        } else if briefing && (enter || (demo && now.duration_since(started).as_secs_f32() >= 2.0))
        {
            briefing = false;
            started = now;
        }
        if window.is_key_pressed(Key::V, KeyRepeat::No) {
            chase = !chase;
        }
        let down = |key| window.is_key_down(key);
        let controls = if demo {
            demo_controls(
                &flight,
                training.is_landing(),
                now.duration_since(started).as_secs_f32(),
            )
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
        if !briefing && !flight.complete {
            flight.step(&terrain, controls, dt);
            training.tick(&mut flight)?;
        }
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
        if briefing || flight.complete {
            draw_briefing_panel(&mut frame, size, scale, &training, flight.complete);
        }
        window
            .update_with_buffer(&frame, size.0, size.1)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn check_training_guards(
    terrain: &Terrain,
    route: &training_data::TrainingData,
) -> Result<(), String> {
    let mut flight = Flight::new(terrain);
    let start = (flight.x, flight.z);
    let mut training = Training::new(route, &mut flight);
    (flight.x, flight.z) = (flight.waypoint_x, flight.waypoint_z);
    training.tick(&mut flight)?;
    if training.progress().0 != 0 || flight.complete {
        return Err("O treinamento aceitou uma passagem no solo".into());
    }

    if let Some(&(dx, dz)) = route.offsets.get(1) {
        (flight.x, flight.z) = (start.0 + dx, start.1 + dz);
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
    }
    Ok(())
}

fn demo_controls(flight: &Flight, landing: bool, elapsed: f32) -> Controls {
    if flight.complete {
        return Controls {
            brake: true,
            ..Controls::default()
        };
    }
    let dx = flight.waypoint_x - flight.x;
    let dz = flight.waypoint_z - flight.z;
    if landing && dx.hypot(dz) < 7.0 {
        return Controls {
            brake: true,
            descend: flight.speed < 2.0,
            ..Controls::default()
        };
    }
    let aim = dx.atan2(dz);
    let error = (aim - flight.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;
    let aligned = error.abs() < 0.35;
    let cruise = if landing { 9.0 } else { 24.0 };
    Controls {
        climb: elapsed < 1.0 || (landing && !flight.is_airborne()),
        accelerate: aligned && flight.speed < cruise,
        brake: !aligned || flight.speed > cruise + 2.0,
        left: error < -0.025,
        right: error > 0.025,
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
    } else if training.is_landing() {
        format!("POUSO  DIST {:03.0}", distance)
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

fn draw_briefing_panel(
    frame: &mut [u32],
    size @ (width, height): (usize, usize),
    scale: usize,
    training: &Training<'_>,
    complete: bool,
) {
    let panel_width = 440 * scale;
    let left = (width - panel_width) / 2;
    let top = height / 3;
    let (visited, total) = training.progress();
    let detail = if complete {
        format!("{visited}/{total} OBJETIVOS CONFIRMADOS")
    } else {
        format!("{total} OBJETIVOS  |  SIGA O MARCADOR")
    };
    let labels = if complete {
        ["MISSAO CONCLUIDA", &detail, "ENTER: VOLTAR AO BRIEFING"]
    } else {
        ["TREINAMENTO DE NAVEGACAO", &detail, "ENTER: INICIAR VOO"]
    };
    for y in top..top + 78 * scale {
        let color = if y < top + 2 * scale {
            0x8cae73
        } else {
            0x152529
        };
        frame[y * width + left..y * width + left + panel_width].fill(color);
    }
    for (row, label) in labels.into_iter().enumerate() {
        text(
            frame,
            size,
            left + 14 * scale,
            top + (13 + row * 19) * scale,
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
