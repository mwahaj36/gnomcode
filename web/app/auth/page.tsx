import { pageMetadata } from "../../lib/seo";
import Client from "./Client";
export const metadata = pageMetadata("/auth", "Sign in to Spawnt", "Sign in to connect your Spawnt desktop account and paired devices.", false);
export default function Page() { return <Client />; }
