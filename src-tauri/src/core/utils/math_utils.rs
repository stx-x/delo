/// 数学工具模块: 提供数学计算相关的函数
use rayon::prelude::*;

/// 2D离散余弦变换(DCT)
/// 将图像从空间域转换为频率域
pub fn dct_2d(matrix: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = matrix.len();
    let mut result = vec![vec![0.0f64; n]; n];
    
    // 行方向DCT (并行)
    result.par_iter_mut().enumerate().for_each(|(y, row)| {
        let dct_row = dct_1d(&matrix[y]);
        row.copy_from_slice(&dct_row);
    });
    
    // 提取列并转置
    let mut transposed = vec![vec![0.0f64; n]; n];
    for y in 0..n {
        for x in 0..n {
            transposed[x][y] = result[y][x];
        }
    }
    
    // 列方向DCT (实际上是对转置矩阵进行行方向DCT) (并行)
    transposed.par_iter_mut().for_each(|row| {
        let dct_row = dct_1d(row);
        row.copy_from_slice(&dct_row);
    });
    
    // 再次转置回原始方向
    for y in 0..n {
        for x in 0..n {
            result[y][x] = transposed[x][y];
        }
    }
    
    result
}

/// 优化版2D离散余弦变换，仅计算低频部分
/// width和height指定只计算左上角的部分，比如8x8
pub fn dct_2d_optimized(matrix: &[Vec<f64>], width: usize, height: usize) -> Vec<Vec<f64>> {
    // 如果输入是32x32矩阵，并且我们只需要8x8的DCT，使用特化的快速实现
    if matrix.len() == 32 && matrix[0].len() == 32 && width == 8 && height == 8 {
        return dct_2d_32x32_to_8x8(matrix);
    }
    
    let n = matrix.len();
    
    // 确保不超出输入矩阵的范围
    let calc_width = width.min(n);
    let calc_height = height.min(n);
    
    // 只为需要的部分分配空间
    let mut result = vec![vec![0.0f64; calc_width]; calc_height];
    
    // 缓存余弦值
    let mut cos_cache: Vec<Vec<Vec<f64>>> = vec![Vec::new(); calc_height];
    for y in 0..calc_height {
        cos_cache[y] = vec![vec![0.0f64; n]; calc_width];
        for x in 0..calc_width {
            for i in 0..n {
                let angle_x = std::f64::consts::PI * (2 * i + 1) as f64 * x as f64 / (2 * n) as f64;
                cos_cache[y][x][i] = angle_x.cos();
            }
        }
    }
    
    // 行方向DCT (只计算需要的列)
    let mut temp = vec![vec![0.0f64; calc_width]; n];
    
    for y in 0..n {
        for k in 0..calc_width {
            let mut sum = 0.0;
            let alpha = if k == 0 { 
                (1.0 / n as f64).sqrt() 
            } else { 
                (2.0 / n as f64).sqrt() 
            };
        
            for i in 0..n {
                sum += matrix[y][i] * cos_cache[0][k][i];
            }
        
            temp[y][k] = alpha * sum;
        }
    }
    
    // 列方向DCT (只计算需要的行)
    for x in 0..calc_width {
        for k in 0..calc_height {
            let mut sum = 0.0;
            let alpha = if k == 0 { 
                (1.0 / n as f64).sqrt() 
            } else { 
                (2.0 / n as f64).sqrt() 
            };
        
            for i in 0..n {
                sum += temp[i][x] * cos_cache[k][0][i];
            }
        
            result[k][x] = alpha * sum;
        }
    }
    
    result
}

/// 使用快速算法计算32x32图像DCT的8x8左上角
/// 针对图像哈希常用尺寸进行特殊优化
pub fn dct_2d_32x32_to_8x8(matrix: &[Vec<f64>]) -> Vec<Vec<f64>> {
    // 确保输入矩阵大小正确
    if matrix.len() != 32 || matrix[0].len() != 32 {
        return dct_2d_optimized(matrix, 8, 8);
    }

    // 结果矩阵, 8x8
    let mut result = vec![vec![0.0f64; 8]; 8];

    // 用于分块DCT的常数
    const N: usize = 32;
    const M: usize = 8;

    // 使用查找表加速计算
    let alphas: Vec<f64> = (0..M).map(|k| {
        if k == 0 { (1.0 / N as f64).sqrt() } else { (2.0 / N as f64).sqrt() }
    }).collect();

    // 预计算所有余弦值 
    let mut cos_table = vec![vec![vec![0.0f64; N]; M]; 2];

    for dim in 0..2 {
        for k in 0..M {
            for i in 0..N {
                let angle = std::f64::consts::PI * (2 * i + 1) as f64 * k as f64 / (2.0 * N as f64);
                cos_table[dim][k][i] = angle.cos();
            }
        }
    }

    // 中间结果，先计算行方向DCT
    let mut temp = vec![vec![0.0f64; M]; N];

    for y in 0..N {
        for x in 0..M {
            let alpha = alphas[x];
            let mut sum = 0.0;
        
            for i in 0..N {
                sum += matrix[y][i] * cos_table[0][x][i];
            }
        
            temp[y][x] = alpha * sum;
        }
    }

    // 然后计算列方向DCT
    for y in 0..M {
        for x in 0..M {
            let alpha = alphas[y];
            let mut sum = 0.0;
        
            for i in 0..N {
                sum += temp[i][x] * cos_table[1][y][i];
            }
        
            result[y][x] = alpha * sum;
        }
    }

    result
}

/// 1D离散余弦变换(DCT)
pub fn dct_1d(input: &[f64]) -> Vec<f64> {
    let n = input.len();
    let mut output = vec![0.0f64; n];
    
    // 预计算余弦值以提高性能
    let mut cos_table = vec![vec![0.0f64; n]; n];
    for k in 0..n {
        for i in 0..n {
            let angle = std::f64::consts::PI * (2 * i + 1) as f64 * k as f64 / (2 * n) as f64;
            cos_table[k][i] = angle.cos();
        }
    }
    
    for k in 0..n {
        let mut sum = 0.0;
        let alpha = if k == 0 { 
            (1.0 / n as f64).sqrt() 
        } else { 
            (2.0 / n as f64).sqrt() 
        };
        
        // 使用预计算的余弦值
        for i in 0..n {
            sum += input[i] * cos_table[k][i];
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