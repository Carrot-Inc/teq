// The forms an object's reference takes in the output (its class `$$$M`, its accessor `$$$` and the
// instance `$$$i`) are among the names a local is named apart from, whatever the object is called, a
// name of dollars alone included: a local named like the accessor, a nested object's accessor or the
// instance is renamed.
object `$$`:
  def value: Int = 3
  object Inner:
    def value: Int = 4

@main def main(): Unit =
  val `$$$` = 7
  println(`$$`.value + `$$$`)
  val `$$$Inner$` = 6
  println(`$$`.Inner.value + `$$$Inner$`)
  val `$$$i` = 1
  println(`$$`.value + `$$$i`)
