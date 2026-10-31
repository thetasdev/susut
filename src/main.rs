//! susut — CLI to compress, resize and convert images.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result, bail};
use clap::Parser;
use rayon::prelude::*;

use susut::{
    Amount, Destination, Format, Lang, Msg, Options, Outcome, Status, human, is_image,
    is_min_output, output_path, parse_amount, parse_percent, pct, process,
};

#[derive(Parser, Debug)]
#[command(
    name = "susut",
    version,
    about = "Compress, resize and convert images (JPEG/PNG/WebP/AVIF)",
    after_help = "Examples:\n  \
        susut photo.jpg                         # -> photo.min.jpg, quality 80\n  \
        susut photo.png -f jpeg -s 200KB        # at most 200 KB\n  \
        susut photo.jpg -s 50%                  # at most half the original size\n  \
        susut -r Photos/ -f webp -L 1600 -o out/   # a whole folder, structure preserved\n  \
        susut *.jpg --resize 50% -n             # dry run: estimate without writing"
)]
struct Cli {
    /// Input files or directories
    #[arg(required = true)]
    inputs: Vec<PathBuf>,

    /// Output file (for a single input) or output directory
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Output format (default: same as input)
    #[arg(short, long, value_enum)]
    format: Option<Format>,

    /// Quality 1-100 (ignored for lossless PNG)
    #[arg(short, long, default_value_t = 80, value_parser = clap::value_parser!(u8).range(1..=100))]
    quality: u8,

    /// Maximum output size: 200KB, 1.5MB, 50000, or 50% of the original size
    #[arg(short = 's', long, value_parser = parse_amount)]
    max_size: Option<Amount>,

    /// Lowest quality to try for --max-size before the image is downscaled
    #[arg(long, default_value_t = 40, value_parser = clap::value_parser!(u8).range(1..=100))]
    min_quality: u8,

    /// Scale the image, e.g. 50%
    #[arg(long, value_parser = parse_percent)]
    resize: Option<f64>,

    /// Maximum width (px)
    #[arg(short = 'W', long)]
    max_width: Option<u32>,

    /// Maximum height (px)
    #[arg(short = 'H', long)]
    max_height: Option<u32>,

    /// Maximum length of the longer side (px); handy for mixed portrait/landscape photos
    #[arg(short = 'L', long)]
    long_edge: Option<u32>,

    /// Maximum length of the shorter side (px)
    #[arg(long)]
    short_edge: Option<u32>,

    /// Process directories recursively (structure is kept under -o)
    #[arg(short, long)]
    recursive: bool,

    /// Only write if at least this much is saved: 10%, 50KB (default: any saving)
    #[arg(long, value_parser = parse_amount)]
    min_savings: Option<Amount>,

    /// Write the result even if it is not smaller than the original
    #[arg(long)]
    force: bool,

    /// Dry run: show results without writing files
    #[arg(short = 'n', long)]
    dry_run: bool,

    /// Overwrite existing output files
    #[arg(short = 'y', long)]
    overwrite: bool,

    /// Language of runtime messages
    #[arg(long, value_enum, env = "SUSUT_LANG", default_value = "en")]
    lang: Lang,
}

impl Cli {
    fn options(&self) -> Options {
        Options {
            format: self.format,
            quality: self.quality,
            min_quality: self.min_quality,
            max_size: self.max_size,
            resize: self.resize,
            max_width: self.max_width,
            max_height: self.max_height,
            long_edge: self.long_edge,
            short_edge: self.short_edge,
            min_savings: self.min_savings,
            force: self.force,
            dry_run: self.dry_run,
            overwrite: self.overwrite,
            lang: self.lang,
        }
    }
}

struct Job {
    input: PathBuf,
    dest: Destination,
}

fn main() {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(true) => {}
        Ok(false) => std::process::exit(1),
        Err(e) => {
            eprintln!("error: {e:#}");
            std::process::exit(1);
        }
    }
}

fn run(cli: &Cli) -> Result<bool> {
    let opts = cli.options();
    let lang = opts.lang;
    opts.validate()?;
    let out_dir = resolve_out_dir(cli)?;
    let jobs = collect_jobs(cli, out_dir.as_deref())?;
    if jobs.is_empty() {
        bail!("{}", Msg::NoImages.render(lang));
    }
    if cli.output.is_some() && out_dir.is_none() && jobs.len() > 1 {
        bail!("{}", Msg::OutputNotDir.render(lang));
    }

    let start = Instant::now();
    let results: Vec<(&Path, Result<Outcome>)> = jobs
        .par_iter()
        .map(|job| (job.input.as_path(), process(&opts, &job.input, &job.dest)))
        .collect();

    let (mut written, mut skipped, mut failed) = (0usize, 0usize, 0usize);
    let (mut total_before, mut total_after) = (0u64, 0u64);
    let verb = if cli.dry_run {
        format!("{} ", Msg::DryRun.render(lang))
    } else {
        String::new()
    };

    for (input, res) in &results {
        match res {
            Ok(o) => {
                total_before += o.before;
                let q = o.quality.map(|q| format!(" q={q}")).unwrap_or_default();
                let stats = format!(
                    "{} → {} ({}) {}×{}{q}",
                    human(o.before),
                    human(o.after),
                    pct(o.before, o.after),
                    o.dims.0,
                    o.dims.1
                );
                let warn = if o.hit_target {
                    String::new()
                } else {
                    format!("  {}", Msg::TargetMissed.render(lang))
                };
                match &o.status {
                    Status::Written => {
                        written += 1;
                        total_after += o.after;
                        let grew = if o.after >= o.before {
                            format!("  {}", Msg::LargerThanOriginal.render(lang))
                        } else {
                            String::new()
                        };
                        println!(
                            "✓ {verb}{} → {}  {stats}{warn}{grew}",
                            input.display(),
                            o.output.display()
                        );
                    }
                    Status::Skipped(why) => {
                        skipped += 1;
                        total_after += o.before;
                        println!(
                            "– {verb}{} {}: {why}  [{stats}]",
                            input.display(),
                            Msg::Skipped.render(lang)
                        );
                    }
                }
            }
            Err(e) => {
                failed += 1;
                eprintln!("✗ {}: {e:#}", input.display());
            }
        }
    }

    if results.len() > 1 {
        let mut parts = vec![Msg::Saved(written).render(lang)];
        if skipped > 0 {
            parts.push(Msg::SkippedCount(skipped).render(lang));
        }
        if failed > 0 {
            parts.push(Msg::Failed(failed).render(lang));
        }
        println!(
            "\n{verb}{} · {} → {} ({}) · {:.1}s",
            parts.join(", "),
            human(total_before),
            human(total_after),
            pct(total_before, total_after),
            start.elapsed().as_secs_f32()
        );
    }
    Ok(failed == 0)
}

/// Some(dir) if the output is a directory, None for a single file / default next to the input.
fn resolve_out_dir(cli: &Cli) -> Result<Option<PathBuf>> {
    let Some(out) = &cli.output else {
        return Ok(None);
    };
    let has_image_ext = Format::from_ext(out).is_some();
    let looks_dir = out.is_dir()
        || out.to_string_lossy().ends_with(['/', '\\'])
        || (!has_image_ext && (cli.inputs.len() > 1 || cli.inputs.iter().any(|p| p.is_dir())));
    if !looks_dir {
        return Ok(None);
    }
    if !cli.dry_run {
        fs::create_dir_all(out)
            .with_context(|| Msg::CreateDirFailed(out.display().to_string()).render(cli.lang))?;
    }
    Ok(Some(out.clone()))
}

fn collect_jobs(cli: &Cli, out_dir: Option<&Path>) -> Result<Vec<Job>> {
    let out_abs = out_dir.and_then(|d| d.canonicalize().ok());
    let mut jobs = Vec::new();

    let single_dest = || match (out_dir, &cli.output) {
        (Some(dir), _) => Destination::Dir {
            dir: dir.to_path_buf(),
            sub: PathBuf::new(),
        },
        (None, Some(file)) => Destination::File(file.clone()),
        (None, None) => Destination::Beside,
    };

    for input in &cli.inputs {
        if !input.is_dir() {
            jobs.push(Job {
                input: input.clone(),
                dest: single_dest(),
            });
            continue;
        }
        let depth = if cli.recursive { usize::MAX } else { 1 };
        let walker = walkdir::WalkDir::new(input)
            .max_depth(depth)
            .sort_by_file_name();
        for entry in walker.into_iter().filter_entry(|e| {
            // Do not descend into the output directory (e.g. `susut -r . -o out/`).
            out_abs
                .as_ref()
                .is_none_or(|o| e.path().canonicalize().ok().as_ref() != Some(o))
        }) {
            let entry = entry?;
            let p = entry.path();
            if !entry.file_type().is_file() || !is_image(p) || is_min_output(p) {
                continue;
            }
            let dest = match (out_dir, &cli.output) {
                (Some(dir), _) => Destination::Dir {
                    dir: dir.to_path_buf(),
                    sub: p
                        .strip_prefix(input)
                        .ok()
                        .and_then(Path::parent)
                        .map(Path::to_path_buf)
                        .unwrap_or_default(),
                },
                _ => single_dest(),
            };
            jobs.push(Job {
                input: p.to_path_buf(),
                dest,
            });
        }
    }

    if cli.inputs.iter().any(|p| p.is_dir()) && !cli.recursive {
        let nested = cli.inputs.iter().filter(|p| p.is_dir()).any(|d| {
            fs::read_dir(d)
                .map(|rd| rd.flatten().any(|e| e.path().is_dir()))
                .unwrap_or(false)
        });
        if nested {
            eprintln!("{}", Msg::SubdirsIgnored.render(cli.lang));
        }
    }

    // Stop two inputs from writing to the same output (e.g. photo.jpg & photo.png -f jpeg).
    let mut seen: HashMap<PathBuf, PathBuf> = HashMap::new();
    for job in &jobs {
        let fmt = cli
            .format
            .or_else(|| Format::from_ext(&job.input))
            .unwrap_or(Format::Jpeg);
        let out = output_path(&job.input, &job.dest, fmt);
        if let Some(prev) = seen.insert(out.clone(), job.input.clone()) {
            bail!(
                "{}",
                Msg::Collision {
                    a: prev.display().to_string(),
                    b: job.input.display().to_string(),
                    out: out.display().to_string(),
                }
                .render(cli.lang)
            );
        }
    }
    Ok(jobs)
}
