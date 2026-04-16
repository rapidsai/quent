// `rate` and `occupancy` are reserved keywords in capacity blocks.
quent_model::resource! {
    Bad {
        capacity: { slots: u64, rate: u64 },
    }
}

fn main() {}
