//! Money: one number of copper pieces, the smallest coin (owner, 2026-09-27; save schema 5).
//! Gold and silver are views of it: shown as whole gold rounded down, or broken out by coin.
//! Pack files keep gold where a designer thinks in gold (starting purses, monster drops) and
//! the simulation converts where it reads them.

/// Copper pieces in one silver piece.
pub const CP_PER_SP: u32 = 10;
/// Copper pieces in one gold piece.
pub const CP_PER_GP: u32 = 100;

/// Whole gold pieces as copper, saturating at the top of the purse.
#[must_use]
pub const fn from_gp(gp: u32) -> u32 {
    gp.saturating_mul(CP_PER_GP)
}

/// Copper as whole gold pieces, rounded down: a display never promises more than there is.
#[must_use]
pub const fn gp_floor(cp: u32) -> u32 {
    cp / CP_PER_GP
}

/// An amount of copper broken out by coin, largest first. A view, not a purse: the party holds
/// one number, and any amount breaks out the same way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Coins {
    /// Gold pieces.
    pub gp: u32,
    /// Silver pieces, 0 to 9.
    pub sp: u32,
    /// Copper pieces, 0 to 9.
    pub cp: u32,
}

impl Coins {
    /// The breakdown of `cp` copper pieces.
    #[must_use]
    pub const fn of(cp: u32) -> Coins {
        Coins {
            gp: cp / CP_PER_GP,
            sp: cp % CP_PER_GP / CP_PER_SP,
            cp: cp % CP_PER_SP,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copper_breaks_out_by_coin() {
        assert_eq!(
            Coins::of(1537),
            Coins {
                gp: 15,
                sp: 3,
                cp: 7
            }
        );
        assert_eq!(
            Coins::of(0),
            Coins {
                gp: 0,
                sp: 0,
                cp: 0
            }
        );
        assert_eq!(
            Coins::of(9),
            Coins {
                gp: 0,
                sp: 0,
                cp: 9
            }
        );
        assert_eq!(
            Coins::of(90),
            Coins {
                gp: 0,
                sp: 9,
                cp: 0
            }
        );
        assert_eq!(
            Coins::of(u32::MAX),
            Coins {
                gp: 42_949_672,
                sp: 9,
                cp: 5
            }
        );
    }

    #[test]
    fn gold_rounds_down_and_converts_back() {
        assert_eq!(gp_floor(1599), 15, "never rounds up");
        assert_eq!(gp_floor(99), 0);
        assert_eq!(gp_floor(from_gp(15)), 15);
        assert_eq!(from_gp(15), 1500);
        assert_eq!(from_gp(u32::MAX), u32::MAX, "saturates");
    }
}
