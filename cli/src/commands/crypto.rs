use crate::helper::write_device_state;
use crate::CLIError::Clap;
use crate::{helper, CLIError};
use bluetooth_mesh::crypto::key::{AppKey, NetKey};
use bluetooth_mesh::crypto::materials::{KeyPair, KeyPhase, NetworkSecurityMaterials};
use bluetooth_mesh::device_state;
use bluetooth_mesh::mesh::{AppKeyIndex, KeyIndex, NetKeyIndex};
use std::convert::TryFrom;
use std::fmt::Write;
use std::str::FromStr;

fn is_key_index(index: String) -> Result<(), String> {
    if u16::from_str(&index)
        .ok()
        .map(KeyIndex::try_from)
        .map(|r| r.is_ok())
        .unwrap_or(false)
    {
        Ok(())
    } else {
        Err(format!("'{}' is not a valid key index", &index))
    }
}

pub fn sub_command() -> clap::App<'static, 'static> {
    clap::SubCommand::with_name("crypto")
        .about("Read/Write crypto information from/to a device_state file")
        .subcommand(
            clap::SubCommand::with_name("devkey")
                .about("show local device key")
                .subcommand(
                    clap::SubCommand::with_name("set")
                        .about("set the local device key")
                        .arg(
                            clap::Arg::with_name("key_hex")
                                .help("128-bit big endian key hex")
                                .required(true)
                                .value_name("KEY_HEX")
                                .validator(helper::is_128_bit_hex_str_validator),
                        ),
                ),
        )
        .subcommand(
            clap::SubCommand::with_name("netkeys")
                .subcommand(
                    clap::SubCommand::with_name("list").arg(
                        clap::Arg::with_name("nid")
                            .short("n")
                            .long("nid")
                            .help("include NID in list")
                            .takes_value(false),
                    ),
                )
                .subcommand(
                    clap::SubCommand::with_name("get").arg(
                        clap::Arg::with_name("index")
                            .required(true)
                            .value_name("INDEX")
                            .validator(is_key_index),
                    ),
                )
                .subcommand(
                    clap::SubCommand::with_name("add")
                        .arg(
                            clap::Arg::with_name("index")
                                .help("new netkey index to add")
                                .required(true)
                                .value_name("INDEX")
                                .validator(is_key_index),
                        )
                        .arg(
                            clap::Arg::with_name("key_hex")
                                .help("128-bit big endian key hex")
                                .required(true)
                                .value_name("KEY_HEX")
                                .validator(helper::is_128_bit_hex_str_validator),
                        ),
                ),
        )
        .subcommand(
            clap::SubCommand::with_name("appkeys")
                .subcommand(
                    clap::SubCommand::with_name("list").arg(
                        clap::Arg::with_name("aid")
                            .short("a")
                            .long("aid")
                            .help("include AID in list")
                            .takes_value(false),
                    ),
                )
                .subcommand(
                    clap::SubCommand::with_name("get").arg(
                        clap::Arg::with_name("index")
                            .required(true)
                            .value_name("INDEX")
                            .validator(is_key_index),
                    ),
                )
                .subcommand(
                    clap::SubCommand::with_name("add")
                        .arg(
                            clap::Arg::with_name("net_index")
                                .help("netkey index to bind to the new appkey")
                                .required(true)
                                .value_name("NET_INDEX")
                                .validator(is_key_index),
                        )
                        .arg(
                            clap::Arg::with_name("app_index")
                                .help("new appkey index to add")
                                .required(true)
                                .value_name("NET_INDEX")
                                .validator(is_key_index),
                        )
                        .arg(
                            clap::Arg::with_name("key_hex")
                                .help("128-bit big endian key hex")
                                .required(true)
                                .value_name("KEY_HEX")
                                .validator(helper::is_128_bit_hex_str_validator),
                        ),
                ),
        )
}
pub fn crypto_matches(
    parent_logger: &slog::Logger,
    device_state_path: &str,
    crypto_matches: &clap::ArgMatches,
) -> Result<(), CLIError> {
    let logger = parent_logger.new(o!("device_state_path" => device_state_path.to_owned()));
    let get_device_state = || {
        let out = helper::load_device_state(device_state_path)?;
        debug!(logger, "loaded_device_state");
        Ok(out)
    };
    debug!(logger, "crypto_sub_command");
    match crypto_matches.subcommand() {
        ("devkey", Some(devkey_matches)) => {
            // print devkey
            let mut device_state = get_device_state()?;
            match devkey_matches.subcommand() {
                ("set", Some(set_matches)) => {
                    let new_key = set_matches.value_of("key_hex").expect("required by clap");
                    info!(logger, "set_devkey"; "new_key" => new_key.to_owned());
                    let new_key_buf =
                        helper::hex_str_to_bytes::<[u8; 16]>(new_key).expect("validated by clap");
                    *device_state.device_key_mut() =
                        bluetooth_mesh::crypto::key::DevKey::new_bytes(new_key_buf);
                    helper::write_device_state(device_state_path, &device_state)?;
                    debug!(logger, "wrote_devkey");
                }
                (_, _) => (),
            }
            println!("device key: {:X}", device_state.device_key().key());
        }
        ("netkeys", Some(netkeys_matches)) => {
            // netkeys
            let mut device_state = get_device_state()?;
            match netkeys_matches.subcommand() {
                ("list", Some(list_matches)) => {
                    let print_nid = list_matches.is_present("nid");
                    for (index, phase) in device_state.security_materials().net_key_map.map.iter() {
                        let mut buf = String::with_capacity(20);
                        write!(
                            &mut buf,
                            "index: {} phase: {}",
                            u16::from(index.0),
                            phase.phase()
                        )
                        .expect("basic fmt");
                        if print_nid {
                            match phase.key_pair() {
                                None => write!(
                                    &mut buf,
                                    " nid: {}",
                                    phase.tx_key().network_keys().nid()
                                )
                                .expect("basic fmt"),
                                Some(pair) => write!(
                                    &mut buf,
                                    " new_nid: {} old_nid: {}",
                                    pair.new.network_keys().nid(),
                                    pair.old.network_keys().nid()
                                )
                                .expect("basic_fmt"),
                            }
                        }
                        println!("{}", &buf);
                        buf.clear();
                    }
                }
                ("add", Some(add_matches)) => {
                    let index = NetKeyIndex(KeyIndex::new(
                        add_matches
                            .value_of("index")
                            .expect("required by clap")
                            .parse()
                            .expect("validated by clap"),
                    ));
                    if device_state
                        .security_materials()
                        .net_key_map
                        .get_keys(index)
                        .is_some()
                    {
                        return Err(CLIError::Clap(clap::Error::with_description(
                            format!(
                                "error: key already exists under index `{}`",
                                u16::from(index.0)
                            )
                            .as_str(),
                            clap::ErrorKind::InvalidValue,
                        )));
                    }
                    let new_key = add_matches.value_of("key_hex").expect("required by clap");
                    let new_key_buf =
                        helper::hex_str_to_bytes::<[u8; 16]>(new_key).expect("validated by clap");
                    device_state
                        .security_materials_mut()
                        .net_key_map
                        .insert(index, &NetKey::new_bytes(new_key_buf));
                    info!(logger, "inserted_netkey"; "new_key"=>new_key);
                    helper::write_device_state(device_state_path, &device_state)?;
                }
                ("get", Some(get_matches)) => {
                    let index = NetKeyIndex(KeyIndex::new(
                        get_matches
                            .value_of("index")
                            .expect("required by clap")
                            .parse()
                            .expect("validated by clap"),
                    ));
                    match device_state
                        .security_materials()
                        .net_key_map
                        .get_keys(index)
                    {
                        Some(phase) => match phase {
                            KeyPhase::Normal(k) => println!("normal: {}", k),
                            KeyPhase::Phase1(p) => {
                                println!("phase 1 new: ({}) old: ({})", p.new, p.old)
                            }
                            KeyPhase::Phase2(p) => {
                                println!("phase 2 new: ({}) old: ({})", p.new, p.old)
                            }
                        },
                        None => {
                            return Err(CLIError::Clap(clap::Error::with_description(
                                format!(
                                    "error: no key exists under index `{}`",
                                    u16::from(index.0)
                                )
                                .as_str(),
                                clap::ErrorKind::InvalidValue,
                            )))
                        }
                    }
                }
                _ => error!(logger, "no_netkeys_subcommand"),
            }
        }
        ("appkeys", Some(appkey_matches)) => {
            let mut device_state = get_device_state()?;
            match appkey_matches.subcommand() {
                ("list", Some(list_matches)) => {
                    let print_aid = list_matches.is_present("aid");
                    for (index, appkey) in device_state.security_materials().app_key_map.map.iter()
                    {
                        if print_aid {
                            println!(
                                "net_index: {} app_index: {} aid: {}",
                                u16::from(appkey.net_key_index.0),
                                u16::from(index.0),
                                u8::from(appkey.aid)
                            );
                        } else {
                            println!(
                                "net_index: {} app_index: {}",
                                u16::from(appkey.net_key_index.0),
                                u16::from(index.0)
                            );
                        }
                    }
                }
                ("add", Some(add_matches)) => {
                    let net_index = NetKeyIndex(KeyIndex::new(
                        add_matches
                            .value_of("net_index")
                            .expect("required by clap")
                            .parse()
                            .expect("validated by clap"),
                    ));
                    let app_index = AppKeyIndex(KeyIndex::new(
                        add_matches
                            .value_of("app_index")
                            .expect("required by clap")
                            .parse()
                            .expect("validated by clap"),
                    ));
                    if device_state
                        .security_materials()
                        .net_key_map
                        .get_keys(net_index)
                        .is_none()
                    {
                        return Err(CLIError::Clap(clap::Error::with_description(
                            format!(
                                "error: no net exists under index `{}`",
                                u16::from(net_index.0)
                            )
                            .as_str(),
                            clap::ErrorKind::InvalidValue,
                        )));
                    }
                    if device_state
                        .security_materials()
                        .app_key_map
                        .get_key(app_index)
                        .is_some()
                    {
                        return Err(CLIError::Clap(clap::Error::with_description(
                            format!(
                                "error: app key already exists under index `{}`",
                                u16::from(app_index.0)
                            )
                            .as_str(),
                            clap::ErrorKind::InvalidValue,
                        )));
                    }
                    let new_key = add_matches.value_of("key_hex").expect("required by clap");
                    let new_key_buf =
                        helper::hex_str_to_bytes::<[u8; 16]>(new_key).expect("validated by clap");
                    device_state.security_materials_mut().app_key_map.insert(
                        net_index,
                        app_index,
                        AppKey::new_bytes(new_key_buf),
                    );
                    write_device_state(device_state_path, &device_state)?;
                }
                ("", None) => error!(logger, "no_appkeys_subcommand"),
                _ => unreachable!("unhandled appkeys subcommand"),
            }
        }
        ("", None) => error!(logger, "no_subcommand"),
        _ => unreachable!("unhandled crypto subcommand"),
    }
    Ok(())
}