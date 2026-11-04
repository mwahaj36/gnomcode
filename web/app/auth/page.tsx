import { appMetadata } from "../../lib/seo";
import Client from "./Client";
export const metadata = appMetadata("/auth", "Sign in to Gnomcode", "Sign in to connect your Gnomcode desktop account and paired devices.");
export default function Page() { return <Client />; }
