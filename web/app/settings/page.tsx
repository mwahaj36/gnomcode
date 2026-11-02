import { pageMetadata } from "../../lib/seo";
import Client from "./Client";
export const metadata = pageMetadata("/settings", "Gnomcode Settings", "Open your Gnomcode profile, appearance and device settings.", false);
export default function Page() { return <Client />; }
