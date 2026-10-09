// teq: --main app.cli.Tool
// `--main` takes the entry point's class as `java` runs it, qualified by its package: the
// object `app.cli.Tool` inherits its `main` from a trait, beside a `@main def` of the package.
package app.cli

trait Launcher:
  def name: String
  def main(args: Array[String]): Unit = println(s"launching $name")

object Tool extends Launcher:
  def name = "tool"

@main def other(): Unit = println("other")
