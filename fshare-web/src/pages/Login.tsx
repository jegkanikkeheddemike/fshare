import { useAuth } from "react-oidc-context";
import { useLocation } from "react-router";

export const LoginPage = () => {

    const auth = useAuth();
    const location = useLocation();

    return <>
        <h1>Login</h1>



        (For now, just go to keycloak)

        <button onClick={() => auth.signinRedirect({ state: { returnTo: location.state?.returnTo || "/browse/" } })}>
            Login
        </button>

    </>
}