-- Add last_seen_message_id column to participants table if it doesn't exist
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_name = 'participants'
        AND column_name = 'last_seen_message_id'
    ) THEN
        ALTER TABLE "participants" ADD COLUMN "last_seen_message_id" uuid;
        ALTER TABLE "participants" ADD CONSTRAINT "participants_last_seen_message_id_messages_id_fk" FOREIGN KEY ("last_seen_message_id") REFERENCES "public"."messages"("id") ON DELETE no action ON UPDATE no action;
    END IF;
END $$;
