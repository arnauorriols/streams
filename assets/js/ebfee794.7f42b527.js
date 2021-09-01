"use strict";(self.webpackChunkdocumentation=self.webpackChunkdocumentation||[]).push([[305],{3905:function(e,t,n){n.d(t,{Zo:function(){return u},kt:function(){return m}});var r=n(7294);function o(e,t,n){return t in e?Object.defineProperty(e,t,{value:n,enumerable:!0,configurable:!0,writable:!0}):e[t]=n,e}function i(e,t){var n=Object.keys(e);if(Object.getOwnPropertySymbols){var r=Object.getOwnPropertySymbols(e);t&&(r=r.filter((function(t){return Object.getOwnPropertyDescriptor(e,t).enumerable}))),n.push.apply(n,r)}return n}function a(e){for(var t=1;t<arguments.length;t++){var n=null!=arguments[t]?arguments[t]:{};t%2?i(Object(n),!0).forEach((function(t){o(e,t,n[t])})):Object.getOwnPropertyDescriptors?Object.defineProperties(e,Object.getOwnPropertyDescriptors(n)):i(Object(n)).forEach((function(t){Object.defineProperty(e,t,Object.getOwnPropertyDescriptor(n,t))}))}return e}function l(e,t){if(null==e)return{};var n,r,o=function(e,t){if(null==e)return{};var n,r,o={},i=Object.keys(e);for(r=0;r<i.length;r++)n=i[r],t.indexOf(n)>=0||(o[n]=e[n]);return o}(e,t);if(Object.getOwnPropertySymbols){var i=Object.getOwnPropertySymbols(e);for(r=0;r<i.length;r++)n=i[r],t.indexOf(n)>=0||Object.prototype.propertyIsEnumerable.call(e,n)&&(o[n]=e[n])}return o}var c=r.createContext({}),s=function(e){var t=r.useContext(c),n=t;return e&&(n="function"==typeof e?e(t):a(a({},t),e)),n},u=function(e){var t=s(e.components);return r.createElement(c.Provider,{value:t},e.children)},p={inlineCode:"code",wrapper:function(e){var t=e.children;return r.createElement(r.Fragment,{},t)}},d=r.forwardRef((function(e,t){var n=e.components,o=e.mdxType,i=e.originalType,c=e.parentName,u=l(e,["components","mdxType","originalType","parentName"]),d=s(n),m=o,f=d["".concat(c,".").concat(m)]||d[m]||p[m]||i;return n?r.createElement(f,a(a({ref:t},u),{},{components:n})):r.createElement(f,a({ref:t},u))}));function m(e,t){var n=arguments,o=t&&t.mdxType;if("string"==typeof e||o){var i=n.length,a=new Array(i);a[0]=d;var l={};for(var c in t)hasOwnProperty.call(t,c)&&(l[c]=t[c]);l.originalType=e,l.mdxType="string"==typeof e?e:o,a[1]=l;for(var s=2;s<i;s++)a[s]=n[s];return r.createElement.apply(null,a)}return r.createElement.apply(null,n)}d.displayName="MDXCreateElement"},6307:function(e,t,n){n.r(t),n.d(t,{frontMatter:function(){return l},contentTitle:function(){return c},metadata:function(){return s},toc:function(){return u},default:function(){return d}});var r=n(7462),o=n(3366),i=(n(7294),n(3905)),a=["components"],l={},c="Welcome",s={unversionedId:"welcome",id:"welcome",isDocsHomePage:!1,title:"Welcome",description:"This is the documentation for the official IOTA Streams software. You can read more about core principles behind IOTA Streams in the following blog post.",source:"@site/docs/welcome.md",sourceDirName:".",slug:"/welcome",permalink:"/docs/welcome",editUrl:"https://github.com/iotaledger/streams/tree/dev/documentation/docs/welcome.md",version:"current",frontMatter:{},sidebar:"docs",next:{title:"Overview",permalink:"/docs/overview/overview"}},u=[{value:"Warning",id:"warning",children:[]},{value:"Joining the discussion",id:"joining-the-discussion",children:[]},{value:"What you will find here",id:"what-you-will-find-here",children:[]}],p={toc:u};function d(e){var t=e.components,n=(0,o.Z)(e,a);return(0,i.kt)("wrapper",(0,r.Z)({},p,n,{components:t,mdxType:"MDXLayout"}),(0,i.kt)("h1",{id:"welcome"},"Welcome"),(0,i.kt)("p",null,"This is the documentation for the official IOTA Streams software. You can read more about core principles behind IOTA Streams in the following blog ",(0,i.kt)("a",{parentName:"p",href:"https://blog.iota.org/iota-streams-alpha-7e91ee326ac0/"},"post"),"."),(0,i.kt)("p",null,(0,i.kt)("inlineCode",{parentName:"p"},"streams")," is an organizational tool for structuring and navigating secure data through the Tangle. Streams organizes data by ordering it in a uniform and interoperable structure Needless to say, it is also based on our official ",(0,i.kt)("inlineCode",{parentName:"p"},"one-source-code-of-truth")," ",(0,i.kt)("a",{parentName:"p",href:"https://github.com/iotaledger/iota.rs"},"IOTA Rust library"),"."),(0,i.kt)("h2",{id:"warning"},"Warning"),(0,i.kt)("p",null,"This library is in active development. The library targets the Chrysalis network and does not work with the IOTA legacy network."),(0,i.kt)("p",null,"More information about Chrysalis components is available at ",(0,i.kt)("a",{parentName:"p",href:"https://chrysalis.docs.iota.org/"},"documentation portal"),"."),(0,i.kt)("h2",{id:"joining-the-discussion"},"Joining the discussion"),(0,i.kt)("p",null,"If you want to get involved in discussions about this library, or you're looking for support, go to the #streams-discussion channel on ",(0,i.kt)("a",{parentName:"p",href:"https://discord.iota.org"},"Discord"),"."),(0,i.kt)("h2",{id:"what-you-will-find-here"},"What you will find here"),(0,i.kt)("p",null,"This documentation has five paths:"),(0,i.kt)("ol",null,(0,i.kt)("li",{parentName:"ol"},"The Overview, an detailed overview of the streams library."),(0,i.kt)("li",{parentName:"ol"},"Libraries, all avaiable programming languages and their resources."),(0,i.kt)("li",{parentName:"ol"},"The Specification, detailed explaination requirements and functionality."),(0,i.kt)("li",{parentName:"ol"},"Contribute, how you can work on the streams software."),(0,i.kt)("li",{parentName:"ol"},"Get in touch, join the community and become part of the X-Team!")))}d.isMDXComponent=!0}}]);