use std::sync::OnceLock;

use zbus::{
    proxy,
    zvariant::{self, OwnedObjectPath, OwnedValue, Type, Value},
};

use serde::{Deserialize, Serialize};
use std::hash::Hash;

use std::sync::atomic::{self, AtomicU64};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
/// The id of the window.
///
/// Internally Iced reserves `window::Id::MAIN` for the first window spawned.
pub struct Id(u64);

static COUNT: AtomicU64 = AtomicU64::new(0);

impl Id {
    /// Creates a new unique window [`Id`].
    pub fn unique() -> Id {
        Id(COUNT.fetch_add(1, atomic::Ordering::Relaxed))
    }
}
#[derive(Debug, Serialize, Deserialize, Type, Clone, Copy)]
#[serde(rename_all = "lowercase")]
#[zvariant(signature = "s")]
pub enum Mode {
    Replace,
    Fail,
    Isolate,
    IgnoreDependencies,
    IgnoreRequirements,
}

#[derive(Debug, Serialize, Type)]
#[zvariant(signature = "sa(sv)")]
struct SystemdAux<'a> {
    name: &'a str,
    properties: Vec<Value<'a>>,
}

#[derive(Debug, Type)]
#[zvariant(signature = "a(sv)")]
struct Properties<'a> {
    description: &'a str,
    exec_start: Vec<ExecCommand>,
    environment: Vec<&'a str>,
    working_directory: Option<&'a str>,
}

impl<'a> serde::Serialize for Properties<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut values: Vec<(String, OwnedValue)> = vec![];
        values.push((
            "Description".to_string(),
            OwnedValue::try_from(Value::new(self.description)).unwrap(),
        ));
        let mut starts: Vec<zvariant::Structure<'_>> = vec![];
        for start in &self.exec_start {
            let value: zvariant::Structure<'_> = start.clone().into();
            starts.push(value);
        }
        if !starts.is_empty() {
            let signature = starts[0].signature();
            let mut array = zvariant::Array::new(signature);
            for unit in starts {
                array.append(Value::new(unit)).unwrap();
            }
            values.push((
                "ExecStart".to_string(),
                Value::new(array).try_into().unwrap(),
            ));
        }

        values.push((
            "Environment".to_string(),
            Value::new(self.environment.clone()).try_into().unwrap(),
        ));
        if let Some(working_directory) = &self.working_directory {
            values.push((
                "WorkingDirectory".to_string(),
                Value::new(working_directory).try_into().unwrap(),
            ));
        }
        values.serialize(serializer)
    }
}

#[derive(Debug, Serialize, Type, Clone)]
struct ExecCommand {
    path: String,
    args: Vec<String>,
    unclean: bool,
}

impl<'a> From<ExecCommand> for zvariant::Structure<'a> {
    fn from(value: ExecCommand) -> Self {
        zvariant::StructureBuilder::new()
            .add_field(value.path)
            .add_field(value.args)
            .add_field(value.unclean)
            .build()
            .unwrap()
    }
}

#[proxy(
    interface = "org.freedesktop.systemd1.Manager",
    default_path = "/org/freedesktop/systemd1"
)]
pub trait Systemd1Manager {
    fn start_transient_unit(
        &self,
        name: &str,
        mode: Mode,
        properties: Properties<'_>,
        aux: Vec<SystemdAux<'_>>,
    ) -> zbus::Result<OwnedObjectPath>;
}

static SESSION: OnceLock<zbus::Connection> = OnceLock::new();

async fn get_connection() -> zbus::Result<zbus::Connection> {
    if let Some(cnx) = SESSION.get() {
        Ok(cnx.clone())
    } else {
        let cnx = zbus::Connection::session().await?;
        SESSION.set(cnx.clone()).expect("Can't reset a OnceCell");
        Ok(cnx)
    }
}

pub async fn launch(id: &str, cmd: &[String], description: &str) -> anyhow::Result<()> {
    let conn = get_connection().await?;
    let systemd = Systemd1ManagerProxy::builder(&conn)
        .destination("org.freedesktop.systemd1")?
        .build()
        .await?;
    let service = format!("layershell-launcher-tmp.{id}@{}.service", Id::unique().0);

    systemd
        .start_transient_unit(
            &service,
            Mode::Replace,
            Properties {
                description,
                exec_start: vec![ExecCommand {
                    path: cmd[0].clone(),
                    args: cmd.to_owned(),
                    unclean: false,
                }],
                environment: vec![],
                working_directory: None,
            },
            vec![],
        )
        .await?;
    Ok(())
}
