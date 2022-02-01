pub fn calculate_percentile(data: &mut Vec<f64>) -> (f64, f64, f64) {
    let (mut p99, mut p95, mut p90) = (0.0, 0.0, 0.0);

    let percentil = 100;
    // ascending sort
    data.sort_by(|a, b| a.partial_cmp(b).unwrap());
    data.retain(|x| *x != 0.0);
    let data_len = data.len();

    // rank of highest duration
    let mut rank = (percentil as f32 / 100.0 * data_len as f32) as usize;
    // index of sorted array
    let mut max_indx = data_len as usize - 1 as usize;

    // calc percentiles and only match on the ones we are currently interested in
    while max_indx != 0 {
        let percentile = (rank as f32 / data_len as f32 * 100.0) as u8;
        rank = rank - 1;
        // println!("{}", data[max_indx - 1]);
        match percentile {
            99 => p99 = data[max_indx - 1],
            95 => p95 = data[max_indx - 1],
            90 => p90 = data[max_indx - 1],
            _ => {}
        }
        max_indx -= 1;
    }

    (p99, p95, p90)
}
