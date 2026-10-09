// jars: scala-library
// An `App` object as the program's entry point: the body runs when the object is made, then the
// launcher's `main` reaches `App.main`, which stores the arguments through the setter the object
// implements (master threw `AbstractMethodError` at `scala$App$$_args_$eq`) and runs what
// `delayedInit` queued, where `args` is readable (in the body it is not stored yet, as under scalac).
object AppMain extends App:
  println("AppMain body")
  println("time kept: " + (executionStart > 0))
  delayedInit(println("args after main: [" + args.mkString(",") + "]"))
