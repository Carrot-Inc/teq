// An object's `main` is no `@main` method: nothing calls it from outside the object, and a wildcard
// import of its own package is unused.
package q
import q.*
object Main { def main(args: Array[String]): Unit = println(2) }
