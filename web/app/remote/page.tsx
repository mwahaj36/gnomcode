import { appMetadata } from "../../lib/seo";
export const metadata = appMetadata("/remote", "Gnomcode Remote", "Pair your phone or browser with Gnomcode to view conversations and send instructions to supported AI coding agents over an encrypted channel.");
import Console from '../console/Client';

export default function RemotePage() {
  return <Console initialView="chat" />;
}
