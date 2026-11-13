// command: build
// expect: several entry points: entry_points_source_order_def_first$package.main, Held.main; choose one with --main
// A package's `main` written above an object's: the package's is named first and the error stands at it.
def main(args: Array[String]): Unit = println("package")

object Held { def main(args: Array[String]): Unit = println("object") }
