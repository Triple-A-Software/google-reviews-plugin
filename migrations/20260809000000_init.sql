-- A Google place (business location) the site shows reviews for. `rating` and
-- `total` are the aggregate figures from the Places API; `fetched_at` drives the
-- TTL refresh. `maps_uri` is Google's URL for the place, used for the required
-- attribution link.
create table if not exists place (
    place_id text primary key,
    label text,
    rating real,
    total int,
    maps_uri text,
    fetched_at timestamptz,
    added_at timestamptz not null default now()
);

-- Cached reviews. The Places API returns at most 5 per place and forbids
-- long-term caching, so a refresh replaces the set (see delete-stale in the
-- code) — but `hidden` is preserved across refreshes by matching `google_id`
-- (the review's Places resource name). `google_id` is unique per place.
create table if not exists review (
    id bigint generated always as identity primary key,
    place_id text not null references place (place_id) on delete cascade,
    google_id text not null,
    author text not null,
    author_url text,
    photo_url text,
    rating int not null,
    text text,
    lang text,
    published_at timestamptz,
    relative_time text,
    hidden boolean not null default false,
    fetched_at timestamptz not null default now(),
    unique (place_id, google_id)
);

create index if not exists review_place_idx on review (place_id, hidden, rating desc);
create index if not exists review_published_idx on review (published_at desc);

create table if not exists review_settings (
    id text primary key default 'settings',
    -- Google Places API key (stored here, never in the CMS).
    api_key text,
    -- default number of reviews a block shows
    default_count int not null default 5,
    -- only show reviews with at least this star rating
    min_rating int not null default 1,
    -- how long a cached place is considered fresh before a render triggers a refresh
    ttl_minutes int not null default 720
);

insert into review_settings (id) values ('settings') on conflict (id) do nothing;
