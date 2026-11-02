import { pageMetadata } from "../../lib/seo";
import Client from "./Client";
export const metadata = pageMetadata("/auth", "Sign in to Gnomcode", "Sign in to connect your Gnomcode desktop account and paired devices.", false);
export default function Page() { return <Client />; }
