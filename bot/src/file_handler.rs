use crate::csv_processor::process_csv;
use crate::telegram::{Document, TelegramClient};
use crate::xlsx_processor::process_xlsx;

pub async fn handle_file_document(
    telegram: &TelegramClient,
    chat_id: i64,
    document: &Document,
) -> Result<(), Box<dyn std::error::Error>> {
    let filename = document.file_name.as_deref().unwrap_or("input");
    let lower = filename.to_lowercase();
    if lower.ends_with(".csv") {
        handle_csv_document(telegram, chat_id, filename, document).await
    } else if lower.ends_with(".xlsx") {
        handle_xlsx_document(telegram, chat_id, filename, document).await
    } else {
        telegram
            .send_message(chat_id, "Hozircha faqat .csv va .xlsx fayl qabul qilinadi.")
            .await
            .map(|_| ())
    }
}

async fn handle_csv_document(
    telegram: &TelegramClient,
    chat_id: i64,
    filename: &str,
    document: &Document,
) -> Result<(), Box<dyn std::error::Error>> {
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
                    "text/csv",
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

async fn handle_xlsx_document(
    telegram: &TelegramClient,
    chat_id: i64,
    filename: &str,
    document: &Document,
) -> Result<(), Box<dyn std::error::Error>> {
    let progress_message_id = telegram
        .send_message(chat_id, "Excel qabul qilindi. Hisoblayapman...")
        .await?;
    let bytes = telegram.download_file(&document.file_id).await?;
    match process_xlsx(&bytes) {
        Ok(report) => {
            telegram
                .send_document(
                    chat_id,
                    &output_xlsx_name(filename),
                    report.output,
                    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                    &format!(
                        "Hisoblandi: {} ta. OK: {}, XATO: {}.",
                        report.processed_count, report.ok_count, report.error_count
                    ),
                )
                .await?;
            let _ = telegram
                .edit_message(chat_id, progress_message_id, "Excel hisoblandi.")
                .await;
        }
        Err(error) => {
            telegram
                .edit_message(
                    chat_id,
                    progress_message_id,
                    &format!("Excel hisoblashda xato: {error}"),
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

fn output_xlsx_name(filename: &str) -> String {
    let stem = filename
        .strip_suffix(".xlsx")
        .or_else(|| filename.strip_suffix(".XLSX"))
        .unwrap_or(filename);
    format!("{stem}_hisoblangan.xlsx")
}
