#!/bin/bash
# bench/app/api-check.sh [--test] <teq> [<master teq>]: `teq compiler check` of the application's API side (its API
# module and the modules it depends on, its generated sources included) on the JVM target, as its sbt build compiles
# it: the sources of the module list APP_MODULES names (one module per line, its source directories or a source file
# outside them, relative to the checkout or absolute, each of which has to exist), the class path of the list
# APP_CLASSPATH names (one jar per line, `~` for the home directory; the jars sbt resolved into the reference machine's coursier
# cache and sbt's scala-library) and the build's flags APP_FLAGS (words, possibly none), all kept outside the repository with the application (bench/app/app-lists.sh writes them), in the
# application's checkout APP_ROOT, which is read and never written. With --test, the check of the API's test
# configuration in its place: the sources of APP_TEST_MODULES with the flags APP_TEST_FLAGS over the class path
# APP_TEST_CLASSPATH, whose lines `@<project>/<configuration>` are the main modules' products: the binary's own build
# of the main lists (`--products`, APP_FLAGS less --werror, whose warnings are the main check's) takes the place of
# the first and the others go, as every one of them is a main module (app-lists.sh checks it), the jars keeping
# their places. Prints what the check prints. A check completes when it exits 0, or 1 with its count of errors
# (or of warnings, under --werror) as the last line; a timeout, a crash, any other exit or a failed build of the
# main products fails. With a master binary the check runs twice, each binary over its own products, and passes
# when both complete with the same exit and the same diagnostics, which for the main lists today are one error:
# the application's several entry points, which a check without --main cannot choose from. Without one it passes
# when the check completes with exit 0.
test=0
[ "$1" = --test ] && { test=1; shift; }
teq=$1
master=$2
[ -x "$teq" ] || { echo "usage: bench/app/api-check.sh [--test] <teq> [<master teq>]" >&2; exit 2; }
# The binaries by absolute paths: the checks run in the checkout.
absolute() { case $1 in /*) echo "$1" ;; *) echo "$PWD/$1" ;; esac; }
teq=$(absolute "$teq")
[ -z "$master" ] || master=$(absolute "$master")
here=$(cd "$(dirname "$0")" && pwd)
. "$here/../../tests/support/compiler-words.sh"
spellings "$teq" ${master:+"$master"}
[ -n "$APP_ROOT" ] && [ -f "$APP_MODULES" ] && [ -f "$APP_CLASSPATH" ] && [ -n "${APP_FLAGS+set}" ] || { echo "api-check: set APP_ROOT, APP_MODULES, APP_CLASSPATH and APP_FLAGS"; exit 1; }
if [ $test = 1 ]; then
  [ -f "$APP_TEST_MODULES" ] && [ -f "$APP_TEST_CLASSPATH" ] && [ -n "${APP_TEST_FLAGS+set}" ] || { echo "api-check: set APP_TEST_MODULES, APP_TEST_CLASSPATH and APP_TEST_FLAGS"; exit 1; }
fi
app=$APP_ROOT
[ -d "$app" ] || { echo "api-check: no application checkout at $app"; exit 1; }
# inputs <list>: every input of the module list, a source directory or a source file (app-lists.sh lists a source
# outside its module's directories itself), relative to the checkout or absolute, exists.
inputs() {
  local w p n=0
  for w in $(grep -v '^#' "$1"); do
    case $w in /*) p=$w ;; *) p=$app/$w ;; esac
    [ -d "$p" ] || [ -f "$p" ] || { echo "api-check: no $w in the application checkout at $app ($1)"; exit 1; }
    n=$((n + 1))
  done
  [ $n -gt 0 ] || { echo "api-check: $1 names no source"; exit 1; }
}
inputs "$APP_MODULES"
cp=$(sed "s|^~/|$HOME/|" "$APP_CLASSPATH" | paste -sd: -)
sources=$(grep -v '^#' "$APP_MODULES" | tr '\n' ' ')
flags=$APP_FLAGS
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
products=$tmp/products
if [ $test = 1 ]; then
  sources=$(grep -v '^#' "$APP_TEST_MODULES" | tr '\n' ' ')
  inputs "$APP_TEST_MODULES"
  grep -q '^@' "$APP_TEST_CLASSPATH" || { echo "api-check: $APP_TEST_CLASSPATH names no main module's products"; exit 1; }
  main_cp=$cp
  main_sources=$(grep -v '^#' "$APP_MODULES" | tr '\n' ' ')
  cp=$(awk -v p="$products" -v home="$HOME" '/^@/ { if (!placed) print p; placed = 1; next } { sub(/^~\//, home "/"); print }' "$APP_TEST_CLASSPATH" | paste -sd: -)
  flags=$APP_TEST_FLAGS
fi
# main_products <teq>: the binary's build of the main lists into $products, which the test check reads.
main_products() {
  rm -rf "$products"
  (cd "$app" && timeout 300 "$1" $(compiler_words "$1") build --products "$products" $main_sources --classpath "$main_cp" \
    $(echo " $APP_FLAGS " | sed 's/ --werror / /g') --std scala-library --target jvm) > "$tmp/products.log" 2>&1 || {
    echo "api-check: $1's build of the main lists' products failed ($(tail -1 "$tmp/products.log"))"
    return 1
  }
}
# check <teq> <output file>: the check's exit, 99 when it did not complete.
check() {
  if [ $test = 1 ]; then main_products "$1" > "$2" || return 99; fi
  (cd "$app" && timeout 170 "$1" $(compiler_words "$1") check $sources --classpath "$cp" $flags \
    --std scala-library --target jvm) > "$2" 2>&1
  local code=$?
  case $code in
    0) return 0 ;;
    1) tail -1 "$2" | grep -q -E '^([0-9]+ errors? found|[0-9]+ warnings? found, errors under --werror)$' && return 1 ;;
  esac
  echo "exit $code" >> "$2"
  return 99
}
what=check
[ $test = 1 ] && what="test check"
check "$teq" "$tmp/landing"
code=$?
cat "$tmp/landing"
[ $code != 99 ] || { echo "api-check: the $what did not complete ($(tail -1 "$tmp/landing"))"; exit 1; }
if [ -z "$master" ]; then
  echo "api-check: exit $code"
  exit $code
fi
check "$master" "$tmp/master"
mcode=$?
[ $mcode != 99 ] || { echo "api-check: master's $what did not complete ($(tail -1 "$tmp/master"))"; exit 1; }
if [ $code != $mcode ]; then
  echo "api-check: the $what exits $code, master's $mcode"
  exit 1
elif cmp -s "$tmp/landing" "$tmp/master"; then
  echo "api-check: the same diagnostics as master's (errors $(grep -c ': error: ' "$tmp/master"), warnings $(grep -c ': warning: ' "$tmp/master"))"
else
  echo "api-check: the diagnostics differ from master's:"
  diff "$tmp/master" "$tmp/landing" | head -40
  echo "api-check: $(diff "$tmp/master" "$tmp/landing" | grep -c '^[<>]') lines differ from master's"
  exit 1
fi
