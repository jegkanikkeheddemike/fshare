export const api = (endpoint?: string) => {
    if (endpoint) {
        return "/api" + endpoint;
    }
    return "/api";

}
