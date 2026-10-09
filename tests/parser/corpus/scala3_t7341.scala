// Adapted from scala3 tests/run/t7341.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
object Obj {
  private var cache: Any = ()
  def returning(f: () => Unit) = ()
  def foo: Unit = {
    returning(() => cache = ())
  }

  def apply(): Any = {
    cache
  }
}

object Test {
  def main(args: Array[String]): Unit = ()
  Obj()
}
