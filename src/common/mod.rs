pub fn parse_port_range(port_str: &str) -> Vec<u16> {
    if let Ok(p) = port_str.parse::<u16>() {
        return vec![p];
    }
    
    if port_str.contains('-') {
        let parts: Vec<&str> = port_str.split('-').collect();
        if parts.len() == 2 {
            if let (Ok(start), Ok(end)) = (parts[0].parse::<u16>(), parts[1].parse::<u16>()) {
                if start <= end {
                    return (start..=end).collect();
                }
            }
        }
    }
    vec![]
}
