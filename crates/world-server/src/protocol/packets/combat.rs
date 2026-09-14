use tokio::io::AsyncWriteExt;
use wow_srp::vanilla_header::EncrypterHalf;
use wow_world_messages::Guid;
use wow_world_messages::vanilla::{
    DamageInfo, HitInfo, SMSG_ATTACKERSTATEUPDATE, SMSG_ATTACKSTART, SMSG_ATTACKSTOP, ServerMessage,
};

use crate::world::{Attack, MeleeHit};

pub async fn attack_start<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    attack: Attack,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_ATTACKSTART {
        attacker: Guid::new(attack.attacker),
        victim: Guid::new(attack.victim),
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

pub async fn attack_stop<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    attack: Attack,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_ATTACKSTOP {
        player: Guid::new(attack.attacker),
        enemy: Guid::new(attack.victim),
        unknown1: if attack.victim_dead { 1 } else { 0 },
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

pub async fn melee_hit<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    hit: MeleeHit,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    if hit.damage > 0 {
        SMSG_ATTACKERSTATEUPDATE {
            hit_info: HitInfo::AffectsVictim,
            attacker: Guid::new(hit.attacker),
            target: Guid::new(hit.victim),
            total_damage: hit.damage,
            damages: vec![DamageInfo {
                spell_school_mask: 1,
                damage_float: hit.damage as f32,
                damage_uint: hit.damage,
                absorb: 0,
                resist: 0,
            }],
            damage_state: 0,
            unknown1: 0,
            spell_id: 0,
            blocked_amount: 0,
        }
        .tokio_write_encrypted_server(&mut *stream, encrypter)
        .await?;
    }
    super::unit_health(
        stream,
        encrypter,
        hit.victim,
        hit.victim_health,
        hit.victim_max_health,
        hit.victim_dead,
        hit.lootable,
    )
    .await
}
