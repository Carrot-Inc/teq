// Context closures nested at one place (`Int ?=> String ?=> String`) are two closures, each with parameters of
// its own number (dotty's `ContextFunctionParamName.fresh()`): the products read them back named as the source's
// typing names them, so the downstream's bundles are one.
package ctx
object A:
  def f: Int ?=> String ?=> String = summon[String] + summon[Int]
  def g: Int ?=> String ?=> String = summon[String] + summon[Int]
  def h: Int ?=> (Int => String ?=> String) = i => summon[String] + summon[Int] + i
