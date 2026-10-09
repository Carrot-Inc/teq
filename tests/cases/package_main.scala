// A package's own `main(args: Array[String])` is the program's entry point, as scalac's discovery finds it
// (`java` runs its file's `package_main$package`); master rejected the program as having none.
def main(args: Array[String]): Unit = println("package main, " + args.length + " arguments")
