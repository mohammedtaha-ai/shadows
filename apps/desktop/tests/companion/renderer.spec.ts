import { test, expect, type Page } from "@playwright/test";
import { mkdirSync, writeFileSync, readFileSync } from 'node:fs';
import { brotliCompressSync, brotliDecompressSync, constants } from 'node:zlib';
import colors from '../../../../assets/aylul/character-colors.json' with { type:'json' };
const expressions = ['idle','happy','listening','speaking','thinking','working','success','blocked','sleeping'];
const outfits = ['default','crown','moon','scarf','starry','winter','headphones','mask','halo','special'];

// Canvas output differs between rasterizers (GPU, SwiftShader, software Skia):
// gradients by a level or two, anti-aliased edges and shadow blur by up to 255 on
// single pixels. Each frame is therefore compared as the mean premultiplied RGBA
// of 4x4 pixel blocks (64x64 blocks for the 256x256 canvas). Across rasterizers
// a block channel moved by at most 17, so a block may move by TOLERANCE and no
// block more. Moving the earring by 2px moves blocks by up to 54, recolouring the
// 3px brow stroke of the special outfit by up to 128. A 2px stroke shifted by 40
// levels in one channel moves blocks by 14 and stays below the tolerance.
const BLOCK = 4, SIZE = 256 / BLOCK, TOLERANCE = 24;
const purpleFile = new URL('./fixtures/purple-frames.json', import.meta.url);
type Frames = Record<string, string>;

/** Renders every purple frame and returns its block means, base64 RGBA, keyed `character/<expression>/<outfit>` or `scene/<expression>`. */
async function purpleFrames(page: Page, color?: string): Promise<Frames> {
  return page.evaluate(async ({ expressions, outfits, color, block, size }) => {
    const fixture = (window as any).fixture;
    const canvas = document.createElement('canvas');
    canvas.width = canvas.height = block * size;
    const context = canvas.getContext('2d', { willReadFrequently: true })!;
    const blocks = async (url: string) => {
      const image = new Image();
      image.src = url;
      await image.decode();
      context.clearRect(0, 0, canvas.width, canvas.height);
      context.drawImage(image, 0, 0);
      const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data, sums = new Float64Array(size * size * 4);
      for (let y = 0; y < canvas.height; y++) for (let x = 0; x < canvas.width; x++) {
        const from = (y * canvas.width + x) * 4, to = (Math.floor(y / block) * size + Math.floor(x / block)) * 4, alpha = pixels[from + 3];
        for (let channel = 0; channel < 3; channel++) sums[to + channel] += pixels[from + channel] * alpha / 255;
        sums[to + 3] += alpha;
      }
      let text = '';
      for (const sum of sums) text += String.fromCharCode(Math.round(sum / (block * block)));
      return btoa(text);
    };
    const frames: Record<string, string> = {};
    for (const expression of expressions) {
      for (const outfit of outfits) frames[`character/${expression}/${outfit}`] = await blocks((await fixture.render(expression, outfit, color)).image);
      frames[`scene/${expression}`] = await blocks((await fixture.scene(expression, color)).image);
    }
    return frames;
  }, { expressions, outfits, color, block: BLOCK, size: SIZE });
}

test('record purple frames', async ({ page, browser }) => {
  test.skip(process.env.RECORD_PURPLE !== '1', 'Explicit baseline recording only');
  await page.goto('/tests/companion/fixture.html');
  const frames = await purpleFrames(page);
  const pixels = Buffer.concat(Object.values(frames).map(frame => Buffer.from(frame, 'base64')));
  const packed = brotliCompressSync(pixels, { params: { [constants.BROTLI_PARAM_QUALITY]: 11, [constants.BROTLI_PARAM_LGWIN]: 24 } });
  mkdirSync(new URL('./', purpleFile), { recursive: true });
  writeFileSync(purpleFile, JSON.stringify({ browser: browser.version(), block: BLOCK, size: SIZE, frames: Object.keys(frames), pixels: packed.toString('base64') }, null, 2) + '\n');
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
  const baseline=JSON.parse(readFileSync(purpleFile,'utf8'));
  expect([baseline.block,baseline.size]).toEqual([BLOCK,SIZE]);
  const pixels=brotliDecompressSync(Buffer.from(baseline.pixels,'base64')),length=SIZE*SIZE*4;
  expect(pixels.length).toBe(baseline.frames.length*length);
  for(const color of [undefined,colors.base,'red']){
    const frames=await purpleFrames(page,color),failures:string[]=[];
    expect(Object.keys(frames)).toEqual(baseline.frames);
    baseline.frames.forEach((key:string,index:number)=>{
      const actual=Buffer.from(frames[key],'base64'),expected=pixels.subarray(index*length,(index+1)*length);
      let over=0,worst=0;
      for(let block=0;block<length;block+=4){
        let difference=0;
        for(let channel=0;channel<4;channel++)difference=Math.max(difference,Math.abs(actual[block+channel]-expected[block+channel]));
        worst=Math.max(worst,difference);
        if(difference>TOLERANCE)over++;
      }
      if(over>0)failures.push(`${color??'absent'} ${key}: ${over} blocks over ${TOLERANCE}, worst ${worst}`);
    });
    expect(failures).toEqual([]);
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
