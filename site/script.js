const TEXT={
'pt-BR':{menu:'Menu','nav.studio':'Studio','nav.team':'Equipe','nav.kamafeu':'Kamafeu','hero.kicker':'UM ESTÚDIO CRIATIVO DO STUDIO POMAR','hero.title':'A voz é sua.<br><em>Faça música.</em>','hero.lede':'No Kamafeu, você escreve a melodia, ajusta a voz e acompanha cada etapa até o áudio.','hero.download':'Conheça o Kamafeu','hero.team':'Quem faz isso acontecer ↓','hero.open':'CÓDIGO ABERTO','hero.stamp':'FAÇA DO SEU JEITO',ribbon:'IDEIAS QUE GANHAM VOZ','intro.kicker':'FERRAMENTAS PARA FAZER MÚSICA','intro.title':'Música feita<br>com voz <em>sintetizada.</em>','intro.body':'A gente faz ferramentas para música e síntese vocal. Kamafeu é o editor de código aberto do Studio Pomar para projetos UTAU e OpenUtau. Composição, edição e renderização ficam no mesmo lugar.','team.kicker':'AS PESSOAS POR TRÁS DO POMAR','team.title':'Quatro pessoas.<br><em>Um pomar inteiro.</em>','team.link':'Acompanhe no GitHub ↗','team.xiao':'Programação · QA macOS','team.mori':'Direção de arte','team.makki':'Relatos de bugs · QA Linux','team.zoneryth':'Testes · QA Windows e Linux','team.note':'Papéis conforme publicados no README do projeto. xiao também realiza testes em macOS.','product.kicker':'CONHEÇA O KAMAFEU','product.title':'Da nota ao áudio,<br>no <em>seu compasso.</em>','product.body':'Componha, ajuste pitch e fonemas e renderize com seu voicebank. Kamafeu é uma workstation vocal em Rust para projetos UTAU e OpenUtau.','product.explore':'Explore o projeto','shot.edit':'EDIÇÃO / PIANO ROLL E PITCH','shot.render':'STUDIO POMAR / VOZ E MÚSICA','feature.compose':'Composição & pitch','feature.composeBody':'Notas, portamentos, vibrato e curvas de pitch contínuas.','feature.phonemes':'Fonemas & voicebanks','feature.phonemesBody':'Controle de fonemas, oto.ini e ferramentas de fonemização.','feature.render':'Renderização','feature.renderBody':'Motores nativos e suporte a ferramentas externas compatíveis.','feature.formats':'Seus formatos','feature.formatsBody':'Importe e exporte formatos usados na criação vocal.','download.kicker':'ENCONTRE SEU PRÓXIMO PASSO','download.title':'Seu próximo som<br>começa <em>aqui.</em>','download.body':'Kamafeu 3.0 Calafate está em estabilização e é distribuído como pré-release. Veja as notas e os pacotes disponíveis para sua plataforma.','download.web':'Ou experimente Kamafeu Web','download.note':'Código aberto para compor, experimentar e compartilhar.','footer.made':'Studio Pomar, feito por pessoas que também fazem música.','footer.downloads':'Downloads'},
en:{menu:'Menu','nav.studio':'Studio','nav.team':'Team','nav.kamafeu':'Kamafeu','hero.kicker':'AN INDEPENDENT STUDIO BY STUDIO POMAR','hero.title':'Your voice.<br><em>Make music.</em>','hero.lede':'With Kamafeu, write your melody, shape the voice, and follow each step through to the audio.','hero.download':'Meet Kamafeu','hero.team':'Meet the people behind it ↓','hero.open':'OPEN SOURCE','hero.stamp':'MAKE IT YOURS',ribbon:'IDEAS THAT FIND A VOICE','intro.kicker':'TOOLS FOR MAKING MUSIC','intro.title':'Music made<br>with <em>synthesized voice.</em>','intro.body':'We make tools for music and voice synthesis. Kamafeu is Studio Pomar’s open source editor for UTAU and OpenUtau projects. Compose, edit, and render in one place.','team.kicker':'THE PEOPLE BEHIND POMAR','team.title':'Four people.<br><em>A whole orchard.</em>','team.link':'Follow along on GitHub ↗','team.xiao':'Programming · macOS QA','team.mori':'Art direction','team.makki':'Bug reports · Linux QA','team.zoneryth':'Testing · Windows and Linux QA','team.note':'Roles as listed in the project README. xiao also tests on macOS.','product.kicker':'MEET KAMAFEU','product.title':'From note to audio,<br>at <em>your tempo.</em>','product.body':'Compose, tune pitch and phonemes, and render with your voicebank. Kamafeu is a vocal workstation in Rust for UTAU and OpenUtau projects.','product.explore':'Explore the project','shot.edit':'EDITING / PIANO ROLL AND PITCH','shot.render':'STUDIO POMAR / VOICE AND MUSIC','feature.compose':'Composition & pitch','feature.composeBody':'Notes, portamento, vibrato, and continuous pitch curves.','feature.phonemes':'Phonemes & voicebanks','feature.phonemesBody':'Phoneme control, oto.ini, and phonemization tools.','feature.render':'Rendering','feature.renderBody':'Native engines and support for compatible external tools.','feature.formats':'Your formats','feature.formatsBody':'Import and export formats used in vocal production.','download.kicker':'FIND YOUR NEXT STEP','download.title':'Your next sound<br>starts <em>here.</em>','download.body':'Kamafeu 3.0 Calafate is in stabilization and distributed as a pre-release. See the notes and packages available for your platform.','download.web':'Or try Kamafeu Web','download.note':'Open source for composing, experimenting, and sharing.','footer.made':'Studio Pomar, made by people who make music, too.','footer.downloads':'Downloads'}};
const select=document.querySelector('#languageSelect');const menuButton=document.querySelector('.menu-toggle');const nav=document.querySelector('#main-nav');function setLanguage(language){document.documentElement.lang=language;document.title=language==='en'?'Kamafeu Studio ✿ Studio Pomar':'Kamafeu Studio ✿ Studio Pomar';document.querySelectorAll('[data-i18n]').forEach(el=>{const value=TEXT[language][el.dataset.i18n];if(value!==undefined)el.innerHTML=value});select.value=language;try{localStorage.setItem('kamafeu-language',language)}catch(_){}}
menuButton.addEventListener('click',()=>{const open=nav.classList.toggle('open');menuButton.setAttribute('aria-expanded',String(open))});nav.querySelectorAll('a').forEach(link=>link.addEventListener('click',()=>{nav.classList.remove('open');menuButton.setAttribute('aria-expanded','false')}));select.addEventListener('change',()=>setLanguage(select.value));let initial='pt-BR';try{initial=localStorage.getItem('kamafeu-language')||initial}catch(_){}setLanguage(initial);

// Motion is opt-in only when the browser can reveal content safely.
const reduceMotion=window.matchMedia('(prefers-reduced-motion: reduce)').matches;
if(!reduceMotion && 'IntersectionObserver' in window){
  document.documentElement.classList.add('motion-ready');
  document.querySelector('.hero').classList.add('hero-enter');
  const revealTargets=document.querySelectorAll('.intro, .team, .team-card, .product-copy, .screenshots, .feature-list, .feature-list article, .download-intro, .download-art');
  revealTargets.forEach((element,index)=>{
    element.classList.add('reveal');
    if(element.matches('.team, .feature-list')){
      element.querySelectorAll('.team-card, article').forEach((card,cardIndex)=>card.style.setProperty('--reveal-delay',`${cardIndex*90}ms`));
    }
  });
  const observer=new IntersectionObserver((entries,activeObserver)=>{
    entries.forEach(entry=>{
      if(entry.isIntersecting){entry.target.classList.add('is-visible');activeObserver.unobserve(entry.target)}
    });
  },{threshold:.14,rootMargin:'0px 0px -45px 0px'});
  revealTargets.forEach(element=>observer.observe(element));

  const hero=document.querySelector('.hero');
  if(window.matchMedia('(hover: hover) and (pointer: fine)').matches){
    let frame=0;
    hero.addEventListener('pointermove',event=>{
      if(frame)return;
      frame=requestAnimationFrame(()=>{
        const bounds=hero.getBoundingClientRect();
        const x=(event.clientX-bounds.left)/bounds.width-.5;
        const y=(event.clientY-bounds.top)/bounds.height-.5;
        hero.style.setProperty('--pointer-x',`${x*8}px`);
        hero.style.setProperty('--pointer-y',`${y*8}px`);
        frame=0;
      });
    });
    hero.addEventListener('pointerleave',()=>{
      hero.style.setProperty('--pointer-x','0px');
      hero.style.setProperty('--pointer-y','0px');
    });
  }
}
