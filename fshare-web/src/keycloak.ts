import Keycloak from 'keycloak-js'

// Setup Keycloak instance as needed
// Pass initialization options as required or leave blank to load from 'keycloak.json'
const keycloak = new Keycloak({
  clientId: "fshare-webauth",
  oidcProvider: "https://auth.f-skipper.com/realms/skippernet/",
  realm: "skippernet",
});
export default keycloak