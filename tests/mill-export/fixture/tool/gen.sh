#!/bin/sh
# Writes tool.Labels from the labels file ($1) into the directory given last ($2).
mkdir -p "$2/tool"
printf 'package tool\n\nobject Labels:\n  val count: Int = %s\n' "$(wc -w < "$1" | tr -d ' ')" > "$2/tool/Labels.scala"
