//! ER contract, CLI, replay and visual regression coverage.

use std::fs;
use std::path::Path;

use assert_cmd::cargo::cargo_bin_cmd;
use mmdflux::graph::GeometryLevel;
use mmdflux::{OutputFormat, RenderConfig, detect_diagram, render_diagram};

const CUSTOMER_ORDER: &str = include_str!("fixtures/er/customer_order.mmd");

#[test]
fn er_facade_and_cli_support_all_formats() {
    assert_eq!(detect_diagram(CUSTOMER_ORDER).unwrap(), "er");
    for (format, token) in [
        (OutputFormat::Text, "text"),
        (OutputFormat::Ascii, "ascii"),
        (OutputFormat::Svg, "svg"),
        (OutputFormat::Mmds, "mmds"),
    ] {
        let output = render_diagram(CUSTOMER_ORDER, format, &RenderConfig::default()).unwrap();
        for text in ["CUSTOMER", "ORDER", "customer_id FK", "id PK", "places"] {
            assert!(output.contains(text), "missing {text} in {token}: {output}");
        }
        if format == OutputFormat::Ascii {
            assert!(output.is_ascii());
            assert!(output.contains("0..*"));
        }
        let cli = cargo_bin_cmd!("mmdflux")
            .args(["--format", token])
            .write_stdin(CUSTOMER_ORDER)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        assert!(String::from_utf8(cli).unwrap().contains("CUSTOMER"));
    }
}

#[test]
fn er_mmds_schema_and_replay_preserve_cardinality_and_attributes() {
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../docs/mmds.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    for level in [GeometryLevel::Layout, GeometryLevel::Routed] {
        let config = RenderConfig {
            geometry_level: level,
            ..RenderConfig::default()
        };
        let json = render_diagram(CUSTOMER_ORDER, OutputFormat::Mmds, &config).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(
            validator.is_valid(&value),
            "schema errors: {:?}",
            validator.iter_errors(&value).collect::<Vec<_>>()
        );
        assert_eq!(value["version"], 1);
        assert_eq!(value["metadata"]["diagram_type"], "er");
        assert_eq!(value["edges"][0]["arrow_start"], "er_one");
        assert_eq!(value["edges"][0]["arrow_end"], "er_zero_or_more");
        assert_eq!(detect_diagram(&json).unwrap(), "er");
        for format in [OutputFormat::Svg, OutputFormat::Text, OutputFormat::Ascii] {
            let replay = render_diagram(&json, format, &RenderConfig::default()).unwrap();
            assert!(replay.contains("customer_id FK"));
            if format == OutputFormat::Svg {
                assert!(replay.contains("marker-start=\"url(#er-one)\""));
                assert!(replay.contains("marker-end=\"url(#er-zero-or-more)\""));
                assert!(!replay.contains("0..*"));
            } else {
                assert!(replay.contains("0..*"), "{replay}");
            }
        }
        assert!(render_diagram(&json, OutputFormat::Mermaid, &RenderConfig::default()).is_err());
    }
}

#[test]
fn er_snapshots() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut fixtures = fs::read_dir(root.join("tests/fixtures/er"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    fixtures.sort();
    for fixture in fixtures {
        let input = fs::read_to_string(&fixture).unwrap();
        let stem = fixture.file_stem().unwrap().to_str().unwrap();
        for (format, dir, extension) in [
            (OutputFormat::Text, "snapshots/er", "txt"),
            (OutputFormat::Ascii, "ascii-snapshots/er", "txt"),
            (OutputFormat::Svg, "svg-snapshots/er", "svg"),
        ] {
            let output = render_diagram(&input, format, &RenderConfig::default()).unwrap();
            if format == OutputFormat::Ascii {
                assert!(output.is_ascii());
            }
            let path = root
                .join("tests")
                .join(dir)
                .join(format!("{stem}.{extension}"));
            if std::env::var_os("GENERATE_ER_SNAPSHOTS").is_some() {
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(&path, &output).unwrap();
            }
            assert_eq!(
                output,
                fs::read_to_string(&path).unwrap(),
                "{}",
                path.display()
            );
        }
    }
}

#[test]
fn er_malformed_or_unsupported_input_is_rejected() {
    for source in [
        "erDiagram\nA { int id",
        "erDiagram\nA { int }",
        "erDiagram\nA ||--o{ B",
        "erDiagram\nA ||--x{ B : bad",
        "erDiagram\nA ||--o{ B : \"unclosed",
        "erDiagram\nA { int id PX }",
        "erDiagram\nA[Alias]",
        "erDiagram\ndirection XX",
        "erDiagram\nA ||--o{ B : fine\nstyle A fill:red",
        "erDiagram\n\"unclosed",
        "erDiagram\nA { int id \"unclosed }",
    ] {
        assert!(
            render_diagram(source, OutputFormat::Svg, &RenderConfig::default()).is_err(),
            "accepted: {source}"
        );
    }
}

#[test]
fn er_svg_markers_and_text_cardinalities_cover_both_endpoints() {
    let source = include_str!("fixtures/er/cardinalities.mmd");
    let svg = render_diagram(source, OutputFormat::Svg, &RenderConfig::default()).unwrap();
    for marker in [
        "er-one",
        "er-zero-or-one",
        "er-one-or-more",
        "er-zero-or-more",
    ] {
        assert!(svg.contains(&format!("marker-start=\"url(#{marker})\"")));
        assert!(svg.contains(&format!("marker-end=\"url(#{marker})\"")));
    }
    assert!(svg.contains("auto-start-reverse"));
    assert!(svg.contains("stroke-dasharray"));
    for cardinality in ["0..1", "1..*", "0..*"] {
        assert!(!svg.contains(cardinality));
        let ascii = render_diagram(source, OutputFormat::Ascii, &RenderConfig::default()).unwrap();
        assert!(
            ascii.contains(cardinality),
            "missing {cardinality}: {ascii}"
        );
    }
}

#[test]
fn er_validation_materialization_fit_and_show_ids_use_graph_facade() {
    let validation: serde_json::Value =
        serde_json::from_str(&mmdflux::validate_diagram(CUSTOMER_ORDER)).unwrap();
    assert_eq!(validation["valid"], true);
    let config = RenderConfig {
        show_ids: true,
        ..RenderConfig::default()
    };
    let document = mmdflux::materialize_diagram(CUSTOMER_ORDER, &config).unwrap();
    assert_eq!(document.metadata.diagram_type, "er");
    assert!(
        document
            .nodes
            .iter()
            .any(|node| node.label.starts_with("CUSTOMER: CUSTOMER"))
    );
    let fitted = mmdflux::render_diagram_fitted(
        CUSTOMER_ORDER,
        OutputFormat::Ascii,
        &config,
        &mmdflux::FitOptions::max_width(80),
    )
    .unwrap();
    assert!(fitted.output.contains("0..*"));
    assert!(fitted.output.is_ascii());
}

#[test]
fn er_simple_svg_labels_clear_marker_envelopes_in_all_directions() {
    for direction in ["TB", "BT", "LR", "RL"] {
        let input = format!("erDiagram\ndirection {direction}\nCUSTOMER ||--o{{ ORDER : places");
        let config = RenderConfig {
            geometry_level: GeometryLevel::Routed,
            ..RenderConfig::default()
        };
        let json = render_diagram(&input, OutputFormat::Mmds, &config).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        let edge = &value["edges"][0];
        let path = edge["path"].as_array().unwrap();
        let axis = usize::from(matches!(direction, "TB" | "BT"));
        let label_axis = if axis == 1 { "y" } else { "x" };
        let label_size = if axis == 1 { "height" } else { "width" };
        let start = path.first().unwrap()[axis].as_f64().unwrap();
        let end = path.last().unwrap()[axis].as_f64().unwrap();
        let label_min = edge["label_rect"][label_axis].as_f64().unwrap();
        let label_max = label_min + edge["label_rect"][label_size].as_f64().unwrap();
        // ER markers occupy at most 22px from the node face (20px viewBox + 2px pullback).
        assert!(label_min >= start.min(end) + 22.0, "{direction}: {edge}");
        assert!(label_max <= start.max(end) - 22.0, "{direction}: {edge}");
    }
}

#[test]
fn er_all_cardinality_tokens_roundtrip_through_both_geometry_levels() {
    let input = include_str!("fixtures/er/cardinalities.mmd");
    for level in [GeometryLevel::Layout, GeometryLevel::Routed] {
        let json = render_diagram(
            input,
            OutputFormat::Mmds,
            &RenderConfig {
                geometry_level: level,
                ..RenderConfig::default()
            },
        )
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../docs/mmds.schema.json")).unwrap();
        assert!(jsonschema::validator_for(&schema).unwrap().is_valid(&value));
        for (edge, token) in value["edges"].as_array().unwrap().iter().zip([
            "er_one",
            "er_zero_or_one",
            "er_one_or_more",
            "er_zero_or_more",
        ]) {
            assert_eq!(edge["arrow_start"], token);
            assert_eq!(edge["arrow_end"], token);
        }
        for format in [OutputFormat::Text, OutputFormat::Ascii, OutputFormat::Svg] {
            let output = render_diagram(&json, format, &RenderConfig::default()).unwrap();
            if format != OutputFormat::Svg {
                for cardinality in ["0..1", "1..*", "0..*"] {
                    assert!(output.contains(cardinality), "{output}");
                }
            } else {
                for token in [
                    "er-one",
                    "er-zero-or-one",
                    "er-one-or-more",
                    "er-zero-or-more",
                ] {
                    assert!(output.contains(&format!("marker-end=\"url(#{token})\"")));
                }
            }
        }
    }
    assert!(
        render_diagram(
            "erDiagram\nA ||--|| B : \"\"",
            OutputFormat::Svg,
            &RenderConfig::default()
        )
        .is_ok()
    );
}
