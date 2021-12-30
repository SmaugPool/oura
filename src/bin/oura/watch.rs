use std::str::FromStr;

use clap::{value_t, ArgMatches};
use oura::{
    framework::*,
    mapping::MapperConfig,
    sources::common::{AddressArg, BearerKind, MagicArg, PointArg},
};

use serde_derive::Deserialize;

use oura::sources::n2c::Config as N2CConfig;
use oura::sources::n2n::Config as N2NConfig;

#[derive(Clone, Debug, Deserialize)]
pub enum PeerMode {
    AsNode,
    AsClient,
}

impl FromStr for PeerMode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_ref() {
            "node" => Ok(PeerMode::AsNode),
            "client" => Ok(PeerMode::AsClient),
            _ => Err("can't parse peer mode (valid values: client|node)"),
        }
    }
}

enum WatchSource {
    N2C(N2CConfig),
    N2N(N2NConfig),
}

impl SourceConfig for WatchSource {
    fn bootstrap(&self) -> PartialBootstrapResult {
        match self {
            WatchSource::N2C(c) => c.bootstrap(),
            WatchSource::N2N(c) => c.bootstrap(),
        }
    }
}

pub fn run(args: &ArgMatches) -> Result<(), Error> {
    let socket = value_t!(args, "socket", String)?;

    let bearer = match args.is_present("bearer") {
        true => value_t!(args, "bearer", BearerKind)?,
        false => BearerKind::Unix,
    };

    let magic = match args.is_present("magic") {
        true => Some(value_t!(args, "magic", MagicArg)?),
        false => None,
    };

    let since = match args.is_present("since") {
        true => Some(value_t!(args, "since", PointArg)?),
        false => None,
    };

    let mode = match (args.is_present("mode"), &bearer) {
        (true, _) => value_t!(args, "mode", PeerMode).expect("invalid value for 'mode' arg"),
        (false, BearerKind::Tcp) => PeerMode::AsNode,
        (false, BearerKind::Unix) => PeerMode::AsClient,
    };

    let source_setup = match mode {
        PeerMode::AsNode => WatchSource::N2N(N2NConfig {
            address: AddressArg(bearer, socket),
            magic,
            well_known: None,
            mapper: MapperConfig::default(),
            since,
        }),
        PeerMode::AsClient => WatchSource::N2C(N2CConfig {
            address: AddressArg(bearer, socket),
            magic,
            well_known: None,
            mapper: MapperConfig::default(),
            since,
        }),
    };

    let sink_setup = oura::sinks::terminal::Config::default();

    let (source_handle, source_output) = source_setup.bootstrap()?;
    let sink_handle = sink_setup.bootstrap(source_output)?;

    sink_handle.join().map_err(|_| "error in sink thread")?;
    source_handle.join().map_err(|_| "error in source thread")?;

    Ok(())
}