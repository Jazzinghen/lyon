//! Bezier curve related maths and tools.

use path_builder::PrimitiveBuilder;

use vodk_math::{ Vector2D, Unit, Untyped };

use std::mem::swap;

pub fn sample_quadratic_bezier<U: Unit>(
    from: Vector2D<U>,
    ctrl: Vector2D<U>,
    to: Vector2D<U>,
    t: f32
) -> Vector2D<U> {
    let t2 = t*t;
    let one_t = 1.0 - t;
    let one_t2 = one_t * one_t;
    return from * one_t2
         + ctrl * 2.0*one_t*t
         + to * t2;
}

pub fn sample_cubic_bezier<U: Unit>(
    from: Vector2D<U>,
    ctrl1: Vector2D<U>,
    ctrl2: Vector2D<U>,
    to: Vector2D<U>,
    t: f32
) -> Vector2D<U> {
    let t2 = t*t;
    let t3 = t2*t;
    let one_t = 1.0 - t;
    let one_t2 = one_t*one_t;
    let one_t3 = one_t2*one_t;
    return from * one_t3
         + ctrl1 * 3.0*one_t2*t
         + ctrl2 * 3.0*one_t*t2
         + to * t3
}

#[derive(Debug)]
pub struct CubicBezierSegment<U: Unit> {
    pub from: Vector2D<U>,
    pub cp1: Vector2D<U>,
    pub cp2: Vector2D<U>,
    pub to: Vector2D<U>,
}

impl<U: Unit> Copy for CubicBezierSegment<U> {}
impl<U: Unit> Clone for CubicBezierSegment<U> {
    fn clone(&self) -> CubicBezierSegment<U> { *self }
}

#[derive(Debug)]
pub struct QuadraticBezierSegment<U: Unit> {
    pub from: Vector2D<U>,
    pub cp: Vector2D<U>,
    pub to: Vector2D<U>,
}

impl<U: Unit> Copy for QuadraticBezierSegment<U> {}
impl<U: Unit> Clone for QuadraticBezierSegment<U> {
    fn clone(&self) -> QuadraticBezierSegment<U> { *self }
}

impl<U: Unit> QuadraticBezierSegment<U> {
    pub fn to_cubic(&self) -> CubicBezierSegment<U> {
        CubicBezierSegment {
            from: self.from,
            cp1: (self.from + self.cp * 2.0) / 3.0,
            cp2: (self.to + self.cp * 2.0) / 3.0,
            to: self.to,
        }
    }
}

impl<U: Unit> CubicBezierSegment<U> {
    pub fn split_in_place(&mut self, t: f32) -> CubicBezierSegment<U> {
        let cp1a = self.from + (self.cp1 - self.from) * t;
        let cp2a = self.cp1 + (self.cp2 - self.cp1) * t;
        let cp1aa = cp1a + (cp2a - cp1a) * t;
        let cp3a = self.cp2 + (self.to - self.cp2) * t;
        let cp2aa = cp2a + (cp3a - cp2a) * t;
        let cp1aaa = cp1aa + (cp2aa - cp1aa) * t;
        let to = self.to;

        self.cp1 = cp1a;
        self.cp2 = cp1aa;
        self.to = cp1aaa;

        return CubicBezierSegment {
            from: cp1aaa,
            cp1: cp2aa,
            cp2: cp3a,
            to: to,
        };
    }

    pub fn split(&self, t: f32) -> (CubicBezierSegment<U>, CubicBezierSegment<U>) {
        let mut a = *self;
        let b = a.split_in_place(t);
        return (a, b);
    }

    pub fn sample(&self, t: f32) -> Vector2D<U> {
        return sample_cubic_bezier(self.from, self.cp1, self.cp2, self.to, t);
    }
}

// TODO: This is not very ergonomic.
pub fn split_cubic_bezier<U: Unit>(
    bezier: &CubicBezierSegment<U>,
    t: f32,
    out_first_segment: Option<&mut CubicBezierSegment<U>>,
    out_second_segment: Option<&mut CubicBezierSegment<U>>
) {
    let cp1a = bezier.from + (bezier.cp1 - bezier.from) * t;
    let cp2a = bezier.cp1 + (bezier.cp2 - bezier.cp1) * t;
    let cp1aa = cp1a + (cp2a - cp1a) * t;
    let cp3a = bezier.cp2 + (bezier.to - bezier.cp2) * t;
    let cp2aa = cp2a + (cp3a - cp2a) * t;
    let cp1aaa = cp1aa + (cp2aa - cp1aa) * t;

    if let Some(first) = out_first_segment {
        first.from = bezier.from;
        first.cp1 = cp1a;
        first.cp2 = cp1aa;
        first.to = cp1aaa;
    }

    if let Some(second) = out_second_segment {
        second.from = cp1aaa;
        second.cp1 = cp2aa;
        second.cp2 = cp3a;
        second.to = bezier.to;
    }
}


// Find the inflection points of a cubic bezier curve.
fn find_cubic_bezier_inflection_points<U: Unit>(
    bezier: &CubicBezierSegment<U>,
) -> (Option<f32>, Option<f32>) {
    // Find inflection points.
    // See www.faculty.idc.ac.il/arik/quality/appendixa.html for an explanation
    // of this approach.
    let pa = bezier.cp1 - bezier.from;
    let pb = bezier.cp2 - (bezier.cp1 * 2.0) + bezier.from;
    let pc = bezier.to - (bezier.cp2 * 3.0) + (bezier.cp1 * 3.0) - bezier.from;

    let a = pb.x * pc.y - pb.y * pc.x;
    let b = pa.x * pc.y - pa.y * pc.x;
    let c = pa.x * pb.y - pa.y * pb.x;

    if a == 0.0 {
        // Not a quadratic equation.
        if b == 0.0 {
            // Instead of a linear acceleration change we have a constant
            // acceleration change. This means the equation has no solution
            // and there are no inflection points, unless the constant is 0.
            // In that case the curve is a straight line, essentially that means
            // the easiest way to deal with is is by saying there's an inflection
            // point at t == 0. The inflection point approximation range found will
            // automatically extend into infinity.
            if c == 0.0 {
               return (Some(0.0), None);
            }
            return (None, None);
        }
        return (Some(-c / b), None);
    }

    let discriminant = b * b - 4.0 * a * c;

    if discriminant < 0.0 {
        return (None, None);
    }

    if discriminant == 0.0 {
        return (Some(-b / (2.0 * a)), None);
    }

    let discriminant_sqrt = discriminant.sqrt();
    let q = if b < 0.0 { b - discriminant_sqrt } else { b + discriminant_sqrt } * -0.5;

    let mut t1 = q / a;
    let mut t2 = c / q;
    if t1 > t2 {
        swap(&mut t1, &mut t2);
    }

    return (Some(t1), Some(t2));
}

pub fn cubic_root(val: f32) -> f32 {
    if val < 0.0 {
        return -cubic_root(-val);
    }

    return val.powf(1.0 / 3.0);
}

fn find_cubic_bezier_inflection_approximation_range<U: Unit>(
    bezier_segment: &CubicBezierSegment<U>,
    t: f32, tolerance: f32,
    min: &mut f32, max: &mut f32
) {
    let mut bezier = *bezier_segment;
    bezier = bezier.split_in_place(t);

    let cp21 = bezier.cp1 - bezier.from;
    let cp41 = bezier.to - bezier.from;

    if cp21.x == 0.0 && cp21.y == 0.0 {
      // In this case s3 becomes lim[n->0] (cp41.x * n) / n - (cp41.y * n) / n = cp41.x - cp41.y.

      // Use the absolute value so that Min and Max will correspond with the
      // minimum and maximum of the range.
      *min = t - cubic_root((tolerance / (cp41.x - cp41.y)).abs());
      *max = t + cubic_root((tolerance / (cp41.x - cp41.y)).abs());
      return;
    }

    let s3 = (cp41.x * cp21.y - cp41.y * cp21.x) / cp21.x.hypot(cp21.y);

    if s3 == 0.0 {
      // This means within the precision we have it can be approximated
      // infinitely by a linear segment. Deal with this by specifying the
      // approximation range as extending beyond the entire curve.
      *min = -1.0;
      *max = 2.0;
      return;
    }

    let tf = cubic_root((tolerance / s3).abs());

    *min = t - tf * (1.0 - t);
    *max = t + tf * (1.0 - t);
}

pub fn flatten_cubic_bezier<Builder: PrimitiveBuilder>(
    bezier: CubicBezierSegment<Untyped>,
    tolerance: f32,
    path: &mut Builder
) {
    let (t1, t2) = find_cubic_bezier_inflection_points(&bezier);
    let count = if t1.is_none() { 0 } else if t2.is_none() { 1 } else { 2 };
    let t1 = if let Some(t) = t1 { t } else { -1.0 };
    let t2 = if let Some(t) = t2 { t } else { -1.0 };

    // Check that at least one of the inflection points is inside [0..1]
    if count == 0 || ((t1 <= 0.0 || t1 >= 1.0) && (count == 1 || (t2 <= 0.0 || t2 >= 1.0))) {
        return flatten_cubic_bezier_segment(bezier, tolerance, path);
    }

    let mut t1min = t1;
    let mut t1max = t1;
    let mut t2min = t2;
    let mut t2max = t2;

    let mut remaining_cp = bezier;

    // For both inflection points, calulate the range where they can be linearly
    // approximated if they are positioned within [0,1]
    if count > 0 && t1 >= 0.0 && t1 < 1.0 {
        find_cubic_bezier_inflection_approximation_range(&bezier, t1, tolerance, &mut t1min, &mut t1max);
    }
    if count > 1 && t2 >= 0.0 && t2 < 1.0 {
        find_cubic_bezier_inflection_approximation_range(&bezier, t2, tolerance, &mut t2min, &mut t2max);
    }
    let mut next_bezier = bezier;
    let mut prev_bezier = bezier;

    // Process ranges. [t1min, t1max] and [t2min, t2max] are approximated by line
    // segments.
    if count == 1 && t1min <= 0.0 && t1max >= 1.0 {
        // The whole range can be approximated by a line segment.
        path.line_to(bezier.to);
        return;
    }

    if t1min > 0.0 {
        // Flatten the Bezier up until the first inflection point's approximation
        // point.
        split_cubic_bezier(&bezier, t1min, Some(&mut prev_bezier), Some(&mut remaining_cp));
        flatten_cubic_bezier_segment(prev_bezier, tolerance, path);
    }
    if t1max >= 0.0 && t1max < 1.0 && (count == 1 || t2min > t1max) {
        // The second inflection point's approximation range begins after the end
        // of the first, approximate the first inflection point by a line and
        // subsequently flatten up until the end or the next inflection point.
        split_cubic_bezier(&bezier, t1max, None, Some(&mut next_bezier));

        path.line_to(next_bezier.from);

        if count == 1 || (count > 1 && t2min >= 1.0) {
            // No more inflection points to deal with, flatten the rest of the curve.
            flatten_cubic_bezier_segment(next_bezier, tolerance, path);
            return;
        }
    } else if count > 1 && t2min > 1.0 {
        // We've already concluded t2min <= t1max, so if this is true the
        // approximation range for the first inflection point runs past the
        // end of the curve, draw a line to the end and we're done.
        path.line_to(bezier.to);
        return;
    }

    if count > 1 && t2min < 1.0 && t2max > 0.0 {
        if t2min > 0.0 && t2min < t1max {
            // In this case the t2 approximation range starts inside the t1
            // approximation range.
            split_cubic_bezier(&bezier, t1max, None, Some(&mut next_bezier));
            path.line_to(next_bezier.from);
        } else if t2min > 0.0 && t1max > 0.0 {
            split_cubic_bezier(&bezier, t1max, None, Some(&mut next_bezier));

            // Find a control points describing the portion of the curve between t1max and t2min.
            let t2mina = (t2min - t1max) / (1.0 - t1max);
            let tmp = next_bezier;
            split_cubic_bezier(&tmp, t2mina, Some(&mut prev_bezier), Some(&mut next_bezier));
            flatten_cubic_bezier_segment(prev_bezier, tolerance, path);
        } else if t2min > 0.0 {
            // We have nothing interesting before t2min, find that bit and flatten it.
            split_cubic_bezier(&bezier, t2min, Some(&mut prev_bezier), Some(&mut next_bezier));
            flatten_cubic_bezier_segment(prev_bezier, tolerance, path);
        }
        if t2max < 1.0 {
            // Flatten the portion of the curve after t2max
            split_cubic_bezier(&bezier, t2max, None, Some(&mut next_bezier));

            // Draw a line to the start, this is the approximation between t2min and
            // t2max.
            path.line_to(next_bezier.from);
            flatten_cubic_bezier_segment(next_bezier, tolerance, path);
        } else {
            // Our approximation range extends beyond the end of the curve.
            path.line_to(bezier.to);
        }
    }
}

fn flatten_cubic_bezier_segment<Builder: PrimitiveBuilder>(
    mut bezier: CubicBezierSegment<Untyped>,
    tolerance: f32,
    path: &mut Builder
) {
    let end = bezier.to;

    // The algorithm implemented here is based on:
    // http://cis.usouthal.edu/~hain/general/Publications/Bezier/Bezier%20Offset%20Curves.pdf
    //
    // The basic premise is that for a small t the third order term in the
    // equation of a cubic bezier curve is insignificantly small. This can
    // then be approximated by a quadratic equation for which the maximum
    // difference from a linear approximation can be much more easily determined.
    let mut t = 0.0;
    while t < 1.0 {
        let v1 = bezier.cp1 - bezier.from;
        let v2 = bezier.cp2 - bezier.from;

        // To remove divisions and check for divide-by-zero, this is optimized from:
        // Float s2 = (v2.x * v1.y - v2.y * v1.x) / hypot(v1.x, v1.y);
        // t = 2 * Float(sqrt(tolerance / (3. * abs(s2))));
        let v1xv2 = v2.x * v1.y - v2.y * v1.x;
        let h = v1.x.hypot(v1.y);
        if v1xv2 * h == 0.0 {
            break;
        }
        let s2inv = h / v1xv2;

        t = 2.0 * (tolerance * s2inv.abs() / 3.0).sqrt();

        if t >= 0.999 {
            break;
        }

        bezier = bezier.split_in_place(t as f32);

        path.line_to(bezier.from);
    }

    path.line_to(end);
}

#[test]
fn test_cubic_inflection_extremity() {
    use vodk_math::vec2;
    use path_builder::flattened_path_builder;

    // This curve has inflection points t1=-0.125 and t2=1.0 which used to fall
    // between the branches of flatten_cubic_bezier and not produce any vertex
    // because of t2 being exactly at the extremity of the curve.
    let mut builder = flattened_path_builder();
    builder.move_to(vec2(141.0, 135.0));
    builder.cubic_bezier_to(vec2(141.0, 130.0), vec2(140.0, 130.0),vec2(131.0, 130.0));
    builder.close();

    let path = builder.build();
    // check that
    assert!(path.num_vertices() > 2);
}