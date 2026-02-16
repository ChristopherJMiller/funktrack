mod beat;
mod chart;
mod decode;
mod difficulty;
mod note_types;
mod onset;
mod path;
mod quantize;
mod stft;

use std::path::{Path, PathBuf};

use clap::Parser;

use chart::{
    ChartFile, ChartTimingPoint, Difficulty, SongMetadata, serialize_chart, serialize_metadata,
};

#[derive(Parser)]
#[command(name = "chart_gen", about = "Auto-generate FunkTrack charts from audio files")]
struct Cli {
    /// Path to the audio file (MP3, OGG, FLAC, WAV)
    audio_file: PathBuf,

    /// Generate a single difficulty
    #[arg(short, long)]
    difficulty: Option<String>,

    /// Generate all four difficulties
    #[arg(long)]
    all_difficulties: bool,

    /// Output file path (for single difficulty)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Output directory (for --all-difficulties)
    #[arg(long)]
    output_dir: Option<PathBuf>,

    /// Override BPM detection with a fixed value
    #[arg(long)]
    bpm: Option<f64>,

    /// Onset detection sensitivity (default: 1.5, higher = fewer notes)
    #[arg(long, default_value = "1.5")]
    sensitivity: f64,

    /// Minimum inter-onset interval in ms (default: 50)
    #[arg(long, default_value = "50")]
    min_interval: f64,

    /// Generate metadata.ron alongside charts
    #[arg(long)]
    metadata: bool,

    /// Song title (for metadata generation)
    #[arg(long)]
    title: Option<String>,

    /// Song artist (for metadata generation)
    #[arg(long)]
    artist: Option<String>,

    /// Show detailed analysis output
    #[arg(short, long)]
    verbose: bool,
}

fn main() {
    let cli = Cli::parse();

    // Determine which difficulties to generate
    let difficulties = if cli.all_difficulties {
        vec![
            Difficulty::Easy,
            Difficulty::Normal,
            Difficulty::Hard,
            Difficulty::Expert,
        ]
    } else if let Some(ref d) = cli.difficulty {
        vec![parse_difficulty(d)]
    } else {
        vec![Difficulty::Normal]
    };

    // Step 1: Decode audio
    eprintln!("Decoding {}...", cli.audio_file.display());
    let audio = decode::decode_audio(&cli.audio_file).unwrap_or_else(|e| {
        eprintln!("Error: {e}");
        std::process::exit(1);
    });
    let duration_seconds = audio.samples.len() as f64 / audio.sample_rate as f64;
    eprintln!(
        "  {} samples, {} Hz, {:.1}s",
        audio.samples.len(),
        audio.sample_rate,
        duration_seconds
    );

    // Step 2: STFT
    eprintln!("Computing STFT...");
    let spectrogram = stft::compute_stft(&audio.samples, audio.sample_rate);
    eprintln!("  {} frames, {} bins", spectrogram.frames.len(), spectrogram.num_bins());

    // Step 3: Onset detection
    eprintln!("Detecting onsets (sensitivity={})...", cli.sensitivity);
    let onsets = onset::detect_onsets(&spectrogram, cli.sensitivity, cli.min_interval);
    eprintln!("  {} onsets detected", onsets.len());

    if cli.verbose && !onsets.is_empty() {
        let strengths: Vec<f32> = onsets.iter().map(|o| o.strength).collect();
        let avg_strength: f32 = strengths.iter().sum::<f32>() / strengths.len() as f32;
        let max_strength = strengths.iter().cloned().fold(0.0f32, f32::max);
        eprintln!("  Avg strength: {avg_strength:.3}, Max: {max_strength:.3}");
    }

    // Step 4: Beat tracking
    eprintln!("Tracking beats...");
    let beat_grid = beat::track_beats(&spectrogram, &onsets, cli.bpm);
    eprintln!("  BPM: {:.1}, {} beats", beat_grid.bpm, beat_grid.beats.len());

    // Step 5: Generate charts for each difficulty
    for diff in &difficulties {
        eprintln!("\nGenerating {:?} chart...", diff);

        // Quantize
        let grid_res = diff.grid_resolution();
        let quantized = quantize::quantize_onsets(&onsets, &beat_grid, grid_res);
        eprintln!("  {} quantized notes (grid: 1/{})", quantized.len(), grid_res);

        // Difficulty filter
        let filtered = difficulty::filter_by_difficulty(&quantized, *diff, beat_grid.bpm);
        let rating = difficulty::compute_difficulty_rating(&filtered, beat_grid.bpm);
        eprintln!("  {} notes after filtering (rating: {})", filtered.len(), rating);

        // Assign note types
        let notes = note_types::assign_note_types(&filtered, *diff, beat_grid.bpm);

        if cli.verbose {
            let mut type_counts = std::collections::HashMap::new();
            for n in &notes {
                let name = match &n.note_type {
                    chart::ChartNoteType::Tap => "Tap",
                    chart::ChartNoteType::Hold { .. } => "Hold",
                    chart::ChartNoteType::Slide { .. } => "Slide",
                    chart::ChartNoteType::Beat => "Beat",
                    chart::ChartNoteType::Scratch => "Scratch",
                    chart::ChartNoteType::Critical => "Critical",
                    chart::ChartNoteType::DualSlide { .. } => "DualSlide",
                    chart::ChartNoteType::AdLib => "AdLib",
                    _ => "Other",
                };
                *type_counts.entry(name).or_insert(0u32) += 1;
            }
            let mut counts: Vec<_> = type_counts.into_iter().collect();
            counts.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
            for (name, count) in &counts {
                eprintln!("    {name}: {count}");
            }
        }

        // Generate path
        let total_beats = if beat_grid.beats.is_empty() {
            duration_seconds * beat_grid.bpm / 60.0
        } else {
            beat_grid.total_beats() + 8.0 // Add 8 beats of buffer
        };
        let path_segment = path::generate_path(&spectrogram, total_beats, beat_grid.bpm);

        // Build chart file
        let chart_file = ChartFile {
            difficulty: *diff,
            difficulty_rating: rating,
            timing_points: vec![ChartTimingPoint {
                beat: 0.0,
                bpm: beat_grid.bpm,
                time_signature: (4, 4),
            }],
            path_segments: vec![path_segment],
            notes,
            events: Vec::new(),
            travel_beats: diff.travel_beats(),
            look_ahead_beats: diff.travel_beats(),
        };

        // Serialize and write
        let ron_str = serialize_chart(&chart_file).unwrap_or_else(|e| {
            eprintln!("Error serializing chart: {e}");
            std::process::exit(1);
        });

        let output_path = determine_output_path(&cli, diff);
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&output_path, &ron_str).unwrap_or_else(|e| {
            eprintln!("Error writing {}: {e}", output_path.display());
            std::process::exit(1);
        });

        eprintln!("  Wrote {}", output_path.display());
    }

    // Optional: generate metadata.ron
    if cli.metadata {
        let audio_filename = cli
            .audio_file
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let title = cli
            .title
            .clone()
            .unwrap_or_else(|| stem_name(&cli.audio_file));
        let artist = cli.artist.clone().unwrap_or_else(|| "Unknown".to_string());

        let metadata = SongMetadata {
            title,
            artist,
            charter: "chart_gen".to_string(),
            audio_file: audio_filename,
            preview_start_ms: 0,
            preview_duration_ms: 15000,
            source: String::new(),
            difficulties: difficulties.clone(),
        };

        let meta_ron = serialize_metadata(&metadata).unwrap_or_else(|e| {
            eprintln!("Error serializing metadata: {e}");
            std::process::exit(1);
        });

        let meta_path = if let Some(ref dir) = cli.output_dir {
            dir.join("metadata.ron")
        } else if let Some(ref out) = cli.output {
            out.parent().unwrap_or(Path::new(".")).join("metadata.ron")
        } else {
            PathBuf::from("metadata.ron")
        };

        std::fs::write(&meta_path, &meta_ron).unwrap_or_else(|e| {
            eprintln!("Error writing metadata: {e}");
            std::process::exit(1);
        });
        eprintln!("\nWrote {}", meta_path.display());
    }

    eprintln!("\nDone!");
}

fn parse_difficulty(s: &str) -> Difficulty {
    match s.to_lowercase().as_str() {
        "easy" => Difficulty::Easy,
        "normal" => Difficulty::Normal,
        "hard" => Difficulty::Hard,
        "expert" => Difficulty::Expert,
        other => {
            eprintln!("Unknown difficulty: {other}. Use easy, normal, hard, or expert.");
            std::process::exit(1);
        }
    }
}

fn determine_output_path(cli: &Cli, difficulty: &Difficulty) -> PathBuf {
    if let Some(ref dir) = cli.output_dir {
        dir.join(difficulty.filename())
    } else if let Some(ref out) = cli.output {
        if cli.all_difficulties {
            // If generating all difficulties with a single --output, use it as a directory
            let dir = out;
            dir.join(difficulty.filename())
        } else {
            out.clone()
        }
    } else {
        PathBuf::from(difficulty.filename())
    }
}

fn stem_name(path: &Path) -> String {
    path.file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}
