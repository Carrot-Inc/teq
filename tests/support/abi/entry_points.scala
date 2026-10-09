// The classes `java` launches: a package's own `main` forwarded statically by its file's `<stem>$package`, an
// object's by its mirror class, and the class scalac's `MainProxies` writes for a `@main` method of an object,
// named after the method in the object's package (a nested object's too); no class for a package's `main`.
// abi: hello#main deep#main entry_points$package#main Run#main main#main
package entries

def main(args: Array[String]): Unit = println("package main: " + args.mkString(","))

object Holder:
  @main def hello(args: String*): Unit = println("hello: " + args.mkString(","))
  object Inner:
    @main def deep(): Unit = println("deep")

object Run:
  def main(args: Array[String]): Unit = println("run: " + args.mkString(","))
