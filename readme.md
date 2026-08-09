# Google Reviews

Show off your Google rating. **Google Reviews** pulls your business's star rating
and latest reviews straight from Google and displays them on your site — no
third-party widget, no tracking scripts, and your reviews match your site's look.

Add the **Google Reviews** block to any page, paste in your Place ID, and you're
done. Reviews refresh themselves in the background, and you can hide any review
you'd rather not feature. Built-in structured data helps your rating show up as
stars in Google search results, too.

## Why you'll want it

- **Build trust.** Real Google reviews and your average star rating, right where
  visitors are deciding.
- **Zero maintenance.** Reviews refresh automatically on a schedule — set it up
  once and forget it.
- **On brand.** The reviews render in your site's style and language, not a
  generic Google iframe.
- **You choose what shows.** Hide individual reviews, set a minimum star rating,
  and pick how many to display.
- **Better search results.** Optional structured data (AggregateRating) can earn
  you star ratings in Google's search snippets.
- **Private.** Reviews are fetched server-side and cached on your own server; no
  Google tracking scripts run on your visitors' browsers.

## How to use it

1. **Install** Google Reviews from the plugin store.
2. In the **Google Reviews** admin panel → **Settings**, paste a **Google Places
   API key** (with the *Places API (New)* enabled).
3. **Add a place** by its **Google Place ID** (find yours with Google's Place ID
   finder). The plugin fetches its rating and reviews right away.
4. Drop the **Google Reviews** block onto a page (or add
   `{{ google_reviews("YOUR_PLACE_ID", 5) }}` to a template) and choose how many
   reviews to show.
5. *(Optional)* add `{{ reviews_aggregate_rating("YOUR_PLACE_ID") }}` to your
   page for rich-result star markup.

## Good to know

- **Google shows up to 5 reviews per place** through the Places API — that's a
  Google limit, not the plugin's.
- **Refreshing** happens automatically (default every 12 hours) and whenever you
  click **Refresh** in the admin. Google's terms ask apps not to store reviews
  long-term, so the plugin always keeps the current set.
- **Moderation:** hide any review from the admin; it stays hidden even after the
  next refresh.
- **Attribution:** each block links back to Google, as Google's terms require.
- **Your API key** is stored with the plugin, never in your CMS content, and is
  never exposed to visitors.

---

Building on or contributing to the plugin? See [DEVELOPMENT.md](DEVELOPMENT.md).
