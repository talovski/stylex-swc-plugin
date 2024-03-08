use crate::shared::constants::application_order::{Aliases, Shorthands};

use super::{order::Order, order_pair::OrderPair};

pub(crate) struct PropertySpecificity {}

impl Order for PropertySpecificity {
    fn get_expansion_fn(property: String) -> Option<fn(Option<String>) -> Vec<OrderPair>> {
        panic!("PropertySpecificity not implemented")
    }
}
