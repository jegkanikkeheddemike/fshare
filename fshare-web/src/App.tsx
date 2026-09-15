import { useEffect, useState } from "react"
import QRCode from "react-qr-code"


function App() {

  const [approvalUrl, setApprovalUrl] = useState<null | string>(null);

  useEffect(() => {
    fetch(`${import.meta.env.VITE_API_URL}/init-session`).then(async (resp) => {
      if (resp.ok) {
        const url = await resp.text();
        setApprovalUrl(url)
      }
    })
  }, []);

  return (
    <div className="flex justify-center items-center h-full">
      <div className="flex flex-col items-center">

        <h2 className="text-white text-4xl m-3">To approve this session, scan the qr code on an authorized device</h2>
        {approvalUrl && <QRCode value={approvalUrl} size={256} />}
      </div>
    </div>
  )
}

export default App
