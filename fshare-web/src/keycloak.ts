import Keycloak from 'keycloak-js'
// import { useEffect, useState } from 'react';

// Setup Keycloak instance as needed
// Pass initialization options as required or leave blank to load from 'keycloak.json'
const keycloak = new Keycloak({
  clientId: "fshare-webauth",
  url: "https://auth.f-skipper.com/protocol/openid-connect/auth",
  oidcProvider: "https://auth.f-skipper.com/realms/skippernet/",
  realm: "skippernet",
});
export default keycloak


// export const useAuth = () => {
//   const [kc, setKc] = useState({ keycloak })
//   const reRender = () => {
//     setKc({ keycloak })
//   }
//   useEffect(() => {
//     keycloak.on
//   },[])
// }