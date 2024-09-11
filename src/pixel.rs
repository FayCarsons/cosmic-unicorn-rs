use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

#[repr(packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pixel {
    r: u8,
    g: u8,
    b: u8,
    _a: u8,
}

impl Pixel {
    pub const WHITE: Self = Pixel::new(255, 255, 255);

    pub const BLACK: Self = Pixel::new(0, 0, 0);

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Pixel { r, g, b, _a: 0 }
    }

    pub const fn splat(byte: u8) -> Self {
        Pixel {
            r: byte,
            g: byte,
            b: byte,
            _a: 0,
        }
    }

    pub fn brightness(mut self, brightness: f32) -> Self {
        let Pixel { r, g, b, .. } = self;

        #[inline]
        fn apply(byte: u8, brightness: f32) -> u8 {
            ((byte as f32) * brightness) as u8
        }

        self.r = apply(r, brightness);
        self.g = apply(g, brightness);
        self.b = apply(b, brightness);

        self
    }
}

impl Add<Pixel> for Pixel {
    type Output = Pixel;

    fn add(self, rhs: Pixel) -> Self::Output {
        let Pixel { r, g, b, .. } = self;

        Pixel::new(
            r.saturating_add(rhs.r),
            g.saturating_add(rhs.g),
            b.saturating_add(rhs.b),
        )
    }
}

impl Sub<Pixel> for Pixel {
    type Output = Pixel;

    fn sub(self, rhs: Pixel) -> Pixel {
        let Pixel { r, g, b, .. } = self;

        Pixel::new(
            r.saturating_sub(rhs.r),
            g.saturating_sub(rhs.g),
            b.saturating_sub(rhs.b),
        )
    }
}

impl Mul<Pixel> for Pixel {
    type Output = Self;

    fn mul(self, rhs: Pixel) -> Self::Output {
        let Pixel { r, g, b, .. } = self;

        Pixel::new(
            r.saturating_mul(rhs.r),
            g.saturating_mul(rhs.g),
            b.saturating_mul(rhs.b),
        )
    }
}

impl Div<Pixel> for Pixel {
    type Output = Self;

    fn div(self, rhs: Pixel) -> Self::Output {
        let Pixel { r, g, b, .. } = self;

        Pixel::new(
            r.saturating_div(rhs.r),
            g.saturating_div(rhs.g),
            b.saturating_div(rhs.b),
        )
    }
}

impl AddAssign for Pixel {
    fn add_assign(&mut self, rhs: Self) {
        let Pixel { r, g, b, .. } = rhs;

        self.r += r;
        self.g += g;
        self.b += b;
    }
}

impl SubAssign for Pixel {
    fn sub_assign(&mut self, rhs: Self) {
        let Pixel { r, g, b, .. } = rhs;

        self.r -= r;
        self.g -= g;
        self.b -= b;
    }
}

impl MulAssign for Pixel {
    fn mul_assign(&mut self, rhs: Self) {
        let Pixel { r, g, b, .. } = rhs;

        self.r *= r;
        self.g *= g;
        self.b *= b;
    }
}

impl DivAssign for Pixel {
    fn div_assign(&mut self, rhs: Self) {
        let Pixel { r, g, b, .. } = rhs;

        self.r /= r;
        self.g /= g;
        self.b /= b;
    }
}
