import { useKeycloak } from "@react-keycloak/web";
import { type ReactNode } from "react";


export const PrivateRoute = (props: { children: ReactNode }) => {
    const { children } = props;
    const kc = useKeycloak();

    if (!kc.initialized) {
        kc.keycloak.init();
    }

    if (!kc.keycloak.authenticated) {
        kc.keycloak.login();
        return <div>
            <p>Redirecting to login...</p>
        </div>
    }

    return children;
}