CREATE TABLE `ai_access_requests` (
	`discord_id` text PRIMARY KEY NOT NULL,
	`message` text,
	`status` text NOT NULL,
	`requested_at` integer NOT NULL,
	FOREIGN KEY (`discord_id`) REFERENCES `users`(`discord_id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
ALTER TABLE `ai_access` ADD `daily_credits` integer;