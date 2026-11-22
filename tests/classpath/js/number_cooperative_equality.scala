// jars: scala-library
// std: scala-library
// targets: js jvm
// A number beside a ScalaNumber (BigInt, BigDecimal) is equal by the ScalaNumber's `equals`, as BoxesRunTime.equals2
// answers it, whichever side the number is on: `==` through `Any`, Option's, List's and Set's `contains`. An unequal
// number or Char is not, and neither is a class of the program with ScalaNumber's method names.
class Fake(v: Int):
  def isWhole: Boolean = true
  def underlying(): AnyRef = Integer.valueOf(v)
  override def equals(o: Any): Boolean = o.isInstanceOf[Fake]
  override def hashCode: Int = v

def eqAny(a: Any, b: Any): Boolean = a == b

@main def run(args: String*): Unit =
  val k = args.length
  val numbers: List[(String, Any)] = List("Int" -> (42 + k), "Long" -> (42L + k), "Double" -> (42.0 + k))
  val bigs: List[(String, Any)] = List("BigInt" -> BigInt(42 + k), "BigDecimal" -> BigDecimal(42 + k))
  for (xn, x) <- numbers; (yn, y) <- bigs do
    println(s"$xn $yn ${eqAny(x, y)} ${eqAny(y, x)} ${Option(x).contains(y)} ${Option(y).contains(x)} ${List(x).contains(y)} ${List(y).contains(x)} ${Set(x).contains(y)} ${Set(y).contains(x)}")
  val unequal: List[Any] = List(43 + k, 43L + k, 43.0 + k, (43 + k).toChar)
  for x <- unequal; (_, y) <- bigs do println(s"${eqAny(x, y)} ${eqAny(y, x)}")
  for (_, x) <- numbers do println(s"${eqAny(x, new Fake(42))} ${eqAny(new Fake(42), x)}")
