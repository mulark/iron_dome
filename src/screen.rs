#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Default)]
pub struct BoundingBox {
    pub left_top: Coord,
    pub right_bottom: Coord,
}

impl BoundingBox {
    pub fn new(left_top: Coord, w: i64, h: i64) -> Self {
        Self {
            left_top,
            right_bottom: Coord {
                w: left_top.w + w,
                h: left_top.h + h,
            },
        }
    }

    pub fn enumerate(&self) -> impl Iterator<Item = (i64, i64)> {
        let ltw = self.left_top.w;
        let rbw = self.right_bottom.w;
        let lth = self.left_top.h;
        let rbh = self.right_bottom.h;
        (lth..=rbh).flat_map(move |h| (ltw..=rbw).map(move |w| (w, h)))
    }

    pub fn collides_with_circle(&self, pos: Coord, radius: u32) -> bool {
        if !self.is_close_enough_to_collide(pos, radius as i64) {
            return false;
        }
        let (mut testx, mut testy) = (pos.w, pos.h);
        // Find closest edge
        if self.left_top.w > pos.w {
            // left of the leftmost point
            testx = self.left_top.w;
        } else if self.right_bottom.w < pos.w {
            // right of the rightmost point
            testx = self.right_bottom.w;
        }
        if self.left_top.h > pos.h {
            // Top of the topmost point
            testy = self.left_top.h;
        } else if self.right_bottom.h < pos.h {
            // Lower than the lowest point
            testy = self.right_bottom.h;
        }
        let distx = pos.w - testx;
        let disty = pos.h - testy;
        let distsq = (distx.pow(2) + disty.pow(2)) as f64;
        let dist = f64::sqrt(distsq);
        if dist as u32 <= radius {
            return true;
        }
        false
    }

    pub fn is_close_enough_to_collide(&self, pos: Coord, radius: i64) -> bool {
        pos.w > self.left_top.w - radius
            && pos.w < self.right_bottom.w + radius
            && pos.h > self.left_top.h - radius
            && pos.h < self.right_bottom.w + radius
    }

    pub fn area(&self) -> i64 {
        self.w() * self.h()
    }

    pub fn w(&self) -> i64 {
        self.right_bottom.w - self.left_top.w
    }

    pub fn h(&self) -> i64 {
        self.right_bottom.h - self.left_top.h
    }

    pub fn ratio(&self) -> f64 {
        self.w() as f64 / self.h() as f64
    }
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Default)]
pub struct Coord {
    pub w: i64,
    pub h: i64,
}
