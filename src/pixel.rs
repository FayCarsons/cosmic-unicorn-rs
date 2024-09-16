use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

pub trait RGB {
    fn to_rgb_packed(&self) -> &u32;
    fn to_rgb(&self) -> [u8; 3];
}

#[repr(packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pixel {
    r: u8,
    g: u8,
    b: u8,
    _blank: u8,
}

impl RGB for Pixel {
    fn to_rgb_packed(&self) -> &u32 {
        unsafe { core::mem::transmute::<&Self, &u32>(self) }
    }

    fn to_rgb(&self) -> [u8; 3] {
        [self.r, self.g, self.b]
    }
}

impl Pixel {
    pub const WHITE: Self = Pixel::splat(u8::MAX);
    pub const BLACK: Self = Pixel::splat(0);

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Pixel { r, g, b, _blank: 0 }
    }

    pub const fn splat(value: u8) -> Self {
        Pixel {
            r: value,
            g: value,
            b: value,
            _blank: 0,
        }
    }

    pub fn brightness(mut self, brightness: f32) -> Self {
        let Pixel { r, g, b, .. } = self;

        #[inline]
        fn apply(value: u8, brightness: f32) -> u8 {
            ((value as f32) * brightness) as u8
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
        Pixel::new(
            self.r.saturating_add(rhs.r),
            self.g.saturating_add(rhs.g),
            self.b.saturating_add(rhs.b),
        )
    }
}

impl Sub<Pixel> for Pixel {
    type Output = Pixel;

    fn sub(self, rhs: Pixel) -> Pixel {
        Pixel::new(
            self.r.saturating_sub(rhs.r),
            self.g.saturating_sub(rhs.g),
            self.b.saturating_sub(rhs.b),
        )
    }
}

impl Mul<Pixel> for Pixel {
    type Output = Self;

    fn mul(self, rhs: Pixel) -> Self::Output {
        Pixel::new(
            self.r.saturating_mul(rhs.r),
            self.g.saturating_mul(rhs.g),
            self.b.saturating_mul(rhs.b),
        )
    }
}

impl Div<Pixel> for Pixel {
    type Output = Self;

    fn div(self, rhs: Pixel) -> Self::Output {
        Pixel::new(
            self.r.saturating_div(rhs.r),
            self.g.saturating_div(rhs.g),
            self.b.saturating_div(rhs.b),
        )
    }
}

impl AddAssign for Pixel {
    fn add_assign(&mut self, rhs: Self) {
        self.r = self.r.saturating_add(rhs.r);
        self.g = self.g.saturating_add(rhs.g);
        self.b = self.b.saturating_add(rhs.b);
    }
}

impl SubAssign for Pixel {
    fn sub_assign(&mut self, rhs: Self) {
        self.r = self.r.saturating_sub(rhs.r);
        self.g = self.g.saturating_sub(rhs.g);
        self.b = self.b.saturating_sub(rhs.b);
    }
}

impl MulAssign for Pixel {
    fn mul_assign(&mut self, rhs: Self) {
        self.r = self.r.saturating_mul(rhs.r);
        self.g = self.g.saturating_mul(rhs.g);
        self.b = self.b.saturating_mul(rhs.b);
    }
}

impl DivAssign for Pixel {
    fn div_assign(&mut self, rhs: Self) {
        self.r = self.r.saturating_div(rhs.r);
        self.g = self.g.saturating_div(rhs.g);
        self.b = self.b.saturating_div(rhs.b);
    }
}
