use std::fmt;
use std::cmp::Ordering;
use std::hash::Hasher;
use std::io::Write;
use bit_vec::BitVec;

/// CompactHash: 紧凑高效的哈希值表示
/// 根据不同类型的哈希/特征使用最合适的内部表示
#[derive(Clone)]
pub enum CompactHash {
    /// 二进制哈希 (如pHash，存储为位向量)
    Binary(BitVec),
    
    /// 整数哈希 (如量化后的64位哈希，存储为u64)
    Integer(u64),
    
    /// 字节数组哈希 (如小型特征或其他哈希值，存储为字节数组)
    Bytes(Vec<u8>),
    
    /// 全文本哈希 (如ORB特征或序列化后的特征，存储为字符串)
    Text(String),
}

impl CompactHash {
    /// 从字符串创建最合适的紧凑表示
    pub fn from_string(s: &str) -> Self {
        // 尝试检测哈希类型并选择最合适的表示
        if s.len() >= 16 && s.chars().all(|c| c == '0' || c == '1') {
            // 二进制字符串，转换为位向量
            let mut bit_vec = BitVec::with_capacity(s.len());
            for c in s.chars() {
                bit_vec.push(c == '1');
            }
            CompactHash::Binary(bit_vec)
        } else if s.len() <= 16 && s.chars().all(|c| c.is_digit(16)) {
            // 16进制表示的哈希，转换为u64
            if let Ok(value) = u64::from_str_radix(s, 16) {
                CompactHash::Integer(value)
            } else {
                CompactHash::Text(s.to_string())
            }
        } else if s.len() > 20 && s.starts_with("eJ") {
            // 很可能是Base64编码的ORB特征
            CompactHash::Text(s.to_string())
        } else {
            // 不确定类型，使用字节数组
            CompactHash::Bytes(s.as_bytes().to_vec())
        }
    }
    
    /// 计算与另一个哈希的相似度（0-100）
    pub fn similarity(&self, other: &CompactHash) -> f32 {
        match (self, other) {
            // 两个二进制哈希用汉明距离计算相似度
            (CompactHash::Binary(a), CompactHash::Binary(b)) => {
                let min_len = a.len().min(b.len());
                if min_len == 0 {
                    return 0.0;
                }
                
                let mut distance = 0;
                for i in 0..min_len {
                    if a[i] != b[i] {
                        distance += 1;
                    }
                }
                
                // 对不同长度的部分增加距离
                distance += a.len().abs_diff(b.len());
                
                100.0 * (1.0 - (distance as f32 / a.len().max(b.len()) as f32))
            },
            
            // 两个整数用位差异计算相似度
            (CompactHash::Integer(a), CompactHash::Integer(b)) => {
                let xor = a ^ b;
                let bits_different = xor.count_ones();
                100.0 * (1.0 - (bits_different as f32 / 64.0))
            },
            
            // 两个字节数组用字节差异计算相似度
            (CompactHash::Bytes(a), CompactHash::Bytes(b)) => {
                let min_len = a.len().min(b.len());
                if min_len == 0 {
                    return 0.0;
                }
                
                let mut byte_diff = 0;
                for i in 0..min_len {
                    byte_diff += (a[i] as i16 - b[i] as i16).abs() as u32;
                }
                
                // 对不同长度的部分增加距离
                byte_diff += (a.len().abs_diff(b.len()) * 255) as u32;
                
                let max_diff = (a.len().max(b.len()) * 255) as u32;
                100.0 * (1.0 - (byte_diff as f32 / max_diff as f32))
            },
            
            // 不同类型或文本哈希，转为字符串比较
            _ => {
                let s1 = self.to_string();
                let s2 = other.to_string();
                
                // 简单的Jaccard相似度：共同字符/总字符
                let set1: std::collections::HashSet<char> = s1.chars().collect();
                let set2: std::collections::HashSet<char> = s2.chars().collect();
                
                let intersection = set1.intersection(&set2).count();
                let union = set1.union(&set2).count();
                
                if union == 0 {
                    0.0
                } else {
                    100.0 * (intersection as f32 / union as f32)
                }
            }
        }
    }
    
    /// 使用LSH的哈希函数计算桶键
    pub fn lsh_bucket_key(&self, seed: u64) -> u64 {
        match self {
            CompactHash::Binary(bv) => {
                // 对位向量进行LSH
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                for i in 0..bv.len().min(64) {
                    if bv[i] {
                        hasher.write_u8(1 ^ ((seed >> (i % 64)) as u8 & 1));
                    } else {
                        hasher.write_u8(0 ^ ((seed >> (i % 64)) as u8 & 1));
                    }
                }
                hasher.finish()
            },
            
            CompactHash::Integer(value) => {
                // 对整数进行LSH（简单的位操作）
                value.wrapping_mul(0x517cc1b727220a95).rotate_left(seed as u32)
            },
            
            CompactHash::Bytes(bytes) => {
                // 对字节数组进行LSH
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                for (i, &b) in bytes.iter().enumerate().take(16) {
                    hasher.write_u8(b ^ ((seed >> (i % 8)) as u8));
                }
                std::hash::Hasher::finish(&hasher)
            },
            
            CompactHash::Text(text) => {
                // 对文本进行LSH（取样计算）
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                let sample_size = text.len().min(64);
                let step = text.len() / sample_size;
                
                for (i, c) in text.chars().step_by(step.max(1)).enumerate().take(sample_size) {
                    hasher.write_u8((c as u8) ^ ((seed >> (i % 8)) as u8));
                }
                std::hash::Hasher::finish(&hasher)
            }
        }
    }
    
    /// 将紧凑表示转换回字符串
    pub fn to_string(&self) -> String {
        match self {
            CompactHash::Binary(bv) => {
                let mut s = String::with_capacity(bv.len());
                for i in 0..bv.len() {
                    s.push(if bv[i] { '1' } else { '0' });
                }
                s
            },
            
            CompactHash::Integer(value) => {
                format!("{:016x}", value)
            },
            
            CompactHash::Bytes(bytes) => {
                // 转换为Base64减少空间
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes)
            },
            
            CompactHash::Text(text) => {
                text.clone()
            }
        }
    }
    
    /// 获取哈希长度（位数、字节数或字符数）
    pub fn len(&self) -> usize {
        match self {
            CompactHash::Binary(bv) => bv.len(),
            CompactHash::Integer(_) => 64,  // 64位
            CompactHash::Bytes(bytes) => bytes.len() * 8,  // 字节数 * 8位/字节
            CompactHash::Text(text) => text.len(),
        }
    }
    
    /// 检查哈希是否为空
    pub fn is_empty(&self) -> bool {
        match self {
            CompactHash::Binary(bv) => bv.is_empty(),
            CompactHash::Integer(v) => *v == 0,
            CompactHash::Bytes(bytes) => bytes.is_empty(),
            CompactHash::Text(text) => text.is_empty(),
        }
    }
}

impl From<&str> for CompactHash {
    fn from(s: &str) -> Self {
        CompactHash::from_string(s)
    }
}

impl From<String> for CompactHash {
    fn from(s: String) -> Self {
        CompactHash::from_string(&s)
    }
}

impl fmt::Display for CompactHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl fmt::Debug for CompactHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompactHash::Binary(bv) => write!(f, "Binary({} bits)", bv.len()),
            CompactHash::Integer(v) => write!(f, "Integer(0x{:x})", v),
            CompactHash::Bytes(bytes) => write!(f, "Bytes({} bytes)", bytes.len()),
            CompactHash::Text(text) => {
                let preview = if text.len() > 20 {
                    format!("{}...", &text[0..17])
                } else {
                    text.clone()
                };
                write!(f, "Text({})", preview)
            }
        }
    }
}

impl PartialEq for CompactHash {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (CompactHash::Binary(a), CompactHash::Binary(b)) => a == b,
            (CompactHash::Integer(a), CompactHash::Integer(b)) => a == b,
            (CompactHash::Bytes(a), CompactHash::Bytes(b)) => a == b,
            (CompactHash::Text(a), CompactHash::Text(b)) => a == b,
            _ => self.to_string() == other.to_string()
        }
    }
}

impl Eq for CompactHash {}

impl PartialOrd for CompactHash {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CompactHash {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (CompactHash::Binary(a), CompactHash::Binary(b)) => {
                // 比较位向量大小
                for i in 0..a.len().min(b.len()) {
                    match a[i].cmp(&b[i]) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                }
                a.len().cmp(&b.len())
            },
            (CompactHash::Integer(a), CompactHash::Integer(b)) => a.cmp(b),
            (CompactHash::Bytes(a), CompactHash::Bytes(b)) => a.cmp(b),
            (CompactHash::Text(a), CompactHash::Text(b)) => a.cmp(b),
            _ => self.to_string().cmp(&other.to_string())
        }
    }
}

impl std::hash::Hash for CompactHash {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            CompactHash::Binary(bv) => {
                // 哈希标记类型
                state.write_u8(0);
                // 创建字节表示并哈希
                let bytes = bv.to_bytes();
                state.write(&bytes);
            },
            CompactHash::Integer(v) => {
                // 哈希标记类型
                state.write_u8(1);
                state.write_u64(*v);
            },
            CompactHash::Bytes(bytes) => {
                // 哈希标记类型
                state.write_u8(2);
                state.write(bytes);
            },
            CompactHash::Text(text) => {
                // 哈希标记类型
                state.write_u8(3);
                state.write(text.as_bytes());
            }
        }
    }
}