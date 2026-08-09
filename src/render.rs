//! Pure HTML rendering for the public reviews block and the AggregateRating
//! JSON-LD. Everything Google-supplied (author names, review text, photo/author
//! URLs) is escaped here — the XSS boundary lives in this module.

use serde_json::json;

use crate::{
    lang::Lang,
    model::{Place, Review},
    utils::{escape_html, escape_multiline},
};

/// Full ★ / empty ☆ stars for an integer 1–5 rating.
fn stars_int(rating: i32) -> String {
    let n = rating.clamp(0, 5) as usize;
    format!("{}{}", "★".repeat(n), "☆".repeat(5 - n))
}

/// Stars for a fractional aggregate rating, rounded to the nearest whole star.
fn stars_round(rating: f32) -> String {
    stars_int(rating.round() as i32)
}

/// Render the reviews block for a place: a rating summary, the review cards, and
/// the required Google attribution.
pub fn reviews_block(place: Option<&Place>, reviews: &[Review], lang: Lang) -> String {
    let mut out = String::new();
    out.push_str(STYLE);
    out.push_str(r#"<section class="gr">"#);

    let maps_uri = place.and_then(|p| p.maps_uri.as_deref());

    if let Some(p) = place
        && let (Some(rating), Some(total)) = (p.rating, p.total)
    {
        out.push_str(&format!(
            r#"<div class="gr-summary"><span class="gr-stars">{stars}</span><span class="gr-score">{rating:.1}</span><span class="gr-count">{count}</span></div>"#,
            stars = stars_round(rating),
            rating = rating,
            count = lang.rating_count(total as i64),
        ));
    }

    if reviews.is_empty() {
        out.push_str(&format!(r#"<p class="gr-empty">{}</p>"#, lang.empty()));
    } else {
        out.push_str(r#"<ul class="gr-list">"#);
        for r in reviews {
            out.push_str(&review_card(r, lang));
        }
        out.push_str("</ul>");
    }

    // Attribution (Google TOS): name Google + link back.
    out.push_str(r#"<div class="gr-attr">"#);
    match maps_uri {
        Some(uri) => out.push_str(&format!(
            r#"<a href="{uri}" target="_blank" rel="noopener nofollow">{label}</a>"#,
            uri = escape_html(uri),
            label = lang.powered_by(),
        )),
        None => out.push_str(lang.powered_by()),
    }
    out.push_str("</div></section>");
    out
}

fn review_card(r: &Review, lang: Lang) -> String {
    let mut c = String::new();
    c.push_str(r#"<li class="gr-item">"#);
    c.push_str(r#"<div class="gr-head">"#);
    if let Some(photo) = r.photo_url.as_deref().filter(|s| !s.is_empty()) {
        c.push_str(&format!(
            r#"<img class="gr-photo" src="{}" alt="" loading="lazy" referrerpolicy="no-referrer">"#,
            escape_html(photo)
        ));
    }
    c.push_str(r#"<div class="gr-who">"#);
    let name = escape_html(&r.author);
    match r.author_url.as_deref().filter(|s| !s.is_empty()) {
        Some(url) => c.push_str(&format!(
            r#"<a class="gr-author" href="{}" target="_blank" rel="noopener nofollow">{name}</a>"#,
            escape_html(url)
        )),
        None => c.push_str(&format!(r#"<span class="gr-author">{name}</span>"#)),
    }
    c.push_str(&format!(
        r#"<div class="gr-meta"><span class="gr-stars">{}</span>{}</div>"#,
        stars_int(r.rating),
        r.relative_time
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(|t| format!(r#"<span class="gr-time">{}</span>"#, escape_html(t)))
            .unwrap_or_default(),
    ));
    c.push_str("</div></div>");
    if let Some(text) = r.text.as_deref().filter(|s| !s.is_empty()) {
        c.push_str(&format!(r#"<p class="gr-text">{}</p>"#, escape_multiline(text)));
    }
    let _ = lang;
    c.push_str("</li>");
    c
}

/// AggregateRating JSON-LD for a place, as a `LocalBusiness` carrying the rating.
/// Returns `""` when the place has no rating yet. `<` is escaped to `<` so
/// the JSON can't break out of the `<script>` tag.
pub fn aggregate_rating_jsonld(place: &Place) -> String {
    let (Some(rating), Some(total)) = (place.rating, place.total) else {
        return String::new();
    };
    if total <= 0 {
        return String::new();
    }
    let name = place.label.clone().unwrap_or_else(|| "Business".to_string());
    // Round in f64 to one decimal so serialization is exact (an f32 like 4.6
    // widens to 4.599999… otherwise).
    let rating_value = ((rating as f64) * 10.0).round() / 10.0;
    let value = json!({
        "@context": "https://schema.org",
        "@type": "LocalBusiness",
        "name": name,
        "aggregateRating": {
            "@type": "AggregateRating",
            "ratingValue": rating_value,
            "reviewCount": total,
            "bestRating": 5,
            "worstRating": 1
        }
    });
    let body = serde_json::to_string(&value).unwrap_or_default().replace('<', "\\u003c");
    format!(r#"<script type="application/ld+json">{body}</script>"#)
}

const STYLE: &str = r#"<style>
.gr { max-width: 760px; margin: 24px auto; font-family: system-ui, -apple-system, sans-serif; }
.gr-summary { display: flex; align-items: center; gap: 10px; margin-bottom: 16px; }
.gr-stars { color: #fbbc04; letter-spacing: 1px; }
.gr-score { font-size: 20px; font-weight: 700; }
.gr-count { color: #6b7280; font-size: 14px; }
.gr-empty { color: #6b7280; }
.gr-list { list-style: none; margin: 0; padding: 0; display: grid; gap: 16px; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); }
.gr-item { border: 1px solid #e5e7eb; border-radius: 12px; padding: 14px 16px; }
.gr-head { display: flex; gap: 10px; align-items: center; margin-bottom: 8px; }
.gr-photo { width: 40px; height: 40px; border-radius: 9999px; object-fit: cover; flex: none; }
.gr-who { min-width: 0; }
.gr-author { font-weight: 600; color: inherit; text-decoration: none; }
.gr-meta { display: flex; align-items: center; gap: 8px; font-size: 12px; }
.gr-time { color: #9ca3af; }
.gr-text { margin: 0; color: #374151; line-height: 1.5; font-size: 14px; word-wrap: break-word; overflow-wrap: anywhere; }
.gr-attr { margin-top: 16px; font-size: 12px; color: #6b7280; }
.gr-attr a { color: #6b7280; }
</style>"#;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn review(author: &str, rating: i32, text: &str) -> Review {
        Review {
            id: 1,
            place_id: "P".into(),
            author: author.into(),
            author_url: Some("https://maps.google.com/x".into()),
            photo_url: None,
            rating,
            text: Some(text.into()),
            lang: Some("de".into()),
            published_at: Some(Utc.with_ymd_and_hms(2026, 8, 9, 12, 0, 0).unwrap()),
            relative_time: Some("vor einem Monat".into()),
            hidden: false,
        }
    }

    fn place(rating: Option<f32>, total: Option<i32>) -> Place {
        Place {
            place_id: "P".into(),
            label: Some("Acme GmbH".into()),
            rating,
            total,
            maps_uri: Some("https://maps.google.com/?cid=1".into()),
            fetched_at: None,
            added_at: Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap(),
        }
    }

    #[test]
    fn stars_render_five_glyphs() {
        assert_eq!(stars_int(3).chars().count(), 5);
        assert_eq!(stars_int(3), "★★★☆☆");
        assert_eq!(stars_round(4.6), "★★★★★");
    }

    #[test]
    fn escapes_author_and_text() {
        let out = reviews_block(
            Some(&place(Some(4.6), Some(120))),
            &[review("<script>x</script>", 5, "nice & <b>bold</b>\nline2")],
            Lang::De,
        );
        assert!(out.contains("&lt;script&gt;x"));
        assert!(!out.contains("<script>x"));
        assert!(out.contains("nice &amp; &lt;b&gt;bold&lt;/b&gt;<br>line2"));
        assert!(out.contains("4.6"));
        assert!(out.contains("120 Bewertungen"));
        assert!(out.contains("Bewertungen von Google"));
    }

    #[test]
    fn empty_state_when_no_reviews() {
        let out = reviews_block(Some(&place(None, None)), &[], Lang::En);
        assert!(out.contains("No reviews yet."));
        assert!(out.contains("Reviews from Google"));
    }

    #[test]
    fn jsonld_has_rating_and_escapes_lt() {
        let out = aggregate_rating_jsonld(&place(Some(4.63), Some(120)));
        assert!(out.contains(r#""@type":"AggregateRating""#));
        assert!(out.contains(r#""reviewCount":120"#));
        assert!(out.contains(r#""ratingValue":4.6"#));
        assert!(!out.contains("</script></script>"));
    }

    #[test]
    fn jsonld_empty_without_rating() {
        assert_eq!(aggregate_rating_jsonld(&place(None, None)), "");
    }
}
