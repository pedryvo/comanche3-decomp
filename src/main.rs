mod flight;
mod render;
mod terrain;

use flight::{Controls, Flight};
use font8x8::{BASIC_FONTS, UnicodeFonts};
use minifb::{Key, KeyRepeat, ScaleMode, Window, WindowOptions};
use std::path::Path;
use std::time::Instant;
use terrain::Terrain;

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
    for argument in std::env::args().skip(1) {
        match argument.as_str() {
            "--1080p" => size = (1920, 1080),
            "--4k" => size = (3840, 2160),
            "--smoke" => smoke = true,
            "--demo" => demo = true,
            "--help" => {
                println!("comanche3-mvp [--1080p | --4k] [--demo | --smoke]");
                return Ok(());
            }
            _ => return Err(format!("Opcao desconhecida: {argument}")),
        }
    }

    let terrain = Terrain::load(Path::new("RESOURCE.RES"))?;
    let mut flight = Flight::new(&terrain);
    let mut frame = vec![0; size.0 * size.1];
    let scale = (size.0 / 640).max(1);

    if smoke {
        let start = Instant::now();
        for tick in 0..240 {
            flight.step(&terrain, demo_controls(tick as f32 / 60.0), 1.0 / 60.0);
        }
        if !flight.complete {
            return Err("O voo de teste nao atingiu o waypoint".into());
        }
        for _ in 0..10 {
            paint(&mut frame, size, scale, &terrain, &flight);
            std::hint::black_box(&frame);
        }
        let distinct = frame.iter().any(|pixel| *pixel != frame[0]);
        if !distinct {
            return Err("O renderer produziu uma imagem uniforme".into());
        }
        println!(
            "Waypoint atingido; {}x{}; {:.2} ms/quadro (renderer + HUD, sem janela)",
            size.0,
            size.1,
            start.elapsed().as_secs_f64() * 100.0
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
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let now = Instant::now();
        if window.is_key_pressed(Key::R, KeyRepeat::No) {
            flight = Flight::new(&terrain);
            started = now;
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
        flight.step(
            &terrain,
            controls,
            now.duration_since(previous).as_secs_f32(),
        );
        previous = now;
        paint(&mut frame, size, scale, &terrain, &flight);
        window
            .update_with_buffer(&frame, size.0, size.1)
            .map_err(|error| error.to_string())?;
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
) {
    render::draw(frame, size.0, size.1, terrain, flight);
    let distance = (flight.x - flight.waypoint_x).hypot(flight.z - flight.waypoint_z);
    let objective = if flight.complete {
        "WAYPOINT ATINGIDO".to_owned()
    } else {
        format!("WAYPOINT {:03.0}", distance)
    };
    let telemetry = format!("ALT {:03.0}  VEL {:02.0}", flight.altitude, flight.speed);
    let controls = "ESPACO SUBIR | SHIFT DESCER | WASD VOAR | R REINICIAR";
    for (line, label) in [objective.as_str(), &telemetry, controls]
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
