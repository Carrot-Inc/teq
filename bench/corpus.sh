#!/bin/bash
# Unpacks the sources jars of the census libraries from the coursier cache into <dir>, one
# directory per library, for bench/census.py. A jar that the cache lacks is fetched by coursier
# when `cs` is on the path, and skipped with a message otherwise.
out=${1:?usage: corpus.sh <dir>}
cache=${COURSIER_CACHE:-$HOME/Library/Caches/Coursier/v1}/https/repo1.maven.org/maven2
libs="org.scala-lang:scala-library:3.8.4 org.typelevel:cats-kernel_3:2.13.0 org.typelevel:cats-core_3:2.13.0
org.typelevel:cats-effect_3:3.7.0 dev.zio:zio_3:2.1.26 dev.zio:zio-json_3:0.9.2
com.softwaremill.sttp.tapir:tapir-core_3:1.13.29 org.http4s:http4s-core_3:0.23.36 org.tpolecat:doobie-core_3:1.0.0-RC12"
mkdir -p "$out"
for lib in $libs; do
  IFS=: read -r org name version <<< "$lib"
  jar="$cache/${org//.//}/$name/$version/$name-$version-sources.jar"
  if [ ! -f "$jar" ] && command -v cs > /dev/null; then
    timeout 120 cs fetch --sources "$lib" > /dev/null 2>&1
  fi
  if [ ! -f "$jar" ]; then
    echo "skipping $lib: no sources jar at $jar"
    continue
  fi
  dir="$out/$name-$version"
  rm -rf "$dir"
  mkdir -p "$dir"
  unzip -q -o "$jar" '*.scala' -d "$dir"
  echo "$dir: $(find "$dir" -name '*.scala' | wc -l | tr -d ' ') files"
done
