//! One-shot binary that renders a sample invoice PDF for visual inspection.
//! Run: `cargo run --bin sample_invoice_pdf -- /tmp/sample-invoice.pdf`

use saas_tauri_lib::services::invoice_pdf_service::{
    InvoicePdfCompany, InvoicePdfContext, InvoicePdfCustomer, InvoicePdfLineItem,
    InvoicePdfService, InvoicePdfTotals,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/sample-invoice.pdf".to_string());

    let ctx = InvoicePdfContext {
        company: InvoicePdfCompany {
            name: "PT Internet Cepat Nusantara".to_string(),
            address: Some("Jl. Sudirman No. 88, Jakarta Pusat 10220".to_string()),
            npwp: Some("NPWP: 01.234.567.8-901.000".to_string()),
            email: Some("billing@ispcepat.id".to_string()),
            phone: Some("+62 21 5550 1234".to_string()),
        },
        customer: InvoicePdfCustomer {
            name: "Budi Santoso".to_string(),
            address: Some("Jl. Melati No. 42, Bandung 40123".to_string()),
            email: Some("budi.santoso@example.com".to_string()),
        },
        invoice_number: "INV-2026-000123".to_string(),
        status_label: "PENDING".to_string(),
        issued_at: "31 Mei 2026".to_string(),
        due_at: "07 Juni 2026".to_string(),
        items: vec![
            InvoicePdfLineItem {
                description: "Internet Home Premium 100 Mbps - Mei 2026".to_string(),
                quantity: "1".to_string(),
                unit_price: "Rp 450.000".to_string(),
                subtotal: "Rp 450.000".to_string(),
            },
            InvoicePdfLineItem {
                description: "Static IP Address (1 IP)".to_string(),
                quantity: "1".to_string(),
                unit_price: "Rp 50.000".to_string(),
                subtotal: "Rp 50.000".to_string(),
            },
            InvoicePdfLineItem {
                description: "Biaya pemasangan awal".to_string(),
                quantity: "1".to_string(),
                unit_price: "Rp 200.000".to_string(),
                subtotal: "Rp 200.000".to_string(),
            },
        ],
        totals: InvoicePdfTotals {
            subtotal: "Rp 700.000".to_string(),
            tax_label: Some("PPN 11%".to_string()),
            tax_amount: Some("Rp 77.000".to_string()),
            grand_total: "Rp 777.000".to_string(),
        },
        payment_url: Some("https://billing.ispcepat.id/pay/abc-def-123".to_string()),
        notes: Some(
            "Mohon lakukan pembayaran sebelum tanggal jatuh tempo. Terima kasih.".to_string(),
        ),
    };

    let bytes = InvoicePdfService::new().render_invoice(&ctx)?;
    std::fs::write(&path, &bytes)?;
    println!("Wrote sample invoice PDF: {} ({} bytes)", path, bytes.len());
    Ok(())
}
