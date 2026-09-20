import { useEffect, useState } from "react";
import QRCode from "react-qr-code";
import { api } from "../api";
import { useNavigate, useSearchParams } from "react-router";
import keycloak from "../keycloak";

export const LoginPage = () => {

    const [params,] = useSearchParams()
    const return_to = params.get("return_to") || "/browse/";
    const navigate = useNavigate();

    useEffect(() => {
        if (keycloak.authenticated) {
            api("/authenticate-session", {
                method: "post",
                body: JSON.stringify({ token: keycloak.token!, refresh: keycloak.refreshToken! })
            }).then(() => {
                navigate(return_to);
            });
        }
    }, [navigate, return_to]);


    const [approvalUrl, setApprovalUrl] = useState<null | string>(null);

    useEffect(() => {
        if (!keycloak.authenticated) {
            api("/init-session").then(async (resp) => {
                if (resp.ok) {
                    const { approval_url: approvalUrl } = await resp.json();
                    setApprovalUrl(approvalUrl);
                }
            })
        }
    }, []);



    useEffect(() => {
        (async () => {
            while (true) {
                const resp = await api(`/await-status`);
                if (!resp.ok) {
                    continue
                }
                console.log("RETURNING!");
                window.location.href = window.location.origin +  return_to;
                

                return;

            }
        })();
    }, [return_to])

    return (
        <div className="flex justify-center items-center h-full">
            <div className="flex flex-col items-center">

                <h2 className="text-white text-4xl m-3">To approve this session, scan the QR code on an authorized device, or <a className="text-blue-600 hover:cursor-pointer" onClick={() => keycloak.login()}>login</a></h2>
                {approvalUrl && <>
                    <QRCode value={approvalUrl} size={256} />
                    <p>Or copy the link: <a target="_blank" className="text-blue-600" href={approvalUrl}>here</a></p>
                </>}
            </div>
        </div>
    )
}