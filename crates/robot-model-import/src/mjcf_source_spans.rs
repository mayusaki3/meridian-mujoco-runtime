//! Exact byte-level source map for the existing MJCF include expansion.
//! This is an opt-in read-only projection; no source XML is rewritten.
use crate::mjcf_portable::PortableMjcfBundle;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceByte {
    pub path: PathBuf,
    pub offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpanError {
    MissingSource(PathBuf),
    InvalidXml(PathBuf),
    InvalidInclude(PathBuf),
    IncludeCycle(PathBuf),
    DuplicateInclude(PathBuf),
    ExpandedMismatch,
}

struct MappedXml {
    text: String,
    bytes: Vec<SourceByte>,
}
fn expand(
    path: &Path,
    sources: &BTreeMap<PathBuf, String>,
    active: &mut HashSet<PathBuf>,
    visited: &mut HashSet<PathBuf>,
) -> Result<MappedXml, SpanError> {
    if !active.insert(path.to_owned()) {
        return Err(SpanError::IncludeCycle(path.to_owned()));
    }
    if !visited.insert(path.to_owned()) {
        return Err(SpanError::DuplicateInclude(path.to_owned()));
    }
    let xml = sources
        .get(path)
        .ok_or_else(|| SpanError::MissingSource(path.to_owned()))?;
    let doc =
        roxmltree::Document::parse(xml).map_err(|_| SpanError::InvalidXml(path.to_owned()))?;
    let root = doc.root_element();
    let expected = if active.len() == 1 {
        "mujoco"
    } else {
        "mujocoinclude"
    };
    if root.tag_name().name() != expected || root.tag_name().namespace().is_some() {
        return Err(SpanError::InvalidInclude(path.to_owned()));
    }
    let mut mapped = MappedXml {
        text: xml.clone(),
        bytes: (0..xml.len())
            .map(|offset| SourceByte {
                path: path.to_owned(),
                offset,
            })
            .collect(),
    };
    let mut includes: Vec<_> = root
        .descendants()
        .filter(|n| {
            n.is_element() && n.tag_name().namespace().is_none() && n.has_tag_name("include")
        })
        .collect();
    includes.reverse();
    for node in includes {
        let relative = node
            .attribute("file")
            .filter(|v| !v.is_empty())
            .ok_or_else(|| SpanError::InvalidInclude(path.to_owned()))?;
        let parent = path.parent().unwrap_or(Path::new(""));
        let child_path = normalize(&parent.join(relative))?;
        let child = expand(&child_path, sources, active, visited)?;
        let child_doc = roxmltree::Document::parse(&child.text)
            .map_err(|_| SpanError::InvalidXml(child_path.clone()))?;
        let wrapper = child_doc.root_element();
        let children: Vec<_> = wrapper.children().filter(|n| n.is_element()).collect();
        let range = match (children.first(), children.last()) {
            (Some(first), Some(last)) => first.range().start..last.range().end,
            _ => 0..0,
        };
        mapped
            .text
            .replace_range(node.range(), &child.text[range.clone()]);
        mapped
            .bytes
            .splice(node.range(), child.bytes[range].iter().cloned());
    }
    active.remove(path);
    Ok(mapped)
}
fn normalize(path: &Path) -> Result<PathBuf, SpanError> {
    let mut parts = Vec::new();
    for c in path.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::Normal(p) => parts.push(p.to_os_string()),
            std::path::Component::ParentDir => {
                if parts.pop().is_none() {
                    return Err(SpanError::InvalidInclude(path.to_owned()));
                }
            }
            _ => return Err(SpanError::InvalidInclude(path.to_owned())),
        }
    }
    Ok(parts.iter().collect())
}

/// Expanded byte offsets -> original file and original byte offsets.
/// Rejects snapshots whose re-expansion differs from the imported model.
pub fn source_byte_map(bundle: &PortableMjcfBundle) -> Result<Vec<SourceByte>, SpanError> {
    let result = expand(
        &bundle.entry_path,
        &bundle.sources,
        &mut HashSet::new(),
        &mut HashSet::new(),
    )?;
    if result.text != bundle.model.original_xml {
        return Err(SpanError::ExpandedMismatch);
    }
    Ok(result.bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{mjcf_model_bundle::from_mjcf_file, mjcf_portable::PortableMjcfBundle};
    use std::fs;
    use uuid::Uuid;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!("mjcf-spans-{}", Uuid::new_v4()));
            fs::create_dir_all(&p).unwrap();
            Self(p)
        }
        fn put(&self, name: &str, xml: &str) {
            let p = self.0.join(name);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, xml).unwrap();
        }
        fn portable(&self) -> PortableMjcfBundle {
            PortableMjcfBundle::from_bundle(
                &from_mjcf_file(&self.0.join("main.xml")).unwrap(),
                &self.0,
            )
            .unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn maps_nested_includes_and_unnamed_elements_to_original_bytes() {
        let f = Fixture::new();
        f.put(
            "main.xml",
            r#"<mujoco><include file="sub/one.xml"/><actuator><motor/></actuator></mujoco>"#,
        );
        f.put(
            "sub/one.xml",
            r#"<mujocoinclude><include file="two.xml"/></mujocoinclude>"#,
        );
        f.put(
            "sub/two.xml",
            r#"<mujocoinclude><worldbody><body><joint/></body></worldbody></mujocoinclude>"#,
        );
        let p = f.portable();
        let map = source_byte_map(&p).unwrap();
        assert_eq!(map.len(), p.model.original_xml.len());
        let joint = p.model.original_xml.find("<joint").unwrap();
        let origin = &map[joint];
        assert_eq!(origin.path, Path::new("sub/two.xml"));
        assert_eq!(
            origin.offset,
            p.sources[Path::new("sub/two.xml")].find("<joint").unwrap()
        );
        let motor = p.model.original_xml.find("<motor").unwrap();
        assert_eq!(map[motor].path, Path::new("main.xml"));
    }

    #[test]
    fn detects_modified_snapshot() {
        let f = Fixture::new();
        f.put("main.xml", "<mujoco><worldbody/></mujoco>");
        let mut p = f.portable();
        p.sources
            .insert(PathBuf::from("main.xml"), "<mujoco/>".into());
        assert_eq!(source_byte_map(&p), Err(SpanError::ExpandedMismatch));
    }
}
