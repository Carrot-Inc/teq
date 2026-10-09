# bench/changelog.sh, sourced: CHANGELOG.md's sections (docs/DEVELOPING.md, "Releases"), for bench/release.sh and
# bench/github-release.sh. CHANGELOG.md, at the root, holds a section per release, the newest first, its heading
# `## <version> (<yyyy-mm-dd>)` over bullets that say what the release changes for a user; the section is the
# notes of the release's GitHub release. Public words: none of the work's process (changelog_words, "pass" among
# them even where it would name a compiler's: write "phase"), no internal file (changelog_files), and none of the
# names a machine lists, one a line, in ~/.config/teq/private-names (TEQ_PRIVATE_NAMES; matched as words, case
# and runs of blanks aside), which the tree never holds; all read in the text the notes show, a bullet's lines
# joined (changelog_join).
changelog_private_names=${TEQ_PRIVATE_NAMES:-$HOME/.config/teq/private-names}
changelog_words='codex|claude|coordinator|agents?|subagents?|vault|gates?|pass|passes'
changelog_files='docs/internal docs/history TASKS.md ROADMAP.md REVIEWING.md'

# changelog_join: the text on stdin with each bullet's continued lines joined to it by a space, as the GitHub
# release's notes show it (GitHub breaks a line where the file does).
changelog_join() {
  awk '/^ +[^ ]/ && have { sub(/^ +/, ""); sub(/ +$/, "", line); line = line " " $0; next } { if (have) print line; line = $0; have = 1 } END { if (have) print line }'
}

# changelog_notes <changelog> <version>: the text of the version's section, without the blank lines at either
# end; fails naming what is wrong: no section or two, one below another (the newest is the first), a heading
# without its date, no bullet with words, a word of the process, an internal file or a private name.
changelog_notes() {
  local file=$1 version=$2 headings heading text joined found f
  [ -f "$file" ] || { echo "no $file" >&2; return 1; }
  headings=$(grep '^## ' "$file")
  heading=$(awk -v v="$version" '$1 == "##" && $2 == v' <<< "$headings")
  [ -n "$heading" ] || { echo "$file has no section for $version: '## $version (<yyyy-mm-dd>)' and its bullets, the newest first" >&2; return 1; }
  [ "$(wc -l <<< "$heading")" -eq 1 ] || { echo "$file has $(wc -l <<< "$heading") sections for $version" >&2; return 1; }
  [[ $heading =~ ^##\ [^\ ]+\ \([0-9]{4}-[0-9]{2}-[0-9]{2}\)$ ]] || { echo "$file's heading '$heading' is not '## $version (<yyyy-mm-dd>)'" >&2; return 1; }
  [ "$(head -1 <<< "$headings")" = "$heading" ] || { echo "$file's section for $version is below '$(head -1 <<< "$headings")': the newest comes first" >&2; return 1; }
  text=$(awk -v h="$heading" '$0 == h { f = 1; next } f && /^## / { exit } f' "$file" | sed '/[^[:space:]]/,$!d')
  grep -qE '^- +[^[:space:]]' <<< "$text" || { echo "$file's section for $version has no bullet" >&2; return 1; }
  joined=$(changelog_join <<< "$text" | tr -s '[:blank:]' ' ')
  found=$(grep -oiwE "$changelog_words" <<< "$joined" | sort -fu | tr '\n' ' ')
  for f in $changelog_files; do grep -qiF "$f" <<< "$joined" && found="$found$f "; done
  [ -z "$found" ] || { echo "$file's section for $version names the work's process or its internal files: $found" >&2; return 1; }
  if [ -f "$changelog_private_names" ]; then
    found=$(grep -oiwF -f <(sed '/^[[:space:]]*#/d; /^[[:space:]]*$/d' "$changelog_private_names" | tr -s '[:blank:]' ' ') <<< "$joined" | sort -fu | tr '\n' ' ')
    [ -z "$found" ] || { echo "$file's section for $version names what $changelog_private_names keeps private: $found" >&2; return 1; }
  fi
  printf '%s\n' "$text"
}
