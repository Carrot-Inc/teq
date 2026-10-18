//> using file echo-args.sc
// An included script that reads its own arguments: they are the selected script's alone, so the
// included one's wrapper throws Scala CLI's "No arguments passed to this script".
println("first")
println(`echo-args`.greeting)
