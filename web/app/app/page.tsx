import { appMetadata } from "../../lib/seo";
import Client from "./Client";

export const metadata = appMetadata(
  "/app",
  "Gnomcode Workspace",
  "Your Gnomcode Progressive Web App hub: remote companion, fleet console, and session controls."
);

export default function AppPage() {
  return <Client />;
}
