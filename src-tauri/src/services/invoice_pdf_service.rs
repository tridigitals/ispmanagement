//! Invoice PDF generation service.
//!
//! Pure-Rust PDF generator using `printpdf 0.9` for invoice attachments
//! sent in bulk-send-invoice and similar flows. Built on the `Op` enum
//! stack-based API (text sections + cursor positioning + builtin fonts).
//!
//! Layout (single page, A4 portrait, mm units):
//!   - Header (top): company name + address + NPWP (left), big "INVOICE" +
//!     invoice number + status badge text (right)
//!   - Sub-header: bill-to (customer) on left, dates (issued, due) on right
//!   - Body: items table with description / qty / unit price / subtotal
//!   - Footer: totals (subtotal, tax, grand total) + payment URL
//!
//! Fonts: built-in Helvetica family (no external font files for v1).
//! Caveat: Helvetica's encoding is WinAnsi (Latin-1). It covers Indonesian
//! text and the "Rp" string fine, but not most non-Latin scripts. If the
//! invoice surfaces non-Latin characters, embed a TTF (e.g. DejaVu Sans)
//! via `PdfDocument::add_ttf_font` in a follow-up.

use crate::error::{AppError, AppResult};
use printpdf::{
    BuiltinFont, Color, Mm, Op, PdfDocument, PdfFontHandle, PdfPage, PdfSaveOptions, Point, Pt,
    Rgb, TextItem,
};

/// One line item on the invoice (description + qty + unit price + subtotal,
/// all formatted by the caller — keeps this service formatting-agnostic).
#[derive(Debug, Clone)]
pub struct InvoicePdfLineItem {
    pub description: String,
    pub quantity: String,
    pub unit_price: String,
    pub subtotal: String,
}

/// Tenant company information rendered in the header.
#[derive(Debug, Clone, Default)]
pub struct InvoicePdfCompany {
    pub name: String,
    pub address: Option<String>,
    pub npwp: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
}

/// Customer (bill-to) information.
#[derive(Debug, Clone, Default)]
pub struct InvoicePdfCustomer {
    pub name: String,
    pub address: Option<String>,
    pub email: Option<String>,
}

/// Pre-formatted total values. The caller uses tenant currency settings.
#[derive(Debug, Clone, Default)]
pub struct InvoicePdfTotals {
    pub subtotal: String,
    pub tax_label: Option<String>, // e.g. "PPN 11%"
    pub tax_amount: Option<String>,
    pub grand_total: String,
}

/// Full rendering context for one invoice page.
#[derive(Debug, Clone)]
pub struct InvoicePdfContext {
    pub company: InvoicePdfCompany,
    pub customer: InvoicePdfCustomer,
    pub invoice_number: String,
    pub status_label: String, // e.g. "PENDING", "PAID"
    pub issued_at: String,    // pre-formatted date string
    pub due_at: String,       // pre-formatted date string
    pub items: Vec<InvoicePdfLineItem>,
    pub totals: InvoicePdfTotals,
    pub payment_url: Option<String>, // shown as text under totals (link annotations: future)
    pub notes: Option<String>,
}

/// Stateless PDF generator. Cheap to clone (zero fields).
#[derive(Debug, Clone, Default)]
pub struct InvoicePdfService;

impl InvoicePdfService {
    pub fn new() -> Self {
        Self
    }

    /// Render the invoice context into a PDF byte buffer.
    pub fn render_invoice(&self, ctx: &InvoicePdfContext) -> AppResult<Vec<u8>> {
        if ctx.invoice_number.trim().is_empty() {
            return Err(AppError::Validation(
                "invoice_number is required".to_string(),
            ));
        }

        let mut ops: Vec<Op> = Vec::new();

        // ---------- Page geometry (A4 portrait) ----------
        let page_w_mm = 210.0_f32;
        let page_h_mm = 297.0_f32;
        let left_mm = 18.0_f32;
        let right_mm = page_w_mm - 18.0_f32;
        let _content_w_mm = right_mm - left_mm;

        // ---------- HEADER ----------
        // Company name (left, big bold)
        push_text(
            &mut ops,
            Mm(left_mm),
            Mm(280.0),
            BuiltinFont::HelveticaBold,
            Pt(16.0),
            Pt(18.0),
            black(),
            &ctx.company.name,
        );

        let mut header_y = 273.0_f32;
        for line in [
            ctx.company.address.as_deref(),
            ctx.company.npwp.as_deref().map(|n| {
                // borrow-friendly fallback
                if !n.is_empty() {
                    n
                } else {
                    ""
                }
            }),
            ctx.company.email.as_deref(),
            ctx.company.phone.as_deref(),
        ]
        .iter()
        .flatten()
        .filter(|s| !s.is_empty())
        {
            push_text(
                &mut ops,
                Mm(left_mm),
                Mm(header_y),
                BuiltinFont::Helvetica,
                Pt(9.0),
                Pt(10.5),
                gray(),
                line,
            );
            header_y -= 4.5;
        }

        // "INVOICE" big right-aligned-ish (printpdf 0.9 has no built-in alignment;
        // we offset the cursor so that, with a known approximate width, it sits
        // on the right margin. For simplicity v1 uses a fixed offset.)
        push_text(
            &mut ops,
            Mm(right_mm - 36.0),
            Mm(280.0),
            BuiltinFont::HelveticaBold,
            Pt(22.0),
            Pt(24.0),
            black(),
            "INVOICE",
        );
        push_text(
            &mut ops,
            Mm(right_mm - 36.0),
            Mm(272.0),
            BuiltinFont::HelveticaBold,
            Pt(11.0),
            Pt(13.0),
            black(),
            &format!("# {}", ctx.invoice_number),
        );
        push_text(
            &mut ops,
            Mm(right_mm - 36.0),
            Mm(266.0),
            BuiltinFont::Helvetica,
            Pt(9.0),
            Pt(11.0),
            gray(),
            &format!("Status: {}", ctx.status_label),
        );

        // ---------- BILL TO + DATES ----------
        let billto_y = 245.0_f32;
        push_text(
            &mut ops,
            Mm(left_mm),
            Mm(billto_y),
            BuiltinFont::HelveticaBold,
            Pt(10.0),
            Pt(12.0),
            black(),
            "BILL TO",
        );
        push_text(
            &mut ops,
            Mm(left_mm),
            Mm(billto_y - 5.5),
            BuiltinFont::Helvetica,
            Pt(10.0),
            Pt(12.0),
            black(),
            &ctx.customer.name,
        );

        let mut bill_y = billto_y - 11.0;
        for line in [
            ctx.customer.address.as_deref(),
            ctx.customer.email.as_deref(),
        ]
        .iter()
        .flatten()
        .filter(|s| !s.is_empty())
        {
            push_text(
                &mut ops,
                Mm(left_mm),
                Mm(bill_y),
                BuiltinFont::Helvetica,
                Pt(9.0),
                Pt(11.0),
                gray(),
                line,
            );
            bill_y -= 4.5;
        }

        // Dates on the right
        push_text(
            &mut ops,
            Mm(right_mm - 50.0),
            Mm(billto_y),
            BuiltinFont::HelveticaBold,
            Pt(10.0),
            Pt(12.0),
            black(),
            "ISSUED",
        );
        push_text(
            &mut ops,
            Mm(right_mm - 50.0),
            Mm(billto_y - 5.5),
            BuiltinFont::Helvetica,
            Pt(10.0),
            Pt(12.0),
            black(),
            &ctx.issued_at,
        );
        push_text(
            &mut ops,
            Mm(right_mm - 50.0),
            Mm(billto_y - 12.0),
            BuiltinFont::HelveticaBold,
            Pt(10.0),
            Pt(12.0),
            black(),
            "DUE",
        );
        push_text(
            &mut ops,
            Mm(right_mm - 50.0),
            Mm(billto_y - 17.5),
            BuiltinFont::Helvetica,
            Pt(10.0),
            Pt(12.0),
            black(),
            &ctx.due_at,
        );

        // ---------- ITEMS TABLE ----------
        // Column x positions (mm). Right-most three are right-aligned via fixed offsets.
        let col_desc_x = left_mm; // description start
        let col_qty_x = right_mm - 78.0;
        let col_price_x = right_mm - 52.0;
        let col_sub_x = right_mm - 24.0;

        let table_top_y = 215.0_f32;
        // Header row
        push_text(
            &mut ops,
            Mm(col_desc_x),
            Mm(table_top_y),
            BuiltinFont::HelveticaBold,
            Pt(9.5),
            Pt(11.0),
            black(),
            "DESCRIPTION",
        );
        push_text(
            &mut ops,
            Mm(col_qty_x),
            Mm(table_top_y),
            BuiltinFont::HelveticaBold,
            Pt(9.5),
            Pt(11.0),
            black(),
            "QTY",
        );
        push_text(
            &mut ops,
            Mm(col_price_x),
            Mm(table_top_y),
            BuiltinFont::HelveticaBold,
            Pt(9.5),
            Pt(11.0),
            black(),
            "PRICE",
        );
        push_text(
            &mut ops,
            Mm(col_sub_x),
            Mm(table_top_y),
            BuiltinFont::HelveticaBold,
            Pt(9.5),
            Pt(11.0),
            black(),
            "SUBTOTAL",
        );

        // Items
        let mut row_y = table_top_y - 7.0;
        for item in &ctx.items {
            push_text(
                &mut ops,
                Mm(col_desc_x),
                Mm(row_y),
                BuiltinFont::Helvetica,
                Pt(9.5),
                Pt(11.0),
                black(),
                &item.description,
            );
            push_text(
                &mut ops,
                Mm(col_qty_x),
                Mm(row_y),
                BuiltinFont::Helvetica,
                Pt(9.5),
                Pt(11.0),
                black(),
                &item.quantity,
            );
            push_text(
                &mut ops,
                Mm(col_price_x),
                Mm(row_y),
                BuiltinFont::Helvetica,
                Pt(9.5),
                Pt(11.0),
                black(),
                &item.unit_price,
            );
            push_text(
                &mut ops,
                Mm(col_sub_x),
                Mm(row_y),
                BuiltinFont::Helvetica,
                Pt(9.5),
                Pt(11.0),
                black(),
                &item.subtotal,
            );
            row_y -= 6.0;

            // Stop early if the page would overflow — v1 caps at one page;
            // additional items spill into a "(…and N more items)" stub line.
            if row_y < 90.0 {
                let remaining = ctx.items.len()
                    - (ctx
                        .items
                        .iter()
                        .take_while(|i| !std::ptr::eq(*i, item))
                        .count()
                        + 1);
                if remaining > 0 {
                    push_text(
                        &mut ops,
                        Mm(col_desc_x),
                        Mm(row_y),
                        BuiltinFont::HelveticaOblique,
                        Pt(9.0),
                        Pt(11.0),
                        gray(),
                        &format!("…and {} more item(s)", remaining),
                    );
                }
                break;
            }
        }

        // ---------- TOTALS ----------
        let totals_x_label = right_mm - 56.0;
        let totals_x_amount = right_mm - 24.0;
        let mut totals_y = 80.0_f32;

        push_text(
            &mut ops,
            Mm(totals_x_label),
            Mm(totals_y),
            BuiltinFont::Helvetica,
            Pt(10.0),
            Pt(12.0),
            black(),
            "Subtotal",
        );
        push_text(
            &mut ops,
            Mm(totals_x_amount),
            Mm(totals_y),
            BuiltinFont::Helvetica,
            Pt(10.0),
            Pt(12.0),
            black(),
            &ctx.totals.subtotal,
        );
        totals_y -= 6.0;

        if let (Some(label), Some(amount)) = (
            ctx.totals.tax_label.as_deref(),
            ctx.totals.tax_amount.as_deref(),
        ) {
            push_text(
                &mut ops,
                Mm(totals_x_label),
                Mm(totals_y),
                BuiltinFont::Helvetica,
                Pt(10.0),
                Pt(12.0),
                black(),
                label,
            );
            push_text(
                &mut ops,
                Mm(totals_x_amount),
                Mm(totals_y),
                BuiltinFont::Helvetica,
                Pt(10.0),
                Pt(12.0),
                black(),
                amount,
            );
            totals_y -= 6.0;
        }

        push_text(
            &mut ops,
            Mm(totals_x_label),
            Mm(totals_y - 2.0),
            BuiltinFont::HelveticaBold,
            Pt(11.0),
            Pt(13.0),
            black(),
            "TOTAL",
        );
        push_text(
            &mut ops,
            Mm(totals_x_amount),
            Mm(totals_y - 2.0),
            BuiltinFont::HelveticaBold,
            Pt(11.0),
            Pt(13.0),
            black(),
            &ctx.totals.grand_total,
        );

        // ---------- FOOTER (payment URL + notes) ----------
        if let Some(url) = ctx.payment_url.as_deref().filter(|s| !s.is_empty()) {
            push_text(
                &mut ops,
                Mm(left_mm),
                Mm(40.0),
                BuiltinFont::HelveticaBold,
                Pt(10.0),
                Pt(12.0),
                black(),
                "Pay online:",
            );
            push_text(
                &mut ops,
                Mm(left_mm),
                Mm(34.0),
                BuiltinFont::Helvetica,
                Pt(9.5),
                Pt(11.0),
                gray(),
                url,
            );
        }

        if let Some(notes) = ctx.notes.as_deref().filter(|s| !s.is_empty()) {
            push_text(
                &mut ops,
                Mm(left_mm),
                Mm(22.0),
                BuiltinFont::HelveticaOblique,
                Pt(9.0),
                Pt(11.0),
                gray(),
                notes,
            );
        }

        // ---------- ASSEMBLE ----------
        let page = PdfPage::new(Mm(page_w_mm), Mm(page_h_mm), ops);
        let mut warnings = Vec::new();
        let pdf_bytes = PdfDocument::new(&format!("Invoice {}", ctx.invoice_number))
            .with_pages(vec![page])
            .save(&PdfSaveOptions::default(), &mut warnings);

        Ok(pdf_bytes)
    }
}

// ---------- helpers ----------

fn push_text(
    ops: &mut Vec<Op>,
    x: Mm,
    y: Mm,
    font: BuiltinFont,
    size: Pt,
    line_height: Pt,
    color: Color,
    text: &str,
) {
    ops.push(Op::StartTextSection);
    ops.push(Op::SetTextCursor {
        pos: Point::new(x, y),
    });
    ops.push(Op::SetFont {
        font: PdfFontHandle::Builtin(font),
        size,
    });
    ops.push(Op::SetLineHeight { lh: line_height });
    ops.push(Op::SetFillColor { col: color });
    ops.push(Op::ShowText {
        items: vec![TextItem::Text(text.to_string())],
    });
    ops.push(Op::EndTextSection);
}

fn black() -> Color {
    Color::Rgb(Rgb {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        icc_profile: None,
    })
}

fn gray() -> Color {
    Color::Rgb(Rgb {
        r: 0.35,
        g: 0.35,
        b: 0.38,
        icc_profile: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_ctx() -> InvoicePdfContext {
        InvoicePdfContext {
            company: InvoicePdfCompany {
                name: "PT ISP Demo".to_string(),
                address: Some("Jl. Merdeka No. 1, Jakarta".to_string()),
                npwp: Some("01.234.567.8-901.000".to_string()),
                email: Some("billing@isp-demo.id".to_string()),
                phone: Some("+62 21 1234 5678".to_string()),
            },
            customer: InvoicePdfCustomer {
                name: "Budi Santoso".to_string(),
                address: Some("Jl. Sudirman 123".to_string()),
                email: Some("budi@example.com".to_string()),
            },
            invoice_number: "INV-2026-000123".to_string(),
            status_label: "PENDING".to_string(),
            issued_at: "2026-05-31".to_string(),
            due_at: "2026-06-07".to_string(),
            items: vec![
                InvoicePdfLineItem {
                    description: "Internet Home 50 Mbps - Mei 2026".to_string(),
                    quantity: "1".to_string(),
                    unit_price: "Rp 350.000".to_string(),
                    subtotal: "Rp 350.000".to_string(),
                },
                InvoicePdfLineItem {
                    description: "Biaya pemasangan".to_string(),
                    quantity: "1".to_string(),
                    unit_price: "Rp 100.000".to_string(),
                    subtotal: "Rp 100.000".to_string(),
                },
            ],
            totals: InvoicePdfTotals {
                subtotal: "Rp 450.000".to_string(),
                tax_label: Some("PPN 11%".to_string()),
                tax_amount: Some("Rp 49.500".to_string()),
                grand_total: "Rp 499.500".to_string(),
            },
            payment_url: Some("https://billing.example.com/pay/abc123".to_string()),
            notes: Some("Terima kasih telah menggunakan layanan kami.".to_string()),
        }
    }

    #[test]
    fn render_invoice_produces_non_empty_pdf_with_magic_header() {
        let svc = InvoicePdfService::new();
        let bytes = svc.render_invoice(&sample_ctx()).expect("render");
        assert!(bytes.len() > 1024, "PDF too small: {} bytes", bytes.len());
        assert!(
            bytes.starts_with(b"%PDF-"),
            "missing %PDF- magic; got {:?}",
            &bytes[..bytes.len().min(8)]
        );
    }

    #[test]
    fn render_invoice_rejects_empty_invoice_number() {
        let mut ctx = sample_ctx();
        ctx.invoice_number = "  ".to_string();
        let err = InvoicePdfService::new().render_invoice(&ctx).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn render_invoice_handles_no_tax() {
        let mut ctx = sample_ctx();
        ctx.totals.tax_label = None;
        ctx.totals.tax_amount = None;
        let bytes = InvoicePdfService::new().render_invoice(&ctx).unwrap();
        assert!(bytes.starts_with(b"%PDF-"));
    }

    #[test]
    fn render_invoice_caps_long_item_list_with_overflow_notice() {
        let mut ctx = sample_ctx();
        // Many items to trigger the row_y < 90.0 overflow branch
        ctx.items = (0..40)
            .map(|i| InvoicePdfLineItem {
                description: format!("Line {}", i),
                quantity: "1".to_string(),
                unit_price: "Rp 10.000".to_string(),
                subtotal: "Rp 10.000".to_string(),
            })
            .collect();
        let bytes = InvoicePdfService::new().render_invoice(&ctx).unwrap();
        assert!(bytes.starts_with(b"%PDF-"));
    }
}
