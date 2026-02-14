CREATE TYPE "public"."conversation_type" AS ENUM('direct', 'group');--> statement-breakpoint
CREATE TABLE "conversations" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"type" "conversation_type" DEFAULT 'direct' NOT NULL,
	"created_at" timestamptz DEFAULT now() NOT NULL,
	"updated_at" timestamptz DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE "group_conversations" (
	"conversation_id" uuid PRIMARY KEY NOT NULL,
	"name" varchar(255) NOT NULL,
	"created_by" uuid NOT NULL,
	"avatar_url" varchar(500),
	"avatar_id" varchar(500)
);
--> statement-breakpoint
CREATE TABLE "last_messages" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"content" text,
	"sender_id" uuid NOT NULL,
	"conversation_id" uuid NOT NULL,
	"created_at" timestamptz DEFAULT now() NOT NULL,
	CONSTRAINT "last_messages_conversation_id_unique" UNIQUE("conversation_id")
);
--> statement-breakpoint
CREATE TABLE "participants" (
	"conversation_id" uuid NOT NULL,
	"user_id" uuid NOT NULL,
	"unread_count" integer DEFAULT 0 NOT NULL,
	"joined_at" timestamptz DEFAULT now() NOT NULL,
	"deleted_at" timestamptz,
	CONSTRAINT "participants_conversation_id_user_id_pk" PRIMARY KEY("conversation_id","user_id"),
	CONSTRAINT "unread_count_non_negative" CHECK ("participants"."unread_count" >= 0)
);
--> statement-breakpoint
ALTER TABLE "group_conversations" ADD CONSTRAINT "group_conversations_conversation_id_conversations_id_fk" FOREIGN KEY ("conversation_id") REFERENCES "public"."conversations"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "group_conversations" ADD CONSTRAINT "group_conversations_created_by_users_id_fk" FOREIGN KEY ("created_by") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "last_messages" ADD CONSTRAINT "last_messages_sender_id_users_id_fk" FOREIGN KEY ("sender_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "last_messages" ADD CONSTRAINT "last_messages_conversation_id_conversations_id_fk" FOREIGN KEY ("conversation_id") REFERENCES "public"."conversations"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "participants" ADD CONSTRAINT "participants_conversation_id_conversations_id_fk" FOREIGN KEY ("conversation_id") REFERENCES "public"."conversations"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "participants" ADD CONSTRAINT "participants_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE INDEX "idx_last_message_conversation" ON "last_messages" USING btree ("conversation_id","created_at" DESC NULLS LAST);--> statement-breakpoint
CREATE INDEX "idx_participants_user_conv_active" ON "participants" USING btree ("user_id","conversation_id") WHERE "participants"."deleted_at" is null;--> statement-breakpoint
CREATE INDEX "idx_participants_conversation" ON "participants" USING btree ("conversation_id") WHERE "participants"."deleted_at" is null;