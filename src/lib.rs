use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

use rdocx::{BodyItemRef, CellItemRef, Document, ParagraphRef, TableRef, VMerge};
use sha2::{Digest, Sha256};

const PARSER_SCHEMA_VERSION: &str = "source-parser-docx-v0.1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeType {
    Document,
    Heading { level: u8 },
    Paragraph,
    ListItem { level: u32, ordered: Option<bool> },
    Table,
    Row,
    Cell,
    Caption,
    Footnote,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractionOrigin {
    DocxDeterministic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractionStatus {
    Valid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalMerge {
    Restart,
    Continue,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SourceLocation {
    Document,
    BodyParagraph {
        body_index: usize,
    },
    Table {
        body_index: usize,
    },
    TableRow {
        body_index: usize,
        row: usize,
    },
    TableCell {
        body_index: usize,
        row: usize,
        column: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalNode {
    pub source_node_id: String,
    pub node_type: NodeType,
    pub content: String,
    pub parent_id: Option<String>,
    pub document_order: usize,
    pub source_location: SourceLocation,
    pub extraction_origin: ExtractionOrigin,
    pub extraction_status: ExtractionStatus,
    pub grid_span: Option<u32>,
    pub vertical_merge: Option<VerticalMerge>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StableAnchor {
    pub anchor_id: String,
    pub document_revision_id: String,
    pub source_node_id: String,
    pub local_start: usize,
    pub local_end: usize,
    pub quote: String,
    pub quote_hash: String,
    pub source_location: SourceLocation,
    pub row: Option<usize>,
    pub column: Option<usize>,
}

#[derive(Debug)]
pub struct ParsedDocument {
    pub document_id: String,
    pub document_revision_id: String,
    pub nodes: Vec<CanonicalNode>,
    pub anchors: Vec<StableAnchor>,
    source_bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParserError {
    UnsupportedFormat,
    ExtractionFailed(String),
    StructureUnsupported(String),
    StructureInvalid(String),
    SourceUnresolved(String),
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedFormat => write!(f, "unsupported input format"),
            Self::ExtractionFailed(message) => write!(f, "DOCX extraction failed: {message}"),
            Self::StructureUnsupported(message) => {
                write!(f, "DOCX structure is not supported by this slice: {message}")
            }
            Self::StructureInvalid(message) => write!(f, "invalid extracted structure: {message}"),
            Self::SourceUnresolved(message) => write!(f, "source could not be resolved: {message}"),
        }
    }
}

impl Error for ParserError {}

impl ParsedDocument {
    pub fn resolve(&self, anchor: &StableAnchor) -> Result<String, ParserError> {
        if anchor.document_revision_id != self.document_revision_id {
            return Err(ParserError::SourceUnresolved(
                "anchor revision does not match parsed document".to_owned(),
            ));
        }

        let node = self
            .nodes
            .iter()
            .find(|node| node.source_node_id == anchor.source_node_id)
            .ok_or_else(|| {
                ParserError::SourceUnresolved("anchor source_node_id is unknown".to_owned())
            })?;

        if node.source_location != anchor.source_location {
            return Err(ParserError::SourceUnresolved(
                "anchor location does not match canonical node".to_owned(),
            ));
        }

        let document = Document::from_bytes(&self.source_bytes).map_err(|error| {
            ParserError::SourceUnresolved(format!("original DOCX could not be reopened: {error}"))
        })?;
        let source = resolve_location(&document, &anchor.source_location)?;

        let quote = source
            .get(anchor.local_start..anchor.local_end)
            .ok_or_else(|| {
                ParserError::SourceUnresolved("anchor byte range is outside source".to_owned())
            })?;

        if quote != anchor.quote || sha256_hex(quote.as_bytes()) != anchor.quote_hash {
            return Err(ParserError::SourceUnresolved(
                "resolved source does not match anchor quote/hash".to_owned(),
            ));
        }

        Ok(quote.to_owned())
    }
}

pub fn parse_docx(bytes: &[u8]) -> Result<ParsedDocument, ParserError> {
    if !looks_like_zip(bytes) {
        return Err(ParserError::UnsupportedFormat);
    }

    let document = Document::from_bytes(bytes)
        .map_err(|error| ParserError::ExtractionFailed(error.to_string()))?;

    let original_hash = sha256_hex(bytes);
    let document_id = format!("doc:{original_hash}");
    let document_revision_id =
        format!("rev:{}", sha256_hex(format!("{original_hash}:{PARSER_SCHEMA_VERSION}").as_bytes()));

    let mut nodes = Vec::new();
    let root_id = push_node(
        &mut nodes,
        &document_revision_id,
        "document",
        NodeType::Document,
        String::new(),
        None,
        SourceLocation::Document,
        None,
        None,
    );

    let mut current_heading = None::<String>;

    for (body_index, item) in document.body_items().enumerate() {
        match item {
            BodyItemRef::Paragraph(paragraph) => {
                let text = paragraph.text();
                if text.is_empty() {
                    continue;
                }

                let node_type = classify_paragraph(&document, &paragraph);
                let parent_id = match node_type {
                    NodeType::Heading { .. } => Some(root_id.clone()),
                    _ => current_heading.clone().or_else(|| Some(root_id.clone())),
                };
                let path = match node_type {
                    NodeType::Heading { .. } => format!("body/{body_index}/heading"),
                    NodeType::ListItem { .. } => format!("body/{body_index}/list"),
                    NodeType::Caption => format!("body/{body_index}/caption"),
                    _ => format!("body/{body_index}/paragraph"),
                };

                let node_id = push_node(
                    &mut nodes,
                    &document_revision_id,
                    &path,
                    node_type.clone(),
                    text,
                    parent_id,
                    SourceLocation::BodyParagraph { body_index },
                    None,
                    None,
                );

                if matches!(node_type, NodeType::Heading { .. }) {
                    current_heading = Some(node_id);
                }
            }
            BodyItemRef::Table(table) => {
                append_table(
                    &mut nodes,
                    &document_revision_id,
                    body_index,
                    &table,
                    current_heading.clone().or_else(|| Some(root_id.clone())),
                )?;
            }
            BodyItemRef::ContentControl(_) => {
                return Err(ParserError::StructureUnsupported(format!(
                    "body[{body_index}] is a content control"
                )));
            }
            BodyItemRef::UnsupportedXml(_) => {
                return Err(ParserError::StructureUnsupported(format!(
                    "body[{body_index}] contains preserved unsupported XML"
                )));
            }
            _ => {
                return Err(ParserError::StructureUnsupported(format!(
                    "body[{body_index}] uses an unhandled body item"
                )));
            }
        }
    }

    validate_structure(&nodes)?;

    let anchors = nodes
        .iter()
        .filter(|node| !node.content.is_empty())
        .map(|node| build_anchor(&document_revision_id, node))
        .collect();

    Ok(ParsedDocument {
        document_id,
        document_revision_id,
        nodes,
        anchors,
        source_bytes: bytes.to_vec(),
    })
}

fn classify_paragraph(document: &Document, paragraph: &ParagraphRef<'_>) -> NodeType {
    if let Some(level) = heading_level(paragraph.style_id()) {
        return NodeType::Heading { level };
    }

    if paragraph
        .style_id()
        .is_some_and(|style| style.eq_ignore_ascii_case("Caption"))
    {
        return NodeType::Caption;
    }

    if let Some((num_id, level)) = paragraph.numbering() {
        let ordered = document.numbering_is_bullet(num_id).map(|is_bullet| !is_bullet);
        return NodeType::ListItem { level, ordered };
    }

    if paragraph
        .style_id()
        .is_some_and(|style| style.to_ascii_lowercase().starts_with("list"))
    {
        return NodeType::ListItem {
            level: 0,
            ordered: None,
        };
    }

    NodeType::Paragraph
}

fn heading_level(style_id: Option<&str>) -> Option<u8> {
    let style = style_id?;
    let lower = style.to_ascii_lowercase();
    let suffix = lower.strip_prefix("heading")?;
    suffix.parse::<u8>().ok().filter(|level| *level > 0)
}

fn append_table(
    nodes: &mut Vec<CanonicalNode>,
    document_revision_id: &str,
    body_index: usize,
    table: &TableRef<'_>,
    parent_id: Option<String>,
) -> Result<(), ParserError> {
    if table.has_unsupported_content() {
        return Err(ParserError::StructureUnsupported(format!(
            "table at body[{body_index}] contains unsupported direct content"
        )));
    }

    let table_id = push_node(
        nodes,
        document_revision_id,
        &format!("body/{body_index}/table"),
        NodeType::Table,
        table_text(table)?,
        parent_id,
        SourceLocation::Table { body_index },
        None,
        None,
    );

    for row_index in 0..table.row_count() {
        let row = table.row(row_index).ok_or_else(|| {
            ParserError::StructureInvalid(format!(
                "table body[{body_index}] row {row_index} disappeared during traversal"
            ))
        })?;

        if row.has_unsupported_content() {
            return Err(ParserError::StructureUnsupported(format!(
                "table body[{body_index}] row {row_index} contains unsupported direct content"
            )));
        }

        let row_id = push_node(
            nodes,
            document_revision_id,
            &format!("body/{body_index}/table/row/{row_index}"),
            NodeType::Row,
            row_text(table, row_index)?,
            Some(table_id.clone()),
            SourceLocation::TableRow {
                body_index,
                row: row_index,
            },
            None,
            None,
        );

        for column_index in 0..row.cell_count() {
            let cell = row.cell(column_index).ok_or_else(|| {
                ParserError::StructureInvalid(format!(
                    "table body[{body_index}] row {row_index} cell {column_index} disappeared during traversal"
                ))
            })?;

            validate_cell_items(body_index, row_index, column_index, &cell)?;

            let vertical_merge = cell.v_merge().map(|merge| match merge {
                VMerge::Restart => VerticalMerge::Restart,
                VMerge::Continue => VerticalMerge::Continue,
            });

            push_node(
                nodes,
                document_revision_id,
                &format!(
                    "body/{body_index}/table/row/{row_index}/cell/{column_index}"
                ),
                NodeType::Cell,
                cell.text(),
                Some(row_id.clone()),
                SourceLocation::TableCell {
                    body_index,
                    row: row_index,
                    column: column_index,
                },
                cell.grid_span(),
                vertical_merge,
            );
        }
    }

    Ok(())
}

fn validate_cell_items(
    body_index: usize,
    row_index: usize,
    column_index: usize,
    cell: &rdocx::CellRef<'_>,
) -> Result<(), ParserError> {
    for item in cell.items() {
        match item {
            CellItemRef::Paragraph(_) => {}
            CellItemRef::Table(_) => {
                return Err(ParserError::StructureUnsupported(format!(
                    "nested table at body[{body_index}] row {row_index} cell {column_index}"
                )));
            }
            CellItemRef::ContentControl(_) => {
                return Err(ParserError::StructureUnsupported(format!(
                    "content control at body[{body_index}] row {row_index} cell {column_index}"
                )));
            }
            CellItemRef::UnsupportedXml(_) => {
                return Err(ParserError::StructureUnsupported(format!(
                    "unsupported XML at body[{body_index}] row {row_index} cell {column_index}"
                )));
            }
            _ => {
                return Err(ParserError::StructureUnsupported(format!(
                    "unhandled cell item at body[{body_index}] row {row_index} cell {column_index}"
                )));
            }
        }
    }
    Ok(())
}

fn push_node(
    nodes: &mut Vec<CanonicalNode>,
    document_revision_id: &str,
    path: &str,
    node_type: NodeType,
    content: String,
    parent_id: Option<String>,
    source_location: SourceLocation,
    grid_span: Option<u32>,
    vertical_merge: Option<VerticalMerge>,
) -> String {
    let source_node_id = format!(
        "node:{}",
        sha256_hex(format!("{document_revision_id}:{path}").as_bytes())
    );
    let document_order = nodes.len();

    nodes.push(CanonicalNode {
        source_node_id: source_node_id.clone(),
        node_type,
        content,
        parent_id,
        document_order,
        source_location,
        extraction_origin: ExtractionOrigin::DocxDeterministic,
        extraction_status: ExtractionStatus::Valid,
        grid_span,
        vertical_merge,
    });

    source_node_id
}

fn build_anchor(document_revision_id: &str, node: &CanonicalNode) -> StableAnchor {
    let local_start = 0;
    let local_end = node.content.len();
    let anchor_id = format!(
        "anchor:{}",
        sha256_hex(
            format!(
                "{document_revision_id}:{}:{local_start}:{local_end}",
                node.source_node_id
            )
            .as_bytes()
        )
    );
    let (row, column) = match node.source_location {
        SourceLocation::TableRow { row, .. } => (Some(row), None),
        SourceLocation::TableCell { row, column, .. } => (Some(row), Some(column)),
        _ => (None, None),
    };

    StableAnchor {
        anchor_id,
        document_revision_id: document_revision_id.to_owned(),
        source_node_id: node.source_node_id.clone(),
        local_start,
        local_end,
        quote: node.content.clone(),
        quote_hash: sha256_hex(node.content.as_bytes()),
        source_location: node.source_location.clone(),
        row,
        column,
    }
}

fn validate_structure(nodes: &[CanonicalNode]) -> Result<(), ParserError> {
    let Some(root) = nodes.first() else {
        return Err(ParserError::StructureInvalid(
            "canonical source has no document root".to_owned(),
        ));
    };
    if root.node_type != NodeType::Document || root.parent_id.is_some() {
        return Err(ParserError::StructureInvalid(
            "first canonical node is not the document root".to_owned(),
        ));
    }

    let mut seen = HashSet::new();
    for (expected_order, node) in nodes.iter().enumerate() {
        if node.document_order != expected_order {
            return Err(ParserError::StructureInvalid(format!(
                "node {} has non-deterministic document order",
                node.source_node_id
            )));
        }
        if !seen.insert(node.source_node_id.clone()) {
            return Err(ParserError::StructureInvalid(format!(
                "duplicate source_node_id {}",
                node.source_node_id
            )));
        }
        if let Some(parent_id) = &node.parent_id
            && !seen.contains(parent_id)
        {
            return Err(ParserError::StructureInvalid(format!(
                "node {} references a missing or forward parent {}",
                node.source_node_id, parent_id
            )));
        }
        if node.grid_span == Some(0) {
            return Err(ParserError::StructureInvalid(format!(
                "cell {} has zero grid span",
                node.source_node_id
            )));
        }
    }

    let by_id: HashMap<_, _> = nodes
        .iter()
        .map(|node| (node.source_node_id.as_str(), &node.node_type))
        .collect();

    for node in nodes {
        match node.node_type {
            NodeType::Row => {
                let parent = node.parent_id.as_deref().and_then(|id| by_id.get(id));
                if !matches!(parent, Some(NodeType::Table)) {
                    return Err(ParserError::StructureInvalid(format!(
                        "row {} is not parented by a table",
                        node.source_node_id
                    )));
                }
            }
            NodeType::Cell => {
                let parent = node.parent_id.as_deref().and_then(|id| by_id.get(id));
                if !matches!(parent, Some(NodeType::Row)) {
                    return Err(ParserError::StructureInvalid(format!(
                        "cell {} is not parented by a row",
                        node.source_node_id
                    )));
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn resolve_location(
    document: &Document,
    location: &SourceLocation,
) -> Result<String, ParserError> {
    match *location {
        SourceLocation::Document => Ok(document.text()),
        SourceLocation::BodyParagraph { body_index } => {
            match document.body_items().nth(body_index) {
                Some(BodyItemRef::Paragraph(paragraph)) => Ok(paragraph.text()),
                _ => Err(ParserError::SourceUnresolved(format!(
                    "body[{body_index}] is not the original paragraph"
                ))),
            }
        }
        SourceLocation::Table { body_index } => {
            let table = body_table(document, body_index)?;
            table_text(&table)
        }
        SourceLocation::TableRow { body_index, row } => {
            let table = body_table(document, body_index)?;
            row_text(&table, row)
        }
        SourceLocation::TableCell {
            body_index,
            row,
            column,
        } => {
            let table = body_table(document, body_index)?;
            table
                .cell(row, column)
                .map(|cell| cell.text())
                .ok_or_else(|| {
                    ParserError::SourceUnresolved(format!(
                        "body[{body_index}] table cell ({row}, {column}) is missing"
                    ))
                })
        }
    }
}

fn body_table(document: &Document, body_index: usize) -> Result<TableRef<'_>, ParserError> {
    match document.body_items().nth(body_index) {
        Some(BodyItemRef::Table(table)) => Ok(table),
        _ => Err(ParserError::SourceUnresolved(format!(
            "body[{body_index}] is not the original table"
        ))),
    }
}

fn table_text(table: &TableRef<'_>) -> Result<String, ParserError> {
    let mut rows = Vec::with_capacity(table.row_count());
    for row in 0..table.row_count() {
        rows.push(row_text(table, row)?);
    }
    Ok(rows.join("\n"))
}

fn row_text(table: &TableRef<'_>, row_index: usize) -> Result<String, ParserError> {
    let row = table.row(row_index).ok_or_else(|| {
        ParserError::SourceUnresolved(format!("table row {row_index} is missing"))
    })?;
    let mut cells = Vec::with_capacity(row.cell_count());
    for column in 0..row.cell_count() {
        let cell = row.cell(column).ok_or_else(|| {
            ParserError::SourceUnresolved(format!(
                "table row {row_index} cell {column} is missing"
            ))
        })?;
        cells.push(cell.text());
    }
    Ok(cells.join(" | "))
}

fn looks_like_zip(bytes: &[u8]) -> bool {
    matches!(
        bytes.get(..4),
        Some(b"PK\x03\x04") | Some(b"PK\x05\x06") | Some(b"PK\x07\x08")
    )
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}
