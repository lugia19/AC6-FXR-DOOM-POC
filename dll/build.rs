// Compiles doomgeneric (https://github.com/ozkl/doomgeneric, cloned into dll/doomgeneric) into the DLL. The platform
// functions it needs (DG_Init, DG_DrawFrame, ...) are implemented in src/lib.rs.
const SOURCES: &str = "dummy am_map doomdef doomstat dstrings d_event d_items d_iwad d_loop d_main d_mode d_net f_finale \
    f_wipe g_game hu_lib hu_stuff info i_cdmus i_endoom i_joystick i_scale i_sound i_system i_timer memio m_argv m_bbox \
    m_cheat m_config m_controls m_fixed m_menu m_misc m_random p_ceilng p_doors p_enemy p_floor p_inter p_lights p_map \
    p_maputl p_mobj p_plats p_pspr p_saveg p_setup p_sight p_spec p_switch p_telept p_tick p_user r_bsp r_data r_draw \
    r_main r_plane r_segs r_sky r_things sha1 sounds statdump st_lib st_stuff s_sound tables v_video wi_stuff \
    w_checksum w_file w_main w_wad z_zone w_file_stdc i_input i_video doomgeneric";

fn main() {
    let dir = std::path::Path::new("doomgeneric/doomgeneric");
    assert!(dir.is_dir(), "doomgeneric missing: git clone https://github.com/ozkl/doomgeneric dll/doomgeneric");
    let mut build = cc::Build::new();
    for name in SOURCES.split_whitespace() {
        build.file(dir.join(format!("{name}.c")));
    }
    build
        .include(dir)
        .define("DOOMGENERIC_RESX", "320")
        .define("DOOMGENERIC_RESY", "200")
        .define("_CRT_SECURE_NO_WARNINGS", None)
        .flag(format!("/FI{}", std::path::absolute("c/dg_platform.h").unwrap().display()))
        .warnings(false)
        .compile("doomgeneric");
    println!("cargo:rerun-if-changed=c/dg_platform.h");
    println!("cargo:rerun-if-changed=doomgeneric/doomgeneric");
}
