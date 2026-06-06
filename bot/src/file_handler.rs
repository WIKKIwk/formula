use crate::csv_processor::process_csv;
use crate::telegram::{Document, TelegramClient};

pub async fn handle_csv_document(
    telegram: &TelegramClient,
    chat_id: i64,
    document: &Document,
) -> Result<(), Box<dyn std::error::Error>> {
    let filename = document.file_name.as_deref().unwrap_or("input.csv");
    if !filename.to_lowercase().ends_with(".csv") {
        telegram
            .send_message(chat_id, "Hozircha faqat .csv fayl qabul qilinadi.")
            .await?;
        return Ok(());
    }

    let progress_message_id = telegram
        .send_message(chat_id, "CSV qabul qilindi. Hisoblayapman...")
        .await?;
    let bytes = telegram.download_file(&document.file_id).await?;
    match process_csv(&bytes) {
        Ok(report) => {
            telegram
                .send_document(
                    chat_id,
                    &output_csv_name(filename),
                    report.output,
                    &format!(
                        "Hisoblandi: {} ta. OK: {}, XATO: {}.",
                        report.processed_count, report.ok_count, report.error_count
                    ),
                )
                .await?;
            let _ = telegram
                .edit_message(chat_id, progress_message_id, "CSV hisoblandi.")
                .await;
        }
        Err(error) => {
            telegram
                .edit_message(
                    chat_id,
                    progress_message_id,
                    &format!("CSV hisoblashda xato: {error}"),
                )
                .await?;
        }
    }
    Ok(())
}

fn output_csv_name(filename: &str) -> String {
    let stem = filename
        .strip_suffix(".csv")
        .or_else(|| filename.strip_suffix(".CSV"))
        .unwrap_or(filename);
    format!("{stem}_hisoblangan.csv")
}
