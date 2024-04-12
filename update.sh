#!/bin/bash

set -eu
shopt -s extglob

depends=( curl jq )
notfound=()

for app in "${depends[@]}"; do
	if ! type "$app" > /dev/null 2>&1; then
		notfound+=("$app")
	fi
done

if [[ ${#notfound[@]} -ne 0 ]]; then
	echo Failed to lookup dependency:

	for app in "${notfound[@]}"; do
		echo "- $app"
	done

	exit 1
fi

DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$DIR/maps"

function new_filename() {
	NUM=0
	DATE="$(date '+%Y%m%d')"

	while : ; do
		f=${DATE}_${NUM}.$1

		if [ ! -e "$f" ]; then
			echo "$f"
			break
		fi

		NUM=$(( NUM + 1 ))
	done
}

f=$(new_filename json)

curl -s https://www.jma.go.jp/bosai/common/const/area.json \
	| jq -c -r '.["class20s"] | keys | sort' \
	> "$f"

echo "Newer file created at: $f"

if [[ $(find . -name "*.json" | wc -l) -eq 1 ]]; then
	echo "It's first time. goodluck!"
	exit 0
fi

diff_lines=$(find . -name "*.json" | sort | tail -n 2 | xargs diff | wc -l)

if [[ "$diff_lines" -eq 0 ]]; then
	echo "Oops, same content is already exists. revert it."
	rm "$f"

	latest_f=$(find . -name "*.json" | sort | tail -n 1)
	echo "latest data at: $latest_f"

	exit 1
fi
