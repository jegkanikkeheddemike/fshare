import { useEffect, useState } from "react";
import QRCode from "react-qr-code";
import { api } from "../api";
import { useKeycloak } from "@react-keycloak/web";

export const Index = () => {

    const kc = useKeycloak();

    const [approvalUrl, setApprovalUrl] = useState<null | string>(null);
    const [sessionId, setSessionId] = useState<null | string>(null);
    const [sessionStatus, setSessionStatus] = useState<null | "Pending" | "Approved">(null);

    useEffect(() => {
        fetch(api("/init-session")).then(async (resp) => {
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
                const resp = await fetch(api(`/await-status/${sessionId}`));
                if (!resp.ok) {
                    continue
                }
                const status = await resp.json()
                console.log("RECEIVED STATUS:", status)
                if (status == "Pending") {
                    continue
                }

                setSessionStatus(status)
                return;

            }
        })();


    }, [sessionId])

    return (
        <div className="flex justify-center items-center h-full">
            <div className="flex flex-col items-center">

                <h2 className="text-white text-4xl m-3">To approve this session, scan the QR code on an authorized device, or <a className="text-blue-600 hover:cursor-pointer" onClick={() => kc.keycloak.login()}>login</a></h2>
                {approvalUrl && <>
                    <QRCode value={approvalUrl} size={256} />
                    {sessionStatus}
                    <p>Or copy the link: <a target="_blank" className="text-blue-600" href={approvalUrl}>here</a></p>
                </>}

            </div>
        </div>
    )
}