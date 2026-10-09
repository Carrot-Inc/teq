// The names the emitter's own text binds, the instance a secondary constructor makes (`$t`) and
// the bindings of object accessors (`$$FAO`, numbered `$$FAO$2` for a second object of those
// capitals, `$$R`), stay apart from the locals and parameters named alike.
class C(x: Int):
  def this(`$t`: String) =
    this(`$t`.length)
    println(`$t`)

object FooAlphaOne:
  val x = 1
object FooAlphaOther:
  val x = 2

object Registry:
  def value: Int = 7

@main def main(): Unit =
  new C("abc")
  val `$$FAO$2` = 100
  println(FooAlphaOne.x + FooAlphaOne.x + FooAlphaOne.x + FooAlphaOne.x)
  println(FooAlphaOther.x + FooAlphaOther.x + FooAlphaOther.x + FooAlphaOther.x)
  println(`$$FAO$2`)
  val `$$R` = 10
  println(Registry.value + Registry.value + Registry.value + Registry.value)
  println(`$$R`)
