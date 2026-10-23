use super::stats::{Samples, fps};
use super::system::MachineInfo;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

pub struct Report<'a> {
    pub scene: &'a str,
    pub preset: &'a str,
    pub machine: &'a MachineInfo,
    pub adapter: &'a wgpu::AdapterInfo,
    pub resolution: (u32, u32),
    pub present_mode: &'a str,
    pub duration_seconds: f32,
    pub frames: &'a Samples,
    pub cpu: &'a Samples,
    pub gpu: Option<&'a Samples>,
    pub peak_memory_bytes: u64,
    pub server_ticks_per_second: f32,
}

const MIB: f64 = 1024.0 * 1024.0;

impl Report<'_> {
    /// `<dir>/<date>-<machine>-<preset>-<scene>.md`, with `-2`, `-3`… if it already exists.
    pub fn free_path(&self, dir: &Path, date: &str) -> PathBuf {
        let stem = format!(
            "{date}-{}-{}-{}",
            slug(&self.machine.host),
            self.preset,
            slug(self.scene)
        );
        let first = dir.join(format!("{stem}.md"));
        std::iter::once(first.clone())
            .chain((2..1000).map(|n| dir.join(format!("{stem}-{n}.md"))))
            .find(|path| !path.exists())
            .unwrap_or(first)
    }

    pub fn to_markdown(&self, date: &str) -> String {
        let mut text = String::new();
        let _ = writeln!(
            text,
            "# Rapport de performance — `{}` — profil `{}`\n",
            self.scene, self.preset
        );
        let _ = writeln!(text, "| Contexte | Valeur |\n|---|---|");
        let rows = [
            ("Date", date.to_owned()),
            ("Version", env!("CARGO_PKG_VERSION").to_owned()),
            ("Machine", self.machine.host.clone()),
            ("Système", self.machine.os.clone()),
            (
                "Processeur",
                format!("{} ({} threads)", self.machine.cpu, self.machine.cores),
            ),
            (
                "Mémoire totale",
                format!("{:.0} Mio", self.machine.total_memory_bytes as f64 / MIB),
            ),
            (
                "Carte graphique",
                format!(
                    "{} ({}, {:?})",
                    self.adapter.name, self.adapter.driver, self.adapter.device_type
                ),
            ),
            (
                "Résolution",
                format!("{}×{}", self.resolution.0, self.resolution.1),
            ),
            ("Présentation", self.present_mode.to_owned()),
            (
                "Durée mesurée",
                format!(
                    "{:.1} s, {} images",
                    self.duration_seconds,
                    self.frames.len()
                ),
            ),
        ];
        for (name, value) in rows {
            let _ = writeln!(text, "| {name} | {value} |");
        }
        let _ = writeln!(text, "\n| Mesure | Valeur |\n|---|---|");
        let ms = |value: Option<f32>| value.map_or("n/d".into(), |v| format!("{v:.2} ms"));
        let rate = |value: Option<f32>| value.map_or("n/d".into(), |v| format!("{:.1}", fps(v)));
        let gpu_mean = self.gpu.and_then(Samples::mean);
        let gpu_worst = self.gpu.and_then(|g| g.worst_mean(0.01));
        let measures = [
            ("FPS moyen", rate(self.frames.mean())),
            ("1 % low (FPS)", rate(self.frames.worst_mean(0.01))),
            ("0,1 % low (FPS)", rate(self.frames.worst_mean(0.001))),
            ("Temps de frame moyen", ms(self.frames.mean())),
            (
                "Temps CPU moyen / 1 % pire",
                format!(
                    "{} / {}",
                    ms(self.cpu.mean()),
                    ms(self.cpu.worst_mean(0.01))
                ),
            ),
            (
                "Temps GPU moyen / 1 % pire",
                format!("{} / {}", ms(gpu_mean), ms(gpu_worst)),
            ),
            (
                "Mémoire (pic, RSS)",
                format!("{:.0} Mio", self.peak_memory_bytes as f64 / MIB),
            ),
            (
                "Ticks serveur par seconde",
                format!("{:.1}", self.server_ticks_per_second),
            ),
        ];
        for (name, value) in measures {
            let _ = writeln!(text, "| {name} | {value} |");
        }
        let _ = writeln!(
            text,
            "\n> v0.1 : les profils règlent pour l'instant les distances de rendu, l'occlusion \
             ambiante et les threads ; les autres réglages arrivent au jalon 8."
        );
        text
    }
}

fn slug(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}
