import { useKeycloak } from "@react-keycloak/web"
import type { ReactNode } from "react";
import { Link } from "react-router";

export const Header = () => {

    const kc = useKeycloak();

    const browse_links: ReactNode[] = ["/"];
    if (window.location.pathname.startsWith("/browse/")) {
        const subs = decodeURI(window.location.pathname).split("/");
        subs.pop()
        subs.shift()
        subs.shift()

        if (subs.length > 0) {
            let c_link = "/browse/";

            for (let i = 0; i < subs.length; i++) {
                c_link += subs[i] + "/"
                browse_links.push(<Link to={c_link}>{subs[i]}</Link>)
                browse_links.push("/")
            }
        }
    }

    return <div className="w-dvw h-20 bg-yellow-400 flex flex-row justify-between">
        <div className="h-full flex items-center">
            <Link to="/browse/">
                <header className="text-3xl p-3">File share!</header>
            </Link>
            {window.location.pathname.startsWith("/browse/") && <h1 className="text-2xl">
                {browse_links}
            </h1>}

        </div>
        <div className="h-full flex flex-row items-center">
            {!kc.keycloak.authenticated && <Link to={`/login?return_to=${encodeURI(window.location.pathname)}`}>
                <div className="px-4 py-2 m-2 rounded bg-gray-400 hover:bg-gray-500 hover:cursor-pointer">Login</div>
            </Link>}
        </div>
    </div>
}