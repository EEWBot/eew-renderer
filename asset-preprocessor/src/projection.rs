use std::f64::consts::PI;

/// GRS80 の離心率。renderer-types の `to_mercator` と同じ値。
const E: f64 = 0.081_819_191_042_815_79;

/// 経緯度 (度) を、レンダラが使う正規化メルカトル座標に投影する。
///
/// 実行時 (renderer-types の `to_mercator`) と同じ式だが、
/// ビルド時なので f64 で計算してから f32 に丸める。
pub(crate) fn to_mercator(longitude: f32, latitude: f32) -> (f32, f32) {
    let longitude = (longitude as f64).to_radians();
    let latitude = (latitude as f64).to_radians();

    let x = longitude / PI;
    let y = (latitude.sin().atanh() - E * (E * latitude.sin()).atanh()) / PI;

    (x as f32, y as f32)
}

/// 経緯度の折れ線を、投影済みの線分 `(x0, y0, x1, y1)` の列にする。
/// 長さ 0 の線分は向きが定まらないので捨てる。
pub(crate) fn to_mercator_segments(
    line: impl IntoIterator<Item = (f32, f32)>,
) -> Vec<(f32, f32, f32, f32)> {
    let points: Vec<_> = line
        .into_iter()
        .map(|(longitude, latitude)| to_mercator(longitude, latitude))
        .collect();

    points
        .windows(2)
        .filter(|w| w[0] != w[1])
        .map(|w| (w[0].0, w[0].1, w[1].0, w[1].1))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use renderer_types::{GeoDegree, Vertex};

    #[test]
    fn matches_runtime_projection() {
        for (longitude, latitude) in [(139.7, 35.7), (127.7, 26.2), (142.2, 27.1), (0.0, 0.0)] {
            let expected = Vertex::<GeoDegree>::new(longitude, latitude).to_mercator();
            let (x, y) = to_mercator(longitude, latitude);

            assert!((x - expected.x()).abs() < 1e-6);
            assert!((y - expected.y()).abs() < 1e-6);
        }
    }

    #[test]
    fn segments_skip_degenerate() {
        let segments = to_mercator_segments([(0.0, 0.0), (0.0, 0.0), (90.0, 0.0), (180.0, 0.0)]);

        assert_eq!(segments, vec![(0.0, 0.0, 0.5, 0.0), (0.5, 0.0, 1.0, 0.0)]);
    }
}
