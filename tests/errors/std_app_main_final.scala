// An object that extends `App` inherits `main`, which is final there: a `main` of its own is refused
// (scalac's E164).
// expect: 5:16: error: method main cannot override final member method main in trait App
object Test extends App {
  override def main(args: Array[String]): Unit = println("mine")
}
