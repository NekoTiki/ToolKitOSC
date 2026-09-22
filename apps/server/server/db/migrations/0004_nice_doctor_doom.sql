DROP TABLE `control_inventory`;--> statement-breakpoint
CREATE TABLE `control_inventory` (
	`discord_id` text NOT NULL,
	`avatar_id` text NOT NULL,
	`type` text NOT NULL,
	`count` integer DEFAULT 0 NOT NULL,
	`updated_at` integer NOT NULL,
	PRIMARY KEY(`discord_id`, `avatar_id`, `type`)
);
