use super::*;
use wow_shared::{
    Appearance, CharacterArea, CharacterClass, CharacterGender, CharacterMap, CharacterRace,
    Position,
};

fn draft(
    account_id: i64,
    name: &str,
    race: CharacterRace,
    class: CharacterClass,
) -> CharacterDraft {
    CharacterDraft {
        account_id,
        name: name.to_string(),
        race,
        class,
        gender: CharacterGender::Male,
        appearance: Appearance::default(),
    }
}

#[tokio::test]
async fn creates_human_at_northshire() {
    let store = CharacterStore::memory();
    let character = store
        .create(draft(
            1,
            "Arthas",
            CharacterRace::Human,
            CharacterClass::Warrior,
        ))
        .await
        .unwrap();
    assert_eq!(character.name, "Arthas");
    assert_eq!(character.map_id, CharacterMap::EasternKingdoms);
    assert_eq!(character.position, Position::NORTHSHIRE);
    assert_eq!(character.race, CharacterRace::Human);
    assert_eq!(character.area, CharacterArea::NorthshireValley);
    assert!(character.first_login);

    let listed = store.list(1).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].guid, character.guid);
}

#[tokio::test]
async fn orc_and_night_elf_start_on_kalimdor() {
    let store = CharacterStore::memory();
    let orc = store
        .create(draft(
            1,
            "Thrall",
            CharacterRace::Orc,
            CharacterClass::Warrior,
        ))
        .await
        .unwrap();
    let night_elf = store
        .create(draft(
            1,
            "Tyrande",
            CharacterRace::NightElf,
            CharacterClass::Hunter,
        ))
        .await
        .unwrap();
    assert_eq!(orc.map_id, CharacterMap::Kalimdor);
    assert_eq!(orc.area, CharacterArea::ValleyOfTrials);
    assert_eq!(night_elf.map_id, CharacterMap::Kalimdor);
    assert_eq!(night_elf.race, CharacterRace::NightElf);
    assert_eq!(night_elf.class, CharacterClass::Hunter);
    assert_eq!(night_elf.gender, CharacterGender::Male);
    assert_eq!(night_elf.area, CharacterArea::Shadowglen);
    assert_ne!(orc.position, Position::NORTHSHIRE);
}

#[tokio::test]
async fn rejects_invalid_name_and_duplicate() {
    let store = CharacterStore::memory();
    let error = store
        .create(draft(1, "A", CharacterRace::Human, CharacterClass::Warrior))
        .await
        .unwrap_err();
    assert!(matches!(error, CreateCharacterError::InvalidName));

    store
        .create(draft(
            1,
            "Jaina",
            CharacterRace::Human,
            CharacterClass::Warrior,
        ))
        .await
        .unwrap();
    let error = store
        .create(draft(
            2,
            "jaina",
            CharacterRace::Human,
            CharacterClass::Warrior,
        ))
        .await
        .unwrap_err();
    assert!(matches!(error, CreateCharacterError::NameInUse));
}

#[tokio::test]
async fn rejects_disabled_race_class() {
    let store = CharacterStore::memory();
    let error = store
        .create(draft(
            1,
            "Paladin",
            CharacterRace::Orc,
            CharacterClass::Paladin,
        ))
        .await
        .unwrap_err();
    assert!(matches!(error, CreateCharacterError::Disabled));
}

#[tokio::test]
async fn save_position_survives_reload() {
    let store = CharacterStore::memory();
    let created = store
        .create(draft(
            1,
            "Bolvar",
            CharacterRace::Human,
            CharacterClass::Warrior,
        ))
        .await
        .unwrap();
    let moved = Position {
        x: created.position.x + 10.0,
        y: created.position.y + 4.0,
        z: created.position.z,
        orientation: 1.5,
    };
    store
        .save_position(created.guid, CharacterMap::EasternKingdoms, moved)
        .await
        .unwrap();
    store.mark_entered_world(created.guid).await.unwrap();

    let loaded = store.get(1, created.guid).await.unwrap().unwrap();
    assert_eq!(loaded.position, moved);
    assert!(!loaded.first_login);
}

#[tokio::test]
async fn delete_removes_character() {
    let store = CharacterStore::memory();
    let created = store
        .create(draft(
            1,
            "Varian",
            CharacterRace::Human,
            CharacterClass::Warrior,
        ))
        .await
        .unwrap();
    assert!(store.delete(1, created.guid).await.unwrap());
    assert!(store.list(1).await.unwrap().is_empty());
    assert!(!store.delete(1, created.guid).await.unwrap());
}

#[tokio::test]
async fn account_limit_is_ten() {
    let store = CharacterStore::memory();
    for index in 0..10 {
        let name = format!("Hero{}", (b'a' + index) as char);
        store
            .create(draft(
                7,
                &name,
                CharacterRace::Human,
                CharacterClass::Warrior,
            ))
            .await
            .unwrap();
    }
    let error = store
        .create(draft(
            7,
            "Heroz",
            CharacterRace::Human,
            CharacterClass::Warrior,
        ))
        .await
        .unwrap_err();
    assert!(matches!(error, CreateCharacterError::AccountLimit));
}
