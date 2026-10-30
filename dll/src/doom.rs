//! Runs doomgeneric on its own thread and implements its platform functions: frames go into the framebuffer the
//! screen FXRs read, input comes from the keyboard while the game window is focused.

use std::collections::VecDeque;
use std::ffi::{CString, c_char, c_int};
use std::path::Path;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::{FRAMEBUFFER, HEIGHT, WIDTH, log};

unsafe extern "C" {
    static DG_ScreenBuffer: *const u32;
    fn doomgeneric_Create(argc: c_int, argv: *const *const c_char);
    fn doomgeneric_Tick();
}

#[link(name = "user32")]
unsafe extern "system" {
    fn GetAsyncKeyState(vkey: i32) -> i16;
    fn GetForegroundWindow() -> isize;
    fn GetWindowThreadProcessId(hwnd: isize, pid: *mut u32) -> u32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn ExitThread(code: u32) -> !;
}

static START: OnceLock<Instant> = OnceLock::new();
/// F8 toggles whether keys go to Doom.
static INPUT_ON: AtomicBool = AtomicBool::new(true);

// doomkeys.h
const KEY_RIGHTARROW: u8 = 0xae;
const KEY_LEFTARROW: u8 = 0xac;
const KEY_UPARROW: u8 = 0xad;
const KEY_DOWNARROW: u8 = 0xaf;
const KEY_STRAFE_L: u8 = 0xa0;
const KEY_STRAFE_R: u8 = 0xa1;
const KEY_USE: u8 = 0xa2;
const KEY_FIRE: u8 = 0xa3;
const KEY_ESCAPE: u8 = 27;
const KEY_ENTER: u8 = 13;

const VK_F8: i32 = 0x77;
/// Windows virtual key -> Doom key. Picked to stay off Armored Core's default keyboard controls.
const KEYMAP: &[(i32, u8)] = &[
    (0x26, KEY_UPARROW),
    (0x28, KEY_DOWNARROW),
    (0x25, KEY_LEFTARROW),
    (0x27, KEY_RIGHTARROW),
    (0xA3, KEY_FIRE),   // right Ctrl
    (0xA1, KEY_USE),    // right Shift
    (0xBC, KEY_STRAFE_L), // ,
    (0xBE, KEY_STRAFE_R), // .
    (0x0D, KEY_ENTER),
    (0x08, KEY_ESCAPE), // Backspace (Esc opens Armored Core's menu)
    (0x31, b'1'),
    (0x32, b'2'),
    (0x33, b'3'),
    (0x34, b'4'),
    (0x35, b'5'),
    (0x36, b'6'),
    (0x37, b'7'),
    (0x59, b'y'),
    (0x4E, b'n'),
];

/// Key state as Doom last saw it, and events not delivered yet. Only Doom's thread touches these.
struct Input {
    down: [bool; KEYMAP.len()],
    f8_down: bool,
    events: VecDeque<(bool, u8)>,
}
static mut INPUT: Input = Input { down: [false; KEYMAP.len()], f8_down: false, events: VecDeque::new() };

fn game_focused() -> bool {
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(GetForegroundWindow(), &mut pid) };
    pid == std::process::id()
}

fn poll_keys(input: &mut Input) {
    let focused = game_focused();
    let f8 = focused && unsafe { GetAsyncKeyState(VK_F8) } < 0;
    if f8 && !input.f8_down {
        let on = !INPUT_ON.fetch_xor(true, Ordering::Relaxed);
        log(&format!("Doom input {}", if on { "on" } else { "off" }));
    }
    input.f8_down = f8;
    let active = focused && INPUT_ON.load(Ordering::Relaxed);
    for (i, &(vk, key)) in KEYMAP.iter().enumerate() {
        let now = active && unsafe { GetAsyncKeyState(vk) } < 0;
        if now != input.down[i] {
            input.down[i] = now;
            input.events.push_back((now, key));
        }
    }
}

#[unsafe(no_mangle)]
extern "C" fn DG_Init() {}

#[unsafe(no_mangle)]
extern "C" fn DG_DrawFrame() {
    let frame = unsafe { std::slice::from_raw_parts(DG_ScreenBuffer, WIDTH * HEIGHT) };
    for (dst, &src) in FRAMEBUFFER.iter().zip(frame) {
        dst.store(src & 0xFF_FFFF, Ordering::Relaxed);
    }
}

#[unsafe(no_mangle)]
extern "C" fn DG_SleepMs(ms: u32) {
    std::thread::sleep(Duration::from_millis(ms.into()));
}

#[unsafe(no_mangle)]
extern "C" fn DG_GetTicksMs() -> u32 {
    START.get_or_init(Instant::now).elapsed().as_millis() as u32
}

/// Doom calls this until it returns 0, every tic.
#[unsafe(no_mangle)]
extern "C" fn DG_GetKey(pressed: *mut c_int, key: *mut u8) -> c_int {
    #[allow(static_mut_refs)]
    let input = unsafe { &mut INPUT };
    if input.events.is_empty() {
        poll_keys(input);
    }
    match input.events.pop_front() {
        Some((down, k)) => {
            unsafe {
                *pressed = down as c_int;
                *key = k;
            }
            1
        }
        None => 0,
    }
}

#[unsafe(no_mangle)]
extern "C" fn DG_SetWindowTitle(_title: *const c_char) {}

/// Doom's exit(), redirected by c/dg_platform.h: ends Doom's thread, not the game.
#[unsafe(no_mangle)]
extern "C" fn dg_exit(code: c_int) {
    log(&format!("Doom exited ({code})"));
    unsafe { ExitThread(code as u32) }
}

/// Starts Doom with the first IWAD found in `wad_dir`. Configs and saves go to the game's working directory.
pub fn start(wad_dir: &Path) {
    let Some(wad) = ["doom2.wad", "doom.wad", "doom1.wad"].iter().map(|n| wad_dir.join(n)).find(|p| p.exists()) else {
        log(&format!("no doom2.wad, doom.wad or doom1.wad in {}", wad_dir.display()));
        return;
    };
    log(&format!("starting Doom with {}", wad.display()));
    let args: Vec<CString> = ["doom", "-iwad", &wad.to_string_lossy()].iter().map(|a| CString::new(*a).unwrap()).collect();
    START.get_or_init(Instant::now);
    let spawned = std::thread::Builder::new().name("doom".into()).stack_size(16 << 20).spawn(move || {
        // Doom keeps pointers into argv.
        let argv: &'static [*const c_char] = Vec::leak(args.iter().map(|a| a.as_ptr()).collect());
        std::mem::forget(args);
        unsafe {
            doomgeneric_Create(argv.len() as c_int, argv.as_ptr());
            loop {
                doomgeneric_Tick();
            }
        }
    });
    if let Err(e) = spawned {
        log(&format!("couldn't start Doom: {e}"));
    }
}
