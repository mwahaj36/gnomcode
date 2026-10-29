import DocPage from "../../../components/DocPage";
import { pageMetadata } from "../../../lib/seo";
import { faqGroups } from "../../../lib/faq";
export const metadata = pageMetadata("/docs/faq", "Spawnt FAQ — Common Questions Answered | Spawnt", "Answers about Spawnt setup, agent support, model switching, encrypted Remote, pricing, privacy and troubleshooting.");
const schema = { "@context": "https://schema.org", "@type": "FAQPage", mainEntity: faqGroups.flatMap(group => group.questions.map(item => ({ "@type": "Question", name: item.question, acceptedAnswer: { "@type": "Answer", text: item.answer } }))) };
export default function Page() { return <DocPage title="Spawnt FAQ" answer="Spawnt runs official coding agents locally, preserves redacted history for continuity and offers encrypted paired Remote. These answers distinguish supported capabilities from remaining prelaunch acceptance and commercial decisions.">
<script type="application/ld+json" dangerouslySetInnerHTML={{ __html: JSON.stringify(schema).replace(/</g, "\u003c") }} />
{faqGroups.map(group => <section key={group.title}><h2>{group.title}</h2>{group.questions.map(item => <div key={item.question}><h3>{item.question}</h3><p>{item.answer}</p></div>)}</section>)}
</DocPage>; }
