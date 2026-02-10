CREATE TABLE "files" (
    "id" uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    "filename" text NOT NULL,
    "original_filename" text NOT NULL,
    "mime_type" text NOT NULL,
    "file_size" bigint NOT NULL,
    "storage_path" text NOT NULL,
    "uploaded_by" uuid NOT NULL,
    "created_at" timestamptz NOT NULL DEFAULT NOW(),
    CONSTRAINT "files_uploaded_by_users_id_fk" FOREIGN KEY ("uploaded_by") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action
);--> statement-breakpoint
CREATE INDEX "idx_files_uploaded_by" ON "files" USING btree ("uploaded_by");--> statement-breakpoint
CREATE INDEX "idx_files_created_at" ON "files" USING btree ("created_at");
