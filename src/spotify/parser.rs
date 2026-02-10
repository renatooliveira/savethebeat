use regex::Regex;

/// Extract Spotify track ID from a URL or URI
///
/// Supports multiple formats:
/// - https://open.spotify.com/track/TRACK_ID
/// - https://open.spotify.com/track/TRACK_ID?si=...
/// - spotify:track:TRACK_ID
///
/// # Arguments
/// * `text` - Text that may contain a Spotify link
///
/// # Returns
/// The track ID if found, None otherwise
///
/// # Examples
/// ```
/// use savethebeat::spotify::parser::extract_track_id;
///
/// let url = "https://open.spotify.com/track/3n3Ppam7vgaVa1iaRUc9Lp?si=abc";
/// assert_eq!(extract_track_id(url), Some("3n3Ppam7vgaVa1iaRUc9Lp".to_string()));
/// ```
pub fn extract_track_id(text: &str) -> Option<String> {
    // Try HTTP/HTTPS URL format first
    let url_pattern = Regex::new(r"https?://open\.spotify\.com/track/([a-zA-Z0-9]+)").unwrap();
    if let Some(captures) = url_pattern.captures(text) {
        return Some(captures[1].to_string());
    }

    // Try Spotify URI format
    let uri_pattern = Regex::new(r"spotify:track:([a-zA-Z0-9]+)").unwrap();
    if let Some(captures) = uri_pattern.captures(text) {
        return Some(captures[1].to_string());
    }

    None
}

/// Find the first Spotify track link in a list of messages
///
/// Searches through messages in chronological order and returns the first
/// Spotify track ID found.
///
/// # Arguments
/// * `messages` - List of message texts to search
///
/// # Returns
/// The first track ID found, None if no track links found
pub fn find_first_track(messages: &[String]) -> Option<String> {
    for message in messages {
        if let Some(track_id) = extract_track_id(message) {
            return Some(track_id);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_track_id_https_url() {
        let url = "https://open.spotify.com/track/3n3Ppam7vgaVa1iaRUc9Lp";
        assert_eq!(
            extract_track_id(url),
            Some("3n3Ppam7vgaVa1iaRUc9Lp".to_string())
        );
    }

    #[test]
    fn test_extract_track_id_https_url_with_query() {
        let url = "https://open.spotify.com/track/3n3Ppam7vgaVa1iaRUc9Lp?si=abc123def456";
        assert_eq!(
            extract_track_id(url),
            Some("3n3Ppam7vgaVa1iaRUc9Lp".to_string())
        );
    }

    #[test]
    fn test_extract_track_id_http_url() {
        let url = "http://open.spotify.com/track/3n3Ppam7vgaVa1iaRUc9Lp";
        assert_eq!(
            extract_track_id(url),
            Some("3n3Ppam7vgaVa1iaRUc9Lp".to_string())
        );
    }

    #[test]
    fn test_extract_track_id_spotify_uri() {
        let uri = "spotify:track:3n3Ppam7vgaVa1iaRUc9Lp";
        assert_eq!(
            extract_track_id(uri),
            Some("3n3Ppam7vgaVa1iaRUc9Lp".to_string())
        );
    }

    #[test]
    fn test_extract_track_id_embedded_in_text() {
        let text = "Check out this track https://open.spotify.com/track/3n3Ppam7vgaVa1iaRUc9Lp it's amazing!";
        assert_eq!(
            extract_track_id(text),
            Some("3n3Ppam7vgaVa1iaRUc9Lp".to_string())
        );
    }

    #[test]
    fn test_extract_track_id_no_track() {
        let text = "No Spotify links here";
        assert_eq!(extract_track_id(text), None);
    }

    #[test]
    fn test_extract_track_id_wrong_type() {
        let url = "https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M";
        assert_eq!(extract_track_id(url), None);
    }

    #[test]
    fn test_find_first_track_first_message() {
        let messages = vec![
            "https://open.spotify.com/track/111".to_string(),
            "https://open.spotify.com/track/222".to_string(),
        ];
        assert_eq!(find_first_track(&messages), Some("111".to_string()));
    }

    #[test]
    fn test_find_first_track_second_message() {
        let messages = vec![
            "No link here".to_string(),
            "https://open.spotify.com/track/222".to_string(),
        ];
        assert_eq!(find_first_track(&messages), Some("222".to_string()));
    }

    #[test]
    fn test_find_first_track_no_tracks() {
        let messages = vec!["No link here".to_string(), "Still no link".to_string()];
        assert_eq!(find_first_track(&messages), None);
    }

    #[test]
    fn test_find_first_track_empty() {
        let messages: Vec<String> = vec![];
        assert_eq!(find_first_track(&messages), None);
    }
}
