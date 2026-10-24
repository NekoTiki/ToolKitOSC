ALTER TABLE `users` ADD `suspended_until` integer;--> statement-breakpoint
ALTER TABLE `users` ADD `suspend_reason` text;--> statement-breakpoint
ALTER TABLE `users` ADD `token_version` integer DEFAULT 0 NOT NULL;