use bitvec::prelude::*;
use rayon::prelude::*;
use reed_solomon_erasure::galois_8::ReedSolomon;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;
use std::time::Instant;

use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, KeyInit},
};
use rand::RngCore;
use sha2::{Digest, Sha256};

pub const WIDTH: u32 = 1920;
pub const HEIGHT: u32 = 1080;
pub const BLOCK_SIZE: u32 = 8;
pub const MARGIN: u32 = 80;

pub const USABLE_WIDTH: u32 = WIDTH - 2 * MARGIN;
pub const USABLE_HEIGHT: u32 = HEIGHT - 2 * MARGIN;
pub const BLOCKS_X: u32 = USABLE_WIDTH / BLOCK_SIZE;
pub const BLOCKS_Y: u32 = USABLE_HEIGHT / BLOCK_SIZE;
pub const BITS_PER_FRAME: usize = (BLOCKS_X * BLOCKS_Y) as usize;

const DATA_SHARDS: usize = 8;
const PARITY_SHARDS: usize = 4;
const TOTAL_SHARDS: usize = DATA_SHARDS + PARITY_SHARDS;

const MAX_FILE_SIZE: usize = 1000 * 1024 * 1024;

pub enum JobMsg {
    Log(String),
    Progress(f32),
    Done(String),
    Error(String),
}

pub fn get_tool_path(tool_name: &str) -> Result<String, String> {
    if let Ok(mut path) = std::env::current_exe() {
        path.pop();

        #[cfg(target_os = "windows")]
        path.push(format!("{}.exe", tool_name));
        #[cfg(not(target_os = "windows"))]
        path.push(tool_name);

        if path.exists() {
            return Ok(path.to_string_lossy().to_string());
        }
    }
    Ok(tool_name.to_string())
}

fn estimate_frame_count(video: &str, ffmpeg_path: &str) -> Option<usize> {
    let output = Command::new(ffmpeg_path)
        .args(["-i", video])
        .stderr(Stdio::piped())
        .stdout(Stdio::null())
        .stdin(Stdio::null())
        .output()
        .ok()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    for line in stderr.lines() {
        if let Some(pos) = line.find("Duration: ") {
            let dur_str = &line[pos + 10..];
            let parts: Vec<&str> = dur_str.split(':').collect();
            if parts.len() >= 3 {
                let h: f64 = parts[0].trim().parse().ok()?;
                let m: f64 = parts[1].trim().parse().ok()?;
                let s: f64 = parts[2].split(',').next()?.trim().parse().ok()?;
                let total_secs = h * 3600.0 + m * 60.0 + s;
                return Some(((total_secs * 6.0) as usize).max(1));
            }
        }
    }
    None
}

pub fn aes_encrypt(data: &[u8], password: &str) -> Result<Vec<u8>, String> {
    if password.is_empty() {
        return Ok(data.to_vec());
    }
    let hash = Sha256::digest(password.as_bytes());
    let key = Key::<Aes256Gcm>::from_slice(&hash);
    let cipher = Aes256Gcm::new(key);
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, data)
        .map_err(|_| "AES encryption failed".to_string())?;
    let mut result = nonce.to_vec();
    result.extend(ciphertext);
    Ok(result)
}

pub fn aes_decrypt(data: &[u8], password: &str) -> Result<Vec<u8>, String> {
    if password.is_empty() {
        return Ok(data.to_vec());
    }
    if data.len() < 12 {
        return Err("Data too short for AES decryption".to_string());
    }
    let hash = Sha256::digest(password.as_bytes());
    let key = Key::<Aes256Gcm>::from_slice(&hash);
    let cipher = Aes256Gcm::new(key);
    cipher
        .decrypt(Nonce::from_slice(&data[0..12]), &data[12..])
        .map_err(|_| "ACCESS DENIED: Wrong password or data corrupted!".to_string())
}

fn simple_hash(data: &[u8]) -> u32 {
    let mut hash: u32 = 2166136261;
    for &b in data {
        hash ^= b as u32;
        hash = hash.wrapping_mul(16777619);
    }
    hash
}

fn apply_fec(data: Vec<u8>) -> Result<Vec<u8>, String> {
    let rs = ReedSolomon::new(DATA_SHARDS, PARITY_SHARDS).map_err(|e| e.to_string())?;
    let mut padded = (data.len() as u64).to_be_bytes().to_vec();
    padded.extend(data);
    let chunk_size = (padded.len() + DATA_SHARDS - 1) / DATA_SHARDS;
    padded.resize(chunk_size * DATA_SHARDS, 0);
    let mut shards: Vec<Vec<u8>> = padded
        .chunks_exact(chunk_size)
        .map(|c| c.to_vec())
        .collect();
    for _ in 0..PARITY_SHARDS {
        shards.push(vec![0; chunk_size]);
    }
    rs.encode(&mut shards).map_err(|e| e.to_string())?;
    let mut flat = Vec::new();
    for shard in shards {
        flat.extend_from_slice(&simple_hash(&shard).to_be_bytes());
        flat.extend(shard);
    }
    Ok(flat)
}

fn remove_fec(data: &[u8]) -> Result<Vec<u8>, String> {
    let rs = ReedSolomon::new(DATA_SHARDS, PARITY_SHARDS).map_err(|e| e.to_string())?;
    let shard_size = data.len() / TOTAL_SHARDS;
    if shard_size < 4 {
        return Err("Data too small for FEC".to_string());
    }
    let mut shards: Vec<Option<Vec<u8>>> = Vec::new();
    for i in 0..TOTAL_SHARDS {
        let sd = &data[i * shard_size..i * shard_size + shard_size];
        if simple_hash(&sd[4..]) == u32::from_be_bytes(sd[0..4].try_into().unwrap()) {
            shards.push(Some(sd[4..].to_vec()));
        } else {
            shards.push(None);
        }
    }
    rs.reconstruct(&mut shards)
        .map_err(|_| "FEC Reconstruction failed!".to_string())?;
    let mut restored = Vec::new();
    for i in 0..DATA_SHARDS {
        restored.extend(shards[i].as_ref().unwrap());
    }
    let orig_len = u64::from_be_bytes(restored[0..8].try_into().unwrap()) as usize;
    if orig_len > restored.len() - 8 {
        return Err("Invalid FEC extraction size".to_string());
    }
    Ok(restored[8..8 + orig_len].to_vec())
}

pub fn pack_folder(folder_path: &str) -> Result<Vec<u8>, String> {
    let mut tar_builder = tar::Builder::new(Vec::new());
    tar_builder
        .append_dir_all(".", folder_path)
        .map_err(|e| format!("Failed to pack folder: {}", e))?;
    tar_builder.into_inner().map_err(|e| e.to_string())
}

pub fn unpack_tar(data: &[u8], out_dir: &str) -> Result<(), String> {
    let mut archive = tar::Archive::new(data);
    archive
        .unpack(out_dir)
        .map_err(|e| format!("Failed to unpack folder: {}", e))?;
    Ok(())
}

fn generate_audio_track(
    bits: &BitSlice<u8, Msb0>,
    frames: usize,
    data_bpf: usize,
    path: &str,
) -> Result<(), String> {
    let mut writer = hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .map_err(|e| e.to_string())?;
    let (spt, mut phase) = (44100 / 6 / 50, 0.0f32);
    for i in 0..(frames + 5) {
        let f_bits = if i * data_bpf < bits.len() {
            &bits[i * data_bpf..usize::min(i * data_bpf + data_bpf, bits.len())]
        } else {
            &bits[0..0]
        };
        for t in 0..50 {
            let offset = if f_bits.is_empty() {
                0
            } else {
                (t * f_bits.len()) / 50
            };
            let mut val = 0u8;
            if !f_bits.is_empty() && offset + 8 <= f_bits.len() {
                for b in 0..8 {
                    if f_bits[offset + b] {
                        val |= 1 << (7 - b);
                    }
                }
            }
            let phase_inc = (600.0 + (val as f32 * 10.0)) * 2.0 * std::f32::consts::PI / 44100.0;
            for _ in 0..spt {
                writer
                    .write_sample((phase.sin() * 8000.0) as i16)
                    .map_err(|e| e.to_string())?;
                phase = (phase + phase_inc) % (2.0 * std::f32::consts::PI);
            }
        }
    }
    writer.finalize().map_err(|e| e.to_string())
}

pub fn encode_raw_frame(data_bits: &BitSlice<u8, Msb0>, frame_idx: u32, use_sync: bool) -> Vec<u8> {
    let mut frame = vec![0u8; (WIDTH * HEIGHT) as usize];
    let mut draw_rect = |xs: u32, ys: u32, sz: u32, val: u8| {
        for y in ys..(ys + sz) {
            for x in xs..(xs + sz) {
                if x < WIDTH && y < HEIGHT {
                    frame[(y * WIDTH + x) as usize] = val;
                }
            }
        }
    };
    draw_rect(0, 0, MARGIN, 255);
    draw_rect(WIDTH - MARGIN, 0, MARGIN, 255);
    draw_rect(0, HEIGHT - MARGIN, MARGIN, 255);
    draw_rect(WIDTH - MARGIN, HEIGHT - MARGIN, MARGIN, 255);
    draw_rect(20, 20, 40, 0);
    draw_rect(WIDTH - MARGIN + 20, 20, 40, 0);
    draw_rect(20, HEIGHT - MARGIN + 20, 40, 0);
    draw_rect(WIDTH - MARGIN + 20, HEIGHT - MARGIN + 20, 40, 0);

    let mut all_bits = BitVec::<u8, Msb0>::new();
    if use_sync {
        all_bits.extend_from_bitslice(frame_idx.to_be_bytes().view_bits::<Msb0>());
    }
    all_bits.extend_from_bitslice(data_bits);

    for (i, bit) in all_bits.iter().enumerate() {
        if i >= BITS_PER_FRAME {
            break;
        }
        let (bx, by) = ((i as u32) % BLOCKS_X, (i as u32) / BLOCKS_X);
        let val = if *bit { 255u8 } else { 0u8 };
        let (start_x, start_y) = (MARGIN + bx * BLOCK_SIZE, MARGIN + by * BLOCK_SIZE);
        for dy in 0..BLOCK_SIZE {
            for dx in 0..BLOCK_SIZE {
                frame[((start_y + dy) * WIDTH + start_x + dx) as usize] = val;
            }
        }
    }
    frame
}

pub fn encode_process(
    input_path: String,
    is_folder: bool,
    out_path: String,
    filename: String,
    key: String,
    use_fec: bool,
    use_mc: bool,
    use_sync: bool,
    use_zstd: bool,
    use_audio: bool,
    hw_accel: u8,
    tx: Sender<JobMsg>,
) -> Result<(), String> {
    let start_time = Instant::now();

    if key.is_empty() {
        let _ = tx.send(JobMsg::Log(
            "WARNING: Access key is empty — payload will NOT be encrypted.".into(),
        ));
    }

    let raw_data = if is_folder {
        let _ = tx.send(JobMsg::Log("Packing folder into TAR archive...".into()));
        pack_folder(&input_path)?
    } else {
        let _ = tx.send(JobMsg::Log("Reading file into memory...".into()));
        std::fs::read(&input_path).map_err(|e| format!("Read error: {}", e))?
    };

    if raw_data.len() > MAX_FILE_SIZE {
        return Err(format!(
            "Data too large! Max allowed is 1000 MB. Size: {:.1} MB.",
            raw_data.len() as f32 / 1024.0 / 1024.0
        ));
    }

    let compressed;
    let process_data = if use_zstd {
        let _ = tx.send(JobMsg::Log("Compressing with Zstandard...".into()));
        compressed = zstd::encode_all(&raw_data[..], 3).map_err(|e| e.to_string())?;
        &compressed
    } else {
        &raw_data
    };

    let mut payload = format!(
        "FILE:{}:SIZE:{}:ZSTD:{}|",
        filename,
        process_data.len(),
        if use_zstd { 1 } else { 0 }
    )
    .into_bytes();

    let _ = tx.send(JobMsg::Log("Encrypting payload (AES-256-GCM)...".into()));
    payload.extend_from_slice(&aes_encrypt(process_data, &key)?);
    payload.extend_from_slice("█".repeat(64).as_bytes());

    if use_fec {
        let _ = tx.send(JobMsg::Log("Applying Reed-Solomon FEC...".into()));
        let fec_data = apply_fec(payload)?;
        let len_b = (fec_data.len() as u64).to_be_bytes();
        payload = Vec::new();
        payload.extend_from_slice(&len_b);
        payload.extend_from_slice(&len_b);
        payload.extend_from_slice(&len_b);
        payload.extend(fec_data);
    }

    let bits = payload.view_bits::<Msb0>();
    let data_bpf = if use_sync {
        BITS_PER_FRAME - 32
    } else {
        BITS_PER_FRAME
    };
    let frames = (bits.len() + data_bpf - 1) / data_bpf;
    let total_pipe_frames = frames + 5;

    let _ = tx.send(JobMsg::Log("Initializing FFmpeg Pipeline...".into()));
    let mut cmd = Command::new(get_tool_path("ffmpeg")?);
    cmd.args([
        "-y",
        "-f",
        "rawvideo",
        "-vcodec",
        "rawvideo",
        "-s",
        "1920x1080",
        "-pix_fmt",
        "gray",
        "-r",
        "6",
        "-i",
        "-",
    ]);

    let audio_dir = std::env::temp_dir().join("eidolon_audio");
    let audio_path = audio_dir.join("audio.wav");
    if use_audio {
        fs::create_dir_all(&audio_dir).unwrap();
        let _ = tx.send(JobMsg::Log(
            "Generating FSK acoustic anti-spam track...".into(),
        ));
        generate_audio_track(&bits, frames, data_bpf, audio_path.to_str().unwrap())?;
        cmd.args([
            "-i",
            audio_path.to_str().unwrap(),
            "-c:a",
            "aac",
            "-b:a",
            "128k",
            "-shortest",
        ]);
    } else {
        cmd.arg("-an");
    }

    match hw_accel {
        1 => {
            cmd.args(["-c:v", "h264_nvenc", "-preset", "p4", "-cq", "18"]);
        }
        2 => {
            cmd.args(["-c:v", "h264_amf", "-quality", "quality"]);
        }
        _ => {
            cmd.args(["-c:v", "libx264", "-preset", "slow", "-crf", "18"]);
        }
    }
    cmd.args(["-pix_fmt", "yuv420p", "-movflags", "+faststart", &out_path]);

    cmd.stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("FFmpeg failed to start: {}", e))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "Failed to open FFmpeg stdin".to_string())?;

    if use_mc {
        let thread_count = rayon::current_num_threads();
        let _ = tx.send(JobMsg::Log(format!(
            "Streaming {} frames to FFmpeg ({} CPU threads)...",
            frames, thread_count
        )));

        let batch_size = (thread_count * 4).max(8);
        for batch_start in (0..total_pipe_frames).step_by(batch_size) {
            let batch_end = usize::min(batch_start + batch_size, total_pipe_frames);

            let batch: Vec<Vec<u8>> = (batch_start..batch_end)
                .into_par_iter()
                .map(|i| {
                    let start = i * data_bpf;
                    let end = usize::min(start + data_bpf, bits.len());
                    let frame_bits = if i < frames { &bits[start..end] } else { &bits[0..0] };
                    encode_raw_frame(frame_bits, i as u32, use_sync)
                })
                .collect();

            for (j, frame) in batch.iter().enumerate() {
                stdin
                    .write_all(frame)
                    .map_err(|e| format!("Pipe broken: {}", e))?;
                let i = batch_start + j;
                let _ = tx.send(JobMsg::Progress(i as f32 / total_pipe_frames as f32 * 0.95));
            }
        }
    } else {
        let _ = tx.send(JobMsg::Log(format!(
            "Streaming {} frames to FFmpeg (single thread)...",
            frames
        )));
        for i in 0..total_pipe_frames {
            let start = i * data_bpf;
            let end = usize::min(start + data_bpf, bits.len());
            let frame_bits = if i < frames { &bits[start..end] } else { &bits[0..0] };
            stdin
                .write_all(&encode_raw_frame(frame_bits, i as u32, use_sync))
                .map_err(|e| format!("Pipe broken: {}", e))?;
            let _ = tx.send(JobMsg::Progress(i as f32 / total_pipe_frames as f32 * 0.95));
        }
    }

    drop(stdin);

    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("FFmpeg encoding error. Try switching to CPU.".to_string());
    }
    if use_audio {
        let _ = fs::remove_dir_all(audio_dir);
    }

    let elapsed = start_time.elapsed();
    if let Ok(meta) = std::fs::metadata(&out_path) {
        let size_mb = meta.len() as f64 / 1024.0 / 1024.0;
        let _ = tx.send(JobMsg::Log(format!(
            "Output: {:.1} MB | Time: {:.1}s",
            size_mb,
            elapsed.as_secs_f64()
        )));
    }

    let _ = tx.send(JobMsg::Progress(1.0));
    Ok(())
}

pub fn decode_process(
    video: String,
    out_dir: String,
    key: String,
    use_fec: bool,
    _use_mc: bool,
    use_sync: bool,
    hw_accel: u8,
    tx: Sender<JobMsg>,
) -> Result<(), String> {
    let start_time = Instant::now();

    if key.is_empty() {
        let _ = tx.send(JobMsg::Log(
            "WARNING: Access key is empty — assuming payload was NOT encrypted.".into(),
        ));
    }

    let _ = tx.send(JobMsg::Log(
        "Initializing FFmpeg Pipeline for Extraction...".into(),
    ));

    let ffmpeg_path = get_tool_path("ffmpeg")?;

    let estimated_frames = estimate_frame_count(&video, &ffmpeg_path).unwrap_or(0);
    if estimated_frames > 0 {
        let _ = tx.send(JobMsg::Log(format!(
            "Video probed: ~{} frames to extract.",
            estimated_frames
        )));
    }

    let mut cmd = Command::new(&ffmpeg_path);
    if hw_accel > 0 {
        cmd.args(["-hwaccel", "auto"]);
    }

    cmd.args([
        "-i",
        &video,
        "-f",
        "rawvideo",
        "-pix_fmt",
        "gray",
        "-vf",
        "scale=1920:1080:flags=neighbor",
        "-",
    ]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::null());

    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to open stdout".to_string())?;

    let data_bpf = if use_sync {
        BITS_PER_FRAME - 32
    } else {
        BITS_PER_FRAME
    };
    let mut frames_data = Vec::new();
    let mut frame_buf = vec![0u8; (WIDTH * HEIGHT) as usize];
    let mut frames_cnt = 0usize;

    let _ = tx.send(JobMsg::Log("Streaming frames into memory...".into()));
    while stdout.read_exact(&mut frame_buf).is_ok() {
        let mut local = Vec::with_capacity(BITS_PER_FRAME);
        for by in 0..BLOCKS_Y {
            let sy = MARGIN + by * BLOCK_SIZE + BLOCK_SIZE / 2;
            for bx in 0..BLOCKS_X {
                local.push(
                    frame_buf[((sy * WIDTH) + MARGIN + bx * BLOCK_SIZE + BLOCK_SIZE / 2) as usize]
                        > 127,
                );
            }
        }
        if use_sync && local.len() >= 32 {
            let mut idx = 0u32;
            for (bi, &b) in local[0..32].iter().enumerate() {
                if b {
                    idx |= 1 << (31 - bi);
                }
            }
            frames_data.push((idx as usize, local[32..].to_vec()));
        } else {
            frames_data.push((frames_cnt, local));
        }
        frames_cnt += 1;

        let progress = if estimated_frames > 0 {
            (frames_cnt as f32 / estimated_frames as f32 * 0.5).min(0.49)
        } else {
            0.49 * (1.0 - 1.0 / (frames_cnt as f32 * 0.005 + 1.0))
        };
        let _ = tx.send(JobMsg::Progress(progress));
    }
    let _ = child.wait();

    let safe_max = usize::min(
        frames_data.iter().map(|(idx, _)| *idx).max().unwrap_or(0),
        frames_cnt * 2,
    );
    let mut final_frames = vec![vec![false; data_bpf]; safe_max + 1];
    for (idx, bits) in frames_data {
        if idx <= safe_max && !bits.is_empty() {
            final_frames[idx] = bits;
        }
    }

    let mut all_bits = BitVec::<u8, Msb0>::new();
    for fb in final_frames {
        for bit in fb {
            all_bits.push(bit);
        }
    }
    let mut raw = all_bits.into_vec();
    let _ = tx.send(JobMsg::Progress(0.6));

    if use_fec {
        let _ = tx.send(JobMsg::Log("Removing Reed-Solomon FEC...".into()));
        if raw.len() < 24 {
            return Err("Video too short for FEC".to_string());
        }
        let (l1, l2, l3) = (
            u64::from_be_bytes(raw[0..8].try_into().unwrap()),
            u64::from_be_bytes(raw[8..16].try_into().unwrap()),
            u64::from_be_bytes(raw[16..24].try_into().unwrap()),
        );
        let len = if l1 == l2 || l1 == l3 { l1 } else { l2 } as usize;
        if raw.len() < 24 + len {
            return Err("Video missing data!".to_string());
        }
        raw = remove_fec(&raw[24..24 + len])?;
    }
    let _ = tx.send(JobMsg::Progress(0.8));

    let eof = "█".repeat(64).into_bytes();
    let pos = raw
        .windows(eof.len())
        .position(|w| w == eof.as_slice())
        .ok_or_else(|| "EOF not found! Video corrupted.".to_string())?;
    let payload = &raw[0..pos];

    let p_str = String::from_utf8_lossy(&payload[0..usize::min(500, payload.len())]);
    if let Some(end) = p_str.find('|') {
        let parts: Vec<&str> = p_str[0..end].split(':').collect();
        if parts.len() >= 4 && parts[0] == "FILE" && parts[2] == "SIZE" {
            let filename = parts[1];
            let is_zstd = parts.len() >= 6 && parts[4] == "ZSTD" && parts[5] == "1";

            let _ = tx.send(JobMsg::Log("Decrypting AES-256-GCM...".into()));
            let decrypted = aes_decrypt(&payload[p_str[0..end].as_bytes().len() + 1..], &key)?;

            let final_data = if is_zstd {
                let _ = tx.send(JobMsg::Log("Decompressing Zstandard...".into()));
                zstd::decode_all(&decrypted[..]).map_err(|e| format!("Zstd err: {}", e))?
            } else {
                decrypted
            };

            if filename.ends_with(".tar") {
                let _ = tx.send(JobMsg::Log("Unpacking folder from TAR...".into()));
                let folder_name = filename.replace(".tar", "");
                let target_dir = Path::new(&out_dir).join(folder_name);
                fs::create_dir_all(&target_dir).unwrap_or_default();
                unpack_tar(&final_data, target_dir.to_str().unwrap())?;
            } else {
                let out_name = Path::new(&out_dir).join(format!("decoded_{}", filename));
                fs::write(&out_name, final_data).map_err(|e| e.to_string())?;
            }

            let elapsed = start_time.elapsed();
            let _ = tx.send(JobMsg::Log(format!(
                "Extraction complete. Time: {:.1}s",
                elapsed.as_secs_f64()
            )));

            let _ = tx.send(JobMsg::Progress(1.0));
            return Ok(());
        }
    }
    Err("Header corrupted.".to_string())
}

pub fn decode_url_process(
    url: String,
    out_dir: String,
    key: String,
    use_fec: bool,
    use_mc: bool,
    use_sync: bool,
    hw_accel: u8,
    tx: Sender<JobMsg>,
) -> Result<(), String> {
    let _ = tx.send(JobMsg::Log(format!("Intercepting URL: {}...", url)));

    let temp_video_path = std::env::temp_dir().join("eidolon_ytdlp.mp4");
    let temp_video_str = temp_video_path.to_string_lossy().to_string();
    let _ = fs::remove_file(&temp_video_path);

    let _ = tx.send(JobMsg::Log("Downloading video stream via yt-dlp...".into()));

    let status = Command::new(get_tool_path("yt-dlp")?)
        .args([
            "-f",
            "bestvideo[ext=mp4]/best[ext=mp4]/best",
            "--no-playlist",
            "-o",
            &temp_video_str,
            &url,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| {
            "yt-dlp not found! Please ensure it is installed or available in PATH.".to_string()
        })?;

    if !status.success() {
        return Err("yt-dlp failed to download the video. Check URL or connection.".to_string());
    }

    let _ = tx.send(JobMsg::Log(
        "Download complete. Routing to extraction pipeline...".into(),
    ));
    let result = decode_process(
        temp_video_str,
        out_dir,
        key,
        use_fec,
        use_mc,
        use_sync,
        hw_accel,
        tx,
    );
    let _ = fs::remove_file(&temp_video_path);
    result
}