use super::*;

#[test]
fn page_ports_parse_format_round_trip() {
    let src = r##"zenith version=1 {
  project id="proj.ports" name="Ports"
  tokens format="zenith-token-v1" {
  }
  styles {
  }
  document id="doc.ports" title="Ports" {
    page id="page.ports" w=(px)640 h=(px)360 {
      ports {
        port node="agent" id="memory.vector" anchor="38/60"
        port node="store" id="in" anchor="4/16"
      }
      rect id="agent" x=(px)40 y=(px)40 w=(px)120 h=(px)80
      rect id="store" x=(px)300 y=(px)60 w=(px)120 h=(px)80
      connector id="c1" from="agent#memory.vector" to="store#in"
    }
  }
}
"##;

    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse must succeed");
    let page = &doc.body.pages[0];
    assert_eq!(page.ports.len(), 2);
    assert_eq!(page.ports[0].node, "agent");
    assert_eq!(page.ports[0].id, "memory.vector");
    assert_eq!(page.ports[0].anchor, "38/60");

    let formatted = format_document(&doc).expect("format must succeed");
    let formatted_str = String::from_utf8(formatted.clone()).expect("formatted must be utf8");
    assert!(
        formatted_str.contains("ports {\n        port node=\"agent\" id=\"memory.vector\" anchor=\"38/60\"\n        port node=\"store\" id=\"in\" anchor=\"4/16\"\n      }"),
        "formatter must emit canonical page ports block; got:\n{formatted_str}"
    );

    let reparsed = adapter
        .parse(&formatted)
        .expect("re-parse after format must succeed");
    assert_eq!(reparsed.body.pages[0].ports.len(), 2);
    assert_eq!(reparsed.body.pages[0].ports[0].node, page.ports[0].node);
    assert_eq!(reparsed.body.pages[0].ports[0].id, page.ports[0].id);
    assert_eq!(reparsed.body.pages[0].ports[0].anchor, page.ports[0].anchor);
}

/// **Image clip round-trip**: `clip="rounded"` + `clip-radius=(token)"..."`
/// must parse onto the `ImageNode`, be re-emitted by the formatter, and survive
/// a format → re-parse round-trip.
#[test]
fn test_image_clip_parse_format_round_trip() {
    let src = r##"zenith version=1 {
  project id="proj.iclip" name="IClip"
  assets {
    asset id="asset.pfp" kind="image" src="assets/pfp.png"
  }
  tokens format="zenith-token-v1" {
    token id="size.radius.avatar" type="dimension" value=(px)24
  }
  styles {
  }
  document id="doc.iclip" title="IClip" {
    page id="page.iclip" w=(px)400 h=(px)300 {
      image id="av" asset="asset.pfp" x=(px)0 y=(px)0 w=(px)100 h=(px)100 fit="cover" clip="rounded" clip-radius=(token)"size.radius.avatar"
    }
  }
}
"##;

    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse must succeed");
    let image_node = match &doc.body.pages[0].children[0] {
        Node::Image(i) => i,
        other => panic!("expected Image node, got {other:?}"),
    };
    assert_eq!(image_node.clip.as_deref(), Some("rounded"));
    use zenith_core::PropertyValue;
    assert_eq!(
        image_node.clip_radius,
        Some(PropertyValue::TokenRef("size.radius.avatar".to_owned())),
        "clip-radius must parse as a token ref"
    );

    let formatted = format_document(&doc).expect("format must succeed");
    let formatted_str = String::from_utf8(formatted).expect("formatted must be utf8");
    assert!(
        formatted_str.contains("clip=\"rounded\""),
        "formatter must emit clip=\"rounded\"; got:\n{formatted_str}"
    );
    assert!(
        formatted_str.contains("clip-radius=(token)\"size.radius.avatar\""),
        "formatter must emit clip-radius token; got:\n{formatted_str}"
    );

    let doc2 = adapter
        .parse(formatted_str.as_bytes())
        .expect("re-parse after format");
    let image2 = match &doc2.body.pages[0].children[0] {
        Node::Image(i) => i,
        other => panic!("expected Image node on re-parse, got {other:?}"),
    };
    assert_eq!(image2.clip.as_deref(), Some("rounded"));
    assert_eq!(
        image2.clip_radius,
        Some(PropertyValue::TokenRef("size.radius.avatar".to_owned())),
        "clip-radius must survive a format → re-parse round-trip"
    );
}

#[test]
fn light_node_parse_format_round_trip() {
    let src = r##"zenith version=1 {
  project id="proj.light" name="Light"
  tokens format="zenith-token-v1" {
    token id="color.glow" type="color" value="#7cc7ff"
    token id="size.glow" type="dimension" value=(px)420
  }
  styles {
  }
  document id="doc.light" title="Light" {
    page id="page.light" w=(px)1080 h=(px)1080 {
      light id="bg.glow" kind="ambient" x=(%)85 y=(%)12 radius=(token)"size.glow" color=(token)"color.glow" opacity=0.35
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse must succeed");
    let light = match &doc.body.pages[0].children[0] {
        Node::Light(l) => l,
        other => panic!("expected light node, got {other:?}"),
    };
    assert_eq!(light.id, "bg.glow");
    assert_eq!(light.kind.as_deref(), Some("ambient"));
    assert_eq!(light.opacity, Some(0.35));

    let formatted = format_document(&doc).expect("format must succeed");
    let formatted_str = String::from_utf8(formatted.clone()).expect("formatted must be utf8");
    assert!(
        formatted_str.contains(
            "light id=\"bg.glow\" kind=\"ambient\" x=(%)85 y=(%)12 radius=(token)\"size.glow\" color=(token)\"color.glow\" opacity=0.35"
        ),
        "formatted light line missing canonical attrs; got:\n{formatted_str}"
    );
    let reparsed = adapter
        .parse(&formatted)
        .expect("re-parse after format must succeed");
    assert!(
        matches!(&reparsed.body.pages[0].children[0], Node::Light(l) if l.id == "bg.glow"),
        "light must survive format round-trip"
    );
}

#[test]
fn mesh_node_parse_format_round_trip() {
    let src = r##"zenith version=1 {
  project id="proj.mesh" name="Mesh"
  tokens format="zenith-token-v1" {
    token id="color.grid" type="color" value="#203040"
    token id="stroke.hairline" type="dimension" value=(px)1
  }
  styles {
  }
  document id="doc.mesh" title="Mesh" {
    page id="page.mesh" w=(px)1920 h=(px)1080 {
      mesh id="bg.mesh" kind="perspective" x=(px)0 y=(px)0 w=(px)1920 h=(px)1080 rows=7 columns=8 vanishing-x=(px)1260 vanishing-y=(px)-420 extend=(px)160 stroke=(token)"color.grid" stroke-width=(token)"stroke.hairline" opacity=0.34
    }
  }
}

"##;
    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse must succeed");
    let mesh = match &doc.body.pages[0].children[0] {
        Node::Mesh(m) => m,
        other => panic!("expected mesh node, got {other:?}"),
    };
    assert_eq!(mesh.id, "bg.mesh");
    assert_eq!(mesh.kind.as_deref(), Some("perspective"));
    assert_eq!(mesh.rows, Some(7));
    assert_eq!(mesh.columns, Some(8));

    let formatted = format_document(&doc).expect("format must succeed");
    let formatted_str = String::from_utf8(formatted.clone()).expect("formatted must be utf8");
    assert!(
        formatted_str.contains(
            "mesh id=\"bg.mesh\" kind=\"perspective\" x=(px)0 y=(px)0 w=(px)1920 h=(px)1080 rows=7 columns=8 vanishing-x=(px)1260 vanishing-y=(px)-420 extend=(px)160 stroke=(token)\"color.grid\" stroke-width=(token)\"stroke.hairline\" opacity=0.34"
        ),
        "formatted mesh line missing canonical attrs; got:\n{formatted_str}"
    );
    let reparsed = adapter
        .parse(&formatted)
        .expect("re-parse after format must succeed");
    assert!(
        matches!(&reparsed.body.pages[0].children[0], Node::Mesh(m) if m.id == "bg.mesh"),
        "mesh must survive format round-trip"
    );
}

#[test]
fn path_node_parse_format_round_trip() {
    let src = r##"zenith version=1 {
  project id="proj.path" name="Path"
  tokens format="zenith-token-v1" {
    token id="color.brand" type="color" value="#112233"
    token id="color.ink" type="color" value="#000000"
    token id="size.stroke" type="dimension" value=(px)2
  }
  styles {
  }
  document id="doc.path" title="Path" {
    page id="page.path" w=(px)400 h=(px)300 {
      path id="logo.mark" closed=#true fill=(token)"color.brand" stroke=(token)"color.ink" stroke-width=(token)"size.stroke" stroke-alignment="center" stroke-linejoin="round" stroke-linecap="round" stroke-miter-limit=7 fill-rule="evenodd" {
        anchor x=(px)0 y=(px)0 out-x=(px)20 out-y=(px)0
        anchor x=(px)80 y=(px)0 kind="smooth" in-x=(px)60 in-y=(px)0 out-x=(px)100 out-y=(px)40
        anchor x=(px)80 y=(px)80 kind="symmetric" in-x=(px)100 in-y=(px)40
      }
    }
  }
}
"##;

    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse must succeed");
    let path = match &doc.body.pages[0].children[0] {
        Node::Path(p) => p,
        other => panic!("expected path node, got {other:?}"),
    };
    assert_eq!(path.id, "logo.mark");
    assert_eq!(path.closed, Some(true));
    assert_eq!(path.stroke_linejoin.as_deref(), Some("round"));
    assert_eq!(path.stroke_linecap.as_deref(), Some("round"));
    assert_eq!(path.stroke_miter_limit, Some(7.0));
    assert_eq!(path.anchors.len(), 3);
    assert_eq!(path.anchors[1].kind, Some(AnchorKind::Smooth));
    assert_eq!(path.anchors[2].kind, Some(AnchorKind::Symmetric));
    assert_eq!(path.anchors[1].in_x, Some(px(60.0)));
    assert_eq!(path.anchors[1].out_y, Some(px(40.0)));

    let formatted = format_document(&doc).expect("format must succeed");
    let formatted_str = String::from_utf8(formatted.clone()).expect("formatted must be utf8");
    assert!(
        formatted_str.contains(
            "path id=\"logo.mark\" closed=#true fill=(token)\"color.brand\" stroke=(token)\"color.ink\" stroke-width=(token)\"size.stroke\" stroke-alignment=\"center\" stroke-linejoin=\"round\" stroke-linecap=\"round\" stroke-miter-limit=7 fill-rule=\"evenodd\""
        ),
        "formatted path line missing canonical attrs; got:\n{formatted_str}"
    );
    assert!(
        formatted_str
            .contains("anchor x=(px)80 y=(px)0 kind=\"smooth\" in-x=(px)60 in-y=(px)0 out-x=(px)100 out-y=(px)40"),
        "formatted anchor line missing handles; got:\n{formatted_str}"
    );

    let reparsed = adapter
        .parse(&formatted)
        .expect("re-parse after format must succeed");
    assert!(
        matches!(&reparsed.body.pages[0].children[0], Node::Path(p) if p.anchors.len() == 3 && p.anchors[1].kind == Some(AnchorKind::Smooth)),
        "path must survive format round-trip"
    );
}

#[test]
fn path_subpaths_parse_format_round_trip() {
    let src = r##"zenith version=1 {
  project id="proj.path" name="Path"
  tokens format="zenith-token-v1" {
  }
  styles {
  }
  document id="doc.path" title="Path" {
    page id="page.path" w=(px)400 h=(px)300 {
      path id="glyph.o" fill-rule="evenodd" {
        subpath closed=#true {
          anchor x=(px)0 y=(px)0
          anchor x=(px)80 y=(px)0
          anchor x=(px)80 y=(px)80
        }
        subpath closed=#true {
          anchor x=(px)20 y=(px)20
          anchor x=(px)60 y=(px)20
          anchor x=(px)60 y=(px)60
        }
      }
    }
  }
}
"##;

    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse must succeed");
    let path = match &doc.body.pages[0].children[0] {
        Node::Path(path) => path,
        other => panic!("expected path node, got {other:?}"),
    };
    assert!(path.anchors.is_empty());
    assert_eq!(path.subpaths.len(), 2);
    assert_eq!(path.subpaths[0].closed, Some(true));
    assert_eq!(path.subpaths[1].anchors.len(), 3);

    let formatted = format_document(&doc).expect("format must succeed");
    let formatted_str = String::from_utf8(formatted.clone()).expect("formatted must be utf8");
    assert!(
        formatted_str.contains("subpath closed=#true"),
        "formatted path missing subpath block; got:\n{formatted_str}"
    );
    assert!(
        formatted_str.contains("anchor x=(px)20 y=(px)20"),
        "formatted subpath missing nested anchors; got:\n{formatted_str}"
    );

    let reparsed = adapter
        .parse(&formatted)
        .expect("formatted subpath path must reparse");
    let reparsed_path = match &reparsed.body.pages[0].children[0] {
        Node::Path(path) => path,
        other => panic!("expected path node after reparse, got {other:?}"),
    };
    assert_eq!(reparsed_path.subpaths.len(), 2);
    assert!(reparsed_path.anchors.is_empty());
}
