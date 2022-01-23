pub trait ArithmeticUtil<RHS = Self> {
    // TODO: 加算しつつ結果を返したほうが良さそう
    fn calc_half_carry(&self, rhs: RHS) -> bool;
    fn calc_carry(&self, rhs: RHS) -> bool;
    fn calc_half_borrow(&self, rhs: RHS) -> bool;
    fn calc_borrow(&self, rhs: RHS) -> bool;
}

impl ArithmeticUtil<u8> for u8 {
    fn calc_half_carry(&self, rhs: u8) -> bool {
        ((self & 0x0F) + (rhs & 0x0F)) & 0x10 == 0x10
    }
    fn calc_carry(&self, rhs: u8) -> bool {
        ((*self as u16 & 0x00FF) + (rhs as u16 & 0x00FF)) & 0x0100 == 0x0100
    }
    fn calc_half_borrow(&self, rhs: u8) -> bool {
        (*self & 0x0F) < (rhs & 0x0F)
    }
    fn calc_borrow(&self, rhs: u8) -> bool {
        *self < rhs
    }
}

// TODO: 正しいか自信ないのでテスト書く
impl ArithmeticUtil<u16> for u16 {
    fn calc_half_carry(&self, rhs: u16) -> bool {
        ((self & 0x000F) + (rhs & 0x000F)) & 0x0010 == 0x0010
    }
    fn calc_carry(&self, rhs: u16) -> bool {
        ((*self as u32 & 0x0000FFFF) + (rhs as u32 & 0x0000FFFF)) & 0x00010000 == 0x00010000
    }
    fn calc_half_borrow(&self, rhs: u16) -> bool {
        (*self & 0x00FF) < (rhs & 0x00FF)
    }
    fn calc_borrow(&self, rhs: u16) -> bool {
        *self < rhs
    }
}

pub trait ToSigned {
    fn to_signed_u16(&self) -> u16;
    fn to_unsigned_u16(&self) -> u16;
}

impl ToSigned for u8 {
    fn to_signed_u16(&self) -> u16 {
        if *self & 0x80 == 0 {
            *self as u16
        } else {
            *self as u16 | 0xFF00
        }
    }
    fn to_unsigned_u16(&self) -> u16 {
        *self as u16
    }
}

pub trait AddSigned {
    fn add_signed_u16(&self, rhs: u16) -> u16;
    fn add_signed_u8(&self, rhs: u8) -> u16;
}

impl AddSigned for u16 {
    fn add_signed_u16(&self, rhs: u16) -> u16 {
        (*self).wrapping_add(rhs)
    }

    fn add_signed_u8(&self, rhs: u8) -> u16 {
        (*self).wrapping_add(rhs.to_signed_u16())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u8_calc_half_carry() {
        assert_eq!(0b0000_1000u8.calc_half_carry(0b0000_1000), true);
        assert_eq!(0b0000_1110u8.calc_half_carry(0b0000_0001), false);
        assert_eq!(0b0000_1111u8.calc_half_carry(0b0000_0001), true);
    }

    #[test]
    fn test_u8_calc_carry() {
        assert_eq!(0b1000_0000u8.calc_carry(0b1000_0000), true);
        assert_eq!(0b1111_1110u8.calc_carry(0b0000_0001), false);
        assert_eq!(0b1111_1111u8.calc_carry(0b0000_0001), true);
    }

    #[test]
    fn test_u8_calc_half_borrow() {
        assert_eq!(0b0000_1000u8.calc_half_borrow(0b0000_1000), false);
        assert_eq!(0b0000_0000u8.calc_half_borrow(0b0000_0001), true);
        assert_eq!(0b0000_1000u8.calc_half_borrow(0b0000_1001), true);
        assert_eq!(0b0000_1110u8.calc_half_borrow(0b0000_1111), true);
    }

    #[test]
    fn test_u8_calc_borrow() {
        assert_eq!(0b1000_0000u8.calc_borrow(0b1000_0000), false);
        assert_eq!(0b0000_0000u8.calc_borrow(0b0000_0001), true);
        assert_eq!(0b1000_0000u8.calc_borrow(0b1000_0001), true);
        assert_eq!(0b1111_1110u8.calc_borrow(0b1111_1111), true);
    }

    #[test]
    fn test_to_signed_u16() {
        assert_eq!((0 as u8).to_signed_u16(), 0);
        assert_eq!((10 as u8).to_signed_u16(), 10);
        assert_eq!((-10 as i8 as u8).to_signed_u16(), 65526);
    }

    #[test]
    fn test_to_unsigned_u16() {
        assert_eq!((0 as u8).to_unsigned_u16(), 0);
        assert_eq!((10 as u8).to_unsigned_u16(), 10);
        assert_eq!((-10 as i8 as u8).to_unsigned_u16(), 246);
    }

    #[test]
    fn test_add_signed_u8() {
        assert_eq!(7u16.add_signed_u8(5), 12);
        assert_eq!(4u16.add_signed_u8(-3 as i8 as u8), 1);
        assert_eq!(300u16.add_signed_u8(-4 as i8 as u8), 296);
    }
}
