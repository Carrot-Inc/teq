// A directory with a script beside a program's main. scala-cli
// compiles both and runs the one main that is no script's
// (Build.retainedMainClass: inferredMainClass, telling
// of the script's); its run's output is the .expected's.
object Main { def main(args: Array[String]): Unit = println("main") }
