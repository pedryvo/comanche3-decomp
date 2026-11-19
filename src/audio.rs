use std::{io::Cursor, path::Path};

use kira::{
    AudioManager, AudioManagerSettings, Decibels, Tween,
    sound::static_sound::{StaticSoundData, StaticSoundHandle},
};

use crate::terrain::ResourceArchive;

struct Clips {
    rotor: StaticSoundData,
    marker: StaticSoundData,
}

impl Clips {
    fn load(path: &Path) -> Result<Self, String> {
        let mut archive = ResourceArchive::open(path).map_err(|error| error.to_string())?;
        let mut read = |name: &str| {
            let bytes = archive
                .read(name.as_bytes())
                .map_err(|error| error.to_string())?;
            StaticSoundData::from_cursor(Cursor::new(bytes))
                .map_err(|error| format!("{name}: {error}"))
        };
        Ok(Self {
            rotor: read("INSIDE.WAV")?,
            marker: read("ALERT.WAV")?,
        })
    }
}

pub(crate) fn check(path: &Path) -> Result<(), String> {
    let clips = Clips::load(path)?;
    for (name, clip) in [("INSIDE.WAV", clips.rotor), ("ALERT.WAV", clips.marker)] {
        println!("{name}: {:.2} s decoded", clip.duration().as_secs_f64());
    }
    Ok(())
}

pub(crate) struct MissionAudio {
    manager: AudioManager,
    clips: Clips,
    rotor: Option<StaticSoundHandle>,
}

impl MissionAudio {
    pub(crate) fn load(path: &Path) -> Result<Self, String> {
        let clips = Clips::load(path)?;
        let manager = AudioManager::new(AudioManagerSettings::default())
            .map_err(|error| format!("output device: {error}"))?;
        Ok(Self {
            manager,
            clips,
            rotor: None,
        })
    }

    pub(crate) fn start(&mut self, chase: bool) {
        self.stop();
        self.rotor = self
            .manager
            .play(self.clips.rotor.loop_region(..).volume(rotor_volume(chase)))
            .ok();
    }

    pub(crate) fn stop(&mut self) {
        if let Some(mut rotor) = self.rotor.take() {
            rotor.stop(Tween::default());
        }
    }

    pub(crate) fn set_camera(&mut self, chase: bool) {
        if let Some(rotor) = self.rotor.as_mut() {
            rotor.set_volume(rotor_volume(chase), Tween::default());
        }
    }

    pub(crate) fn waypoint(&mut self) {
        let _ = self.manager.play(self.clips.marker.volume(Decibels(-15.0)));
    }
}

fn rotor_volume(chase: bool) -> Decibels {
    Decibels(if chase { -22.0 } else { -16.0 })
}
