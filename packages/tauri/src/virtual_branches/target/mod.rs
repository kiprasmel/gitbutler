mod reader;
mod writer;

use serde::{ser::SerializeStruct, Serialize, Serializer};

pub use reader::TargetReader as Reader;
pub use writer::TargetWriter as Writer;

use crate::git;

#[derive(Debug, PartialEq, Clone)]
pub struct Target {
    pub branch: git::RemoteBranchName,
    pub remote_url: String,
    pub sha: git::Oid,
}

impl Serialize for Target {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Target", 5)?;
        state.serialize_field("branchName", &self.branch.branch())?;
        state.serialize_field("remoteName", &self.branch.remote())?;
        state.serialize_field("remoteUrl", &self.remote_url)?;
        state.serialize_field("sha", &self.sha.to_string())?;
        state.end()
    }
}

// this is a backwards compatibile with the old format
fn read_remote_url(reader: &dyn crate::reader::Reader) -> Result<String, crate::reader::Error> {
    match reader.read_string("remote_url") {
        Ok(url) => Ok(url),
        // fallback to the old format
        Err(crate::reader::Error::NotFound) => reader.read_string("remote"),
        Err(e) => Err(e),
    }
}

// returns (remote_name, branch_name)
fn read_remote_name_branch_name(
    reader: &dyn crate::reader::Reader,
) -> Result<(String, String), crate::reader::Error> {
    match reader.read_string("name") {
        Ok(branch) => {
            let parts = branch.split('/').collect::<Vec<_>>();
            Ok((parts[0].to_string(), branch))
        }
        Err(crate::reader::Error::NotFound) => {
            // fallback to the old format
            let remote_name = reader.read_string("remote_name")?;
            let branch_name = reader.read_string("branch_name")?;
            Ok((remote_name, branch_name))
        }
        Err(e) => Err(e),
    }
}

impl TryFrom<&dyn crate::reader::Reader> for Target {
    type Error = crate::reader::Error;

    fn try_from(reader: &dyn crate::reader::Reader) -> Result<Self, Self::Error> {
        let (_, branch_name) = read_remote_name_branch_name(reader).map_err(|e| {
            crate::reader::Error::IOError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("branch: {}", e),
            ))
        })?;
        let remote_url = read_remote_url(reader).map_err(|e| {
            crate::reader::Error::IOError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("remote: {}", e),
            ))
        })?;
        let sha = reader
            .read_string("sha")
            .map_err(|e| {
                crate::reader::Error::IOError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("sha: {}", e),
                ))
            })?
            .parse()
            .map_err(|e| {
                crate::reader::Error::IOError(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("sha: {}", e),
                ))
            })?;

        Ok(Self {
            branch: format!("refs/remotes/{}", branch_name).parse().unwrap(),
            remote_url,
            sha,
        })
    }
}