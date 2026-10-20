import {cp,mkdir,rm,stat,lstat,rename} from 'node:fs/promises';
import path from 'node:path';
export async function assembleDesktop(web,companion,output) {
  const target=path.resolve(output);
  const sources=[path.resolve(web),path.resolve(companion)];
  if(target===path.parse(target).root || sources.some(source=>{
    return [path.relative(target,source),path.relative(source,target)].some(relative=>
      relative==='' || (!relative.startsWith('..'+path.sep) && relative!=='..' && !path.isAbsolute(relative)));
  })) throw new Error('Desktop output must be separate from its inputs');
  await stat(path.join(web,'index.html'));
  await stat(path.join(companion,'companion.html'));
  const current=await lstat(target).catch(error=>{if(error.code==='ENOENT')return null;throw error});
  if(current?.isSymbolicLink())throw new Error('Desktop output cannot be a symbolic link');
  const staged=target+'.assembling';
  await rm(staged,{recursive:true,force:true});
  await mkdir(staged,{recursive:true});
  await cp(web,staged,{recursive:true});
  await cp(companion,path.join(staged,'companion'),{recursive:true});
  // Both paths are resolved generated directories under the caller's explicit output parent.
  await rm(target,{recursive:true,force:true});
  await rename(staged,target);
}
