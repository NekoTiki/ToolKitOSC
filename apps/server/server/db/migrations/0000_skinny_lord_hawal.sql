CREATE TABLE `ai_access` (
	`discord_id` text PRIMARY KEY NOT NULL,
	`can_select_model` integer DEFAULT false NOT NULL,
	`granted_by` text NOT NULL,
	`granted_at` integer NOT NULL,
	`note` text,
	FOREIGN KEY (`discord_id`) REFERENCES `users`(`discord_id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `ai_avatar_usage` (
	`discord_id` text NOT NULL,
	`avatar_id` text NOT NULL,
	`day` text NOT NULL,
	`count` integer DEFAULT 0 NOT NULL,
	PRIMARY KEY(`discord_id`, `avatar_id`, `day`)
);
--> statement-breakpoint
CREATE TABLE `ai_credit_bonus` (
	`discord_id` text NOT NULL,
	`day` text NOT NULL,
	`bonus` integer DEFAULT 0 NOT NULL,
	PRIMARY KEY(`discord_id`, `day`)
);
--> statement-breakpoint
CREATE TABLE `ai_credit_usage` (
	`discord_id` text NOT NULL,
	`day` text NOT NULL,
	`spent` integer DEFAULT 0 NOT NULL,
	PRIMARY KEY(`discord_id`, `day`)
);
--> statement-breakpoint
CREATE TABLE `ai_generation_log` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`discord_id` text NOT NULL,
	`avatar_id` text,
	`avatar_name` text,
	`provider` text,
	`profile` text,
	`model` text,
	`success` integer NOT NULL,
	`failure_reason` text,
	`error_message` text,
	`credit_cost` integer,
	`duration_ms` integer,
	`created_at` integer NOT NULL
);
--> statement-breakpoint
CREATE INDEX `ai_generation_log_created_at_idx` ON `ai_generation_log` (`created_at`);--> statement-breakpoint
CREATE INDEX `ai_generation_log_discord_id_idx` ON `ai_generation_log` (`discord_id`);--> statement-breakpoint
CREATE TABLE `users` (
	`discord_id` text PRIMARY KEY NOT NULL,
	`username` text,
	`display_name` text,
	`avatar_url` text,
	`last_seen_at` integer NOT NULL
);
