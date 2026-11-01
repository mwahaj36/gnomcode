'use client';

export type ModelChoice = {agent:string; model:string; display_name:string; availability_source?:string};
export type ProviderCatalog = {agent:string; name:string; status:string; message:string; models:ModelChoice[]};

export default function ModelChoices({providers,loading,error,disabled,onRefresh,onSelect}: {
  providers:ProviderCatalog[]; loading:boolean; error:string; disabled:boolean;
  onRefresh:()=>void; onSelect:(model:ModelChoice)=>void;
}) {
  return <>
    <p>Models from the coding-agent accounts on your desktop. Switching provider keeps this conversation.</p>
    <button type="button" disabled={loading || disabled} onClick={onRefresh}>Refresh models</button>
    {loading && <p role="status">Checking your desktop CLI accounts...</p>}
    {error && <p role="alert">{error}</p>}
    {!loading && !error && !providers.length && <p>No model catalog received. Refresh, or update Spawnt on your desktop.</p>}
    {providers.map(provider=><section key={provider.agent} aria-label={provider.name}>
      <h3>{provider.name}</h3><p>{provider.message}</p>
      <nav aria-label={`${provider.name} models`}>{provider.models.map(model=><button type="button" key={model.model} disabled={loading || disabled} onClick={()=>onSelect(model)}>{model.display_name}</button>)}</nav>
    </section>)}
  </>;
}
