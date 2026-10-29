import { pageMetadata } from "../../lib/seo";
import Client from "./Client";
export const metadata = pageMetadata("/settings", "Spawnt Settings", "Open your Spawnt profile, appearance and device settings.", false);
export default function Page() { return <Client />; }
