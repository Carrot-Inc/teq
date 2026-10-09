# The budget's programs, sourced from the repository root by bench/budget.sh and bench/layout.sh:
# sets $programs and defines `program_args <program>`, the arguments of its `teq compiler check`. The
# programs are generated into $work once, again when their generator is newer; the ones reading
# jars are left out without them, and $left_out names each with the jars it lacks, a line per
# program, for a caller that must have them all (bench/budget.sh). macro-cls is the class-string
# validator of bench/macros/gen.py, the macro shape whose expansions are the largest single cost
# of the application's type phase. Its class catalog, a memo of parsed class lists
# (`bench.cls.Vocabularies` there, `meridian.web.css.Catalog` in realistic-frontend), is declared a
# cache, as the application's build file declares Tailwind's loader: undeclared, every attempt of
# several workers gives way to one.
mkdir -p "$work"
fresh() { [ -d "$1" ] && [ ! "$2" -nt "$1" ]; }
fresh "$work/core-only" bench/gen.py || { rm -rf "$work/core-only" && timeout 60 python3 bench/gen.py "$work/core-only" 51 22 > /dev/null; } || return 1
# The variants are the generator's, whatever a directory generated earlier holds.
variants=$(timeout 60 python3 bench/features.py --names) && [ -n "$variants" ] || return 1
variants=$(echo "$variants" | LC_ALL=C sort)
for v in $variants; do ls "$work/features/$v"/*.scala > /dev/null 2>&1 || rm -rf "$work/features"; done
fresh "$work/features" bench/features.py || { rm -rf "$work/features" && timeout 60 python3 bench/features.py "$work/features" > /dev/null; } || return 1
fresh "$work/macros" bench/macros/gen.py || { rm -rf "$work/macros" && timeout 60 python3 bench/macros/gen.py "$work/macros" > /dev/null; } || return 1
programs="core-only"
for v in $variants; do
  programs="$programs $v"
done
programs="$programs macro-cls"
left_out=""
# with_jars <program> <missing jars>: the program among the programs, or left out.
with_jars() {
  if [ -z "$2" ]; then
    programs="$programs $1"
  else
    left_out="$left_out$1 (without$2)
"
  fi
}
. tests/support/jars.sh
app_cp=""
app_missing=""
for name in scala-library cats-kernel cats-core sourcecode; do
  path=$(jar_of "$name")
  [ -e "$path" ] || app_missing="$app_missing $name"
  app_cp="$app_cp:$path"
done
app_cp=${app_cp#:}
if [ -z "$app_missing" ]; then
  fresh "$work/app" bench/app/gen.py || { rm -rf "$work/app" && timeout 120 python3 bench/app/gen.py "$work/app" > /dev/null; } || return 1
fi
with_jars realistic-frontend "$app_missing"
with_jars realistic-api "$app_missing"
jars_of bench/derive/zio30.scala
derive_cp=$JARS_CP
with_jars derive30 "$JARS_MISSING"
jars_of bench/derive/kittens30.scala
kittens_cp=$JARS_CP
with_jars kittens30 "$JARS_MISSING"
jars_of bench/derive/schema30.scala
schema_cp=$JARS_CP
with_jars schema30 "$JARS_MISSING"

program_args() {
  case $1 in
    core-only) echo "$work/core-only" ;;
    macro-cls) echo "$work/macros/cls --cacheable-state bench.cls.Vocabularies" ;;
    realistic-frontend) echo "$work/app/shared $work/app/frontend --classpath $app_cp --cacheable-state meridian.web.css.Catalog" ;;
    realistic-api) echo "$work/app/shared $work/app/api --classpath $app_cp" ;;
    derive30) echo "bench/derive/zio30.scala --classpath $derive_cp" ;;
    kittens30) echo "bench/derive/kittens30.scala --classpath $kittens_cp" ;;
    schema30) echo "bench/derive/schema30.scala --classpath $schema_cp" ;;
    *) echo "$work/features/$1" ;;
  esac
}
