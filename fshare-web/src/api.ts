import { userManager } from "./oidc";

export const api = async (
    endpoint: string,
    options: RequestInit = {}
) => {
    const user = await userManager.getUser();

    const headers = new Headers(options.headers);

    if (options.body) {
        headers.set("Content-Type", "application/json");
    }

    if (user?.access_token) {
        headers.set("Authorization", `Bearer ${user.access_token}`);
    }

    return fetch("/api" + endpoint, {
        ...options,
        headers,
    });
};