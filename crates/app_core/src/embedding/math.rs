//! 向量数学与相似度计算基础

/// 计算两个浮点向量的点积
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// 计算向量的 L2 范数 (模长)
pub fn l2_norm(v: &[f32]) -> f32 {
    let sum_sq: f32 = v.iter().map(|x| x * x).sum();
    sum_sq.sqrt()
}

/// 计算两个向量的余弦相似度 [-1.0, 1.0]
///
/// 若维度不一致或任意向量模长接近 0，则返回 0.0
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot = dot_product(a, b);
    let norm_a = l2_norm(a);
    let norm_b = l2_norm(b);

    if norm_a <= 1e-7 || norm_b <= 1e-7 {
        0.0
    } else {
        (dot / (norm_a * norm_b)).clamp(-1.0, 1.0)
    }
}

/// 对向量进行原位 L2 归一化
pub fn normalize_in_place(v: &mut [f32]) {
    let norm = l2_norm(v);
    if norm > 1e-7 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}
