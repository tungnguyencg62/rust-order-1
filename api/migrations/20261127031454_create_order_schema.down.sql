-- Add down migration script here
DROP TABLE idempotency_keys;
DROP TABLE outbox;
DROP TABLE order_events;
DROP TABLE order_items;
DROP TABLE orders;
DROP TABLE products;