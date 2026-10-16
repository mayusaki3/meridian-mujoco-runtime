//! Project-local logical Robot catalog. No FW UI or Registry ownership here.
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::{collections::HashSet, io};

const SCHEMA_VERSION: u32 = 1;
static SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

// Use OS randomness for independent IDs without adding a runtime dependency.
fn new_robot_id() -> io::Result<String> {
    use std::io::Read;
    let mut bytes = [0u8; 16];
    let mut file = std::fs::File::open(if cfg!(windows) {
        r"\\.\CNG"
    } else {
        "/dev/urandom"
    });
    if let Ok(ref mut file) = file {
        if file.read_exact(&mut bytes).is_ok() {
            bytes[6] = (bytes[6] & 0x0f) | 0x40;
            bytes[8] = (bytes[8] & 0x3f) | 0x80;
            return Ok(format_uuid(bytes));
        }
    }
    // Fallback uses process/time/counter, still checked for project-local uniqueness.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| invalid(e.to_string()))?
        .as_nanos();
    let n = SEQUENCE.fetch_add(1, Ordering::Relaxed) as u128;
    let mut v = (now ^ ((std::process::id() as u128) << 64) ^ n).to_be_bytes();
    v[6] = (v[6] & 0x0f) | 0x40;
    v[8] = (v[8] & 0x3f) | 0x80;
    Ok(format_uuid(v))
}

fn format_uuid(b: [u8; 16]) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0],
        b[1],
        b[2],
        b[3],
        b[4],
        b[5],
        b[6],
        b[7],
        b[8],
        b[9],
        b[10],
        b[11],
        b[12],
        b[13],
        b[14],
        b[15]
    )
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RobotOrigin {
    pub source_namespace: String,
    pub source_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RobotRepresentation {
    pub registry_resource_id: String,
    pub format: String,
    pub role: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RobotRecord {
    pub robot_id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<RobotOrigin>,
    #[serde(default)]
    pub representations: Vec<RobotRepresentation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RobotCatalog {
    schema_version: u32,
    robots: Vec<RobotRecord>,
}

impl Default for RobotCatalog {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            robots: Vec::new(),
        }
    }
}

impl RobotCatalog {
    pub fn robots(&self) -> &[RobotRecord] {
        &self.robots
    }

    pub fn create(&mut self, name: &str) -> io::Result<String> {
        self.create_with_origin(name, None)
    }

    pub fn create_with_origin(
        &mut self,
        name: &str,
        origin: Option<RobotOrigin>,
    ) -> io::Result<String> {
        if name.trim().is_empty() {
            return Err(invalid("Robot name is empty"));
        }
        if let Some(o) = &origin {
            if o.source_namespace.trim().is_empty() || o.source_id.trim().is_empty() {
                return Err(invalid("Robot origin must have nonempty namespace and id"));
            }
        }
        for _ in 0..16 {
            let id = new_robot_id()?;
            if !self.robots.iter().any(|r| r.robot_id == id) {
                self.robots.push(RobotRecord {
                    robot_id: id.clone(),
                    name: name.into(),
                    origin,
                    representations: Vec::new(),
                });
                return Ok(id);
            }
        }
        Err(invalid("Unable to allocate unique Robot ID"))
    }

    pub fn rename(&mut self, id: &str, name: &str) -> io::Result<()> {
        if name.trim().is_empty() {
            return Err(invalid("Robot name is empty"));
        }
        let robot = self
            .robots
            .iter_mut()
            .find(|r| r.robot_id == id)
            .ok_or_else(|| invalid("Robot not found"))?;
        robot.name = name.into();
        Ok(())
    }

    pub fn duplicate(&mut self, id: &str, name: &str) -> io::Result<String> {
        let source = self
            .robots
            .iter()
            .find(|r| r.robot_id == id)
            .ok_or_else(|| invalid("Robot not found"))?
            .clone();
        let new_id = self.create_with_origin(name, source.origin)?;
        let robot = self
            .robots
            .iter_mut()
            .find(|r| r.robot_id == new_id)
            .unwrap();
        robot.representations = source.representations;
        Ok(new_id)
    }

    pub fn add_representation(
        &mut self,
        id: &str,
        representation: RobotRepresentation,
    ) -> io::Result<()> {
        if representation.registry_resource_id.trim().is_empty()
            || representation.format.trim().is_empty()
            || representation.role.trim().is_empty()
        {
            return Err(invalid("Invalid representation"));
        }
        let robot = self
            .robots
            .iter_mut()
            .find(|r| r.robot_id == id)
            .ok_or_else(|| invalid("Robot not found"))?;
        if robot.representations.iter().any(|r| {
            r.registry_resource_id == representation.registry_resource_id
                && r.format == representation.format
                && r.role == representation.role
        }) {
            return Err(invalid("Duplicate representation"));
        }
        robot.representations.push(representation);
        Ok(())
    }

    pub fn to_json(&self) -> io::Result<String> {
        self.validate()?;
        serde_json::to_string_pretty(self).map_err(io::Error::other)
    }

    pub fn from_json(json: &str) -> io::Result<Self> {
        let catalog: Self =
            serde_json::from_str(json).map_err(|error| invalid(error.to_string()))?;
        catalog.validate()?;
        Ok(catalog)
    }

    fn validate(&self) -> io::Result<()> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(invalid("Unsupported Robot Catalog schema"));
        }
        let mut ids = HashSet::new();
        for robot in &self.robots {
            if robot.robot_id.trim().is_empty()
                || robot.name.trim().is_empty()
                || !ids.insert(&robot.robot_id)
            {
                return Err(invalid("Invalid or duplicate Robot ID/name"));
            }
            if let Some(o) = &robot.origin {
                if o.source_namespace.trim().is_empty() || o.source_id.trim().is_empty() {
                    return Err(invalid("Incomplete Robot origin"));
                }
            }
            let mut refs = HashSet::new();
            for r in &robot.representations {
                if r.registry_resource_id.trim().is_empty()
                    || r.format.trim().is_empty()
                    || r.role.trim().is_empty()
                    || !refs.insert((&r.registry_resource_id, &r.format, &r.role))
                {
                    return Err(invalid("Invalid or duplicate Robot representation"));
                }
            }
        }
        Ok(())
    }
}
