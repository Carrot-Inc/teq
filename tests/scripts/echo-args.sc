#!/usr/bin/env -S scala-cli shebang
// A script's own arguments, its top-level definitions and statements, and its path.
val greeting = "args"
def joined(xs: Seq[String]): String = xs.mkString("[", "|", "]")
println(greeting + " " + args.length + " " + joined(args.toSeq))
println(scriptPath.endsWith("echo-args.sc"))
