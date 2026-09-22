CREATE TABLE `control_activation_daily` (
	`discord_id` text NOT NULL,
	`type` text NOT NULL,
	`day` text NOT NULL,
	`count` integer DEFAULT 0 NOT NULL,
	PRIMARY KEY(`discord_id`, `type`, `day`)
);
--> statement-breakpoint
CREATE INDEX `control_activation_daily_discord_id_idx` ON `control_activation_daily` (`discord_id`);--> statement-breakpoint
CREATE TABLE `control_inventory` (
	`discord_id` text NOT NULL,
	`type` text NOT NULL,
	`count` integer DEFAULT 0 NOT NULL,
	`updated_at` integer NOT NULL,
	PRIMARY KEY(`discord_id`, `type`)
);
