export function matchingInstances(instances,version){
 const wanted=(version||'').trim();return instances.filter(i=>!i.error&&(!wanted||i.version.trim()===wanted));
}
export function chooseInstance(instances,version,previous=''){
 const matches=matchingInstances(instances,version);
 if(matches.some(i=>i.id===previous))return previous;
 return version.trim()&&matches.length===1?matches[0].id:'';
}
export function normalizeServer(form){return {...form,name:form.name.trim(),address:form.address.trim(),minecraft:(form.minecraft||'').trim(),project:(form.project||'').trim()}}
export function validateServer(form,instances){
 const f=normalizeServer(form),errors={};
 if(!f.name||f.name.length>80||/[\x00-\x1f\x7f]/.test(f.name))errors.name='Укажи название сервера, до 80 символов.';
 if(!f.address)errors.address='Укажи адрес сервера, например play.example.org.';
 else try{const url=new URL('http://'+f.address);if(f.address.length>255||/[\s/\\?#@]/.test(f.address)||url.port==='0')throw Error();}catch{errors.address='Укажи адрес без http:// и пробелов. Порт — от 1 до 65535.'}
 if(f.minecraft.length>80||/[\x00-\x1f\x7f]/.test(f.minecraft))errors.minecraft='Проверь версию Minecraft.';
 if(f.project&&!/^[a-zA-Z0-9_-]{1,100}$/.test(f.project))errors.project='Нужен ID или slug Modrinth, например fabulously-optimized, без ссылки.';
 if(f.instanceId&&!matchingInstances(instances,f.minecraft).some(i=>i.id===f.instanceId))errors.instanceId=f.minecraft?`Выбери установленную сборку ${f.minecraft} или оставь сервер без привязки.`:'Эта сборка недоступна. Выбери другую.';
 return errors;
}
