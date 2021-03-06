extern crate geo;

use self::geo::{LineString, MultiPolygon, Polygon, Point};
use ffi::{CoordSeq, GGeom};
use error::Error;
use std;

// define our own TryInto while the std trait is not stable
pub trait TryInto<T> {
    type Err;
    fn try_into(self) -> Result<T, Self::Err>;
}

fn create_coord_seq_from_vec<'a>(points: &'a[Point<f64>]) -> Result<CoordSeq, Error> {
    create_coord_seq(points.iter(), points.len())
}

fn create_coord_seq<'a, It>(points: It, len: usize) -> Result<CoordSeq, Error>
where It: Iterator<Item = &'a Point<f64>> {
    let coord_seq = CoordSeq::new(len as u32, 2);
    for (i, p) in points.enumerate() {
        let j = i as u32;
        coord_seq.set_x(j, p.x())?;
        coord_seq.set_y(j, p.y())?;
    }
    Ok(coord_seq)
}

impl<'a> TryInto<GGeom> for &'a LineString<f64> {
    type Err = Error;

    fn try_into(self) -> Result<GGeom, Self::Err> {
        let coord_seq = create_coord_seq_from_vec(self.0.as_slice())?;

        GGeom::create_line_string(coord_seq)
    }
}

// rust geo does not have the distinction LineString/LineRing, so we create a wrapper

struct LineRing<'a>(&'a LineString<f64>);

/// Convert a geo::LineString to a geos LinearRing
/// a LinearRing should be closed so cloase the geometry if needed
impl<'a> TryInto<GGeom> for &'a LineRing<'a> {
    type Err = Error;

    fn try_into(self) -> Result<GGeom, Self::Err> {
        let points = &(self.0).0;
        let nb_points = points.len();
        if nb_points > 0 && nb_points < 3 {
            return Err(Error::InvalidGeometry("impossible to create a LinearRing, A LinearRing must have at least 3 coordinates".into()));
        }

        // if the geom is not closed we close it
        let is_closed = nb_points > 0 && points.first() == points.last();
        // Note: we also need to close a 2 points closed linearring, cf test closed_2_points_linear_ring
        let need_closing = nb_points > 0 && (! is_closed || nb_points == 3);
        let coord_seq = if need_closing {
            create_coord_seq(points.iter().chain(std::iter::once(&points[0])), nb_points + 1)?
        } else {
            create_coord_seq(points.iter(), nb_points)?
        };
        GGeom::create_linear_ring(coord_seq)
    }
}

impl<'a> TryInto<GGeom> for &'a Polygon<f64> {
    type Err = Error;

    fn try_into(self) -> Result<GGeom, Self::Err> {
        let geom_exterior: GGeom = LineRing(&self.exterior).try_into()?;

        let interiors: Vec<_> = self.interiors
            .iter()
            .map(|i| LineRing(i).try_into())
            .collect::<Result<Vec<_>, _>>()?;

        GGeom::create_polygon(geom_exterior, interiors)
    }
}

impl<'a> TryInto<GGeom> for &'a MultiPolygon<f64> {
    type Err = Error;

    fn try_into(self) -> Result<GGeom, Self::Err> {
        let polygons: Vec<_> = self.0
            .iter()
            .map(|p| p.try_into())
            .collect::<Result<Vec<_>, _>>()?;

        GGeom::create_multipolygon(polygons)
    }
}

#[cfg(test)]
mod test {
    use from_geo::geo::{LineString, MultiPolygon, Point, Polygon};
    use ffi::GGeom;
    use from_geo::TryInto;
    use super::LineRing;

    #[test]
    fn polygon_contains_test() {
        let exterior = LineString(vec![
            Point::new(0., 0.),
            Point::new(0., 1.),
            Point::new(1., 1.),
            Point::new(1., 0.),
            Point::new(0., 0.),
        ]);
        let interiors = vec![
            LineString(vec![
                Point::new(0.1, 0.1),
                Point::new(0.1, 0.9),
                Point::new(0.9, 0.9),
                Point::new(0.9, 0.1),
                Point::new(0.1, 0.1),
            ]),
        ];
        let p = Polygon::new(exterior.clone(), interiors.clone());

        assert_eq!(p.exterior, exterior);
        assert_eq!(p.interiors, interiors);

        let geom: GGeom = (&p).try_into().unwrap();

        assert!(geom.contains(&geom).unwrap());
        assert!(!geom.contains(&(&exterior).try_into().unwrap()).unwrap());

        assert!(geom.covers(&(&exterior).try_into().unwrap()).unwrap());
        assert!(geom.touches(&(&exterior).try_into().unwrap()).unwrap());
    }

    #[test]
    fn multipolygon_contains_test() {
        let exterior = LineString(vec![
            Point::new(0., 0.),
            Point::new(0., 1.),
            Point::new(1., 1.),
            Point::new(1., 0.),
            Point::new(0., 0.),
        ]);
        let interiors = vec![
            LineString(vec![
                Point::new(0.1, 0.1),
                Point::new(0.1, 0.9),
                Point::new(0.9, 0.9),
                Point::new(0.9, 0.1),
                Point::new(0.1, 0.1),
            ]),
        ];
        let p = Polygon::new(exterior, interiors);
        let mp = MultiPolygon(vec![p.clone()]);

        let geom: GGeom = (&mp).try_into().unwrap();

        assert!(geom.contains(&geom).unwrap());
        assert!(geom.contains(&(&p).try_into().unwrap()).unwrap());
    }

    #[test]
    fn incorrect_multipolygon_test() {
        let exterior = LineString(vec![
            Point::new(0., 0.)
        ]);
        let interiors = vec![];
        let p = Polygon::new(exterior, interiors);
        let mp = MultiPolygon(vec![p.clone()]);

        let geom = (&mp).try_into();

        assert!(geom.is_err());
    }

    #[test]
    fn incorrect_polygon_not_closed() {
        // even if the polygon is not closed we can convert it to geos (we close it)
        let exterior = LineString(vec![
            Point::new(0., 0.),
            Point::new(0., 2.),
            Point::new(2., 2.),
            Point::new(2., 0.),
            Point::new(0., 0.),
        ]);
        let interiors = vec![
            LineString(vec![
            Point::new(0., 0.),
            Point::new(0., 1.),
            Point::new(1., 1.),
            Point::new(1., 0.),
            Point::new(0., 10.),
            ]),
        ];
        let p = Polygon::new(exterior, interiors);
        let mp = MultiPolygon(vec![p]);

        let _g = (&mp).try_into().unwrap(); // no error
    }

    /// a linear ring can be empty
    #[test]
    fn empty_linear_ring() {
        let ls = LineString(vec![]);
        let geom: GGeom = LineRing(&ls).try_into().unwrap();

        assert!(geom.is_valid());
        assert!(geom.is_ring().unwrap());
        assert_eq!(geom.get_coord_seq().unwrap().len().unwrap(), 0);
    }

    /// a linear ring should have at least 3 elements
    #[test]
    fn one_elt_linear_ring() {
        let ls = LineString(vec![
            Point::new(0., 0.),
        ]);
        let geom: Result<GGeom, _> = LineRing(&ls).try_into();
        let error = geom.err().unwrap();
        assert_eq!(format!("{}", error), "Invalid geometry, impossible to create a LinearRing, A LinearRing must have at least 3 coordinates".to_string());
    }

    /// a linear ring should have at least 3 elements
    #[test]
    fn two_elt_linear_ring() {
        let ls = LineString(vec![
            Point::new(0., 0.),
            Point::new(0., 1.),
        ]);
        let geom: Result<GGeom, _> = LineRing(&ls).try_into();
        let error = geom.err().unwrap();
        assert_eq!(format!("{}", error), "Invalid geometry, impossible to create a LinearRing, A LinearRing must have at least 3 coordinates".to_string());
    }

    /// an unclosed linearring is valid since we close it before giving it to geos
    #[test]
    fn unclosed_linear_ring() {
        let ls = LineString(vec![
            Point::new(0., 0.),
            Point::new(0., 1.),
            Point::new(1., 2.),
         ]);
        let geom: GGeom = LineRing(&ls).try_into().unwrap();

        assert!(geom.is_valid());
        assert!(geom.is_ring().unwrap());
        assert_eq!(geom.get_coord_seq().unwrap().len().unwrap(), 4);
    }

    /// a bit tricky
    /// a ring should have at least 3 points.
    /// in the case of a closed ring with only element eg:
    ///
    /// let's take a point list: [p1, p2, p1]
    ///
    /// p1 ----- p2
    ///  ^-------|
    ///
    /// we consider it like a 3 points not closed ring (with the 2 last elements being equals...)
    ///
    /// shapely (the python geos wrapper) considers that too
    #[test]
    fn closed_2_points_linear_ring() {
        let ls = LineString(vec![
            Point::new(0., 0.),
            Point::new(0., 1.),
            Point::new(0., 0.),
         ]);
        let geom: GGeom = LineRing(&ls).try_into().unwrap();

        assert!(geom.is_valid());
        assert!(geom.is_ring().unwrap());
        assert_eq!(geom.get_coord_seq().unwrap().len().unwrap(), 4);
    }

    /// a linear ring can be empty
    #[test]
    fn good_linear_ring() {
        let ls = LineString(vec![
            Point::new(0., 0.),
            Point::new(0., 1.),
            Point::new(1., 2.),
            Point::new(0., 0.),
         ]);
        let geom: GGeom = LineRing(&ls).try_into().unwrap();

        assert!(geom.is_valid());
        assert!(geom.is_ring().unwrap());
        assert_eq!(geom.get_coord_seq().unwrap().len().unwrap(), 4);
    }
}
