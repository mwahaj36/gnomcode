import { appMetadata } from "../../lib/seo";
import Client from "./Client";
export const metadata = appMetadata("/console", "Gnomcode Console", "Your Gnomcode sessions, usage metadata and encrypted companion workspace.");
export default function Page() { return <Client />; }
