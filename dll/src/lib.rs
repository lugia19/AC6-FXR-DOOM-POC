//! fxr_doom: runs Doom and serves its 320x200 frames to FXRs as custom external values.
//!
//! Hooks the function effects read external values through (0x142b56480 in Armored Core VI 1.09.1) and answers
//! IDs BASE + (y * 320 + x) * 4 + c (c: 0 = R, 1 = G, 2 = B) with the channel as 0..1. Every other ID goes to the
//! game. Doom (doomgeneric, see doom.rs) runs on its own thread and writes its frames into FRAMEBUFFER.
//!
//! Log: Game\debug-logs\fxr_doom.log

mod doom;

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use retour::RawDetour;

const GET_ADDR: usize = 0x142b56480;
// mov [rsp+18],r8d
const GET_PROLOGUE: [u8; 5] = [0x44, 0x89, 0x44, 0x24, 0x18];

pub(crate) const WIDTH: usize = 320;
pub(crate) const HEIGHT: usize = 200;
const BASE: i32 = 1_000_000;
const END: i32 = BASE + (WIDTH * HEIGHT * 4) as i32;

static ORIG_GET: AtomicUsize = AtomicUsize::new(0);
/// Doom's current frame, 0xRRGGBB per pixel.
pub(crate) static FRAMEBUFFER: [AtomicU32; WIDTH * HEIGHT] = [const { AtomicU32::new(0) }; WIDTH * HEIGHT];
static LOG: OnceLock<Mutex<File>> = OnceLock::new();

pub(crate) fn log(msg: &str) {
    if let Some(Ok(mut f)) = LOG.get().map(|l| l.lock()) {
        let _ = writeln!(f, "{msg}");
    }
}

unsafe extern "system" fn get_impl(block: *mut u8, out: *mut u32, id: i32) -> *mut u32 {
    if (BASE..END).contains(&id) && !out.is_null() {
        // Our IDs never come from the game, so its lookup is skipped.
        let i = (id - BASE) as usize;
        let rgb = FRAMEBUFFER[i / 4].load(Ordering::Relaxed);
        let value = match i % 4 {
            c @ 0..=2 => ((rgb >> (16 - 8 * c)) & 0xFF) as f32 / 255.0,
            _ => 0.0,
        };
        unsafe { *out = value.to_bits() };
        return out;
    }
    let orig: unsafe extern "system" fn(*mut u8, *mut u32, i32) -> *mut u32 =
        unsafe { std::mem::transmute(ORIG_GET.load(Ordering::Relaxed)) };
    unsafe { orig(block, out, id) }
}

fn install_hook() -> Result<(), String> {
    let prologue = unsafe { std::slice::from_raw_parts(GET_ADDR as *const u8, 5) };
    if prologue != GET_PROLOGUE {
        return Err(format!("unexpected bytes at {GET_ADDR:#x} (not 1.09.1?)"));
    }
    let detour = unsafe { RawDetour::new(GET_ADDR as *const (), get_impl as *const ()) }.map_err(|e| e.to_string())?;
    ORIG_GET.store(detour.trampoline() as *const () as usize, Ordering::Relaxed);
    unsafe { detour.enable() }.map_err(|e| e.to_string())?;
    // The hook stays installed for the life of the process.
    std::mem::forget(detour);
    Ok(())
}

/// Folder of the DLL itself (the WAD goes in doom\ next to it).
fn dll_dir(module: *mut u8) -> Option<PathBuf> {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetModuleFileNameW(module: *mut u8, name: *mut u16, size: u32) -> u32;
    }
    let mut buf = [0u16; 1024];
    let len = unsafe { GetModuleFileNameW(module, buf.as_mut_ptr(), buf.len() as u32) } as usize;
    (len > 0).then(|| PathBuf::from(String::from_utf16_lossy(&buf[..len])).parent().map(|p| p.to_path_buf()))?
}

fn init(module: *mut u8) {
    let dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join("debug-logs")))
        .unwrap_or_else(|| PathBuf::from("debug-logs"));
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(f) = OpenOptions::new().create(true).write(true).truncate(true).open(dir.join("fxr_doom.log")) {
        let _ = LOG.set(Mutex::new(f));
    }
    if let Err(e) = install_hook() {
        log(&format!("not active: {e}"));
        return;
    }
    log("hook installed");
    match dll_dir(module) {
        Some(dir) => doom::start(&dir.join("doom")),
        None => log("couldn't find the DLL's folder"),
    }
}

#[unsafe(no_mangle)]
extern "system" fn DllMain(module: *mut u8, reason: u32, _reserved: *mut u8) -> i32 {
    const DLL_PROCESS_ATTACH: u32 = 1;
    if reason == DLL_PROCESS_ATTACH {
        init(module);
    }
    1
}
