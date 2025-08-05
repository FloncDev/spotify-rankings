create table if not exists songs (
    id serial primary key,
    song_id text not null,
    playlist_id text not null,
    rating float8 not null default 1500,
    deviation float8 not null default 350,
    volatility float8 not null default 0.06,
    total_matches integer not null default 0
);