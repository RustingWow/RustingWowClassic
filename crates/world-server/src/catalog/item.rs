use sqlx::PgPool;

use super::Catalog;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ItemRow {
    pub entry: u32,
    pub class: u8,
    pub subclass: u8,
    pub name: String,
    pub display_id: u32,
    pub quality: u8,
    pub flags: u32,
    pub buy_count: u32,
    pub buy_price: u32,
    pub sell_price: u32,
    pub inventory_type: u8,
    pub item_level: u8,
    pub required_level: u8,
    pub stackable: u32,
    pub max_durability: u32,
    pub description: String,
    pub delay: u32,
    pub armor: i32,
}

impl Catalog {
    pub fn item(&self, entry: u32) -> Option<&ItemRow> {
        self.items.get(&entry)
    }

    pub fn lookup_items(&self, query: &str) -> Vec<&ItemRow> {
        super::lookup_named(self.items.values(), query, |item| item.name.as_str())
    }

    pub(crate) async fn load_items(&mut self, pool: &PgPool) -> anyhow::Result<()> {
        let rows = sqlx::query(
            "SELECT entry, class, subclass, name, display_id, quality, flags, buy_count, buy_price,
                    sell_price, inventory_type, item_level, required_level, stackable,
                    max_durability, description, delay, armor FROM items",
        )
        .fetch_all(pool)
        .await?;
        for row in rows {
            use sqlx::Row;
            let entry: i32 = row.get(0);
            self.items.insert(
                entry as u32,
                ItemRow {
                    entry: entry as u32,
                    class: row.get::<i16, _>(1) as u8,
                    subclass: row.get::<i16, _>(2) as u8,
                    name: row.get(3),
                    display_id: row.get::<i32, _>(4) as u32,
                    quality: row.get::<i16, _>(5) as u8,
                    flags: row.get::<i32, _>(6) as u32,
                    buy_count: row.get::<i16, _>(7) as u32,
                    buy_price: row.get::<i32, _>(8) as u32,
                    sell_price: row.get::<i32, _>(9) as u32,
                    inventory_type: row.get::<i16, _>(10) as u8,
                    item_level: row.get::<i16, _>(11) as u8,
                    required_level: row.get::<i16, _>(12) as u8,
                    stackable: row.get::<i32, _>(13) as u32,
                    max_durability: row.get::<i32, _>(14) as u32,
                    description: row.get(15),
                    delay: row.get::<i32, _>(16) as u32,
                    armor: row.get(17),
                },
            );
        }
        Ok(())
    }
}
