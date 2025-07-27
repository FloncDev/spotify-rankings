create table sessions (
    id serial primary key,
    user_id integer references users(id),
    token varchar(255) not null,
    created_at timestamptz default current_timestamp
);