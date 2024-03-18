#!/usr/bin/env bash

set -e

cd "$(dirname "${0}")"

curl -fsSLo tailwind.js 'https://cdn.tailwindcss.com'
curl -fsSLo daisyui.full.min.css 'https://cdn.jsdelivr.net/npm/daisyui@4.7.3/dist/full.min.css'
