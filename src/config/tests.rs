use super::AppConfig;

#[test]
fn parses_toml_http_settings() {
    //
    let config = AppConfig::parse(
        r#"
            [http]
            host = "127.0.0.1"
            port = 8888

            [image]
            user_avatar_limit = 1
            team_avatar_limit = 1
            comic_cover_limit = 2
            page_image_limit = 25
        "#,
    )
    .unwrap();

    assert_eq!(config.http.host, "127.0.0.1");

    assert_eq!(config.http.port, 8888);

    assert_eq!(config.image.user_avatar_limit, 1);

    assert_eq!(config.image.team_avatar_limit, 1);

    assert_eq!(config.image.comic_cover_limit, 2);

    assert_eq!(config.image.page_image_limit, 25);

    assert_eq!(config.artwork.chapter_artwork_limit, 512);
}

#[test]
fn rejects_legacy_json_settings() {
    //
    let result = AppConfig::parse(
        r#"
            {
              "http": {
                "host": "127.0.0.1",
                "port": 8888
              }
            }
        "#,
    );

    assert!(result.is_err());
}

#[test]
fn rejects_zero_image_mib_limits() {
    //
    let zero_limit_configs = [
        r#"
            [http]
            host = "127.0.0.1"
            port = 8888

            [image]
            user_avatar_limit = 0
            team_avatar_limit = 1
            comic_cover_limit = 1
            page_image_limit = 1
        "#,
        r#"
            [http]
            host = "127.0.0.1"
            port = 8888

            [image]
            user_avatar_limit = 1
            team_avatar_limit = 0
            comic_cover_limit = 1
            page_image_limit = 1
        "#,
        r#"
            [http]
            host = "127.0.0.1"
            port = 8888

            [image]
            user_avatar_limit = 1
            team_avatar_limit = 1
            comic_cover_limit = 0
            page_image_limit = 1
        "#,
        r#"
            [http]
            host = "127.0.0.1"
            port = 8888

            [image]
            user_avatar_limit = 1
            team_avatar_limit = 1
            comic_cover_limit = 1
            page_image_limit = 0
        "#,
    ];

    for config_content in zero_limit_configs {
        //
        assert!(AppConfig::parse(config_content).is_err());
    }
}

// artwork_config(AppConfig)(negative): custom limits are honored and invalid limits fail at startup.
#[test]
fn artwork_config_validates_custom_limits() {
    let config_prefix = r#"
        [http]
        host = "127.0.0.1"
        port = 8888
        [image]
        user_avatar_limit = 1
        team_avatar_limit = 1
        comic_cover_limit = 2
        page_image_limit = 25
    "#;

    let custom = AppConfig::parse(&format!(
        "{config_prefix}\n[artwork]\nchapter_artwork_limit = 128"
    ))
    .unwrap();

    assert_eq!(custom.artwork.chapter_artwork_limit, 128);

    for limit in [0, u64::MAX] {
        assert!(
            AppConfig::parse(&format!(
                "{config_prefix}\n[artwork]\nchapter_artwork_limit = {limit}"
            ))
            .is_err()
        );
    }
}
