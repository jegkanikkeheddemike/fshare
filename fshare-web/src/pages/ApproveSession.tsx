import { useState } from "react";
import { useParams } from "react-router"
import { api } from "../api";

export const ApproveSessionPage = () => {

    const { sessionId } = useParams<{ sessionId: string }>();
    console.log("SESSION_ID:", sessionId);
    const [status, setStatus] = useState<"waiting" | "loading" | "success" | "error">("waiting");



    const approve = async () => {
        setStatus("loading");

        const resp = await api(`/approve-session/${sessionId}`)

        if (resp.ok) {
            setStatus("success")
        } else {
            setStatus("error");
        }
    }

    return <>
        <div className="flex flex-col items-center justify-center w-full h-full">
            {status == "waiting" && <>
                <h1 className="text-white text-3xl">Approve the session?</h1>
                <button className="text-white text-xl" onClick={approve}>approve</button>
            </>}

            {status == "loading" && <h1 className="text-2xl text-white">Thinking...</h1>}

            {status == "success" && <>
                <h1 className="text-3xl text-white">Session approved</h1>
                <p className="text-xl text-white">You can now close this window.</p>
                <button className="text-white text-2xl bg-gray-400 px-10 py-5 rounded-lg border-4 border-gray-900 hover:bg-gray-500" onClick={() => window.close()}>Close</button>
            </>}
            {status == "error" && <>
                <h1 className="text-3xl text-white">Approval failed</h1>
                <p>Refresh to try again, or retry with a new QR code.</p>
            </>}
        </div>
    </>
}