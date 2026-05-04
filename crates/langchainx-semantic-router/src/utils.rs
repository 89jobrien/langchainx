pub fn combine_embeddings(embeddings: &[Vec<f64>]) -> Vec<f64> {
    embeddings
        .iter()
        // Initialize a vector with zeros based on the length of the first embedding vector.
        // It's assumed all embeddings have the same dimensions.
        .fold(
            vec![0f64; embeddings[0].len()],
            |mut accumulator, embedding_vec| {
                for (i, &value) in embedding_vec.iter().enumerate() {
                    accumulator[i] += value;
                }
                accumulator
            },
        )
        // Calculate the mean for each element across all embeddings.
        .iter()
        .map(|&sum| sum / embeddings.len() as f64)
        .collect()
}

pub fn cosine_similarity(vec1: &[f64], vec2: &[f64]) -> f64 {
    let dot_product: f64 = vec1.iter().zip(vec2.iter()).map(|(a, b)| a * b).sum();
    let magnitude_vec1: f64 = vec1.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();
    let magnitude_vec2: f64 = vec2.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();
    dot_product / (magnitude_vec1 * magnitude_vec2)
}

pub fn sum_vectors(vectors: &[Vec<f64>]) -> Vec<f64> {
    let mut sum_vec = vec![0.0; vectors[0].len()];
    for vec in vectors {
        for (i, &value) in vec.iter().enumerate() {
            sum_vec[i] += value;
        }
    }
    sum_vec
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_orthogonal() {
        // Orthogonal vectors → similarity = 0
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        // Identical vectors → similarity = 1
        let a = vec![1.0, 1.0];
        let sim = cosine_similarity(&a, &a);
        assert!((sim - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_cosine_similarity_known_value() {
        // [1,0] vs [1,1]/sqrt(2) → dot=1, mag1=1, mag2=sqrt(2) → sim=1/sqrt(2)
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        let expected = 1.0_f64 / 2.0_f64.sqrt();
        assert!((sim - expected).abs() < 1e-9);
    }

    #[test]
    fn test_combine_embeddings_mean() {
        let vecs = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let result = combine_embeddings(&vecs);
        assert_eq!(result, vec![2.0, 3.0]);
    }

    #[test]
    fn test_sum_vectors() {
        let vecs = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let result = sum_vectors(&vecs);
        assert_eq!(result, vec![4.0, 6.0]);
    }
}
