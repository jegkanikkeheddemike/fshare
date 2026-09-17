import keycloak from "./keycloak";

export const api = async (endpoint: string, options?: RequestInit) => {

    const token = keycloak.idToken;

    options = options || { headers: {} }

    if (token) {
        options.headers = {
            ...options.headers,
            "Authorization": `Bearer ${token}`
        }
    }

    return await fetch("/api"+ endpoint, options)
}
