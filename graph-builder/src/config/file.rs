//! TOML file configuration options.

use super::options;
use super::AppSettings;
use commons::de::de_loglevel;
use commons::MergeOptions;
use failure::{Fallible, ResultExt};
use std::io::Read;
use std::{fs, io, path};

/// TOML configuration, top-level.
#[derive(Debug, Deserialize)]
pub struct FileOptions {
    /// Verbosity level.
    #[serde(default = "Option::default", deserialize_with = "de_loglevel")]
    pub verbosity: Option<log::LevelFilter>,

    /// Upstream options.
    pub upstream: Option<UpstreamOptions>,

    /// Web frontend options.
    pub service: Option<options::ServiceOptions>,

    /// Status service options.
    pub status: Option<options::StatusOptions>,
}

impl FileOptions {
    pub fn read_filepath<P>(cfg_path: P) -> Fallible<Self>
    where
        P: AsRef<path::Path>,
    {
        let cfg_file = fs::File::open(&cfg_path).context(format!(
            "failed to open config path {:?}",
            cfg_path.as_ref()
        ))?;
        let mut bufrd = io::BufReader::new(cfg_file);

        let mut content = vec![];
        bufrd.read_to_end(&mut content)?;
        let cfg = toml::from_slice(&content).context(format!(
            "failed to read config file {}",
            cfg_path.as_ref().display()
        ))?;

        Ok(cfg)
    }
}

impl MergeOptions<Option<FileOptions>> for AppSettings {
    fn try_merge(&mut self, opts: Option<FileOptions>) -> Fallible<()>{
        if let Some(file) = opts {
            assign_if_some!(self.verbosity, file.verbosity);
            self.try_merge(file.service)?;
            self.try_merge(file.status)?;
            self.try_merge(file.upstream)?;
        }
        Ok(())
    }
}

/// Options for upstream fetcher.
#[derive(Debug, Deserialize)]
pub struct UpstreamOptions {
    /// Fetcher method.
    pub method: Option<String>,

    /// Docker-registry-v2 upstream options.
    pub registry: Option<options::DockerRegistryOptions>,
}

impl MergeOptions<Option<UpstreamOptions>> for AppSettings {
    fn try_merge(&mut self, opts: Option<UpstreamOptions>) -> Fallible<()> {
        if let Some(upstream) = opts {
            self.try_merge(upstream.registry)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::FileOptions;
    use crate::config::AppSettings;
    use commons::MergeOptions;

    #[test]
    fn toml_basic() {
        let toml_input = "[upstream.registry]\npause_secs=25";
        let file_opts: FileOptions = toml::from_str(toml_input).unwrap();

        let pause = file_opts
            .upstream
            .unwrap()
            .registry
            .unwrap()
            .pause_secs
            .unwrap();
        assert_eq!(pause, std::time::Duration::from_secs(25));
    }

    #[test]
    fn toml_merge_settings() {
        let mut settings = AppSettings::default();
        assert_eq!(settings.status_port, 9080);

        let toml_input = "status.port = 2222";
        let file_opts: FileOptions = toml::from_str(toml_input).unwrap();

        settings.try_merge(Some(file_opts)).unwrap();
        assert_eq!(settings.status_port, 2222);
    }

    #[test]
    fn toml_sample_config() {
        use tempfile;

        let opts = {
            use std::io::Write;

            let sample_config = r#"
                verbosity = 3

                [upstream]
                method = "registry"

                [upstream.registry]
                pause_secs = 35
                url = "quay.io"
                repository = "openshift-release-dev/ocp-release"

                [service]
                address = "0.0.0.0"
                port = 8383

                [status]
                address = "127.0.0.1"
            "#;

            let mut config_file = tempfile::NamedTempFile::new().unwrap();
            config_file
                .write_fmt(format_args!("{}", sample_config))
                .unwrap();
            FileOptions::read_filepath(config_file.path()).unwrap()
        };

        assert_eq!(opts.verbosity, Some(log::LevelFilter::Trace));
        assert!(opts.service.is_some());

        let ups_registry = opts.upstream.unwrap().registry.unwrap();
        assert!(ups_registry.credentials_path.is_none());
        let repo = ups_registry.repository.unwrap();
        assert_eq!(repo, "openshift-release-dev/ocp-release");
    }
}
