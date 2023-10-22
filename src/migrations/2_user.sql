CREATE TABLE "user" (
  user_id uuid PRIMARY KEY default gen_random_uuid(),
  username varchar(20) COLLATE "case_insensitive" UNIQUE NOT NULL,
  email varchar(40) COLLATE "case_insensitive" UNIQUE NOT NULL,
  password_hash varchar(180) NOT NULL
);