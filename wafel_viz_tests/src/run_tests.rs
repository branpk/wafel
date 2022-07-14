use std::{env, fmt::Write, fs};

use image::{Rgb, RgbImage};
use itertools::Itertools;

use crate::{game_runner::GameRunner, renderer::Renderer, TestCase};

pub fn run_tests(mut test_cases: Vec<TestCase>) -> Result<(), Box<dyn std::error::Error>> {
    let calc_diffs = env::args().any(|arg| arg == "--diff");
    let target = env::args()
        .tuple_windows()
        .find(|(flag, _)| flag == "--target")
        .map(|(_, target)| target);

    env_logger::init();

    if !env::args().any(|arg| arg == "--no-delete") {
        let _ = fs::remove_dir_all("wafel_viz_tests/output");
    }
    fs::create_dir_all("wafel_viz_tests/output/all")?;
    fs::create_dir_all("wafel_viz_tests/output/mismatches")?;
    if calc_diffs {
        fs::create_dir_all("wafel_viz_tests/output/diff")?;
    }

    let mut runner = GameRunner::default();
    let mut renderer = Renderer::new();

    test_cases.sort_by_key(|case| (case.game_version, case.m64, case.frame));

    let mut mismatches = Vec::new();

    for (i, case) in test_cases.iter().enumerate() {
        let game = runner.get_frame(case.game_version, case.m64, case.frame);

        let actual = renderer.render(game, &case.config);

        let expected = image::open(format!(
            "wafel_viz_tests/{}/{}.png",
            target.as_deref().unwrap_or_else(|| renderer.device_info()),
            case.name
        ))
        .ok()
        .map(|img| img.to_rgb8());

        actual.save(format!("wafel_viz_tests/output/all/{}.png", case.name))?;

        let matches = Some(&actual) == expected.as_ref();
        if !matches {
            actual.save(format!(
                "wafel_viz_tests/output/mismatches/{}.png",
                case.name
            ))?;
            mismatches.push(case.name.clone());

            if calc_diffs {
                if let Some(expected) = expected.as_ref() {
                    let w = expected.width();
                    let h = expected.height();
                    if actual.width() == w && actual.height() == h {
                        let mut diff = RgbImage::new(w, h);
                        for y in 0..h {
                            for x in 0..w {
                                let actual_rgb = *actual.get_pixel(x, y);
                                let expected_rgb = *expected.get_pixel(x, y);
                                let diff_rgb = Rgb::from([
                                    actual_rgb.0[0].abs_diff(expected_rgb.0[0]),
                                    actual_rgb.0[1].abs_diff(expected_rgb.0[1]),
                                    actual_rgb.0[2].abs_diff(expected_rgb.0[2]),
                                ]);
                                diff.put_pixel(x, y, diff_rgb);
                            }
                            diff.save(format!("wafel_viz_tests/output/diff/{}.png", case.name))?;
                        }
                    }
                }
            }
        };

        eprintln!(
            "[{:3}/{}] \x1b[{}m{}\x1b[0m",
            i + 1,
            test_cases.len(),
            if matches { 32 } else { 31 },
            case.name
        );
    }

    eprintln!();
    if mismatches.is_empty() {
        eprintln!("\x1b[32mAll cases match!\x1b[0m");
    } else {
        eprintln!("\x1b[31m{} mismatches\x1b[0m", mismatches.len());
    }

    let template = fs::read_to_string("wafel_viz_tests/index.html.template")?;
    let mut image_html = String::new();
    for name in &mismatches {
        writeln!(
            image_html,
            r#"
                <tr>
                    <td>{}</td>
                    <td><img src="{}/{}.png"></td>
                    <td><img src="output/all/{}.png"></td>
                    {}
                </tr>
            "#,
            name,
            target.as_deref().unwrap_or_else(|| renderer.device_info()),
            name,
            name,
            if calc_diffs {
                format!(r#"<td><img src="output/diff/{}.png"></td>"#, name)
            } else {
                "".to_string()
            }
        )?;
    }
    let html = template.replace("[[images]]", &image_html);
    fs::write("wafel_viz_tests/index.html", html)?;

    Ok(())
}
