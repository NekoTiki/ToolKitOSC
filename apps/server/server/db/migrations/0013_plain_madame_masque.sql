CREATE TABLE `relay_daily` (
	`day` text PRIMARY KEY NOT NULL,
	`peak_hosts` integer DEFAULT 0 NOT NULL,
	`peak_viewers` integer DEFAULT 0 NOT NULL,
	`joins` integer DEFAULT 0 NOT NULL,
	`rate_limited` integer DEFAULT 0 NOT NULL
);
