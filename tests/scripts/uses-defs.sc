//> using file defs.sc
// A script using another's val and def through that script's name; its own arguments.
println("before")
println(defs.inc(defs.x))
println(defs.inc(1))
println(args.mkString(","))
