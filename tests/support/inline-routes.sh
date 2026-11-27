#!/bin/bash
# tests/support/inline-routes.sh <teq> <case dir> <work dir> <scala-library jar>: the routes an expansion of an upstream's
# inline method takes in a tests/modules case's last module, held to the case's inline-routes.txt. Every module is
# built split on the JVM (`compiler build --target jvm --products`, each over the products of the ones before it), the
# last with TEQ_INLINE_COUNTS naming a file; that build's `inline counts:` line (typer/substitution.rs: the calls the
# expansion by substitution took and those that fell back to the retype path) must hold inline-routes.txt's one line,
# `substituted <n> fell back <m>`. Prints what differs; exit 0 when it holds, 1 when it does not or a build fails.
# On master a4c91a911 no inline call of a module over its upstream's products takes the retype path: a body the
# source does not write has its stored record alone (typer/inline.rs 1307-1310), and a class an inline body makes is
# that record's own (typer/inline_definition.rs stores_classes, at quote.level == 0). Every case's line is
# `substituted <n> fell back 0`. A class a quote makes (quote.level > 0) is not the record's: a macro's
# implementation, a loaded body, is typed from its pickle's trees where it runs, its quote's class and that class's
# parents with it. That route is no inline call's, so the counts do not see it (fpinlinemacro's line counts its two
# inline calls). The class is in the last module's class files and JavaScript, not in its pickle, which keeps the
# inline call as scalac's does: the case's classes check holds the split build's class to the whole build's, byte for
# byte, and its run legs run it.
teq=$1 dir=$2 work=$3 sl=$4
want=$(grep -v '^#' "$dir/inline-routes.txt" | head -1)
mkdir -p "$work/routes"
mods=($(cd "$dir" && ls -d */ | tr -d / | sort))
cp=""
for m in "${mods[@]}"; do
  srcs=($(find "$dir/$m" -name '*.scala' | sort))
  counts=""
  [ "$m" = "${mods[${#mods[@]}-1]}" ] && counts=$work/routes/counts.txt && : > "$counts"
  if ! TEQ_INLINE_COUNTS=$counts "$teq" compiler build --target jvm --products "$work/routes/$m" --sourceroot "$dir" --classpath "$sl${cp:+:$cp}" "${srcs[@]}" > "$work/routes/$m.log" 2>&1; then
    echo "routes: the build of $m failed: $(head -2 "$work/routes/$m.log")"
    exit 1
  fi
  cp=${cp:+$cp:}$work/routes/$m
done
got=$(grep -m1 '^inline counts: ' "$work/routes/counts.txt" | sed -E 's/^inline counts: (substituted [0-9]+ fell back [0-9]+).*/\1/')
[ "$got" = "$want" ] && exit 0
echo "routes: the last module's expansions: ${got:-no counts line}, inline-routes.txt: $want"
exit 1
