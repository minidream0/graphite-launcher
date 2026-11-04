import React,{useId} from 'react';

export function CatArt({pose='sleep',className='',...props}){
 const id=useId();
 return <svg {...props} viewBox="0 0 240 180" className={`smoky-cat cat-pose-${pose} ${className}`} aria-hidden="true" focusable="false">
  <defs><linearGradient id={`${id}-fur`} x1="40" y1="70" x2="180" y2="150" gradientUnits="userSpaceOnUse"><stop stopColor="#9a979e"/><stop offset=".5" stopColor="#77757f"/><stop offset="1" stopColor="#53515c"/></linearGradient><linearGradient id={`${id}-head`} x1="120" y1="62" x2="181" y2="130" gradientUnits="userSpaceOnUse"><stop stopColor="#b1acb2"/><stop offset="1" stopColor="#77717d"/></linearGradient></defs>
  <ellipse cx="115" cy="153" rx="82" ry="9" fill="#0d0c11" opacity=".22"/>
  <g className="cat-tail"><path d="M83 132C61 123 42 114 39 130C35 149 66 158 86 145" fill="none" stroke="#53515d" strokeWidth="18" strokeLinecap="round"/><path d="M79 130C56 120 45 120 45 130" fill="none" stroke="#88838f" strokeWidth="5" strokeLinecap="round"/></g>
  <g className="cat-body"><path d="M50 128C48 104 61 82 85 77C112 71 143 88 157 112L164 138L135 151L77 150Q55 146 50 128Z" fill={`url(#${id}-fur)`}/><path d="M57 107Q65 86 86 83L111 88L93 112L62 129Z" fill="#b3abb5" opacity=".3"/><path d="M96 112L125 98L150 117L151 142L128 150L91 145Z" fill="#5b5764" opacity=".55"/></g>
  <path d="M84 136Q79 126 69 130L63 142Q63 151 84 151H115Q119 141 107 138Z" fill="#76717e"/>
  <g className="cat-head"><path d="M112 101L111 78L120 46L144 61L169 48L184 76L185 108L173 126L143 133L122 124Z" fill={`url(#${id}-head)`}/><g className="cat-ear cat-ear-left"><path d="M111 79L120 46L144 61L128 78Z" fill="#69636f"/><path d="M119 70L122 55L135 64Z" fill="#b6a5ad"/></g><g className="cat-ear cat-ear-right"><path d="M155 64L169 48L184 76L173 81Z" fill="#645e6b"/><path d="M164 65L169 56L177 71Z" fill="#b6a5ad"/></g><path d="M119 81L143 71L171 77L179 102L168 119L142 125L121 113Z" fill="#9f98a3"/>
   <path d="M132 103Q148 96 166 103L169 115Q151 128 131 116Z" fill="#cfc5bd"/>
   <g className="cat-eyes-open"><path d="M127 92Q134 88 141 92M157 91Q164 87 172 91" fill="none" stroke="#433d4a" strokeWidth="3" strokeLinecap="round"/><ellipse cx="135" cy="95" rx="3" ry="3.5" fill="#c5b691"/><ellipse cx="164" cy="94" rx="3" ry="3.5" fill="#c5b691"/><path d="M135 93V97M164 92V96" stroke="#413c47" strokeWidth="2" strokeLinecap="round"/></g>
   <g className="cat-eyes-closed" fill="none" stroke="#4e4655" strokeWidth="3" strokeLinecap="round"><path d="M128 95Q135 100 142 94M157 94Q165 99 172 93"/></g>
   <path d="M146 106L154 106L150 111Z" fill="#a67f83"/><path d="M150 111V114M150 114L145 116M150 114L155 116" fill="none" stroke="#897982" strokeWidth="1.5" strokeLinecap="round"/><path d="M128 109L115 106M131 115L116 116M170 108L185 103M169 114L185 114" fill="none" stroke="#bcb1ba" strokeWidth="1.5" strokeLinecap="round"/>
  </g>
  <g className="cat-front-paw"><path d="M137 133L159 134L173 142Q179 152 167 154H133Q124 151 129 142Z" fill="#a6a0aa"/><path d="M133 146H167Q174 148 170 152H135Z" fill="#d0c7c4"/><path d="M151 149V153M158 149V153" stroke="#928994" strokeWidth="1.5"/></g>
 </svg>;
}

