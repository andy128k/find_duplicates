pub fn human_num_decimal(num: u64) -> String {
    let powers = ["", "K", "M", "G", "T", "P"];
    let mut power_index = 0;
    let mut num: f64 = num as f64;
    while num >= 1000.0 && power_index < powers.len() {
        num /= 1000.0;
        power_index += 1;
    }

    if power_index > 0 {
        format!("{:.1}{}", num, powers[power_index])
    } else {
        format!("{:.1}", num)
    }
}

pub fn human_num_binary(num: u64) -> String {
    let powers = ["", "Ki", "Mi", "Gi", "Ti", "Pi"];
    let mut power_index = 0;
    let mut num: f64 = num as f64;
    while num >= 1024.0 && power_index < powers.len() {
        num /= 1024.0;
        power_index += 1;
    }

    if power_index > 0 {
        format!("{:.1}{}", num, powers[power_index])
    } else {
        format!("{:.1}", num)
    }
}
