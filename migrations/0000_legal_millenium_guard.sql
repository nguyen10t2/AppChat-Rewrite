CREATE TYPE "public"."user_role" AS ENUM('USER', 'ADMIN');--> statement-breakpoint
CREATE TABLE "users" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"username" varchar(255) NOT NULL,
	"hash_password" text NOT NULL,
	"email" varchar(255) NOT NULL,
	"role" "user_role" DEFAULT 'USER' NOT NULL,
	"display_name" varchar(255) NOT NULL,
	"avatar_url" text,
	"avatar_id" text,
	"bio" varchar(300),
	"phone" varchar(20),
	"deleted_at" timestamptz,
	"created_at" timestamptz DEFAULT now() NOT NULL,
	"updated_at" timestamptz DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE "friend_requests" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"from_user_id" uuid NOT NULL,
	"to_user_id" uuid NOT NULL,
	"message" varchar(300),
	"created_at" timestamptz DEFAULT now() NOT NULL,
	CONSTRAINT "friend_request_not_self" CHECK ("friend_requests"."from_user_id" <> "friend_requests"."to_user_id")
);
--> statement-breakpoint
CREATE TABLE "friends" (
	"user_a" uuid NOT NULL,
	"user_b" uuid NOT NULL,
	"deleted_at" timestamptz,
	"created_at" timestamptz DEFAULT now() NOT NULL,
	CONSTRAINT "friends_user_a_user_b_pk" PRIMARY KEY("user_a","user_b"),
	CONSTRAINT "friends_user_order" CHECK ("friends"."user_a" < "friends"."user_b"),
	CONSTRAINT "friends_not_self" CHECK ("friends"."user_a" <> "friends"."user_b")
);
--> statement-breakpoint
ALTER TABLE "friend_requests" ADD CONSTRAINT "friend_requests_from_user_id_users_id_fk" FOREIGN KEY ("from_user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "friend_requests" ADD CONSTRAINT "friend_requests_to_user_id_users_id_fk" FOREIGN KEY ("to_user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "friends" ADD CONSTRAINT "friends_user_a_users_id_fk" FOREIGN KEY ("user_a") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "friends" ADD CONSTRAINT "friends_user_b_users_id_fk" FOREIGN KEY ("user_b") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE UNIQUE INDEX "idx_user_username" ON "users" USING btree (lower("username")) WHERE "users"."deleted_at" is null;--> statement-breakpoint
CREATE UNIQUE INDEX "idx_user_email" ON "users" USING btree (lower("email")) WHERE "users"."deleted_at" is null;--> statement-breakpoint
CREATE UNIQUE INDEX "idx_user_phone" ON "users" USING btree ("phone") WHERE ("users"."phone" is not null and "users"."deleted_at" is null);--> statement-breakpoint
CREATE INDEX "idx_user_created_desc" ON "users" USING btree ("created_at" DESC NULLS LAST);--> statement-breakpoint
CREATE UNIQUE INDEX "idx_friend_requests_from_user_to_user" ON "friend_requests" USING btree ("from_user_id","to_user_id");--> statement-breakpoint
CREATE INDEX "idx_friend_requests_to_user" ON "friend_requests" USING btree ("to_user_id");--> statement-breakpoint
CREATE INDEX "idx_friend_requests_from_user" ON "friend_requests" USING btree ("from_user_id");--> statement-breakpoint
CREATE INDEX "idx_friends_userA_active" ON "friends" USING btree ("user_a") WHERE "friends"."deleted_at" is null;--> statement-breakpoint
CREATE INDEX "idx_friends_userB_active" ON "friends" USING btree ("user_b") WHERE "friends"."deleted_at" is null;