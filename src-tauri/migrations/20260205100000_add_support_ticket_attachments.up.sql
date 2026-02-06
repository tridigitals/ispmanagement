-- Support ticket attachments (references existing file_records)

CREATE TABLE IF NOT EXISTS public.support_ticket_attachments (
    id text PRIMARY KEY NOT NULL,
    message_id text NOT NULL REFERENCES public.support_ticket_messages(id) ON DELETE CASCADE,
    file_id text NOT NULL REFERENCES public.file_records(id) ON DELETE CASCADE,
    created_at timestamp with time zone NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_support_ticket_attachments_message_file
    ON public.support_ticket_attachments (message_id, file_id);

CREATE INDEX IF NOT EXISTS idx_support_ticket_attachments_message
    ON public.support_ticket_attachments (message_id);

