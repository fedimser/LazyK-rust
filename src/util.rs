pub(crate) enum NumRepr {
    Pow(usize, usize),
    Mul(usize, usize),
    Inc(usize),
}

/// Finds the optimal way to represent Church numeral using lower numerals.
pub(crate) fn num_repr(x: u16) -> NumRepr {
    match x {
        8 => NumRepr::Pow(2, 3),
        16 => NumRepr::Pow(2, 4),
        32 => NumRepr::Pow(2, 5),
        64 => NumRepr::Pow(2, 6),
        128 => NumRepr::Pow(2, 7),
        256 => NumRepr::Pow(2, 8),
        27 => NumRepr::Pow(3, 3),
        81 => NumRepr::Pow(3, 4),
        125 => NumRepr::Pow(5, 3),
        216 => NumRepr::Pow(6, 3),
        _ => num_repr_no_high_powers(x),
    }
}

fn num_repr_no_high_powers(x: u16) -> NumRepr {
    // Try squares.
    for d in 2..15 {
        if d * d == x {
            return NumRepr::Pow(d as usize, 2);
        }
    }
    // Try products of 2 numbers.
    let sqrt = (x as f32).sqrt().floor() as u16;
    for d in (2..=sqrt).rev() {
        if x % d == 0 {
            return NumRepr::Mul(d as usize, (x/d) as usize);
        }
    }
    NumRepr::Inc((x - 1) as usize)
}
