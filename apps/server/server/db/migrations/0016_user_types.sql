ALTER TABLE `users` ADD `client_seen_at` integer;--> statement-breakpoint
ALTER TABLE `users` ADD `web_seen_at` integer;--> statement-breakpoint
UPDATE `users` SET `client_seen_at` = `last_seen_at` WHERE `app_version` IS NOT NULL
  OR `discord_id` IN (SELECT `discord_id` FROM `addon_usage`)
  OR `discord_id` IN (SELECT `discord_id` FROM `control_inventory`)
  OR `discord_id` IN (SELECT `discord_id` FROM `control_activation_daily`)
  OR `discord_id` IN (SELECT `discord_id` FROM `ai_generation_log`)
  OR `discord_id` IN (SELECT `discord_id` FROM `ai_credit_usage`)
  OR `discord_id` IN (SELECT `discord_id` FROM `ai_access_requests`);