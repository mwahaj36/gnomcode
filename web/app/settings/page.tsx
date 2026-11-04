import { appMetadata } from "../../lib/seo";
import Client from "./Client";
export const metadata = appMetadata("/settings", "Gnomcode Settings", "Open your Gnomcode profile, appearance and device settings.");
export default function Page() { return <Client />; }
