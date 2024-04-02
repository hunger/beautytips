// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 Tobias Hunger <tobias.hunger@gmail.com>

pub fn foo() {
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        foo();
        assert!(true);
    }
}
