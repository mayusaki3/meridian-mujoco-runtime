//! Source-aware MJCF include expansion. Original files are retained byte-for-byte.
use crate::mjcf::{inspect_mjcf_standalone, InspectedMjcf, MjcfError};
use roxmltree::Document;
use std::{collections::{BTreeMap, HashSet}, fs, path::{Path, PathBuf}};

#[derive(Debug, Clone)]
pub struct ResolvedMjcf {
    pub inspected: InspectedMjcf,
    pub expanded_xml: String,
    /// Canonical source paths and exact original XML, including the entry file.
    pub sources: BTreeMap<PathBuf, String>,
}

#[derive(Debug)]
pub enum ResolveError {
    Io { path: PathBuf, message: String },
    InvalidXml { path: PathBuf, message: String },
    InvalidIncludeRoot(PathBuf),
    MissingIncludeFile(PathBuf),
    IncludeCycle(PathBuf),
    DuplicateInclude(PathBuf),
    InvalidIncludePath(PathBuf),
    Inspection(MjcfError),
}

fn load(path: &Path, sources: &mut BTreeMap<PathBuf, String>, active: &mut HashSet<PathBuf>) -> Result<String, ResolveError> {
    let canonical = fs::canonicalize(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ResolveError::MissingIncludeFile(path.to_owned())
        } else {
            ResolveError::Io { path: path.to_owned(), message: e.to_string() }
        }
    })?;
    if active.contains(&canonical) { return Err(ResolveError::IncludeCycle(canonical)); }
    if sources.contains_key(&canonical) { return Err(ResolveError::DuplicateInclude(canonical)); }
    let xml = fs::read_to_string(&canonical).map_err(|e| ResolveError::Io { path: canonical.clone(), message: e.to_string() })?;
    let doc = Document::parse(&xml).map_err(|e| ResolveError::InvalidXml { path: canonical.clone(), message: e.to_string() })?;
    let root = doc.root_element();
    if sources.is_empty() {
        if root.tag_name().name() != "mujoco" || root.tag_name().namespace().is_some() {
            return Err(ResolveError::InvalidIncludeRoot(canonical));
        }
    } else if root.tag_name().name() != "mujocoinclude" || root.tag_name().namespace().is_some() {
        return Err(ResolveError::InvalidIncludeRoot(canonical));
    }
    active.insert(canonical.clone());
    let mut expanded = xml.clone();
    let mut includes: Vec<_> = root.descendants().filter(|n| n.is_element() && n.tag_name().name() == "include" && n.tag_name().namespace().is_none()).collect();
    includes.reverse();
    for node in includes {
        let relative = node.attribute("file").filter(|v| !v.is_empty()).ok_or_else(|| ResolveError::InvalidIncludePath(canonical.clone()))?;
        let child_path = canonical.parent().unwrap_or(Path::new(".")).join(relative);
        let child = load(&child_path, sources, active)?;
        // Includes are replaced by the children of <mujocoinclude>.
        let child_doc = Document::parse(&child).map_err(|e| ResolveError::InvalidXml { path: child_path.clone(), message: e.to_string() })?;
        let wrapper = child_doc.root_element();
        let replacement = if wrapper.tag_name().name() == "mujocoinclude" {
            let children: Vec<_> = wrapper.children().filter(|n| n.is_element()).collect();
            match (children.first(), children.last()) {
                (Some(first), Some(last)) => child[first.range().start..last.range().end].to_owned(),
                _ => String::new(),
            }
        } else { child };
        expanded.replace_range(node.range(), &replacement);
    }
    active.remove(&canonical);
    sources.insert(canonical, xml);
    Ok(expanded)
}

/// Resolve all includes relative to their containing file, then validate the
/// fully expanded model. No files are written or modified.
pub fn inspect_mjcf_file(path: &Path) -> Result<ResolvedMjcf, ResolveError> {
    let mut sources = BTreeMap::new();
    let mut active = HashSet::new();
    let expanded_xml = load(path, &mut sources, &mut active)?;
    let inspected = inspect_mjcf_standalone(&expanded_xml).map_err(ResolveError::Inspection)?;
    Ok(ResolvedMjcf { inspected, expanded_xml, sources })
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!("mjcf-include-{}", Uuid::new_v4()));
            fs::create_dir_all(&p).unwrap();
            Self(p)
        }
        fn put(&self, name: &str, xml: &str) {
            let path = self.0.join(name);
            if let Some(parent) = path.parent() { fs::create_dir_all(parent).unwrap(); }
            fs::write(path, xml).unwrap();
        }
    }
    impl Drop for Fixture { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); } }

    #[test]
    fn nested_include_resolves_and_retains_sources() {
        let f = Fixture::new();
        f.put("main.xml", r#"<mujoco><include file="parts/first.xml"/><actuator><motor joint="hinge"/></actuator></mujoco>"#);
        f.put("parts/first.xml", r#"<mujocoinclude><include file="second.xml"/></mujocoinclude>"#);
        f.put("parts/second.xml", r#"<mujocoinclude><worldbody><body name="b"><joint name="hinge"/></body></worldbody></mujocoinclude>"#);
        let result = inspect_mjcf_file(&f.0.join("main.xml")).unwrap();
        assert_eq!(result.sources.len(), 3);
        assert_eq!(result.inspected.joints[0].name.as_deref(), Some("hinge"));
        assert!(result.sources.values().any(|s| s.contains("parts/first.xml")));
    }
    #[test]
    fn rejects_missing_cycle_and_duplicate() {
        let f = Fixture::new();
        f.put("main.xml", r#"<mujoco><include file="missing.xml"/></mujoco>"#);
        assert!(matches!(inspect_mjcf_file(&f.0.join("main.xml")), Err(ResolveError::MissingIncludeFile(_))));
        f.put("main.xml", r#"<mujoco><include file="a.xml"/></mujoco>"#);
        f.put("a.xml", r#"<mujocoinclude><include file="b.xml"/></mujocoinclude>"#);
        f.put("b.xml", r#"<mujocoinclude><include file="a.xml"/></mujocoinclude>"#);
        assert!(matches!(inspect_mjcf_file(&f.0.join("main.xml")), Err(ResolveError::IncludeCycle(_))));
        f.put("main.xml", r#"<mujoco><include file="a.xml"/><include file="a.xml"/></mujoco>"#);
        f.put("a.xml", r#"<mujocoinclude><worldbody/></mujocoinclude>"#);
        assert!(matches!(inspect_mjcf_file(&f.0.join("main.xml")), Err(ResolveError::DuplicateInclude(_))));
    }
    #[test]
    fn validates_cross_file_references_and_duplicate_names() {
        let f = Fixture::new();
        f.put("main.xml", r#"<mujoco><include file="part.xml"/><actuator><motor joint="absent"/></actuator></mujoco>"#);
        f.put("part.xml", r#"<mujocoinclude><worldbody><body name="b"><joint name="j"/></body></worldbody></mujocoinclude>"#);
        assert!(matches!(inspect_mjcf_file(&f.0.join("main.xml")), Err(ResolveError::Inspection(MjcfError::UnknownActuatorTarget { .. }))));
        f.put("main.xml", r#"<mujoco><include file="part.xml"/><worldbody><body name="b"/></worldbody></mujoco>"#);
        assert!(matches!(inspect_mjcf_file(&f.0.join("main.xml")), Err(ResolveError::Inspection(MjcfError::DuplicateBody(_)))));
    }
}
