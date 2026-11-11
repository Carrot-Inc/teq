// A result that names a parameter through a member that is an alias in the parameter's type as the receiver
// sees it (`step.Out`, `step.Next[S]` with `Step` the receiver's class argument): a false dependency, which
// the method type's result dealiases (TermLambda#resultType), so the application's type names no parameter,
// whether the argument is a path or a call and whether it is passed or found.
trait AbstractStep:
  type Out
  type Next[A]
  def out: Out
  def next[A](a: A): Next[A]

final class Sub[I, H](val h: H):
  def buildReturning[O](f: H => O): O = f(h)

final class FirstStep[I] extends AbstractStep:
  type Out = Sub[I, Int]
  type Next[H] = Sub[I, H]
  def out: Sub[I, Int] = new Sub[I, Int](7)
  def next[A](a: A): Sub[I, A] = new Sub[I, A](a)

class Primary[I, Step <: AbstractStep]:
  final def use(step: Step): step.Out = step.out
  final def useState[S](s: => S)(using step: Step): step.Next[S] = step.next(s)

final class First[I] extends Primary[I, FirstStep[I]]

object FirstStep:
  given firstStep: [P] => FirstStep[P] = new FirstStep[P]

object Builder:
  def apply[I]: First[I] = new First[I]
  def made[P]: FirstStep[P] = new FirstStep[P]

@main def run(): Unit =
  println(Builder[String].use(Builder.made[String]).buildReturning(_ + 1))
  println(Builder[String].useState[Option[Int]](None).buildReturning(_.size))
  println(Builder[() => Int].useState[Option[Int]](Some(3))(using Builder.made[() => Int]).buildReturning[Int](o => o.size))
  val step = Builder.made[Int]
  println(Builder[Int].use(step).buildReturning(_ * 2))
