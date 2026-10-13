//! Application-owned error labels. External error details are never translated.
pub fn format_error(code: &str, details: &str) -> String {
    format!(
        "{}\n{}: {}\n{}: {}",
        workflow_ide_framework::localization::text("meridian.error.occurred"),
        workflow_ide_framework::localization::text("meridian.error.code"),
        code,
        workflow_ide_framework::localization::text("meridian.error.details"),
        details,
    )
}

#[cfg(test)]
mod tests {
    const EN: &str = include_str!("../resources/meridian-locales/en-US.toml");
    const JA: &str = include_str!("../resources/meridian-locales/ja-JP.toml");

    fn value<'a>(toml: &'a str, key: &str) -> &'a str {
        let prefix = format!("\"{key}\" = \"");
        let line = toml.lines().find(|line| line.starts_with(&prefix))
            .unwrap_or_else(|| panic!("missing localization key: {key}"));
        line.strip_prefix(&prefix).unwrap().strip_suffix('"').unwrap()
    }

    #[test]
    fn japanese_and_english_have_matching_required_keys() {
        for key in [
            "meridian.panel.project", "meridian.panel.logs",
            "meridian.error.occurred", "meridian.error.code", "meridian.error.details",
            "meridian.menu.data", "meridian.menu.urdf.import",
            "meridian.menu.urdf.export", "meridian.menu.mjcf.import",
            "meridian.menu.mjcf.reimport",
        ] {
            assert!(!value(EN, key).is_empty());
            assert!(!value(JA, key).is_empty());
        }
        assert!(EN.contains("locale = \"en-US\""));
        assert!(JA.contains("locale = \"ja-JP\""));
    }

    #[test]
    fn japanese_and_english_key_sets_match_exactly() {
        fn keys(source: &str) -> std::collections::BTreeSet<&str> {
            source.lines().filter_map(|line| {
                let (key, _) = line.split_once(" = ")?;
                if key.starts_with('"') && key.ends_with('"') {
                    Some(key.trim_matches('"'))
                } else {
                    None
                }
            }).collect()
        }
        assert_eq!(keys(EN), keys(JA), "JP/EN translation keys diverged");
    }

    #[test]
    fn error_details_remain_identical_across_languages() {
        let details = "The system cannot find the file specified. (os error 2)";
        let code = "MJCF-0003";
        let render = |source: &str| format!(
            "{}\n{}: {}\n{}: {}",
            value(source, "meridian.error.occurred"),
            value(source, "meridian.error.code"),
            code,
            value(source, "meridian.error.details"),
            details,
        );
        let en = render(EN);
        let ja = render(JA);
        assert!(en.contains("An error occurred."));
        assert!(ja.contains("エラーが発生しました。"));
        for message in [&en, &ja] {
            assert!(message.contains(code));
            assert!(message.contains(details));
        }
    }
}
