CREATE TABLE `admin_log` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`admin_id` text NOT NULL,
	`action` text NOT NULL,
	`target_id` text,
	`detail` text,
	`created_at` integer NOT NULL
);
--> statement-breakpoint
CREATE INDEX `admin_log_target_id_idx` ON `admin_log` (`target_id`);--> statement-breakpoint
CREATE INDEX `admin_log_created_at_idx` ON `admin_log` (`created_at`);