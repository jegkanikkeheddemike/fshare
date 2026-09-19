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
            navigate(return_to);
        }
    }, [navigate, return_to]);


    const [approvalUrl, setApprovalUrl] = useState<null | string>(null);
    const [sessionId, setSessionId] = useState<null | string>(null);
    const [sessionStatus, setSessionStatus] = useState<null | "Pending" | "Approved">(null);

    useEffect(() => {
        api("/init-session").then(async (resp) => {
            if (resp.ok) {
                const { approval_url: approvalUrl, session_id: sessionId } = await resp.json();
                setApprovalUrl(approvalUrl);
                setSessionId(sessionId);
                setSessionStatus("Pending");
            }
        })
    }, []);



    useEffect(() => {
        if (!sessionId) {
            return
        }
        (async () => {
            while (true) {
                const resp = await api(`/await-status/${sessionId}`);
                if (!resp.ok) {
                    continue
                }
                const status = await resp.json()
                if (status == "Pending") {
                    continue
                }

                setSessionStatus(status)
                return;

            }
        })();
    }, [sessionId])

    useEffect(() => {
        // NOTE, custom login through QR or link ignores keycloak. 
        // TODO: link them later
        if (sessionStatus === "Approved") {

            navigate(return_to)
            // window.location.href = window.location.origin + return_to;
        }
    }, [sessionStatus, return_to, navigate])

    return (
        <div className="flex justify-center items-center h-full">
            <div className="flex flex-col items-center">

                <h2 className="text-white text-4xl m-3">To approve this session, scan the QR code on an authorized device, or <a className="text-blue-600 hover:cursor-pointer" onClick={() => keycloak.login()}>login</a></h2>
                {approvalUrl && <>
                    <QRCode value={approvalUrl} size={256} />
                    {sessionStatus}
                    <p>Or copy the link: <a target="_blank" className="text-blue-600" href={approvalUrl}>here</a></p>
                </>}
            </div>
        </div>
    )
}