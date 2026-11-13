import packageInfo from '../../../package.json';
import branding from '../../../branding.json';
const assets=new Set(['Gnomcode-Setup.exe','gnomcode.exe',`Gnomcode-v${packageInfo.version}-windows-x64.zip`,`SHA256SUMS-v${packageInfo.version}.txt`,'SHA256SUMS','latest.json']);
export async function GET(_request:Request,{params}:{params:Promise<{asset:string}>}){
 const {asset}=await params;
 if(!assets.has(asset))return new Response('Release asset not found.',{status:404});
 return Response.redirect(`${branding.releasesRepository}/releases/download/v${packageInfo.version}/${asset}`,302);
}
