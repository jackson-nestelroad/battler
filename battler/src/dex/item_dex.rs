pub use crate::common::Id;
use crate::{
    dex::{
        DataLookupResult,
        DataStore,
        ResourceDex,
        ResourceLookup,
        ResourceWrapper,
    },
    items::{
        Item,
        ItemData,
    },
};

/// Lookup type for [`ItemDex`].
pub struct ItemLookup<'d> {
    data: &'d dyn DataStore,
}

impl<'d> ResourceLookup<'d, ItemData> for ItemLookup<'d> {
    fn new(data: &'d dyn DataStore) -> Self {
        Self { data }
    }

    fn lookup(&self, id: &Id) -> DataLookupResult<ItemData> {
        self.data.get_item(id)
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
