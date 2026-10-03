import { UserManager, WebStorageStateStore } from "oidc-client-ts";

export const userManager = new UserManager({
    authority: "https://auth.f-skipper.com/realms/skippernet",
    client_id: "fshare-webauth",
    redirect_uri: window.location.origin + "/auth_callback",
    post_logout_redirect_uri: window.location.origin,
    response_type: "code",
    scope: "openid profile email",
    automaticSilentRenew: true,
    userStore: new WebStorageStateStore({
        store: window.localStorage,
    }),
});