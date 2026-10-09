// A type variable the result uses covariantly, bounded above only by another variable, is
// instantiated from below: `LowerE` of zio's `ZIO.@@` over an effect that cannot fail is
// `Nothing`, not the aspect's upper bound.
class Aspect[+LowerE, -UpperE]
class Eff[+E](val tag: String):
  def @@[LowerE >: E, UpperE >: LowerE](aspect: => Aspect[LowerE, UpperE]): Eff[LowerE] = Eff(tag + "@")
  def flatMap[E1 >: E](f: Unit => Eff[E1]): Eff[E1] = Eff(tag + ">" + f(()).tag)
  def map(f: Unit => Unit): Eff[E] = Eff(tag + "m")

val counter: Aspect[Nothing, Any] = Aspect()

def pushed: Eff[Nothing] = for
  _ <- Eff[Nothing]("u") @@ counter
  _ <- Eff[Nothing]("v")
yield ()

@main def run(): Unit =
  val r = Eff[Nothing]("u") @@ counter
  val s: Eff[Nothing] = r
  println(s.tag)
  println(pushed.tag)
