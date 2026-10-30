
export DATABASE_URL=postgres://fshare:fshare-password@localhost:5432/fshare-md
sea-orm-cli migrate up && sea-orm-cli generate entity -o fshare-backend/src/db