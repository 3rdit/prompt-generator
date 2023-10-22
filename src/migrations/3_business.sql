CREATE TABLE "business" (
  business_id uuid PRIMARY KEY default gen_random_uuid(),
  owner uuid NOT NULL REFERENCES "user" (user_id) ON DELETE CASCADE,
  name varchar(40) NOT NULL,
  prompt text NOT NULL
);
