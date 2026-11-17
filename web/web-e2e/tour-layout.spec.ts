import { expect, test } from '@playwright/test';
for (const size of [{width:1440,height:900},{width:390,height:844},{width:844,height:390}]) {
 test(`tour fits ${size.width}x${size.height} without tab or disclosure jumps`, async ({page},info)=>{
  await page.setViewportSize(size); await page.goto('/');
  const tour=page.locator('.sp-tour');
  await tour.scrollIntoViewIfNeeded();
  const heights:number[]=[];
  for(const name of ['Overview','Sessions','Usage']) {
   await tour.getByRole('tab',{name,exact:true}).click();
   await expect.poll(async()=>{const box=await tour.boundingBox();return box!.y>=70 && box!.y+box!.height<=size.height+1;}).toBe(true);
   if(name==='Overview')await page.screenshot({path:info.outputPath('tour.png')});
   const panel=await tour.locator('.sp-tour-panel').boundingBox(); heights.push(panel!.height);
   expect((await tour.boundingBox())!.height).toBeLessThanOrEqual(size.height-80);
   expect(await tour.locator('details').count()).toBe(0);
  }
  expect(Math.max(...heights)-Math.min(...heights)).toBeLessThanOrEqual(1);
 });
}
