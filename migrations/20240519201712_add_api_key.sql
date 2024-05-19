CREATE TABLE api_keys (
	id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
	user_id UUID NOT NULL REFERENCES "user"(id),
	created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
