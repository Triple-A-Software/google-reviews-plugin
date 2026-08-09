//! Minimal i18n for the visitor-facing reviews UI. The active language comes
//! from the CMS page (`page.language` in the inline-helper request), so the
//! block renders in whatever language the surrounding page is in.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    En,
    De,
}

impl Lang {
    /// Pick a language from a CMS page language code such as `"de"`, `"de-DE"`
    /// or `"en-US"`. Anything that isn't German falls back to English.
    pub fn from_code(code: Option<&str>) -> Self {
        match code {
            Some(c) if c.trim().to_ascii_lowercase().starts_with("de") => Lang::De,
            _ => Lang::En,
        }
    }

    /// `"234 Bewertungen"` / `"234 reviews"`.
    pub fn rating_count(self, total: i64) -> String {
        match (self, total) {
            (Lang::De, 1) => "1 Bewertung".to_string(),
            (Lang::De, n) => format!("{n} Bewertungen"),
            (Lang::En, 1) => "1 review".to_string(),
            (Lang::En, n) => format!("{n} reviews"),
        }
    }

    pub fn empty(self) -> &'static str {
        match self {
            Lang::De => "Noch keine Bewertungen.",
            Lang::En => "No reviews yet.",
        }
    }

    /// Required Google attribution shown under the reviews.
    pub fn powered_by(self) -> &'static str {
        match self {
            Lang::De => "Bewertungen von Google",
            Lang::En => "Reviews from Google",
        }
    }

    pub fn read_on_google(self) -> &'static str {
        match self {
            Lang::De => "Auf Google ansehen",
            Lang::En => "View on Google",
        }
    }
}
