// DottyPrimitives maps the selected Object.eq/ne declarations to reference
// comparison, including singleton results and eta-expanded method values.
object Token
class Named:
  def eq(n: Int): Boolean = n == 7

@main def run(): Unit =
  val module = valueOf[Token.type]
  println(module eq Token)
  println(module ne Token)
  val a: AnyRef = new Object
  val b: AnyRef = new Object
  val same: AnyRef => Boolean = a.eq
  val different: AnyRef => Boolean = a.ne
  println(same(a))
  println(same(b))
  println(different(a))
  println(different(b))
  println(null.asInstanceOf[AnyRef] eq null)
  println(new Named().eq(7))
