use std::env;

use wafel_viz::SM64RenderConfig;

#[derive(Debug, Clone)]
pub struct TestCase {
    pub name: String,
    pub game_version: &'static str,
    pub m64: &'static str,
    pub frame: u32,
    pub config: SM64RenderConfig,
}

fn u120(cases: &mut Vec<TestCase>) {
    let mut case = |name: &str, frame| {
        cases.push(TestCase {
            name: name.to_string(),
            game_version: "us",
            m64: "120_u",
            frame,
            config: SM64RenderConfig::default(),
        })
    };
    case("u120_000000_power_on", 0);
    case("u120_000052_logo", 52);
    case("u120_000100_intro_star", 100);
    case("u120_000120_mario_head", 120);
    case("u120_000135_before_file_select", 135);
    case("u120_000140_file_select", 140);
    case("u120_000169_after_file_select", 169);
    case("u120_000279_peach_letter", 279);
    case("u120_000975_castle", 975);
    case("u120_001059_water_shadow", 1059);
    case("u120_001176_tree_shadows", 1176);
    case("u120_001502_mario", 1502);
    case("u120_001624_tilted_text", 1624);
    case("u120_001627_scrolling_text", 1627);
    case("u120_001722_dust", 1722);
    case("u120_001905_key_door", 1905);
    case("u120_002037_pause", 2037);
    case("u120_002299_wkww", 2299);
    case("u120_002418_star_grab", 2418);
    case("u120_002628_continue1", 2628);
    case("u120_002632_continue2", 2632);
    case("u120_003079_high_poly", 3079);
    case("u120_003080_low_poly", 3080);
    case("u120_003363_exit_slide", 3363);
    case("u120_003422_slide_star", 3422);
    case("u120_003509_slide_star_spawn", 3509);
    case("u120_003603_slide_star_grab", 3603); // TODO: Mupen shows an extra snow particle
    case("u120_004375_snow_run", 4375);
    case("u120_004575_star_fade", 4575);
    case("u120_005577_slide_ice", 5577);
    case("u120_005785_penguin_star", 5785);
    case("u120_006122_dust_butt", 6122);
    case("u120_006180_star_head", 6180);
    case("u120_008282_hold_penguin", 8282);
    case("u120_008488_penguin_shadow", 8488);
    case("u120_009044_star_select", 9044);
    case("u120_009738_ice", 9738);
    case("u120_010543_caught_in_the_undertoad", 10543);
    case("u120_010565_pss_room", 10565);
    case("u120_010717_pss_fog", 10717);
    case("u120_010802_pss_cull", 10802);
    case("u120_010903_pss_wall", 10903);
    case("u120_010908_pss_fog2", 10908);
    case("u120_011024_pss_fog3", 11024);
    case("u120_011326_mario_mario", 11326);
    case("u120_011360_lobby_decal", 11360);
    case("u120_011638_lod_peach1", 11638);
    case("u120_011645_lod_peach2", 11645);
    case("u120_011652_lod_peach3", 11652);
    case("u120_011749_transparent_box", 11749);
    case("u120_011845_flames", 11845);
    case("u120_012222_amps", 12222);
    case("u120_013385_bowser_key_1", 13385);
    case("u120_013408_bowser_key_2", 13408);
    case("u120_013480_bowser_key_3", 13480);
    case("u120_013808_key_cutscene", 13808);
    case("u120_015056_totwc_light", 15056);
    case("u120_015098_totwc_light_2", 15098);
    case("u120_015117_totwc_light_3", 15117);
    case("u120_015156_totwc_1", 15156);
    case("u120_015410_totwc_2", 15410);
    case("u120_015547_totwc_3_blocky_clouds", 15547);
    case("u120_015744_totwc_4", 15744);
    case("u120_015829_totwc_5", 15829);
    case("u120_015852_totwc_6", 15852);
    case("u120_016282_wf_entry", 16282);
    case("u120_017233_cage_cull", 17233);
    case("u120_022651_box_squish", 22651);
    case("u120_022659_cap_grab", 22659);
    case("u120_022839_winged_goomba", 22839);
    case("u120_022871_tree_hold_1", 22871);
    case("u120_022874_tree_hold_2", 22874);
    case("u120_023097_bob_fog", 23097);
    case("u120_024183_bob_fog_2", 24183);
    case("u120_024386_bob_oob", 24386);
    case("u120_025086_fading_shadow", 25086);
    case("u120_025090_tilted_shadow", 25090);
    case("u120_025141_limbo", 25141);
    case("u120_026614_small_box_hold", 26614);
    case("u120_026638_box_kick", 26638);
    case("u120_032013_whirlpool", 32013);
    case("u120_032935_bowser_door", 32935);
    case("u120_034660_ddd_entrance", 34660);
    case("u120_034725_ddd_entrance_2", 34725);
    case("u120_035247_inside_chest", 35247);
    case("u120_036082_ddd_entry", 36082);
    case("u120_038713_ddd_rings", 38713);
    case("u120_041689_lava", 41689);
    case("u120_044732_mips_shadow", 44732);
    case("u120_045459_hmc_entrance_1", 45459);
    case("u120_045467_hmc_entrance_2", 45467);
    case("u120_045833_hmc_fog", 45833);
    case("u120_045966_hmc_limbo", 45966);
    case("u120_052884_dorrie_1", 52884);
    case("u120_052943_dorrie_2", 52943);
    case("u120_052968_dorrie_3", 52968);
    case("u120_053197_metal_cap", 53197);
    case("u120_053496_metal_cap_2", 53496);
    case("u120_054664_hmc_star_grab", 54664);
    case("u120_055577_pokey_face", 55577);
    case("u120_055642_quicksand", 55642);
    case("u120_055945_ssl_flame", 55945);
    case("u120_056273_pyramid_fog", 56273);
    case("u120_060881_star_grab_in_sand", 60881); // TODO: Mupen has black bars on top/bottom, which is noticeable here
    case("u120_061098_ssl_ripple", 61098);
    case("u120_061473_bomb_clip", 61473);
    case("u120_063451_lll_ripple", 63451);
    case("u120_064590_smoke_texture", 64590);
    case("u120_066344_lll_puzzle", 66344);
    case("u120_068798_lava_shell", 68798);
    case("u120_069260_lavafall", 69260); // TODO: Texture is different than on mupen
    case("u120_071009_vanish", 71009); // TODO: Dithering not implemented
    case("u120_071323_vanish_2", 71323);
    case("u120_072554_jrb_fog_and_metal", 72554);
    case("u120_081881_bbh_entry", 81881);
    case("u120_081960_bbh_entry_2", 81960);
    case("u120_082906_killing_a_ghost", 82906);
    case("u120_085194_vc_box_top", 85194);
    case("u120_085282_bbh_door", 85282);
    case("u120_085800_bbh_star_grab", 85800);
    case("u120_090671_bbh_window", 90671);
    case("u120_092354_koopa_underwear", 92354);
    case("u120_099903_wiggler_1", 99903);
    case("u120_100449_wiggler_2", 100449);
    case("u120_100758_mirror_room", 100758);
    case("u120_102446_ice_bully", 102446);
    case("u120_103259_snowmans_head", 103259);
    case("u120_104109_igloo_star", 104109); // TODO: Extra snow particle on mupen
    case("u120_105905_moneybags", 105905);
    case("u120_109556_ttc_slide_ripples", 109556);
    case("u120_109612_ttc_slide_fade", 109612);
    case("u120_110107_slide_trap", 110107);
    case("u120_110127_slide_smile", 110127);
    case("u120_110130_slide_smile_gone", 110130);
    case("u120_110147_slide_smile_back", 110147);
    case("u120_114943_wdw_water_shadow_1", 114943);
    case("u120_114955_wdw_water_shadow_2", 114955);
    case("u120_117872_crystal_tap", 117872);
    case("u120_122216_cloud_entry", 122216);
    case("u120_122942_cannon_shot_1", 122942);
    case("u120_123263_cannon_shot_2", 123263);
    case("u120_123589_rain_cloud", 123589);
    case("u120_125576_ttc_fog", 125576);
    case("u120_126160_ttc_fog_2", 126160);
    case("u120_128143_clock", 128143);
    case("u120_129859_toad_star", 129859);
    case("u120_130266_rr_rainbow", 130266);
    case("u120_130373_rr_blue_flame", 130373);
    case("u120_134235_cannon_shot_3", 134235);
    case("u120_137580_staircase_fog", 137580);
    case("u120_138322_bits_seam", 138322);
    case("u120_138984_bits_blj", 138984);
    case("u120_139601_bowser3", 139601);
    case("u120_139721_boomer3", 139721);
    case("u120_141811_peach_cutscene_1", 141811);
    case("u120_141930_peach_cutscene_2", 141930);
    case("u120_144714_credits_wf", 144714);
    case("u120_145414_credits_bbh_1", 145414);
    case("u120_145473_credits_bbh_2", 145473);
    case("u120_147682_credits_ttc", 147682);
    case("u120_148424_credits_cotmc", 148424);
    case("u120_148484_credits_ddd_1", 148484);
    case("u120_148573_credits_ddd_2", 148573);
    case("u120_149182_credits_zoom", 149182);
    case("u120_149706_thank_you", 149706);
}

fn res(cases: &mut Vec<TestCase>) {
    let mut case = |name: &str, frame| {
        cases.push(TestCase {
            name: format!("res_800_500_{}", name),
            game_version: "us",
            m64: "120_u",
            frame,
            config: SM64RenderConfig {
                screen_size: (800, 500),
                ..Default::default()
            },
        });
        cases.push(TestCase {
            name: format!("res_400_600_{}", name),
            game_version: "us",
            m64: "120_u",
            frame,
            config: SM64RenderConfig {
                screen_size: (400, 600),
                ..Default::default()
            },
        });
        cases.push(TestCase {
            name: format!("res_160_120_{}", name),
            game_version: "us",
            m64: "120_u",
            frame,
            config: SM64RenderConfig {
                screen_size: (160, 120),
                ..Default::default()
            },
        });
    };
    case("u120_000052_logo", 52);
    case("u120_000120_mario_head", 120);
    case("u120_000140_file_select", 140);
    case("u120_000279_peach_letter", 279);
    case("u120_000902_doorknob_lod", 902);
    case("u120_001059_water_shadow", 1059);
    case("u120_001502_mario", 1502);
    case("u120_001624_tilted_text", 1624);
    case("u120_001627_scrolling_text", 1627);
    case("u120_001905_key_door", 1905);
    case("u120_002037_pause", 2037);
    case("u120_002628_continue1", 2628);
    case("u120_002632_continue2", 2632);
    case("u120_003363_exit_slide", 3363);
    case("u120_009044_star_select", 9044);
    case("u120_013808_key_cutscene", 13808);
    case("u120_071009_vanish", 71009);
    case("u120_071323_vanish_2", 71323);
    case("u120_141811_peach_cutscene_1", 141811);
    case("u120_141930_peach_cutscene_2", 141930);
    case("u120_144714_credits_wf", 144714);
    case("u120_145414_credits_bbh_1", 145414);
    case("u120_145473_credits_bbh_2", 145473);
    case("u120_147682_credits_ttc", 147682);
    case("u120_148424_credits_cotmc", 148424);
    case("u120_148484_credits_ddd_1", 148484);
    case("u120_148573_credits_ddd_2", 148573);
    case("u120_149182_credits_zoom", 149182);
    case("u120_149706_thank_you", 149706);
}

fn reg(cases: &mut Vec<TestCase>) {
    let mut case = |name: &str, frame| {
        for version in ["us", "jp", "eu", "sh"] {
            cases.push(TestCase {
                name: format!("reg_{}_{}", version, name),
                game_version: version,
                m64: "cross_version",
                frame,
                config: SM64RenderConfig::default(),
            });
        }
    };
    // TODO: Broken texture/text on SH
    case("000030_title", 30);
    case("000129_mario_head", 129); // Broken texture on SH
    case("000505_mario_head_lol", 505); // Broken texture on SH
    case("000975_wf", 975);
    case("002505_ccm", 2505);
    case("004245_bbh", 4245);
    case("005905_jrb", 5905);
    case("007607_hmc", 7607);
    case("009507_pss", 9507);
    case("010209_fade", 10209);
    case("010242_file_select_1", 10242);
    case("010555_file_select_2", 10555); // Broken text on SH
    case("010822_peach_letter", 10822);
    case("011240_lakitu_1", 11240);
    case("011396_lakitu_2", 11396);
    case("011708_mario", 11708); // Broken text on SH
    case("011829_tilted_text", 11829);
    case("011838_text", 11838);
    case("012802_pause", 12802); // Broken text on SH
    case("013366_lakitu_scroll", 13366); // Broken text on SH
    case("013796_enter_castle", 13796);
    case("014268_castle", 14268);
    case("014475_ripple", 14475);
    case("014578_star_select", 14578); // Broken text on SH
    case("014757_bob_text", 14757); // Broken text on SH
    case("015064_pause", 15064); // Broken text on SH
}

pub fn all() -> Vec<TestCase> {
    let mut cases = Vec::new();

    u120(&mut cases);
    if !env::args().any(|s| s == "--no-res") {
        res(&mut cases);
    }
    reg(&mut cases);

    cases
}
