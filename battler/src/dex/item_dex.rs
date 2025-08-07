use anyhow::Result;
use battler_data::{
    DataStore,
    Id,
    ItemData,
};

use crate::{
    WrapOptionError,
    dex::{
        ResourceDex,
        ResourceLookup,
        ResourceWrapper,
    },
    items::Item,
};

/// Lookup type for [`ItemDex`].
#[derive(Clone)]
pub struct ItemLookup<'d> {
    data: &'d dyn DataStore,
}

impl<'d> ResourceLookup<'d, ItemData> for ItemLookup<'d> {
    fn new(data: &'d dyn DataStore) -> Self {
        Self { data }
    }

    fn lookup(&self, id: &Id) -> Result<ItemData> {
        self.data
            .get_item(id)?
            .wrap_not_found_error_with_format(format_args!("item {id}"))
    }
}

/// Wrapper type for [`ItemDex`].
pub struct ItemWrapper;

impl ResourceWrapper<ItemData, Item> for ItemWrapper {
    fn wrap(id: Id, data: ItemData) -> Item {
        Item::new(id, data)
    }
}

/// Indexed collection of items.
pub type ItemDex<'d> = ResourceDex<'d, ItemData, Item, ItemLookup<'d>, ItemWrapper>;
