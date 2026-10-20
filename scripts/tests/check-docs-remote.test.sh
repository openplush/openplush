#!/usr/bin/env bash
# Tests for scripts/check-docs-remote.sh: each case builds a throwaway site of
# HTML and CSS and asserts whether the check accepts or rejects it.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
check="$here/../check-docs-remote.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

pass=0
fail=0

# expect <pass|fail> <case name> <dir> [<text the failure output must contain>...]
expect() {
  local want=$1 name=$2 dir=$3
  shift 3
  local out status=0 pattern matched=1
  out=$("$check" "$dir" 2>&1) || status=$?
  for pattern in "$@"; do [[ $out == *"$pattern"* ]] || matched=0; done
  if [[ $want == pass && $status -eq 0 ]] || [[ $want == fail && $status -ne 0 && $matched -eq 1 ]]; then
    pass=$((pass + 1))
    echo "ok   $name"
  else
    fail=$((fail + 1))
    echo "FAIL $name (wanted $want, exit $status)"
    while IFS= read -r line; do echo "     | $line"; done <<<"$out"
  fi
}

# file <dir> <path> <content>: write a file of the site.
file() { mkdir -p "$(dirname "$1/$2")" && printf '%s\n' "$3" >"$1/$2"; }

# new_case <name>: a site whose every resource is local.
new_case() {
  local dir="$tmp/$1"
  file "$dir" index.html '<!DOCTYPE html>
<html><head>
<link rel="stylesheet" href="css/general.css">
<link rel="icon" href='"'"'favicon.svg'"'"'>
<script src="book.js"></script>
<script src="/searcher.js"></script>
</head><body>
<img src="../images/seam.png" alt="a seam">
<img srcset="../images/seam.png 1x, ../images/seam@2x.png 2x" alt="">
<video poster="poster.png"></video>
<object data="diagram.svg"></object>
<svg><use href="#icon"/><image xlink:href="icon.png"/></svg>
<div data-src="https://example.com/lazy.png"></div>
<img src="data:image/svg+xml;utf8,<svg xmlns='"'"'http://www.w3.org/2000/svg'"'"'/>">
<a href="https://github.com/openplush/openplush">source</a>
<a href='"'"'http://example.com/'"'"'>elsewhere</a>
<a href="//example.org/page">protocol-relative link</a>
<p>See https://example.com for more.</p>
</body></html>'
  file "$dir" css/general.css '@font-face { src: url("../fonts/open-sans.woff2") format("woff2"); }
@import "variables.css";
@import url(chrome.css);
.mark { background: url('"'"'data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg"/>'"'"'); }'
  echo "$dir"
}

d=$(new_case local)
expect pass "a site with only local resources and links elsewhere" "$d"

d=$(new_case script-https)
file "$d" remote.html '<script src="https://cdn.example.com/x.js"></script>'
expect fail "an https script src" "$d" "remote.html" "https://cdn.example.com/x.js"

d=$(new_case script-http-single)
file "$d" remote.html "<script type='module' src='http://cdn.example.com/x.js'></script>"
expect fail "an http script src in single quotes" "$d" "remote.html" "http://cdn.example.com/x.js"

d=$(new_case link-https)
file "$d" remote.html '<link rel="stylesheet" href="https://fonts.example.com/css">'
expect fail "an https link href" "$d" "remote.html" "https://fonts.example.com/css"

d=$(new_case link-single)
file "$d" remote.html "<link href='https://fonts.example.com/css' rel='stylesheet'>"
expect fail "an https link href in single quotes" "$d" "remote.html" "https://fonts.example.com/css"

d=$(new_case img-https)
file "$d" remote.html '<p><img src="https://img.example.com/a.png" alt="a"></p>'
expect fail "an https img src" "$d" "remote.html" "https://img.example.com/a.png"

d=$(new_case img-protocol-relative)
file "$d" remote.html '<img alt="a" src="//img.example.com/a.png">'
expect fail "a protocol-relative img src" "$d" "remote.html" "//img.example.com/a.png"

d=$(new_case script-protocol-relative)
file "$d" remote.html "<script src='//cdn.example.com/x.js'></script>"
expect fail "a protocol-relative script src" "$d" "remote.html" "//cdn.example.com/x.js"

d=$(new_case unquoted)
file "$d" remote.html '<img src=https://img.example.com/a.png alt=a>'
expect fail "an unquoted https img src" "$d" "remote.html" "https://img.example.com/a.png"

d=$(new_case upper-case)
file "$d" remote.html '<SCRIPT SRC="HTTPS://CDN.EXAMPLE.COM/X.JS"></SCRIPT>'
expect fail "an upper-case script src" "$d" "remote.html" "HTTPS://CDN.EXAMPLE.COM/X.JS"

d=$(new_case multi-line)
file "$d" remote.html '<script
  type="text/javascript"
  src="https://cdn.example.com/x.js"></script>'
expect fail "a script tag across lines" "$d" "remote.html" "https://cdn.example.com/x.js"

d=$(new_case nested)
file "$d" adr/0017.html '<img src="https://img.example.com/a.png">'
expect fail "a remote reference in a subdirectory names its path" "$d" "adr/0017.html" "https://img.example.com/a.png"

d=$(new_case css-url-double)
file "$d" css/remote.css '@font-face { src: url("https://fonts.example.com/a.woff2"); }'
expect fail "a CSS url() in double quotes" "$d" "css/remote.css" "https://fonts.example.com/a.woff2"

d=$(new_case css-url-single)
file "$d" css/remote.css ".a { background: url( 'http://img.example.com/a.png' ); }"
expect fail "a CSS url() in single quotes" "$d" "css/remote.css" "http://img.example.com/a.png"

d=$(new_case css-url-bare)
file "$d" css/remote.css '.a { background: url(https://img.example.com/a.png); }'
expect fail "an unquoted CSS url()" "$d" "css/remote.css" "https://img.example.com/a.png"

d=$(new_case css-url-protocol-relative)
file "$d" css/remote.css '.a { background: url(//img.example.com/a.png); }'
expect fail "a protocol-relative CSS url()" "$d" "css/remote.css" "//img.example.com/a.png"

d=$(new_case css-import-double)
file "$d" css/remote.css '@import "https://fonts.example.com/css";'
expect fail "a CSS @import in double quotes" "$d" "css/remote.css" "https://fonts.example.com/css"

d=$(new_case css-import-single)
file "$d" css/remote.css "@import 'http://fonts.example.com/css';"
expect fail "a CSS @import in single quotes" "$d" "css/remote.css" "http://fonts.example.com/css"

d=$(new_case css-import-url)
file "$d" css/remote.css '@import url("https://fonts.example.com/css") screen;'
expect fail "a CSS @import url()" "$d" "css/remote.css" "https://fonts.example.com/css"

d=$(new_case inline-style)
file "$d" remote.html '<div style="background: url(https://img.example.com/a.png)">x</div>'
expect fail "a url() in an inline style" "$d" "remote.html" "https://img.example.com/a.png"

d=$(new_case style-element)
file "$d" remote.html '<style>
@import "https://fonts.example.com/css";
</style>'
expect fail "an @import in a style element" "$d" "remote.html" "https://fonts.example.com/css"

d=$(new_case srcset)
file "$d" remote.html '<img src="a.png" srcset="a.png 1x, https://img.example.com/a@2x.png 2x">'
expect fail "a remote candidate in srcset" "$d" "remote.html" "srcset" "https://img.example.com/a@2x.png"

d=$(new_case poster)
file "$d" remote.html '<video poster="https://img.example.com/p.png"></video>'
expect fail "a remote video poster" "$d" "remote.html" "poster" "https://img.example.com/p.png"

d=$(new_case object-data)
file "$d" remote.html "<object data='//e.example.com/x.svg'></object>"
expect fail "a remote object data" "$d" "remote.html" "<object> data" "//e.example.com/x.svg"

d=$(new_case svg-image-href)
file "$d" remote.html '<svg><image href="https://img.example.com/a.png"/></svg>'
expect fail "a remote SVG image href" "$d" "remote.html" "<image> href" "https://img.example.com/a.png"

d=$(new_case svg-use-xlink)
file "$d" remote.html '<svg><use xlink:href="https://e.example.com/s.svg#i"/></svg>'
expect fail "a remote SVG use xlink:href" "$d" "remote.html" "<use> xlink:href" "https://e.example.com/s.svg#i"

d=$(new_case every-reference)
file "$d" a.html '<img src="https://img.example.com/a.png">'
file "$d" b.css '.b { background: url(https://img.example.com/b.png); }'
expect fail "every remote reference is reported" "$d" "a.html" "b.css" "https://img.example.com/a.png" "https://img.example.com/b.png"

expect fail "a missing directory" "$tmp/missing" "$tmp/missing"

mkdir -p "$tmp/empty"
expect fail "a directory with no HTML" "$tmp/empty" "no HTML"

echo
echo "$pass passed, $fail failed"
[[ $fail -eq 0 ]]
