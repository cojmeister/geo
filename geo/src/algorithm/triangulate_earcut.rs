use crate::{coord, CoordFloat, CoordsIter, Polygon, Triangle};

/// Triangulate polygons using an [ear-cutting algorithm](https://www.geometrictools.com/Documentation/TriangulationByEarClipping.pdf).
pub trait TriangulateEarcut<T: CoordFloat> {
    /// # Examples
    ///
    /// ```
    /// use geo::{coord, polygon, Triangle, TriangulateEarcut};
    ///
    /// let square_polygon = polygon![
    ///     (x: 0., y: 0.), // SW
    ///     (x: 10., y: 0.), // SE
    ///     (x: 10., y: 10.), // NE
    ///     (x: 0., y: 10.), // NW
    ///     (x: 0., y: 0.), // SW
    /// ];
    ///
    /// let triangles = square_polygon.triangulate_earcut();
    ///
    /// assert_eq!(
    ///     vec![
    ///         Triangle(
    ///             coord! { x: 0., y: 10. }, // NW
    ///             coord! { x: 10., y: 10. }, // NE
    ///             coord! { x: 10., y: 0. }, // SE
    ///         ),
    ///         Triangle(
    ///             coord! { x: 10., y: 0. }, // SE
    ///             coord! { x: 0., y: 0. }, // SW
    ///             coord! { x: 0., y: 10. }, // NW
    ///         ),
    ///     ],
    ///     triangles,
    /// );
    /// ```
    fn triangulate_earcut(&self) -> Vec<Triangle<T>> {
        self.triangulate_earcut_iter().collect()
    }

    /// # Examples
    ///
    /// ```
    /// use geo::{coord, polygon, Triangle, TriangulateEarcut};
    ///
    /// let square_polygon = polygon![
    ///     (x: 0., y: 0.), // SW
    ///     (x: 10., y: 0.), // SE
    ///     (x: 10., y: 10.), // NE
    ///     (x: 0., y: 10.), // NW
    ///     (x: 0., y: 0.), // SW
    /// ];
    ///
    /// let mut triangles_iter = square_polygon.triangulate_earcut_iter();
    ///
    /// assert_eq!(
    ///     Some(Triangle(
    ///             coord! { x: 0., y: 10. }, // NW
    ///             coord! { x: 10., y: 10. }, // NE
    ///             coord! { x: 10., y: 0. }, // SE
    ///     )),
    ///     triangles_iter.next(),
    /// );
    ///
    /// assert_eq!(
    ///     Some(Triangle(
    ///         coord! { x: 10., y: 0. }, // SE
    ///         coord! { x: 0., y: 0. }, // SW
    ///         coord! { x: 0., y: 10. }, // NW
    ///     )),
    ///     triangles_iter.next(),
    /// );
    ///
    /// assert!(triangles_iter.next().is_none());
    /// ```
    fn triangulate_earcut_iter(&self) -> Iter<T> {
        Iter(self.triangulate_earcut_raw())
    }

    /// Return the raw result from the `earcutr` library: a one-dimensional vector of polygon
    /// vertices (in XY order), and the indicies of the triangles within the vertices vector. This
    /// method is less ergonomic than the `triangulate_earcut` and `triangulate_earcut_iter`
    /// methods, but can be helpful when working in graphics contexts that expect flat vectors of
    /// data.
    ///
    /// # Examples
    ///
    /// ```
    /// use geo::{coord, polygon, Triangle, TriangulateEarcut};
    /// use geo::triangulate_earcut::Raw;
    ///
    /// let square_polygon = polygon![
    ///     (x: 0., y: 0.), // SW
    ///     (x: 10., y: 0.), // SE
    ///     (x: 10., y: 10.), // NE
    ///     (x: 0., y: 10.), // NW
    ///     (x: 0., y: 0.), // SW
    /// ];
    ///
    /// let mut triangles_raw = square_polygon.triangulate_earcut_raw();
    ///
    /// assert_eq!(
    ///     Raw {
    ///         vertices: vec![
    ///             0., 0., // SW
    ///             10., 0., // SE
    ///             10., 10., // NE
    ///             0., 10., // NW
    ///             0., 0., // SW
    ///         ],
    ///         triangle_indices: vec![
    ///             3, 0, 1, // NW-SW-SE
    ///             1, 2, 3, // SE-NE-NW
    ///         ],
    ///     },
    ///     triangles_raw,
    /// );
    /// ```
    fn triangulate_earcut_raw(&self) -> Raw<T>;
}

impl<T: CoordFloat> TriangulateEarcut<T> for Polygon<T> {
    fn triangulate_earcut_raw(&self) -> Raw<T> {
        let input = polygon_to_earcutr_input(self);
        let triangle_indices =
            earcutr::earcut(&input.vertexes, &input.interior_indexes, 2).unwrap();
        Raw {
            vertices: input.vertexes,
            triangle_indices,
        }
    }
}

/// The raw result of triangulating a polygon from `earcutr`.
#[derive(Debug, PartialEq, Clone)]
pub struct Raw<T: CoordFloat> {
    /// Flattened one-dimensional vector of polygon vertices (in XY order).
    pub vertices: Vec<T>,

    /// Indices of the triangles within the vertices vector.
    pub triangle_indices: Vec<usize>,
}

#[derive(Debug)]
pub struct Iter<T: CoordFloat>(Raw<T>);

impl<T: CoordFloat> Iterator for Iter<T> {
    type Item = Triangle<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let triangle_index_1 = self.0.triangle_indices.pop()?;
        let triangle_index_2 = self.0.triangle_indices.pop()?;
        let triangle_index_3 = self.0.triangle_indices.pop()?;
        Some(Triangle(
            self.triangle_index_to_coord(triangle_index_1),
            self.triangle_index_to_coord(triangle_index_2),
            self.triangle_index_to_coord(triangle_index_3),
        ))
    }
}

impl<T: CoordFloat> Iter<T> {
    fn triangle_index_to_coord(&self, triangle_index: usize) -> crate::Coord<T> {
        coord! {
            x: self.0.vertices[triangle_index * 2],
            y: self.0.vertices[triangle_index * 2 + 1],
        }
    }
}

struct EarcutrInput<T: CoordFloat> {
    pub vertexes: Vec<T>,
    pub interior_indexes: Vec<usize>,
}

fn polygon_to_earcutr_input<T: CoordFloat>(polygon: &crate::Polygon<T>) -> EarcutrInput<T> {
    let mut vertexes = Vec::with_capacity(polygon.coords_count() * 2);
    let mut interior_indexes = Vec::with_capacity(polygon.interiors().len());
    debug_assert!(polygon.exterior().0.len() >= 4);

    flat_line_string_coords_2(polygon.exterior(), &mut vertexes);

    for interior in polygon.interiors() {
        debug_assert!(interior.0.len() >= 4);
        interior_indexes.push(vertexes.len() / 2);
        flat_line_string_coords_2(interior, &mut vertexes);
    }

    EarcutrInput {
        vertexes,
        interior_indexes,
    }
}

fn flat_line_string_coords_2<T: CoordFloat>(
    line_string: &crate::LineString<T>,
    vertexes: &mut Vec<T>,
) {
    for coord in &line_string.0 {
        vertexes.push(coord.x);
        vertexes.push(coord.y);
    }
}

#[cfg(test)]
mod test {
    use super::TriangulateEarcut;
    use crate::{coord, polygon, Triangle};

    #[test]
    fn test_triangle() {
        let triangle_polygon = polygon![
            (x: 0., y: 0.),
            (x: 10., y: 0.),
            (x: 10., y: 10.),
            (x: 0., y: 0.),
        ];

        let triangles = triangle_polygon.triangulate_earcut();

        assert_eq!(
            &[Triangle(
                coord! { x: 10.0, y: 0.0 },
                coord! { x: 0.0, y: 0.0 },
                coord! { x: 10.0, y: 10.0 },
            ),][..],
            triangles,
        );
    }

    #[test]
    fn test_square() {
        let square_polygon = polygon![
            (x: 0., y: 0.),
            (x: 10., y: 0.),
            (x: 10., y: 10.),
            (x: 0., y: 10.),
            (x: 0., y: 0.),
        ];

        let mut triangles = square_polygon.triangulate_earcut();
        triangles.sort_by(|t1, t2| t1.1.x.partial_cmp(&t2.2.x).unwrap());

        assert_eq!(
            &[
                Triangle(
                    coord! { x: 10.0, y: 0.0 },
                    coord! { x: 0.0, y: 0.0 },
                    coord! { x: 0.0, y: 10.0 },
                ),
                Triangle(
                    coord! { x: 0.0, y: 10.0 },
                    coord! { x: 10.0, y: 10.0 },
                    coord! { x: 10.0, y: 0.0 },
                ),
            ][..],
            triangles,
        );
    }
}
