# UserID=$(id -u) GroupID=$(id -g) 
AVAHI_PID=""

cleanup() {
    [ -n "$AVAHI_PID" ] && kill "$AVAHI_PID" 2>/dev/null
}

trap cleanup EXIT INT TERM

avahi-publish -aR fshare.local "$(ip -4 route get 1.1.1.1 | awk '{print $7; exit}')" &
AVAHI_PID=$!

UserID=0 GroupID=0 docker compose -f docker-compose.yml -f docker-compose.dev.yml up