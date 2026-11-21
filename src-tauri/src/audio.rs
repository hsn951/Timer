use crate::store::{sounds_dir, SoundMeta};
use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::time::Duration;

pub enum Cmd {
    Play {
        path: PathBuf,
        volume: f32,
        fade: bool,
    },
    Stop,
}

#[derive(Clone)]
pub struct AudioHandle {
    tx: Sender<Cmd>,
}

impl AudioHandle {
    pub fn play(&self, path: PathBuf, volume: f32, fade: bool) {
        let _ = self.tx.send(Cmd::Play {
            path,
            volume: volume.clamp(0.0, 1.0),
            fade,
        });
    }

    pub fn stop(&self) {
        let _ = self.tx.send(Cmd::Stop);
    }
}

/// 音频线程独占 OutputStream（cpal 的 Stream 不是 Send），通过 channel 通信。
pub fn spawn() -> AudioHandle {
    let (tx, rx) = mpsc::channel::<Cmd>();
    std::thread::Builder::new()
        .name("audio".into())
        .spawn(move || {
            let (_stream, handle) = match OutputStream::try_default() {
                Ok(v) => v,
                Err(err) => {
                    eprintln!("[audio] 没有可用的音频设备: {err}");
                    return;
                }
            };
            let mut current: Option<Sink> = None;
            let mut fade_target: Option<f32> = None;
            let mut fade_value: f32 = 0.0;

            loop {
                match rx.recv_timeout(Duration::from_millis(30)) {
                    Ok(Cmd::Play {
                        path,
                        volume,
                        fade,
                    }) => {
                        if let Some(sink) = current.take() {
                            sink.stop();
                        }
                        fade_target = None;

                        let file = match File::open(&path) {
                            Ok(f) => f,
                            Err(err) => {
                                eprintln!("[audio] 打不开音频文件 {:?}: {err}", path);
                                continue;
                            }
                        };
                        let decoder = match Decoder::new(BufReader::new(file)) {
                            Ok(d) => d,
                            Err(err) => {
                                eprintln!("[audio] 解码失败 {:?}: {err}", path);
                                continue;
                            }
                        };
                        match Sink::try_new(&handle) {
                            Ok(sink) => {
                                sink.set_volume(if fade { 0.0 } else { volume });
                                sink.append(decoder);
                                sink.play();
                                if fade {
                                    fade_value = 0.0;
                                    fade_target = Some(volume);
                                }
                                current = Some(sink);
                            }
                            Err(err) => eprintln!("[audio] 创建输出失败: {err}"),
                        }
                    }
                    Ok(Cmd::Stop) => {
                        if let Some(sink) = current.take() {
                            sink.stop();
                        }
                        fade_target = None;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }

                // 渐强：在音频线程里按 30ms 步进抬音量，避免跨线程共享 Sink
                if let Some(target) = fade_target {
                    if let Some(sink) = current.as_ref() {
                        fade_value = (fade_value + 0.08).min(target);
                        sink.set_volume(fade_value);
                        if fade_value >= target {
                            fade_target = None;
                        }
                    }
                }
            }
        })
        .ok();
    AudioHandle { tx }
}

/* ------------------------- 内置铃声（首次运行生成） ------------------------- */

const CHIME_ID: &str = "builtin-chime";
const BELL_ID: &str = "builtin-bell";

pub fn ensure_builtins(dir: &Path, sounds: &mut Vec<SoundMeta>) {
    let base = sounds_dir(dir);
    let _ = std::fs::create_dir_all(&base);

    let chime_file = "builtin-chime.wav";
    let bell_file = "builtin-bell.wav";

    if !base.join(chime_file).exists() {
        let _ = write_wav(&base.join(chime_file), &synth_chime(), 44_100);
    }
    if !base.join(bell_file).exists() {
        let _ = write_wav(&base.join(bell_file), &synth_bell(), 44_100);
    }

    for (id, name, file) in [
        (CHIME_ID, "清脆铃声", chime_file),
        (BELL_ID, "柔和钟声", bell_file),
    ] {
        if !sounds.iter().any(|s| s.id == id) {
            sounds.push(SoundMeta {
                id: id.to_string(),
                name: name.to_string(),
                file: file.to_string(),
                builtin: true,
            });
        }
    }
}

fn synth_chime() -> Vec<f32> {
    let rate = 44_100.0f32;
    let len = (rate * 1.8) as usize;
    let mut out = vec![0.0f32; len];
    for i in 0..len {
        let t = i as f32 / rate;
        let env = (-3.6 * t).exp();
        let a = (2.0 * std::f32::consts::PI * 1046.5 * t).sin();
        let b = if t > 0.11 {
            (2.0 * std::f32::consts::PI * 1568.0 * (t - 0.11)).sin() * (-3.0 * (t - 0.11)).exp()
        } else {
            0.0
        };
        out[i] = (a * 0.45 + b * 0.35) * env;
    }
    out
}

fn synth_bell() -> Vec<f32> {
    let rate = 44_100.0f32;
    let len = (rate * 2.4) as usize;
    let mut out = vec![0.0f32; len];
    for i in 0..len {
        let t = i as f32 / rate;
        let env = (-2.0 * t).exp();
        let mut sample = 0.0;
        for (mult, amp) in [(1.0, 0.5), (2.01, 0.22), (3.02, 0.12), (4.17, 0.06)] {
            sample += (2.0 * std::f32::consts::PI * 523.25 * mult * t).sin() * amp;
        }
        out[i] = sample * env * 0.8;
    }
    out
}

fn write_wav(path: &Path, samples: &[f32], rate: u32) -> std::io::Result<()> {
    let mut pcm = Vec::with_capacity(samples.len() * 2);
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32_767.0) as i16;
        pcm.extend_from_slice(&v.to_le_bytes());
    }
    let data_len = pcm.len() as u32;
    let byte_rate = rate * 2;
    let mut out = Vec::with_capacity(44 + pcm.len());
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    out.extend_from_slice(&pcm);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = File::create(path)?;
    f.write_all(&out)?;
    Ok(())
}
