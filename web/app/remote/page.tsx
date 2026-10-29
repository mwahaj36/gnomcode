import { pageMetadata } from "../../lib/seo";
export const metadata = pageMetadata("/remote", "Spawnt Remote — Control Your Agents From Anywhere", "Pair your phone or browser with Spawnt to view conversations and send instructions to supported AI coding agents over an encrypted channel.");
import RemoteCompanion from '../../components/RemoteCompanion';

export default function RemotePage() {
  return <main className="sp-remote-page"><RemoteCompanion /></main>;
}
