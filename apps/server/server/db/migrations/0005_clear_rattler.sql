CREATE TABLE `share_links` (
	`code` text PRIMARY KEY NOT NULL,
	`discord_id` text NOT NULL,
	`kind` text NOT NULL,
	`expires_at` integer,
	`created_at` integer NOT NULL,
	FOREIGN KEY (`discord_id`) REFERENCES `users`(`discord_id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE INDEX `share_links_discord_id_idx` ON `share_links` (`discord_id`);--> statement-breakpoint
CREATE UNIQUE INDEX `share_links_one_main_and_custom` ON `share_links` (`discord_id`,`kind`) WHERE "share_links"."kind" != 'invite';--> statement-breakpoint
ALTER TABLE `users` ADD `legacy_share_link` integer DEFAULT true NOT NULL;