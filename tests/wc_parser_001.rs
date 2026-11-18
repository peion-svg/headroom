use lightweight_source_parser::{
    NodeType, OffsetUnit, ParserError, SourceLocation, VerticalMerge, parse_docx,
};
use sha2::{Digest, Sha256};

const FIXTURE: &[u8] = include_bytes!("fixtures/wc_parser_001.docx");

#[test]
fn wc_parser_001_preserves_structure_identity_and_roundtrip() {
    let parsed = parse_docx(FIXTURE).expect("fixture must parse");
    let reparsed = parse_docx(FIXTURE).expect("same fixture must parse again");

    let ids: Vec<_> = parsed
        .nodes
        .iter()
        .map(|node| node.source_node_id.as_str())
        .collect();
    let reparsed_ids: Vec<_> = reparsed
        .nodes
        .iter()
        .map(|node| node.source_node_id.as_str())
        .collect();
    assert_eq!(ids, reparsed_ids, "source identity must be deterministic");

    let anchor_ids: Vec<_> = parsed
        .anchors
        .iter()
        .map(|anchor| anchor.anchor_id.as_str())
        .collect();
    let reparsed_anchor_ids: Vec<_> = reparsed
        .anchors
        .iter()
        .map(|anchor| anchor.anchor_id.as_str())
        .collect();
    assert_eq!(
        anchor_ids, reparsed_anchor_ids,
        "stable anchor identity must be deterministic"
    );

    for (expected_order, node) in parsed.nodes.iter().enumerate() {
        assert_eq!(node.document_order, expected_order);
    }

    let heading = parsed
        .nodes
        .iter()
        .find(|node| node.content == "Fixture Heading")
        .expect("heading node");
    assert_eq!(heading.node_type, NodeType::Heading { level: 1 });

    let paragraph = parsed
        .nodes
        .iter()
        .find(|node| node.content == "Deterministic paragraph.")
        .expect("paragraph node");
    assert_eq!(paragraph.node_type, NodeType::Paragraph);
    assert_eq!(paragraph.parent_id, heading.parent_id);

    let list_item = parsed
        .nodes
        .iter()
        .find(|node| node.content == "Bullet item")
        .expect("list node");
    assert_eq!(
        list_item.node_type,
        NodeType::ListItem {
            level: 0,
            ordered: Some(false),
        }
    );
    assert_eq!(list_item.parent_id, heading.parent_id);

    let duplicate_nodes: Vec<_> = parsed
        .nodes
        .iter()
        .filter(|node| node.content == "중복 문장 Ω😀")
        .collect();
    assert_eq!(duplicate_nodes.len(), 2);
    assert_ne!(duplicate_nodes[0].source_node_id, duplicate_nodes[1].source_node_id);
    assert_ne!(duplicate_nodes[0].source_location, duplicate_nodes[1].source_location);

    let duplicate_anchors: Vec<_> = parsed
        .anchors
        .iter()
        .filter(|anchor| anchor.quote == "중복 문장 Ω😀")
        .collect();
    assert_eq!(duplicate_anchors.len(), 2);
    assert_ne!(duplicate_anchors[0].anchor_id, duplicate_anchors[1].anchor_id);
    for anchor in duplicate_anchors {
        assert_eq!(anchor.offset_unit, OffsetUnit::Utf8Byte);
        assert_eq!(anchor.local_start, 0);
        assert_eq!(anchor.local_end, anchor.quote.len());
        assert!(anchor.local_end > anchor.quote.chars().count());
        assert_eq!(parsed.resolve(anchor).unwrap(), anchor.quote);
    }

    let empty_paragraph = parsed
        .nodes
        .iter()
        .find(|node| node.node_type == NodeType::Paragraph && node.content.is_empty())
        .expect("empty paragraph must be preserved structurally");
    assert!(!parsed
        .anchors
        .iter()
        .any(|anchor| anchor.source_node_id == empty_paragraph.source_node_id));

    let table = parsed
        .nodes
        .iter()
        .find(|node| node.node_type == NodeType::Table)
        .expect("table node");
    assert_eq!(table.parent_id, heading.parent_id);

    let rows: Vec<_> = parsed
        .nodes
        .iter()
        .filter(|node| node.node_type == NodeType::Row)
        .collect();
    assert_eq!(rows.len(), 3);
    assert!(rows.iter().all(|row| row.parent_id.as_ref() == Some(&table.source_node_id)));

    let merged = parsed
        .nodes
        .iter()
        .find(|node| node.content == "Merged Header")
        .expect("merged header cell");
    assert_eq!(merged.node_type, NodeType::Cell);
    assert_eq!(merged.grid_span, Some(2));
    assert_eq!(merged.vertical_merge, None);
    assert_eq!(merged.logical_column, Some(0));

    let merged_anchor = parsed
        .anchors
        .iter()
        .find(|anchor| anchor.source_node_id == merged.source_node_id)
        .expect("merged cell anchor");
    assert_eq!(merged_anchor.row, Some(0));
    assert_eq!(merged_anchor.column, Some(0));
    assert_eq!(merged_anchor.logical_column, Some(0));

    let offset_cell = parsed
        .nodes
        .iter()
        .find(|node| node.content == "C")
        .expect("gridBefore cell");
    assert_eq!(offset_cell.logical_column, Some(1));
    assert!(matches!(
        offset_cell.source_location,
        SourceLocation::TableCell { column: 0, .. }
    ));
    let offset_anchor = parsed
        .anchors
        .iter()
        .find(|anchor| anchor.source_node_id == offset_cell.source_node_id)
        .expect("gridBefore cell anchor");
    assert_eq!(offset_anchor.column, Some(0));
    assert_eq!(offset_anchor.logical_column, Some(1));

    let cells: Vec<_> = parsed
        .nodes
        .iter()
        .filter(|node| node.node_type == NodeType::Cell)
        .collect();
    assert_eq!(cells.len(), 4);
    assert!(cells.iter().all(|cell| {
        let parent = parsed
            .nodes
            .iter()
            .find(|node| Some(&node.source_node_id) == cell.parent_id.as_ref())
            .expect("cell parent");
        parent.node_type == NodeType::Row
    }));

    let caption = parsed
        .nodes
        .iter()
        .find(|node| node.content == "Table 1. Fixture table")
        .expect("caption node");
    assert_eq!(caption.node_type, NodeType::Caption);

    assert!(parsed
        .nodes
        .iter()
        .filter(|node| matches!(node.node_type, NodeType::Table | NodeType::Row))
        .all(|node| node.content.is_empty()));
    assert!(parsed.anchors.iter().all(|anchor| {
        parsed
            .nodes
            .iter()
            .find(|node| node.source_node_id == anchor.source_node_id)
            .is_some_and(|node| !matches!(node.node_type, NodeType::Table | NodeType::Row))
    }));

    assert!(!parsed.anchors.is_empty());
    for anchor in &parsed.anchors {
        assert_eq!(anchor.offset_unit, OffsetUnit::Utf8Byte);
        assert_eq!(
            parsed.resolve(anchor).expect("anchor must resolve"),
            anchor.quote
        );
        assert_eq!(anchor.quote_hash, sha256(anchor.quote.as_bytes()));
    }
}

#[test]
fn wc_parser_001_separates_unsupported_and_malformed_input() {
    assert!(matches!(
        parse_docx(b"not a DOCX"),
        Err(ParserError::UnsupportedFormat)
    ));

    assert!(matches!(
        parse_docx(b"PK\x03\x04broken"),
        Err(ParserError::ExtractionFailed(_))
    ));
}

#[test]
fn wc_parser_001_rejects_tampered_anchor_instead_of_silently_resolving() {
    let parsed = parse_docx(FIXTURE).expect("fixture must parse");
    let mut anchor = parsed.anchors[0].clone();
    anchor.quote.push('x');

    assert!(matches!(
        parsed.resolve(&anchor),
        Err(ParserError::SourceUnresolved(_))
    ));
}

#[test]
fn vertical_merge_enum_is_explicit_even_when_fixture_uses_grid_span() {
    let _ = VerticalMerge::Restart;
    let _ = VerticalMerge::Continue;
}

fn sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}
