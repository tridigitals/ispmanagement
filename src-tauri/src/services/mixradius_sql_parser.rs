use anyhow::{anyhow, bail, Context, Result};
use flate2::read::GzDecoder;
use std::fs;
use std::io::Read;
use std::path::Path;

const REQUIRED_TABLES: &[&str] = &[
    "nas",
    "tbl_customers",
    "tbl_customers_sub",
    "tbl_customers_map",
    "tbl_odp_data",
    "tbl_plans",
    "tbl_bandwidth",
    "tbl_transactions",
    "radcheck",
    "radreply",
    "radusergroup",
];

const SUPPORTED_TABLES: &[&str] = &[
    "nas",
    "radacct",
    "tbl_customers",
    "tbl_customers_sub",
    "tbl_customers_map",
    "tbl_odp_data",
    "tbl_plans",
    "tbl_bandwidth",
    "tbl_transactions",
    "radcheck",
    "radreply",
    "radusergroup",
    "tbl_usage_reports",
];

#[derive(Debug, Clone, Default)]
pub struct MixradiusParsedBackup {
    pub detected_tables: Vec<String>,
    pub summary: MixradiusParsedSummary,
    pub nas_rows: Vec<MixradiusSourceRow>,
    pub customer_rows: Vec<MixradiusSourceRow>,
    pub customer_sub_rows: Vec<MixradiusSourceRow>,
    pub customer_location_rows: Vec<MixradiusSourceRow>,
    pub odp_rows: Vec<MixradiusSourceRow>,
    pub plan_rows: Vec<MixradiusSourceRow>,
    pub bandwidth_rows: Vec<MixradiusSourceRow>,
    pub transaction_rows: Vec<MixradiusSourceRow>,
    pub radius_check_rows: Vec<MixradiusSourceRow>,
    pub radius_reply_rows: Vec<MixradiusSourceRow>,
    pub radius_user_group_rows: Vec<MixradiusSourceRow>,
    pub usage_rows: Vec<MixradiusSourceRow>,
}

#[derive(Debug, Clone, Default)]
pub struct MixradiusParsedSummary {
    pub customers_total_count: usize,
    pub customers_ppp_count: usize,
    pub plans_ppp_count: usize,
    pub nas_count: usize,
    pub transactions_count: usize,
    pub radacct_count: usize,
}

#[derive(Debug, Clone, Default)]
pub struct MixradiusSourceRow {
    pub values: Vec<String>,
}

pub fn parse_mixradius_backup<P: AsRef<Path>>(path: P) -> Result<MixradiusParsedBackup> {
    let path = path.as_ref();
    let sql = read_backup_sql(path)?;
    let detected_tables = detect_tables(&sql);
    ensure_required_tables(&detected_tables)?;

    let nas_rows = parse_table_rows(&sql, "nas")?;
    let customer_rows = parse_table_rows(&sql, "tbl_customers")?;
    let customer_sub_rows = parse_table_rows(&sql, "tbl_customers_sub")?;
    let customer_location_rows = parse_table_rows(&sql, "tbl_customers_map")?;
    let odp_rows = parse_table_rows(&sql, "tbl_odp_data")?;
    let plan_rows = parse_table_rows(&sql, "tbl_plans")?;
    let bandwidth_rows = parse_table_rows(&sql, "tbl_bandwidth")?;
    let transaction_rows = parse_table_rows(&sql, "tbl_transactions")?;
    let radius_check_rows = parse_table_rows(&sql, "radcheck")?;
    let radius_reply_rows = parse_table_rows(&sql, "radreply")?;
    let radius_user_group_rows = parse_table_rows(&sql, "radusergroup")?;
    let usage_rows = parse_optional_table_rows(&sql, "tbl_usage_reports")?;
    let radacct_count = parse_optional_table_rows(&sql, "radacct")?.len();

    let customers_ppp_count = customer_rows
        .iter()
        .filter(|row| row.values.get(3).map(|v| v == "PPP").unwrap_or(false))
        .count();
    let plans_ppp_count = plan_rows
        .iter()
        .filter(|row| row.values.get(8).map(|v| v == "PPP").unwrap_or(false))
        .count();

    Ok(MixradiusParsedBackup {
        detected_tables,
        summary: MixradiusParsedSummary {
            customers_total_count: customer_rows.len(),
            customers_ppp_count,
            plans_ppp_count,
            nas_count: nas_rows.len(),
            transactions_count: transaction_rows.len(),
            radacct_count,
        },
        nas_rows,
        customer_rows,
        customer_sub_rows,
        customer_location_rows,
        odp_rows,
        plan_rows,
        bandwidth_rows,
        transaction_rows,
        radius_check_rows,
        radius_reply_rows,
        radius_user_group_rows,
        usage_rows,
    })
}

fn read_backup_sql(path: &Path) -> Result<String> {
    let bytes = fs::read(path)
        .with_context(|| format!("failed to read backup file `{}`", path.display()))?;
    let looks_gzip = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("gz"))
        .unwrap_or(false)
        || bytes.starts_with(&[0x1f, 0x8b]);

    if looks_gzip {
        let mut decoder = GzDecoder::new(bytes.as_slice());
        let mut sql = String::new();
        decoder
            .read_to_string(&mut sql)
            .with_context(|| format!("gzip backup `{}` could not be decoded", path.display()))?;
        return Ok(sql);
    }

    String::from_utf8(bytes)
        .with_context(|| format!("plain SQL backup `{}` is not valid UTF-8", path.display()))
}

fn detect_tables(sql: &str) -> Vec<String> {
    let mut tables = Vec::new();

    for line in sql.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("CREATE TABLE `") {
            if let Some((table, _)) = rest.split_once('`') {
                if SUPPORTED_TABLES.contains(&table) {
                    tables.push(table.to_string());
                }
            }
        }
    }

    tables
}

fn ensure_required_tables(detected_tables: &[String]) -> Result<()> {
    let missing: Vec<&str> = REQUIRED_TABLES
        .iter()
        .copied()
        .filter(|table| !detected_tables.iter().any(|detected| detected == table))
        .collect();

    if missing.is_empty() {
        return Ok(());
    }

    bail!(
        "MixRadius backup is missing required tables: {}",
        missing.join(", ")
    );
}

fn parse_table_rows(sql: &str, table_name: &str) -> Result<Vec<MixradiusSourceRow>> {
    parse_optional_table_rows(sql, table_name)
}

fn parse_optional_table_rows(sql: &str, table_name: &str) -> Result<Vec<MixradiusSourceRow>> {
    let marker = format!("INSERT INTO `{table_name}` VALUES ");
    let mut rows = Vec::new();
    let mut search_offset = 0usize;

    while let Some(relative_start) = sql[search_offset..].find(&marker) {
        let start = search_offset + relative_start;
        let values_start = start + marker.len();
        let tail = &sql[values_start..];
        let end = find_statement_end(tail)
            .ok_or_else(|| anyhow!("INSERT statement for `{table_name}` is not terminated"))?;
        let values_block = &tail[..end];

        for row in split_insert_rows(values_block)? {
            rows.push(MixradiusSourceRow {
                values: split_row_fields(&row)?,
            });
        }

        search_offset = values_start + end + 1;
    }

    Ok(rows)
}

fn split_insert_rows(values_block: &str) -> Result<Vec<String>> {
    let mut rows = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for ch in values_block.chars() {
        if in_string {
            current.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '\'' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '\'' => {
                in_string = true;
                current.push(ch);
            }
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                if depth == 0 {
                    bail!("unbalanced closing parenthesis in INSERT values");
                }
                depth -= 1;
                current.push(ch);
                if depth == 0 {
                    rows.push(current.trim().to_string());
                    current.clear();
                }
            }
            ',' if depth == 0 => {}
            _ => current.push(ch),
        }
    }

    if in_string || depth != 0 {
        bail!("unterminated INSERT row in MixRadius dump");
    }

    Ok(rows)
}

fn split_row_fields(row: &str) -> Result<Vec<String>> {
    let inner = row
        .trim()
        .strip_prefix('(')
        .and_then(|v| v.strip_suffix(')'))
        .ok_or_else(|| anyhow!("INSERT row is missing tuple delimiters"))?;

    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut escaped = false;
    let mut field_was_quoted = false;

    for ch in inner.chars() {
        if in_string {
            if escaped {
                current.push(ch);
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '\'' {
                in_string = false;
            } else {
                current.push(ch);
            }
            continue;
        }

        match ch {
            '\'' => {
                in_string = true;
                field_was_quoted = true;
            }
            ',' => {
                fields.push(normalize_field(&current, field_was_quoted));
                current.clear();
                field_was_quoted = false;
            }
            _ => current.push(ch),
        }
    }

    if in_string {
        bail!("unterminated string literal in INSERT row");
    }

    fields.push(normalize_field(&current, field_was_quoted));
    Ok(fields)
}

fn normalize_field(raw: &str, was_quoted: bool) -> String {
    let trimmed = raw.trim();
    if !was_quoted && trimmed.eq_ignore_ascii_case("NULL") {
        String::new()
    } else {
        trimmed.to_string()
    }
}

fn find_statement_end(sql_tail: &str) -> Option<usize> {
    let mut in_string = false;
    let mut escaped = false;

    for (idx, ch) in sql_tail.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '\'' {
                in_string = false;
            }
            continue;
        }

        if ch == '\'' {
            in_string = true;
            continue;
        }

        if ch == ';' {
            return Some(idx);
        }
    }

    None
}
