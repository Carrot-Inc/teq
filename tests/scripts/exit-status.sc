// A script's statements after a failure do not run; `sys.exit` ends it with its status.
println("before exit")
if args.headOption.contains("fail") then throw new IllegalStateException("failed on purpose")
sys.exit(args.headOption.map(_.toInt).getOrElse(0))
