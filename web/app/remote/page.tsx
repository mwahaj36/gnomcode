import { pageMetadata } from "../../lib/seo";
export const metadata = pageMetadata("/remote", "Gnomcode Remote — Control Your Agents From Anywhere", "Pair your phone or browser with Gnomcode to view conversations and send instructions to supported AI coding agents over an encrypted channel.", false);
import Console from '../console/Client';

export default function RemotePage() {
  return <Console initialView="chat" />;
}
