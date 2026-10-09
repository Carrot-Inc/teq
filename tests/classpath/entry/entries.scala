// Entry points as scalac's discovery finds them and `java` launches them by name: a package's own
// `main(args: Array[String])` through its file's `entries$package`, an object's `main`, and the
// class scalac's `MainProxies` makes for a `@main` method of an object, named after the method in
// the object's package, a nested object's too, its name encoded as the class's (`hello$minusworld`).
package entries

def main(args: Array[String]): Unit = println("package main: " + args.mkString(","))

object Holder:
  @main def hello(args: String*): Unit = println("hello: " + args.mkString(","))
  @main def `hello-world`(): Unit = println("hello-world")
  object Inner:
    @main def deep(): Unit = println("deep")

object Run:
  def main(args: Array[String]): Unit = println("run: " + args.mkString(","))
