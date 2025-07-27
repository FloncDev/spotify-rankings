create table if not exists users (
    id serial primary key,
    spotify_id varchar(255) not null unique,
    access_token text not null,
    refresh_token text not null,
    expires_at timestamptz not null
);