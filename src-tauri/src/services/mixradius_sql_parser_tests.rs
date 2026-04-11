#[cfg(test)]
mod mixradius_sql_parser_tests {
    use crate::services::mixradius_sql_parser::parse_mixradius_backup;
    use std::fs;
    use std::path::PathBuf;
    use uuid::Uuid;

    const VALIDATED_BACKUP_GZ: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../MixRadiusDB_Gasal_2026-04-11_101103.sql.gz"
    );

    fn temp_sql_copy_path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "mixradius-import-parser-fixture-{}.sql",
            Uuid::new_v4().simple()
        ))
    }

    fn write_plain_sql_fixture() -> PathBuf {
        let gz_bytes = fs::read(VALIDATED_BACKUP_GZ).expect("validated gzip fixture should exist");
        let mut decoder = flate2::read::GzDecoder::new(gz_bytes.as_slice());
        let mut sql = String::new();
        std::io::Read::read_to_string(&mut decoder, &mut sql)
            .expect("validated gzip fixture should decompress");

        let path = temp_sql_copy_path();
        fs::write(&path, sql).expect("plain sql fixture should be writable");
        path
    }

    #[test]
    fn mixradius_sql_parser_accepts_plain_sql_file() {
        let sql_path = write_plain_sql_fixture();
        let parsed = parse_mixradius_backup(&sql_path).expect("plain sql backup should parse");

        assert_eq!(parsed.summary.customers_ppp_count, 543);
        assert_eq!(parsed.summary.plans_ppp_count, 12);
        assert_eq!(parsed.summary.nas_count, 2);
        assert!(parsed.detected_tables.contains(&"tbl_customers".to_string()));
    }

    #[test]
    fn mixradius_sql_parser_accepts_gzip_file() {
        let parsed =
            parse_mixradius_backup(PathBuf::from(VALIDATED_BACKUP_GZ)).expect("gzip should parse");

        assert_eq!(parsed.summary.customers_total_count, 545);
        assert_eq!(parsed.summary.transactions_count, 1902);
        assert_eq!(parsed.summary.radacct_count, 3811);
    }

    #[test]
    fn mixradius_sql_parser_detects_required_tables_and_ignores_unsupported_ones() {
        let parsed =
            parse_mixradius_backup(PathBuf::from(VALIDATED_BACKUP_GZ)).expect("gzip should parse");

        for table_name in [
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
        ] {
            assert!(parsed.detected_tables.contains(&table_name.to_string()));
        }

        for table_name in ["radgroupcheck", "radgroupreply", "tbl_activation"] {
            assert!(
                !parsed.detected_tables.contains(&table_name.to_string()),
                "unsupported table `{table_name}` should be ignored"
            );
        }
    }

    #[test]
    fn mixradius_sql_parser_counts_validated_backup_rows() {
        let parsed =
            parse_mixradius_backup(PathBuf::from(VALIDATED_BACKUP_GZ)).expect("gzip should parse");

        assert_eq!(parsed.nas_rows.len(), 2);
        assert_eq!(parsed.customer_rows.len(), 545);
        assert_eq!(parsed.plan_rows.len(), 15);
        assert_eq!(parsed.transaction_rows.len(), 1902);
        assert_eq!(parsed.radius_check_rows.len(), 1089);
        assert_eq!(parsed.radius_reply_rows.len(), 1088);
        assert_eq!(parsed.radius_user_group_rows.len(), 545);
        assert_eq!(parsed.customer_location_rows.len(), 460);
    }

    #[test]
    fn mixradius_sql_parser_rejects_malformed_gzip() {
        let broken_path = std::env::temp_dir().join(format!(
            "mixradius-import-broken-{}.sql.gz",
            Uuid::new_v4().simple()
        ));
        fs::write(&broken_path, b"not-a-real-gzip").expect("broken fixture should be writable");

        let error = parse_mixradius_backup(&broken_path).expect_err("broken gzip should fail");
        assert!(error.to_string().contains("gzip"));
    }

    #[test]
    fn mixradius_sql_parser_rejects_missing_required_tables() {
        let minimal_path = std::env::temp_dir().join(format!(
            "mixradius-import-missing-{}.sql",
            Uuid::new_v4().simple()
        ));
        fs::write(
            &minimal_path,
            "CREATE TABLE `nas` (`id` int);\nINSERT INTO `nas` VALUES (1);",
        )
        .expect("minimal fixture should be writable");

        let error =
            parse_mixradius_backup(&minimal_path).expect_err("missing required tables should fail");
        assert!(error.to_string().contains("required tables"));
    }
}
