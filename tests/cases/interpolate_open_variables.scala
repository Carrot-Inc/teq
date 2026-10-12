// dotty's interpolation (`Inferencing.interpolateTypeVars`, `instantiateTypeVars`, `variances`): at
// the end of each tree its variables are instantiated by the variance its type and expected type
// give them, through their bounds: covariant from below (an upper bound alone gives `Nothing`),
// contravariant from above, nonvariant left open for the trees around, so a selection's expected
// singleton reaches an invariant variable; an inferred `val` forces what stays open
// (`IsFullyDefinedAccumulator`: from below where something bounds it below or nothing above).
trait In[-A]
class Box[T](val zero: T)
class Pair[A, B <: A](val b: B)
class Sink[-T]:
  def take(x: T): String = "took"
class Inv[T]
def box[T](t: T): Box[T] = new Box(t)
def pair[A, B <: A](x: B): Pair[A, B] = new Pair[A, B](x)
def mk[A](a: A): A => Boolean = _ == a
def sink[T](t: T): Sink[T] = new Sink[T]
def both[A](a: A): (A, A => Unit) = (a, _ => ())
def upper[T <: AnyVal]: List[T] = Nil
def nullable[T >: Null <: String]: List[T] = Nil
def inv[T <: AnyVal]: Inv[T] = new Inv[T]
@main def run() =
  val z: 1 = box(1).zero
  val s: "s" = box("s").zero
  val b: 1 = pair(1).b
  val p = mk(1)
  val anyP: Any => Boolean = p
  val k = sink(1)
  val anyK: Sink[Any] = k
  val q = both(1)
  val intQ: (Int, Int => Unit) = q
  val u = upper
  val nothingU: List[Nothing] = u
  val n = nullable
  val nullN: List[Null] = n
  val i = inv
  val anyValI: Inv[AnyVal] = i
  println((z, s, b, anyP("x"), anyK.take("y"), intQ._1, nothingU.size, nullN.size))
  println(Map("a" -> 1) == Map(1 -> 1))
  println(Map("a" -> 1) == Map("a" -> 1))
