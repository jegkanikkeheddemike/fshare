
#!/usr/bin/env python3

import argparse
import os
import secrets
import sys
import webbrowser

from http.server import HTTPServer, BaseHTTPRequestHandler
from pathlib import Path
from urllib.parse import quote, urlparse, parse_qs

import requests

# ---------------------------------------------------------
# Configuration
# ---------------------------------------------------------

ISSUER = "https://auth.f-skipper.com/realms/skippernet"
CLIENT_ID = "fshare-webauth"

REDIRECT_HOST = "127.0.0.1"
REDIRECT_PORT = 8080
REDIRECT_URI = f"http://{REDIRECT_HOST}:{REDIRECT_PORT}/callback"

SCOPE = "openid profile email"


# ---------------------------------------------------------
# OAuth login
# ---------------------------------------------------------
import time
import requests

ISSUER = "https://auth.f-skipper.com/realms/skippernet"
CLIENT_ID = "fshare-migration-cli"
SCOPE = "openid profile email"


def login():
    metadata_url = f"{ISSUER}/.well-known/openid-configuration"

    metadata = requests.get(metadata_url, timeout=15)
    metadata.raise_for_status()
    metadata = metadata.json()

    device_endpoint = metadata.get(
        "device_authorization_endpoint",
        f"{ISSUER}/protocol/openid-connect/auth/device",
    )

    token_endpoint = metadata["token_endpoint"]

    # Request a device code.
    response = requests.post(
        device_endpoint,
        data={
            "client_id": CLIENT_ID,
            "scope": SCOPE,
        },
        timeout=15,
    )
    response.raise_for_status()

    device = response.json()

    verification_uri = device.get(
        "verification_uri_complete",
        device["verification_uri"],
    )

    print("\n" + "=" * 55)
    print("AUTHENTICATION REQUIRED")
    print("=" * 55)
    print(f"\nOpen this URL on your computer or phone:\n")
    print(verification_uri)

    if not device.get("verification_uri_complete"):
        print(f"\nEnter this code: {device['user_code']}")

    print("\nWaiting for authentication...")
    print("=" * 55 + "\n")

    # Poll the token endpoint.
    interval = device.get("interval", 5)
    expires_at = time.monotonic() + device["expires_in"]

    while time.monotonic() < expires_at:
        time.sleep(interval)

        response = requests.post(
            token_endpoint,
            data={
                "grant_type": (
                    "urn:ietf:params:oauth:grant-type:device_code"
                ),
                "client_id": CLIENT_ID,
                "device_code": device["device_code"],
            },
            timeout=15,
        )

        result = response.json()

        if response.ok:
            print("Authentication successful!\n")
            return result, token_endpoint

        error = result.get("error")

        if error == "authorization_pending":
            print("Waiting for login...", end="\r")
            continue

        if error == "slow_down":
            interval += 5
            continue

        if error == "access_denied":
            raise RuntimeError("Login was denied.")

        if error == "expired_token":
            raise RuntimeError("Device code expired. Please try again.")

        raise RuntimeError(
            f"Authentication failed: {result}"
        )

    raise TimeoutError("Device authorization timed out.")

# ---------------------------------------------------------
# API helpers
# ---------------------------------------------------------



class FileMigrator:
    def __init__(self, api_base, token, token_endpoint):
        self.api_base = api_base.rstrip("/")
        self.token = token
        self.token_endpoint = token_endpoint

        self.session = requests.Session()
        self.update_auth_header()

        self.directories_created = 0
        self.files_uploaded = 0
        self.files_skipped = 0
    def update_auth_header(self):
        self.session.headers["Authorization"] = (
            f"Bearer {self.token['access_token']}"
        )
def refresh_token(self):
    old_refresh_token = self.token.get("refresh_token")

    if not old_refresh_token:
        raise RuntimeError(
            "Access token expired and no refresh token is available."
        )

    print("\nAccess token expired. Refreshing...")

    response = requests.post(
        self.token_endpoint,
        data={
            "grant_type": "refresh_token",
            "client_id": CLIENT_ID,
            "refresh_token": old_refresh_token,
        },
        timeout=15,
    )

    response.raise_for_status()

    new_token = response.json()

    if not new_token.get("refresh_token"):
        new_token["refresh_token"] = old_refresh_token

    self.token = new_token
    self.update_auth_header()

    print("Token refreshed successfully.\n")
    def request(self, method, url, **kwargs):
        response = self.session.request(
            method, url, **kwargs
        )

        if response.status_code == 401:
            if "expired" not in response.text.lower():
                response.raise_for_status()

            self.refresh_token()

            # Rewind file streams before retrying uploads.
            data = kwargs.get("data")
            if hasattr(data, "seek"):
                data.seek(0)

            response = self.session.request(
                method, url, **kwargs
            )

        return response

    def url(self, endpoint, path="", trailing_slash=False):
        from urllib.parse import quote

        encoded_path = quote(path.strip("/"), safe="/")

        result = f"{self.api_base}/{endpoint}/{encoded_path}"

        if trailing_slash and not result.endswith("/"):
            result += "/"

        return result

    def list_directory(self, path):
        """
        Returns a dictionary:
        {
            "filename": {"is_dir": False, ...},
            "subdirectory": {"is_dir": True, ...}
        }
        """

        url = self.url("dir", path, trailing_slash=True)

        response = self.request(
            "GET",
            url,
            timeout=(10, 60),
        )

        # A missing destination directory can be created.
        if response.status_code == 404:
            return None

        self.check_response(response, path or "/")

        entries = response.json().get("entries", [])

        return {
            entry["relative_name"]: entry
            for entry in entries
        }

    def mkdir(self, path):
        url = self.url("mkdir", path)

        print(f"[DIR]  {path}")

        response = self.request(
            "POST",
            url,
            timeout=(10, 60),
        )

        self.check_response(response, path)

        self.directories_created += 1

    def upload(self, local_path, remote_path):
        url = self.url("upload", remote_path)

        print(
            f"[FILE] {remote_path} "
            f"({local_path.stat().st_size:,} bytes)"
        )

        with local_path.open("rb") as file:
            response = self.request(
                "POST",
                url,
                data=file,
                headers={
                    "Content-Type": "application/octet-stream",
                },
                timeout=(10, 3600),
            )

        self.check_response(response, remote_path)

        self.files_uploaded += 1

    @staticmethod
    def check_response(response, path):
        if not response.ok:
            raise RuntimeError(
                f"Request failed for '{path}': "
                f"HTTP {response.status_code}\n"
                f"{response.text[:1000]}"
            )


def migrate_directory(local_root, migrator, remote_root=""):
    local_root = Path(local_root).resolve()

    if not local_root.is_dir():
        raise ValueError(f"Not a directory: {local_root}")

    remote_root = remote_root.strip("/")

    print(f"Source:      {local_root}")
    print(f"Destination: {remote_root or '/'}\n")

    def migrate(local_dir, remote_dir):
        # Get the current contents of the remote directory.
        existing = migrator.list_directory(remote_dir)

        # Create the destination directory if it doesn't exist.
        if existing is None:
            if not remote_dir:
                raise RuntimeError(
                    "Could not access the remote root directory."
                )

            migrator.mkdir(remote_dir)
            existing = migrator.list_directory(remote_dir)

            if existing is None:
                raise RuntimeError(
                    f"Could not access directory after creation: "
                    f"{remote_dir}"
                )

        # Sort for predictable migration order.
        children = sorted(
            local_dir.iterdir(),
            key=lambda p: p.name,
        )

        # Process directories first.
        for child in children:
            if child.is_symlink() or not child.is_dir():
                continue

            remote_child = "/".join(
                filter(None, [remote_dir, child.name])
            )

            remote_entry = existing.get(child.name)

            if remote_entry and not remote_entry["is_dir"]:
                raise RuntimeError(
                    f"Remote file conflicts with local directory: "
                    f"{remote_child}"
                )

            migrate(child, remote_child)

        # Process files.
        for child in children:
            if child.is_symlink() or not child.is_file():
                continue

            remote_entry = existing.get(child.name)

            if remote_entry:
                if remote_entry["is_dir"]:
                    raise RuntimeError(
                        f"Remote directory conflicts with local file: "
                        f"{remote_dir}/{child.name}"
                    )

                print(f"[SKIP] {remote_dir}/{child.name}")
                migrator.files_skipped += 1
                continue

            remote_file = "/".join(
                filter(None, [remote_dir, child.name])
            )

            migrator.upload(child, remote_file)

    migrate(local_root, remote_root)

    print("\nMigration complete!")
    print(f"Directories created: {migrator.directories_created}")
    print(f"Files uploaded:      {migrator.files_uploaded}")
    print(f"Files skipped:       {migrator.files_skipped}")



# ---------------------------------------------------------
# Main
# ---------------------------------------------------------

def main():
    global CLIENT_ID
    parser = argparse.ArgumentParser(
        description="Migrate a local directory to f-skipper."
    )

    parser.add_argument(
        "directory",
        help="Local directory to migrate",
    )

    parser.add_argument(
        "--api-base",
        default="https://f-skipper.com",
        help="API base URL (default: https://f-skipper.com)",
    )

    parser.add_argument(
        "--remote-root",
        default="",
        help="Optional destination directory on the server",
    )

    parser.add_argument(
        "--client-id",
        default=CLIENT_ID,
        help="Keycloak public client ID",
    )

    args = parser.parse_args()

    
    CLIENT_ID = args.client_id

    try:
        token, token_endpoint = login()

        migrator = FileMigrator(
            args.api_base,
            token,
            token_endpoint,
        )

        migrate_directory(
            args.directory,
            migrator,
            args.remote_root,
        )

    except KeyboardInterrupt:
        print("\nMigration interrupted.")
        sys.exit(1)

    except Exception as error:
        print(f"\nERROR: {error}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
