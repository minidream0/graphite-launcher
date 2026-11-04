export function retainSnapshot(previous,next){return JSON.stringify(previous)===JSON.stringify(next)?previous:next}
export function createPoller({read,accept,onError,getDelay}){
 let stopped=false,pending=null,timer=null,again=false;
 function refresh(){
  if(stopped)return Promise.resolve();
  if(pending){again=true;return pending}
  clearTimeout(timer);
  pending=Promise.resolve().then(read).then(value=>{if(!stopped)accept(value)}).catch(error=>{if(!stopped)onError(error)}).finally(()=>{
   pending=null;if(stopped)return;const delay=again?0:getDelay();again=false;timer=setTimeout(refresh,delay);
  });
  return pending;
 }
 return {refresh,stop(){stopped=true;clearTimeout(timer)}};
}
