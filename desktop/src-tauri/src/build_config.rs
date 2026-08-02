pub fn api_origin(value: Option<String>) -> Result<String, String> {
    let configured = value.unwrap_or_else(|| "http://127.0.0.1:8080".to_owned());
    let parsed = url::Url::parse(&configured)
        .map_err(|error| format!("NEXT_PUBLIC_MARK_TIME_API is not a URL: {error}"))?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.path() != "/"
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(
            "NEXT_PUBLIC_MARK_TIME_API must be an HTTP(S) origin without credentials, path, query, or fragment"
                .to_owned(),
        );
    }
    Ok(parsed.origin().ascii_serialization())
}

#[cfg(test)]
mod tests {
    use super::api_origin;

    #[test]
    fn defaults_to_the_local_service_origin() {
        assert_eq!(api_origin(None).unwrap(), "http://127.0.0.1:8080");
    }

    #[test]
    fn accepts_and_normalizes_an_https_origin() {
        assert_eq!(
            api_origin(Some("https://market-time.example".to_owned())).unwrap(),
            "https://market-time.example"
        );
    }

    #[test]
    fn rejects_a_missing_host_and_non_origin_components() {
        assert!(api_origin(Some("http://:".to_owned())).is_err());
        assert!(api_origin(Some("https://example.test/api".to_owned())).is_err());
        assert!(api_origin(Some("https://user@example.test".to_owned())).is_err());
    }
}
