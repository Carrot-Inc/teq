// Instantiation direction of type variables: minimised, maximised, improved lower bounds.
// expect: 24:31: error: type mismatch: found Map[Nothing, Nothing], required Map[String, Int]
class Inv[T](val t: T)
class Cov[+T](val t: T)
class Con[-T]:
  def use(t: T): String = t.toString

object D:
  val c = true
  def sink[A](fn: A => Unit): A => Unit = fn
  val s1 = sink((x: Int) => ())              // A := Int from upper bound
  val s1t: Int => Unit = s1
  def emptyList[A]: List[A] = Nil
  val e1 = emptyList
  val e1t: List[Nothing] = e1               // ok: A := Nothing
  val e2: List[Int] = emptyList             // ok
  def needFn[A](fn: A => Int): Int = 1
  val n1 = needFn(_.toString.length)        // scalac: error, missing parameter type
  val n2 = needFn((x: Int) => x)
  def both[A](a: A, fn: A => Unit): A = a
  val b1 = both(1, (x: Any) => ())          // A := Int (lower bound wins, Int <: Any)
  val b1t: Int = b1
  val m0 = Map.empty
  val m0t: Map[String, Int] = m0            // scalac: error (Map[Nothing, Nothing] invariant key)
  def takesMap(m: Map[String, Int]): Int = m.size
  val m1 = takesMap(Map.empty)              // ok: expected type
  val m2 = takesMap(Map())                  // ok
  val nil = Nil
  val nilt: List[Int] = nil                 // ok: covariant
  val none = None
  val nonet: Option[Int] = none             // ok
  var xs = Nil
  // xs = List(1)                            // scalac: error (xs: Nil.type)
  var ys = List.empty[Int]
  val fold = List(1, 2).foldLeft(Nil)((acc, x) => x :: acc)   // accepted as scalac accepts it: the accumulator is improved to List[Int]
  val fold2 = List(1, 2).foldLeft(List.empty[Int])((acc, x) => x :: acc)
  val fold3 = List(1, 2).foldLeft(Map.empty[Int, Int])((acc, x) => acc + (x -> x))
  def pick[A](xs: List[A]): A = xs.head
  val p1 = pick(Nil)                        // A := Nothing
  val opt = Option.empty
  val optt: Option[Int] = opt               // ok (covariant)
  val con = Con[Int]()
  val con2: Con[Nothing] = con              // ok (contravariant)
  def mk[A]: Con[A] = Con[A]()
  val mk1 = mk
  val mk1t: Con[Int] = mk1                  // scalac: A := Nothing then Con[Nothing] not <: Con[Int] → error
  val inv1 = Inv(if c then 1 else 2L)
  val inv1t: Inv[Long] = inv1               // ok: harmonised
  val cov1 = Cov(if c then Some(1) else None)
  val cov1t: Cov[Option[Int]] = cov1        // ok
  def ret[A](x: A): List[A] = List(x)
  val r1 = ret(if c then 1 else "a")
  val r1t: List[Int | String] = r1          // ok
  val lam = (x: Int) => x
  val lamt: Int => Int = lam
  val idf = 0
  val stmt = List(1).map(x => x + 1).sum
  val stmt2: Int = List(1, 2).map(_ * 2).filter(_ > 2).headOption.getOrElse(0)
  val chain = List(Some(1), None).flatten
  val chaint: List[Int] = chain

@main def main(): Unit =
  println(D.b1)
  println(D.m1)
  println(D.stmt)
  println(D.chain)
