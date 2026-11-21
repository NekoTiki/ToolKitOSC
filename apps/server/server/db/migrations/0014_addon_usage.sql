CREATE TABLE `addon_usage` (
	`discord_id` text NOT NULL,
	`addon_id` text NOT NULL,
	`enabled` integer NOT NULL,
	`devices` integer,
	`updated_at` integer NOT NULL,
	PRIMARY KEY(`discord_id`, `addon_id`)
);
