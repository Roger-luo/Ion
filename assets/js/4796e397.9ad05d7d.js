"use strict";(self.webpackChunkdocs=self.webpackChunkdocs||[]).push([[93],{3905:(e,t,n)=>{n.d(t,{Zo:()=>p,kt:()=>f});var o=n(7294);function r(e,t,n){return t in e?Object.defineProperty(e,t,{value:n,enumerable:!0,configurable:!0,writable:!0}):e[t]=n,e}function a(e,t){var n=Object.keys(e);if(Object.getOwnPropertySymbols){var o=Object.getOwnPropertySymbols(e);t&&(o=o.filter((function(t){return Object.getOwnPropertyDescriptor(e,t).enumerable}))),n.push.apply(n,o)}return n}function i(e){for(var t=1;t<arguments.length;t++){var n=null!=arguments[t]?arguments[t]:{};t%2?a(Object(n),!0).forEach((function(t){r(e,t,n[t])})):Object.getOwnPropertyDescriptors?Object.defineProperties(e,Object.getOwnPropertyDescriptors(n)):a(Object(n)).forEach((function(t){Object.defineProperty(e,t,Object.getOwnPropertyDescriptor(n,t))}))}return e}function c(e,t){if(null==e)return{};var n,o,r=function(e,t){if(null==e)return{};var n,o,r={},a=Object.keys(e);for(o=0;o<a.length;o++)n=a[o],t.indexOf(n)>=0||(r[n]=e[n]);return r}(e,t);if(Object.getOwnPropertySymbols){var a=Object.getOwnPropertySymbols(e);for(o=0;o<a.length;o++)n=a[o],t.indexOf(n)>=0||Object.prototype.propertyIsEnumerable.call(e,n)&&(r[n]=e[n])}return r}var l=o.createContext({}),s=function(e){var t=o.useContext(l),n=t;return e&&(n="function"==typeof e?e(t):i(i({},t),e)),n},p=function(e){var t=s(e.components);return o.createElement(l.Provider,{value:t},e.children)},u="mdxType",d={inlineCode:"code",wrapper:function(e){var t=e.children;return o.createElement(o.Fragment,{},t)}},m=o.forwardRef((function(e,t){var n=e.components,r=e.mdxType,a=e.originalType,l=e.parentName,p=c(e,["components","mdxType","originalType","parentName"]),u=s(n),m=r,f=u["".concat(l,".").concat(m)]||u[m]||d[m]||a;return n?o.createElement(f,i(i({ref:t},p),{},{components:n})):o.createElement(f,i({ref:t},p))}));function f(e,t){var n=arguments,r=t&&t.mdxType;if("string"==typeof e||r){var a=n.length,i=new Array(a);i[0]=m;var c={};for(var l in t)hasOwnProperty.call(t,l)&&(c[l]=t[l]);c.originalType=e,c[u]="string"==typeof e?e:r,i[1]=c;for(var s=2;s<a;s++)i[s]=n[s];return o.createElement.apply(null,i)}return o.createElement.apply(null,n)}m.displayName="MDXCreateElement"},4102:(e,t,n)=>{n.r(t),n.d(t,{assets:()=>l,contentTitle:()=>i,default:()=>d,frontMatter:()=>a,metadata:()=>c,toc:()=>s});var o=n(7462),r=(n(7294),n(3905));const a={sidebar_position:4},i="Cloning the repository of a package",c={unversionedId:"commands/clone",id:"commands/clone",title:"Cloning the repository of a package",description:"- Have you gotten annoyed that cloning a Julia package using git ends up in a folder with xxxx.jl by default?",source:"@site/docs/commands/clone.md",sourceDirName:"commands",slug:"/commands/clone",permalink:"/Ion/docs/commands/clone",draft:!1,editUrl:"https://github.com/Roger-luo/Ion/tree/main/website/docs/docs/commands/clone.md",tags:[],version:"current",sidebarPosition:4,frontMatter:{sidebar_position:4},sidebar:"tutorialSidebar",previous:{title:"Releasing a new version with Ion",permalink:"/Ion/docs/commands/release"},next:{title:"Authentication",permalink:"/Ion/docs/commands/auth"}},l={},s=[],p={toc:s},u="wrapper";function d(e){let{components:t,...n}=e;return(0,r.kt)(u,(0,o.Z)({},p,n,{components:t,mdxType:"MDXLayout"}),(0,r.kt)("h1",{id:"cloning-the-repository-of-a-package"},"Cloning the repository of a package"),(0,r.kt)("ul",null,(0,r.kt)("li",{parentName:"ul"},"Have you gotten annoyed that cloning a Julia package using git ends up in a folder with ",(0,r.kt)("inlineCode",{parentName:"li"},"xxxx.jl")," by default?"),(0,r.kt)("li",{parentName:"ul"},"Have you been opening a browser, searching the package, copying the package git URL, then cloning the package somewhere?"),(0,r.kt)("li",{parentName:"ul"},"Have you tried to let dev command use your own directory instead of ",(0,r.kt)("inlineCode",{parentName:"li"},".julia/dev")," ?"),(0,r.kt)("li",{parentName:"ul"},"Have you cloned a Julia package, ready to contribute to it, but realize you need to fork it and change remote origin to remote upstream and add your own fork?")),(0,r.kt)("p",null,"Now ion clone handles all above with just one line! if you try ",(0,r.kt)("inlineCode",{parentName:"p"},"ion clone Example")," it will look for the registered URL and try to clone it, and because you don't seem to have access to this repo, we will ask if you want to fork it and if you say yes, we will do it for you. ",(0,r.kt)("strong",{parentName:"p"},"No opening browser is needed!")))}d.isMDXComponent=!0}}]);