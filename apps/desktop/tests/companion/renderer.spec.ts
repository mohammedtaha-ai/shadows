import { test, expect } from "@playwright/test";
import { createHash } from 'node:crypto';
import { mkdirSync, writeFileSync, readFileSync } from 'node:fs';
import colors from '../../../../assets/aylul/character-colors.json' with { type:'json' };
const expressions = ['idle','happy','listening','speaking','thinking','working','success','blocked','sleeping'];
const outfits = ['default','crown','moon','scarf','starry','winter','headphones','mask','halo','special'];

test('record purple frames', async ({ page, browser }) => {
  test.skip(process.env.RECORD_PURPLE !== '1', 'Explicit baseline recording only');
  await page.goto('/tests/companion/fixture.html');
  const frames = await page.evaluate(async ({ expressions, outfits }) => {
    const fixture = (window as any).fixture;
    const character: Record<string,string> = {}, scene: Record<string,string> = {};
    for (const expression of expressions) {
      for (const outfit of outfits) character[`${expression}/${outfit}`] = (await fixture.render(expression, outfit)).image;
      scene[expression] = (await fixture.scene(expression)).image;
    }
    return { character, scene };
  }, { expressions, outfits });
  const hash = (values: Record<string,string>) => Object.fromEntries(Object.entries(values).map(([key,value]) => [key,createHash('sha256').update(value).digest('hex')]));
  const directory = new URL('./fixtures/',import.meta.url);
  mkdirSync(directory,{ recursive:true });
  writeFileSync(new URL('purple-frames.json',directory),JSON.stringify({ browser:browser.version(), character:hash(frames.character), scene:hash(frames.scene) },null,2)+'\n');
});
test.beforeEach(async ({ page }) => {
  await page.goto("/tests/companion/fixture.html");
});
test("all expressions and outfits draw without global engine", async ({
  page,
}) => {
  const samples = await page.evaluate(async () => {
    const fixture = (window as any).fixture;
    const images = [];
    for (const expression of [
      "idle",
      "happy",
      "listening",
      "speaking",
      "thinking",
      "working",
      "success",
      "blocked",
      "sleeping",
    ]) {
      for (const outfit of [
        "default",
        "crown",
        "moon",
        "scarf",
        "starry",
        "winter",
        "headphones",
        "mask",
        "halo",
        "special",
      ]) {
        const result = await fixture.render(expression, outfit);
        images.push(result);
      }
    }
    return images;
  });
  for (const sample of samples) {
    expect(sample.error).toBeUndefined();
    expect(sample.visible).toBeGreaterThan(100);
  }
  expect(new Set(samples.map((s) => s.image)).size).toBeGreaterThan(10);
});
test("same inputs produce same pixels", async ({ page }) => {
  expect(
    await page.evaluate(
      async () => (await (window as any).fixture.render()).deterministic,
    ),
  ).toBe(true);
});
test("character render preserves transparent corners", async ({ page }) => {
  expect(
    await page.evaluate(
      async () => (await (window as any).fixture.render()).corner,
    ),
  ).toBe(0);
});
test("defaults match source", async ({ page }) => {
  const settings = await page.evaluate(
    async () => (await (window as any).fixture.render()).defaults,
  );
  expect(settings).toMatchObject({
    depth3D: 1.35,
    glowIntensity: 1.25,
    floatSpeed: 1,
    earringSwing: 1.25,
  });
});
test("purple draws today's pixels", async ({ page }) => {
  const baseline=JSON.parse(readFileSync(new URL('./fixtures/purple-frames.json',import.meta.url),'utf8'));
  for(const color of [undefined,colors.base,'red']){
    const frames=await page.evaluate(async({expressions,outfits,color})=>{
      const fixture=(window as any).fixture,character:Record<string,string>={},scene:Record<string,string>={};
      for(const expression of expressions){
        for(const outfit of outfits)character[`${expression}/${outfit}`]=(await fixture.render(expression,outfit,color)).image;
        scene[expression]=(await fixture.scene(expression,color)).image;
      }
      return {character,scene};
    },{expressions,outfits,color});
    for(const kind of ['character','scene'] as const)for(const [key,data] of Object.entries(frames[kind]))expect(createHash('sha256').update(data).digest('hex'),`${color??'absent'} ${kind} ${key}`).toBe(baseline[kind][key]);
  }
});
test('each colour in the table draws its own look',async({page})=>{
  const images=await page.evaluate(async(ids)=>Promise.all(ids.map(id=>(window as any).fixture.render('idle','default',id))),colors.colors.map(color=>color.id));
  const base=images[colors.colors.findIndex(color=>color.id===colors.base)];
  for(let i=0;i<images.length;i++){
    expect(images[i].error).toBeUndefined();expect(images[i].corner).toBe(0);
    expect(images[i].visible).toBeGreaterThanOrEqual(base.visible*0.99);
    if(colors.colors[i].id!==colors.base)expect(images[i].image).not.toBe(base.image);
  }
});
test('signals keep their colours in every colour',async({page})=>{
  for(const color of colors.colors){
    const samples=await page.evaluate(async(id)=>({warning:(await(window as any).fixture.render('blocked','default',id)).warning,success:(await(window as any).fixture.render('success','default',id)).success}),color.id);
    for(const [key,expected] of [['warning',[244,63,94]],['success',[251,191,36]]] as const){
      expect(samples[key][3]).toBe(255);
      for(let channel=0;channel<3;channel++)expect(Math.abs(samples[key][channel]-expected[channel]),`${color.id} ${key}`).toBeLessThanOrEqual(2);
    }
  }
});
test('export colour review stills',async({page},info)=>{
  test.skip(process.env.EXPORT_COLORS!=='1','Explicit review export only');
  const folder=new URL('../../../../output/playwright/character-colours/',import.meta.url);
  mkdirSync(folder,{recursive:true});
  const frames=[...colors.colors.map(color=>({id:color.id,expression:'idle'})),...['working','blocked'].map(expression=>({id:colors.colors[0].id,expression}))];
  for(const frame of frames){
    const result=await page.evaluate(async({id,expression})=>(window as any).fixture.render(expression,'default',id),frame);
    expect(result.error).toBeUndefined();
    const name=`${frame.id}-${frame.expression}.png`,path=new URL(name,folder);
    writeFileSync(path,Buffer.from(result.image.split(',')[1],'base64'));
    await info.attach(name,{body:Buffer.from(result.image.split(',')[1],'base64'),contentType:'image/png'});
  }
});
