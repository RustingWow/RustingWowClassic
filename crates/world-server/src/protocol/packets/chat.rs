use tokio::io::AsyncWriteExt;
use wow_srp::vanilla_header::EncrypterHalf;
use wow_world_messages::Guid;
use wow_world_messages::vanilla::{
    Language, PlayerChatTag, SMSG_CHAT_PLAYER_NOT_FOUND, SMSG_MESSAGECHAT,
    SMSG_MESSAGECHAT_ChatType, SMSG_NOTIFICATION, ServerMessage,
};

use crate::world::{ChatDelivery, SpokenChat};

pub async fn chat_message<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    spoken: &SpokenChat,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    let guid = Guid::new(spoken.from_guid);
    let chat_type = match spoken.delivery {
        ChatDelivery::Say => SMSG_MESSAGECHAT_ChatType::Say {
            chat_credit: guid,
            speech_bubble_credit: guid,
        },
        ChatDelivery::Yell => SMSG_MESSAGECHAT_ChatType::Yell {
            chat_credit: guid,
            speech_bubble_credit: guid,
        },
        ChatDelivery::Emote => SMSG_MESSAGECHAT_ChatType::Emote { sender2: guid },
        ChatDelivery::Whisper => SMSG_MESSAGECHAT_ChatType::Whisper { sender2: guid },
        ChatDelivery::WhisperInform => SMSG_MESSAGECHAT_ChatType::WhisperInform { sender2: guid },
        ChatDelivery::System => SMSG_MESSAGECHAT_ChatType::System {
            sender2: Guid::new(0),
        },
    };
    SMSG_MESSAGECHAT {
        chat_type,
        language: Language::Universal,
        message: spoken.text.clone(),
        tag: if spoken.gm_tag {
            PlayerChatTag::Gm
        } else {
            PlayerChatTag::None
        },
    }
    .tokio_write_encrypted_server(stream, encrypter)
    .await?;
    Ok(())
}

pub async fn notification<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    text: String,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_NOTIFICATION { notification: text }
        .tokio_write_encrypted_server(stream, encrypter)
        .await?;
    Ok(())
}

pub async fn chat_player_not_found<W>(
    stream: &mut W,
    encrypter: &mut EncrypterHalf,
    name: String,
) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin + Send,
{
    SMSG_CHAT_PLAYER_NOT_FOUND { name }
        .tokio_write_encrypted_server(stream, encrypter)
        .await?;
    Ok(())
}
