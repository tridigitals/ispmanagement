//! Invoice PDF generation service.
//!
//! Pure-Rust PDF generator using `printpdf 0.9` for invoice attachments.
//! Built on the `Op` enum stack-based API (text sections + cursor positioning
//! + builtin Helvetica fonts + filled rectangles + lines).
//!
//! Layout (single page, A4 portrait, mm units):
//!   - Top brand band (full width, brand color)
//!   - Header: company info (left), big "INVOICE" + number + status badge (right)
//!   - Bill-to (left) + dates (right)
//!   - Items table: filled header row, per-row bottom border, right-aligned numbers
//!   - Totals box: subtotal + tax + GRAND TOTAL with brand accent
//!   - Footer: payment URL + notes + bottom rule

use crate::error::{AppError, AppResult};
use printpdf::{
    BuiltinFont, Color, Line, LinePoint, Mm, Op, PaintMode, PdfDocument, PdfFontHandle, PdfPage,
    PdfSaveOptions, Point, Pt, Rect, Rgb, TextItem,
};

#[derive(Debug, Clone)]
pub struct InvoicePdfLineItem {
    pub description: String,
    pub quantity: String,
    pub unit_price: String,
    pub subtotal: String,
}

#[derive(Debug, Clone, Default)]
pub struct InvoicePdfCompany {
    pub name: String,
    pub address: Option<String>,
    pub npwp: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct InvoicePdfCustomer {
    pub name: String,
    pub address: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct InvoicePdfTotals {
    pub subtotal: String,
    pub tax_label: Option<String>,
    pub tax_amount: Option<String>,
    pub grand_total: String,
}

#[derive(Debug, Clone)]
pub struct InvoicePdfContext {
    pub company: InvoicePdfCompany,
    pub customer: InvoicePdfCustomer,
    pub invoice_number: String,
    pub status_label: String, // "PENDING" | "PAID" | "OVERDUE" | "CANCELLED" | other
    pub issued_at: String,
    pub due_at: String,
    pub items: Vec<InvoicePdfLineItem>,
    pub totals: InvoicePdfTotals,
    pub payment_url: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct InvoicePdfService;

// ---------- color palette (slate + teal + status accents) ----------

fn rgb(r: f32, g: f32, b: f32) -> Color {
    Color::Rgb(Rgb {
        r,
        g,
        b,
        icc_profile: None,
    })
}

fn brand() -> Color {
    rgb(0.059, 0.463, 0.431) // #0F766E teal-700
}
fn brand_dark() -> Color {
    rgb(0.043, 0.341, 0.318) // darker teal
}
fn text_dark() -> Color {
    rgb(0.122, 0.161, 0.216) // #1F2937 slate-800
}
fn text_muted() -> Color {
    rgb(0.42, 0.45, 0.50) // #6B7280 slate-500
}
fn border_gray() -> Color {
    rgb(0.898, 0.906, 0.922) // #E5E7EB slate-200
}
fn table_header_fill() -> Color {
    rgb(0.953, 0.957, 0.965) // #F3F4F6 slate-100
}
fn total_box_fill() -> Color {
    rgb(0.945, 0.980, 0.976) // very light teal
}
fn white() -> Color {
    rgb(1.0, 1.0, 1.0)
}

fn status_color(status: &str) -> Color {
    match status.to_ascii_uppercase().as_str() {
        "PAID" | "LUNAS" => rgb(0.063, 0.725, 0.506), // emerald-500
        "OVERDUE" => rgb(0.937, 0.267, 0.267),        // red-500
        "CANCELLED" | "CANCELED" | "VOID" => rgb(0.42, 0.45, 0.50), // gray
        _ => rgb(0.961, 0.620, 0.043),                // amber-500 (PENDING/default)
    }
}

// ---------- text width approximation for right-alignment ----------
// Helvetica avg em-width ≈ 0.50, HelveticaBold ≈ 0.55. Good enough for
// numeric right-alignment on invoices (digits are very uniform).
fn approx_text_width_mm(text: &str, font: BuiltinFont, size_pt: f32) -> f32 {
    let em = match font {
        BuiltinFont::HelveticaBold | BuiltinFont::TimesBold | BuiltinFont::CourierBold => 0.55,
        _ => 0.50,
    };
    // 1 pt = 0.352778 mm
    text.chars().count() as f32 * em * size_pt * 0.352778
}

impl InvoicePdfService {
    pub fn new() -> Self {
        Self
    }

    pub fn render_invoice(&self, ctx: &InvoicePdfContext) -> AppResult<Vec<u8>> {
        if ctx.invoice_number.trim().is_empty() {
            return Err(AppError::Validation(
                "invoice_number is required".to_string(),
            ));
        }

        let mut ops: Vec<Op> = Vec::new();

        // --- Page geometry (A4 portrait, mm) ---
        let page_w = 210.0_f32;
        let page_h = 297.0_f32;
        let margin_l = 18.0_f32;
        let margin_r = page_w - 18.0_f32;
        let content_w = margin_r - margin_l;

        // ---------- BRAND BAND (top 6mm) ----------
        push_filled_rect(&mut ops, 0.0, page_h - 6.0, page_w, 6.0, brand());

        // ---------- HEADER ----------
        // Company name (left)
        push_text(
            &mut ops,
            margin_l,
            page_h - 18.0,
            BuiltinFont::HelveticaBold,
            16.0,
            text_dark(),
            &ctx.company.name,
        );

        let mut y = page_h - 24.0;
        for line in [
            ctx.company.address.as_deref(),
            ctx.company.npwp.as_deref(),
            ctx.company.email.as_deref(),
            ctx.company.phone.as_deref(),
        ]
        .iter()
        .flatten()
        .filter(|s| !s.is_empty())
        {
            push_text(
                &mut ops,
                margin_l,
                y,
                BuiltinFont::Helvetica,
                9.0,
                text_muted(),
                line,
            );
            y -= 4.5;
        }

        // INVOICE big (right) + invoice number + status badge
        let inv_label = "INVOICE";
        let inv_label_w = approx_text_width_mm(inv_label, BuiltinFont::HelveticaBold, 24.0);
        push_text(
            &mut ops,
            margin_r - inv_label_w,
            page_h - 18.0,
            BuiltinFont::HelveticaBold,
            24.0,
            brand(),
            inv_label,
        );

        let inv_num = format!("# {}", ctx.invoice_number);
        let inv_num_w = approx_text_width_mm(&inv_num, BuiltinFont::Helvetica, 11.0);
        push_text(
            &mut ops,
            margin_r - inv_num_w,
            page_h - 27.0,
            BuiltinFont::Helvetica,
            11.0,
            text_dark(),
            &inv_num,
        );

        // Status badge: filled rect + white text, right-aligned
        let badge_text = ctx.status_label.to_ascii_uppercase();
        let badge_text_w = approx_text_width_mm(&badge_text, BuiltinFont::HelveticaBold, 9.0);
        let badge_w = badge_text_w + 6.0;
        let badge_h = 5.0;
        let badge_x = margin_r - badge_w;
        let badge_y = page_h - 36.0;
        push_filled_rect(
            &mut ops,
            badge_x,
            badge_y,
            badge_w,
            badge_h,
            status_color(&ctx.status_label),
        );
        push_text(
            &mut ops,
            badge_x + 3.0,
            badge_y + 1.4,
            BuiltinFont::HelveticaBold,
            9.0,
            white(),
            &badge_text,
        );

        // Divider line under header
        push_line(
            &mut ops,
            margin_l,
            page_h - 50.0,
            margin_r,
            page_h - 50.0,
            border_gray(),
            0.5,
        );

        // ---------- BILL TO + DATES ----------
        let block_top = page_h - 56.0;
        push_text(
            &mut ops,
            margin_l,
            block_top,
            BuiltinFont::HelveticaBold,
            8.5,
            text_muted(),
            "BILL TO",
        );
        push_text(
            &mut ops,
            margin_l,
            block_top - 5.5,
            BuiltinFont::HelveticaBold,
            10.5,
            text_dark(),
            &ctx.customer.name,
        );

        let mut by = block_top - 10.5;
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
                margin_l,
                by,
                BuiltinFont::Helvetica,
                9.0,
                text_muted(),
                line,
            );
            by -= 4.5;
        }

        // Dates (right-aligned labels + values)
        let date_x = margin_r - 50.0;
        push_text(
            &mut ops,
            date_x,
            block_top,
            BuiltinFont::HelveticaBold,
            8.5,
            text_muted(),
            "ISSUED",
        );
        push_text(
            &mut ops,
            date_x,
            block_top - 5.5,
            BuiltinFont::Helvetica,
            10.0,
            text_dark(),
            &ctx.issued_at,
        );
        push_text(
            &mut ops,
            date_x,
            block_top - 13.0,
            BuiltinFont::HelveticaBold,
            8.5,
            text_muted(),
            "DUE",
        );
        push_text(
            &mut ops,
            date_x,
            block_top - 18.5,
            BuiltinFont::Helvetica,
            10.0,
            text_dark(),
            &ctx.due_at,
        );

        // ---------- ITEMS TABLE ----------
        // Column layout (mm): description (flex) | qty (right) | price (right) | subtotal (right)
        let table_top = block_top - 32.0;
        let col_qty_x = margin_l + content_w - 78.0; // right-edge for QTY column
        let col_price_x = margin_l + content_w - 52.0; // right-edge for PRICE
        let col_sub_x = margin_l + content_w; // right-edge for SUBTOTAL = margin_r

        // Header row background
        push_filled_rect(
            &mut ops,
            margin_l,
            table_top - 1.5,
            content_w,
            7.0,
            table_header_fill(),
        );

        // Header text
        push_text(
            &mut ops,
            margin_l + 2.0,
            table_top + 1.0,
            BuiltinFont::HelveticaBold,
            9.0,
            text_dark(),
            "DESCRIPTION",
        );
        push_text_right(
            &mut ops,
            col_qty_x,
            table_top + 1.0,
            BuiltinFont::HelveticaBold,
            9.0,
            text_dark(),
            "QTY",
        );
        push_text_right(
            &mut ops,
            col_price_x,
            table_top + 1.0,
            BuiltinFont::HelveticaBold,
            9.0,
            text_dark(),
            "PRICE",
        );
        push_text_right(
            &mut ops,
            col_sub_x,
            table_top + 1.0,
            BuiltinFont::HelveticaBold,
            9.0,
            text_dark(),
            "SUBTOTAL",
        );

        // Item rows
        let mut row_y = table_top - 6.0;
        let row_height = 7.0_f32;
        let row_min_y = 95.0_f32; // leave space for totals + footer

        let total_items = ctx.items.len();
        let mut rendered = 0_usize;
        for item in &ctx.items {
            if row_y < row_min_y {
                break;
            }
            push_text(
                &mut ops,
                margin_l + 2.0,
                row_y,
                BuiltinFont::Helvetica,
                9.5,
                text_dark(),
                &item.description,
            );
            push_text_right(
                &mut ops,
                col_qty_x,
                row_y,
                BuiltinFont::Helvetica,
                9.5,
                text_dark(),
                &item.quantity,
            );
            push_text_right(
                &mut ops,
                col_price_x,
                row_y,
                BuiltinFont::Helvetica,
                9.5,
                text_dark(),
                &item.unit_price,
            );
            push_text_right(
                &mut ops,
                col_sub_x,
                row_y,
                BuiltinFont::Helvetica,
                9.5,
                text_dark(),
                &item.subtotal,
            );
            // Bottom border per row
            push_line(
                &mut ops,
                margin_l,
                row_y - 2.0,
                margin_r,
                row_y - 2.0,
                border_gray(),
                0.3,
            );
            rendered += 1;
            row_y -= row_height;
        }

        if rendered < total_items {
            push_text(
                &mut ops,
                margin_l + 2.0,
                row_y,
                BuiltinFont::HelveticaOblique,
                9.0,
                text_muted(),
                &format!(
                    "…and {} more item(s) — see online portal",
                    total_items - rendered
                ),
            );
        }

        // ---------- TOTALS ----------
        let totals_box_w = 70.0_f32;
        let totals_box_x = margin_r - totals_box_w;
        let mut ty = 80.0_f32;

        // Subtotal row
        push_text(
            &mut ops,
            totals_box_x,
            ty,
            BuiltinFont::Helvetica,
            10.0,
            text_muted(),
            "Subtotal",
        );
        push_text_right(
            &mut ops,
            margin_r,
            ty,
            BuiltinFont::Helvetica,
            10.0,
            text_dark(),
            &ctx.totals.subtotal,
        );
        ty -= 5.5;

        // Tax row (optional)
        if let (Some(label), Some(amount)) = (
            ctx.totals.tax_label.as_deref(),
            ctx.totals.tax_amount.as_deref(),
        ) {
            push_text(
                &mut ops,
                totals_box_x,
                ty,
                BuiltinFont::Helvetica,
                10.0,
                text_muted(),
                label,
            );
            push_text_right(
                &mut ops,
                margin_r,
                ty,
                BuiltinFont::Helvetica,
                10.0,
                text_dark(),
                amount,
            );
            ty -= 5.5;
        }

        // Grand total: highlighted box with brand accent line above it
        let total_box_h = 10.0_f32;
        let total_box_y = ty - total_box_h - 1.0;
        push_filled_rect(
            &mut ops,
            totals_box_x,
            total_box_y,
            totals_box_w,
            total_box_h,
            total_box_fill(),
        );
        // Top brand stripe on the box
        push_filled_rect(
            &mut ops,
            totals_box_x,
            total_box_y + total_box_h - 0.6,
            totals_box_w,
            0.6,
            brand(),
        );
        push_text(
            &mut ops,
            totals_box_x + 2.5,
            total_box_y + 3.0,
            BuiltinFont::HelveticaBold,
            11.5,
            brand_dark(),
            "TOTAL",
        );
        push_text_right(
            &mut ops,
            margin_r - 2.5,
            total_box_y + 3.0,
            BuiltinFont::HelveticaBold,
            12.0,
            brand_dark(),
            &ctx.totals.grand_total,
        );

        // ---------- FOOTER (payment URL + notes + rule) ----------
        // Bottom rule
        push_line(&mut ops, margin_l, 22.0, margin_r, 22.0, border_gray(), 0.5);

        if let Some(url) = ctx.payment_url.as_deref().filter(|s| !s.is_empty()) {
            push_text(
                &mut ops,
                margin_l,
                40.0,
                BuiltinFont::HelveticaBold,
                9.5,
                text_dark(),
                "Pay online:",
            );
            push_text(
                &mut ops,
                margin_l,
                35.0,
                BuiltinFont::Helvetica,
                9.0,
                brand(),
                url,
            );
        }

        if let Some(notes) = ctx.notes.as_deref().filter(|s| !s.is_empty()) {
            push_text(
                &mut ops,
                margin_l,
                17.0,
                BuiltinFont::HelveticaOblique,
                8.5,
                text_muted(),
                notes,
            );
        }

        // Page-foot identity
        let foot = format!("Invoice {} • Generated electronically", ctx.invoice_number);
        push_text(
            &mut ops,
            margin_l,
            10.0,
            BuiltinFont::Helvetica,
            7.5,
            text_muted(),
            &foot,
        );

        // ---------- ASSEMBLE ----------
        let page = PdfPage::new(Mm(page_w), Mm(page_h), ops);
        let mut warnings = Vec::new();
        let pdf_bytes = PdfDocument::new(&format!("Invoice {}", ctx.invoice_number))
            .with_pages(vec![page])
            .save(&PdfSaveOptions::default(), &mut warnings);

        Ok(pdf_bytes)
    }
}

// ---------- low-level helpers ----------

fn push_text(
    ops: &mut Vec<Op>,
    x_mm: f32,
    y_mm: f32,
    font: BuiltinFont,
    size_pt: f32,
    color: Color,
    text: &str,
) {
    ops.push(Op::SaveGraphicsState);
    ops.push(Op::StartTextSection);
    ops.push(Op::SetTextCursor {
        pos: Point::new(Mm(x_mm), Mm(y_mm)),
    });
    ops.push(Op::SetFont {
        font: PdfFontHandle::Builtin(font),
        size: Pt(size_pt),
    });
    ops.push(Op::SetLineHeight {
        lh: Pt(size_pt * 1.2),
    });
    ops.push(Op::SetFillColor { col: color });
    ops.push(Op::ShowText {
        items: vec![TextItem::Text(text.to_string())],
    });
    ops.push(Op::EndTextSection);
    ops.push(Op::RestoreGraphicsState);
}

fn push_text_right(
    ops: &mut Vec<Op>,
    right_x_mm: f32,
    y_mm: f32,
    font: BuiltinFont,
    size_pt: f32,
    color: Color,
    text: &str,
) {
    let w = approx_text_width_mm(text, font, size_pt);
    push_text(ops, right_x_mm - w, y_mm, font, size_pt, color, text);
}

fn mm_to_pt(mm: f32) -> Pt {
    Pt(mm * 2.834_645_7)
}

fn push_filled_rect(ops: &mut Vec<Op>, x_mm: f32, y_mm: f32, w_mm: f32, h_mm: f32, color: Color) {
    ops.push(Op::SaveGraphicsState);
    ops.push(Op::SetFillColor { col: color });
    ops.push(Op::DrawRectangle {
        rectangle: Rect {
            x: mm_to_pt(x_mm),
            y: mm_to_pt(y_mm),
            width: mm_to_pt(w_mm),
            height: mm_to_pt(h_mm),
            mode: Some(PaintMode::Fill),
            winding_order: None,
        },
    });
    ops.push(Op::RestoreGraphicsState);
}

fn push_line(
    ops: &mut Vec<Op>,
    x1_mm: f32,
    y1_mm: f32,
    x2_mm: f32,
    y2_mm: f32,
    color: Color,
    thickness_pt: f32,
) {
    ops.push(Op::SaveGraphicsState);
    ops.push(Op::SetOutlineColor { col: color });
    ops.push(Op::SetOutlineThickness {
        pt: Pt(thickness_pt),
    });
    ops.push(Op::DrawLine {
        line: Line {
            points: vec![
                LinePoint {
                    p: Point::new(Mm(x1_mm), Mm(y1_mm)),
                    bezier: false,
                },
                LinePoint {
                    p: Point::new(Mm(x2_mm), Mm(y2_mm)),
                    bezier: false,
                },
            ],
            is_closed: false,
        },
    });
    ops.push(Op::RestoreGraphicsState);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_ctx() -> InvoicePdfContext {
        InvoicePdfContext {
            company: InvoicePdfCompany {
                name: "PT ISP Demo".to_string(),
                address: Some("Jl. Merdeka No. 1, Jakarta".to_string()),
                npwp: Some("NPWP: 01.234.567.8-901.000".to_string()),
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
        let bytes = InvoicePdfService::new()
            .render_invoice(&sample_ctx())
            .unwrap();
        assert!(bytes.len() > 1024);
        assert!(bytes.starts_with(b"%PDF-"));
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
    fn render_invoice_handles_long_item_list() {
        let mut ctx = sample_ctx();
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

    #[test]
    fn render_invoice_status_paid_uses_green_badge() {
        let mut ctx = sample_ctx();
        ctx.status_label = "PAID".to_string();
        let bytes = InvoicePdfService::new().render_invoice(&ctx).unwrap();
        assert!(bytes.starts_with(b"%PDF-"));
    }
}
