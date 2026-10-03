import { type ReactNode } from "react";
import { useAuth } from "react-oidc-context";

export const PrivateRoute = ({ children }: { children: ReactNode }) => {

    const auth = useAuth();


    if (!auth.isAuthenticated) {
        auth.signinRedirect({ state: { returnTo: window.location.pathname } });
        return <p>Redirecting to login...</p>;
    }

    return children;
};