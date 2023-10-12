use if_addrs::IfAddr;

pub fn ipv4_interfaces() -> Vec<String> {
    if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|i| {
            if i.is_loopback() {
                None
            } else {
                match i.addr {
                    IfAddr::V4(ifv4) => Some(format!("{}", ifv4.ip)),
                    _ => None,
                }
            }
        })
        .collect()
}
