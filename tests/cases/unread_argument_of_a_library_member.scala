// An argument that a member's result does not depend on is evaluated in its place all the same:
// the message of `requireNonNull` for a value that is there, the key of a property that is not.
def loud[A](tag: String, a: A): A = { println(tag); a }

object Main {
  def main(args: Array[String]): Unit = {
    println("start")
    println(java.util.Objects.requireNonNull(loud("value", "v"), loud("message", "m")))
    println(System.getProperty(loud("key", "no.such.key.here"), loud("default", "d")))
  }
}
