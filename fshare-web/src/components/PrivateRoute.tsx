import { useEffect, type ReactNode } from "react";
import keycloak from "../keycloak";

export const PrivateRoute = ({ children }: { children: ReactNode }) => {
    useEffect(() => {
        if (!keycloak.authenticated) {
            keycloak.login();
        }
    }, [])

    if (!keycloak.authenticated) {
        return <p>Redirecting to login...</p>;
    }

    return children;
};