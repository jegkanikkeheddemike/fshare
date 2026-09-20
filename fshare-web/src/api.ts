
export const api = async (endpoint: string, options?: RequestInit) => {
    if (options?.body) {
        options.headers = {
            ...options.headers,
            "Content-Type": "Application/json"
        }
    }
    return await fetch("/api" + endpoint, options)
}
