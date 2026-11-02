import { pageMetadata } from "../../lib/seo";
import Client from "./Client";
export const metadata = pageMetadata("/console", "Gnomcode Console", "Your Gnomcode sessions, usage metadata and encrypted companion workspace.", false);
export default function Page() { return <Client />; }
