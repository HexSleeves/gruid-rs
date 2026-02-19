//! Combat system — damage calculation matching Go shamogu.

use rand::{Rng, RngExt};

/// Maximum damage per individual attack roll.
const MAX_ATTACK_DAMAGE: i32 = 3;

/// Attack success probability table (indexed by attack stat).
const PROB_A: [i32; 12] = [0, 35, 44, 52, 59, 66, 72, 78, 84, 90, 95, 100];
/// Defense success probability table (indexed by defense stat).
const PROB_D: [i32; 10] = [0, 29, 42, 53, 62, 70, 76, 81, 86, 90];

/// Compute raw damage from attack vs defense using the probability tables.
///
/// Algorithm:
/// 1. 5% guaranteed miss chance
/// 2. Roll 3 attack dice, each has probA[attack]% to add +1
/// 3. Roll min(2, dmg) defense dice, each has probD[defense]% to subtract 1
/// 4. Clamp to [0, attack]
pub fn compute_damage(rng: &mut impl Rng, attack: i32, defense: i32) -> i32 {
    // 5% miss
    if rng.random_range(0..100) < 5 {
        return 0;
    }

    let atk_idx = (attack as usize).min(PROB_A.len() - 1);
    let def_idx = (defense as usize).min(PROB_D.len() - 1);

    // Attack rolls
    let mut dmg = 0;
    for _ in 0..MAX_ATTACK_DAMAGE {
        if rng.random_range(0..100) < PROB_A[atk_idx] {
            dmg += 1;
        }
    }

    // Defense rolls
    let def_rolls = dmg.min(2);
    for _ in 0..def_rolls {
        if rng.random_range(0..100) < PROB_D[def_idx] {
            dmg -= 1;
        }
    }

    dmg.clamp(0, attack)
}
