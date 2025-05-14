/// 数学工具模块: 提供数学计算相关的函数

/// 2D离散余弦变换(DCT)
/// 将图像从空间域转换为频率域
pub fn dct_2d(matrix: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = matrix.len();
    let mut result = vec![vec![0.0f64; n]; n];
    
    // 行方向DCT
    for y in 0..n {
        let dct_row = dct_1d(&matrix[y]);
        for x in 0..n {
            result[y][x] = dct_row[x];
        }
    }
    
    // 列方向DCT
    for x in 0..n {
        let mut col = Vec::with_capacity(n);
        for y in 0..n {
            col.push(result[y][x]);
        }
        
        let dct_col = dct_1d(&col);
        for y in 0..n {
            result[y][x] = dct_col[y];
        }
    }
    
    result
}

/// 1D离散余弦变换(DCT)
pub fn dct_1d(input: &[f64]) -> Vec<f64> {
    let n = input.len();
    let mut output = vec![0.0f64; n];
    
    for k in 0..n {
        let mut sum = 0.0;
        let alpha = if k == 0 { 
            (1.0 / n as f64).sqrt() 
        } else { 
            (2.0 / n as f64).sqrt() 
        };
        
        for i in 0..n {
            let angle = std::f64::consts::PI * (2 * i + 1) as f64 * k as f64 / (2 * n) as f64;
            sum += input[i] * angle.cos();
        }
        
        output[k] = alpha * sum;
    }
    
    output
}

/// 计算两点之间的欧几里得距离
pub fn euclidean_distance(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    (dx * dx + dy * dy).sqrt()
}

/// 计算两点之间的曼哈顿距离
pub fn manhattan_distance(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    (x2 - x1).abs() + (y2 - y1).abs()
}

/// 计算一组数据的中位数
pub fn median(values: &mut [f64]) -> f64 {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    }
}

/// 计算一组数据的均值
pub fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

/// 计算一组数据的标准差
pub fn standard_deviation(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    
    let avg = mean(values);
    let variance = values.iter()
        .map(|&x| (x - avg).powi(2))
        .sum::<f64>() / values.len() as f64;
    
    variance.sqrt()
}

/// 对向量进行归一化处理(使其范数为1)
pub fn normalize_vector(vec: &mut [f64]) {
    let norm: f64 = vec.iter().map(|&x| x * x).sum::<f64>().sqrt();
    
    if norm > 0.0 {
        for x in vec.iter_mut() {
            *x /= norm;
        }
    }
}

/// 生成一个2D高斯核
pub fn gaussian_kernel(size: usize, sigma: f64) -> Vec<Vec<f64>> {
    let mut kernel = vec![vec![0.0; size]; size];
    let center = (size as f64 - 1.0) / 2.0;
    
    let mut sum = 0.0;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f64 - center;
            let dy = y as f64 - center;
            let g = (-((dx * dx + dy * dy) / (2.0 * sigma * sigma))).exp();
            kernel[y][x] = g;
            sum += g;
        }
    }
    
    // 归一化
    for y in 0..size {
        for x in 0..size {
            kernel[y][x] /= sum;
        }
    }
    
    kernel
}