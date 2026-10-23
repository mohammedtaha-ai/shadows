//! Image upload validation and public attachment values.
use super::{ThreadId, Threads};
use crate::db::CommandContext;
use crate::db::id::newtype_id;
use crate::{CoreError, ErrorCode};
use image::{AnimationDecoder, ImageDecoder};
use sha2::{Digest, Sha256};

newtype_id! {
    /// An immutable image owned by Threads.
    AttachmentId
}

pub const MAX_IMAGE_BYTES: usize = 5_242_880;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct AttachmentRef {
    pub id: AttachmentId,
    pub media_type: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct Attachment {
    pub id: AttachmentId,
    pub media_type: String,
    pub byte_len: u64,
    pub name: Option<String>,
}

impl Threads {
    /// Validates the complete image before the upload command can write anything.
    pub async fn upload_attachment(
        &self,
        command_id: String,
        thread: &ThreadId,
        claimed_type: &str,
        name: Option<String>,
        bytes: Vec<u8>,
    ) -> Result<Attachment, CoreError> {
        let invalid = |message: &str| CoreError::Refused {
            code: ErrorCode::InvalidCommand,
            message: message.into(),
        };
        if bytes.len() > MAX_IMAGE_BYTES {
            return Err(invalid("Images must be at most 5 MiB (5242880 bytes)"));
        }
        if command_id.len() > 256 || name.as_ref().is_some_and(|n| n.len() > 255) {
            return Err(invalid("Attachment metadata is too long"));
        }
        let claimed_type = claimed_type.to_owned();
        let (media_type, digest, bytes) = tokio::task::spawn_blocking(move || {
            let media_type = checked_type(&claimed_type, &bytes)?;
            let digest = Sha256::digest(&bytes).to_vec();
            Ok::<_, CoreError>((media_type, digest, bytes))
        })
        .await
        .map_err(|_| invalid("Image validation failed"))??;
        let command = CommandContext::user(
            command_id,
            "thread.attachment",
            serde_json::json!({
                "thread_id": thread, "media_type": media_type, "byte_len": bytes.len(),
                "sha256": digest, "name": name,
            }),
        );
        self.store
            .upload_attachment(&command, thread, media_type, name, digest, bytes)
            .await
    }

    /// Reads bytes only through a live thread's ownership reference.
    pub async fn attachment(
        &self,
        thread: &ThreadId,
        id: &AttachmentId,
    ) -> Result<(Attachment, Vec<u8>), CoreError> {
        self.store.read_attachment(thread, id).await
    }
}

fn checked_type(claimed_type: &str, bytes: &[u8]) -> Result<&'static str, CoreError> {
    let invalid = |message: &str| CoreError::Refused {
        code: ErrorCode::InvalidCommand,
        message: message.into(),
    };
    let format =
        image::guess_format(bytes).map_err(|_| invalid("This file type is not supported"))?;
    let media_type = match format {
        image::ImageFormat::Png => "image/png",
        image::ImageFormat::Jpeg => "image/jpeg",
        image::ImageFormat::Gif => "image/gif",
        image::ImageFormat::WebP => "image/webp",
        _ => return Err(invalid("This file type is not supported")),
    };
    if claimed_type != media_type {
        return Err(invalid("This file type is not supported"));
    }
    // Bound decoder allocations independently of the compressed upload size.
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(64 * 1024 * 1024);
    if format == image::ImageFormat::Gif {
        let mut decoder = image::codecs::gif::GifDecoder::new(std::io::Cursor::new(bytes))
            .map_err(|_| invalid("This file type is not supported"))?;
        decoder
            .set_limits(limits.clone())
            .map_err(|_| invalid("This file type is not supported"))?;
        checked_animation(decoder.into_frames(), limits)?;
        return Ok(media_type);
    }
    if format == image::ImageFormat::Png {
        let decoder = image::codecs::png::PngDecoder::with_limits(
            std::io::Cursor::new(bytes),
            limits.clone(),
        )
        .map_err(|_| invalid("This file type is not supported"))?;
        if decoder
            .is_apng()
            .map_err(|_| invalid("This file type is not supported"))?
        {
            let animation = decoder
                .apng()
                .map_err(|_| invalid("This file type is not supported"))?;
            checked_animation(animation.into_frames(), limits)?;
            return Ok(media_type);
        }
    }
    if format == image::ImageFormat::WebP {
        let mut decoder = image::codecs::webp::WebPDecoder::new(std::io::Cursor::new(bytes))
            .map_err(|_| invalid("This file type is not supported"))?;
        decoder
            .set_limits(limits.clone())
            .map_err(|_| invalid("This file type is not supported"))?;
        if decoder.has_animation() {
            let (width, height) = decoder.dimensions();
            // WebP's frame iterator does not enforce an allocation budget itself.
            // Reserve for its canvas, frame, output and color conversion buffers.
            let mut buffers = limits.clone();
            for _ in 0..4 {
                buffers
                    .reserve_buffer(width, height, image::ColorType::Rgba8)
                    .map_err(|_| invalid("This file type is not supported"))?;
            }
            checked_animation(decoder.into_frames(), limits)?;
            return Ok(media_type);
        }
    }
    let mut reader = image::ImageReader::with_format(std::io::Cursor::new(bytes), format);
    reader.limits(limits);
    reader
        .decode()
        .map_err(|_| invalid("This file type is not supported"))?;
    Ok(media_type)
}

fn checked_animation(
    mut frames: image::Frames<'_>,
    mut limits: image::Limits,
) -> Result<(), CoreError> {
    // A valid first frame cannot vouch for the rest of an animation.
    let valid = frames
        .try_fold(false, |_, frame| {
            let frame = frame?;
            // Bound total decoding work as well as simultaneous frame allocations.
            limits.reserve(frame.buffer().len() as u64)?;
            Ok::<_, image::ImageError>(true)
        })
        .unwrap_or(false);
    if valid {
        Ok(())
    } else {
        Err(CoreError::Refused {
            code: ErrorCode::InvalidCommand,
            message: "This file type is not supported".into(),
        })
    }
}
